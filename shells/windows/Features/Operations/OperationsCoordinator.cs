using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;

namespace Eitmad.WindowsShell.Features.Operations;

public interface IShellLifetimeCoordinator : IAsyncDisposable
{
    Task StopAsync(CancellationToken cancellationToken = default);
}

public sealed class OperationsCoordinator : IShellLifetimeCoordinator
{
    private readonly IEngineShellBridge engine;
    private readonly OperationsViewModel viewModel;
    private readonly IShellDispatcher dispatcher;
    private readonly EventOrderGate eventOrder = new();
    private readonly Dictionary<string, IEngineSubscription> subscriptions = [];
    private readonly CancellationTokenSource lifetime = new();
    private readonly SemaphoreSlim sessionRefresh = new(1, 1);
    private EngineLaunchRequest? launchRequest;
    private bool connected;
    private bool disposed;

    public OperationsCoordinator(
        IEngineShellBridge engine,
        OperationsViewModel viewModel,
        IShellDispatcher dispatcher)
    {
        this.engine = engine;
        this.viewModel = viewModel;
        this.dispatcher = dispatcher;
        viewModel.SubmitConfigurationPatch = SubmitConfigurationPatchAsync;
        viewModel.RestartEngine = RestartAsync;
    }

    public async Task StartAsync(EngineLaunchRequest request, CancellationToken cancellationToken = default)
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        launchRequest = request;
        engine.StateChanged += ObserveSupervision;
        await engine.StartAsync(request, cancellationToken);
        ObserveSupervision(engine.Snapshot);
    }

    public Task StopAsync(CancellationToken cancellationToken = default) => engine.StopAsync(cancellationToken);

    public async ValueTask DisposeAsync()
    {
        if (disposed)
        {
            return;
        }

        disposed = true;
        lifetime.Cancel();
        engine.StateChanged -= ObserveSupervision;
        foreach (var subscription in subscriptions.Values)
        {
            await subscription.DisposeAsync();
        }
        subscriptions.Clear();
        sessionRefresh.Dispose();
        lifetime.Dispose();
        await engine.DisposeAsync();
    }

    private void ObserveSupervision(EngineSupervisionSnapshot snapshot)
    {
        dispatcher.Invoke(() => viewModel.ObserveSupervision(snapshot));
        var nowConnected = snapshot.IpcHealth == EngineIpcHealthState.Connected
            && snapshot.LastLifecycle?.Ready == true;
        if (!nowConnected)
        {
            connected = false;
            return;
        }

        if (!connected)
        {
            connected = true;
            _ = RestoreSessionAsync(lifetime.Token);
        }
    }

    private async Task RestoreSessionAsync(CancellationToken cancellationToken)
    {
        await sessionRefresh.WaitAsync(cancellationToken);
        try
        {
            await RefreshSnapshotsAsync(cancellationToken);
            dispatcher.Invoke(viewModel.MarkSnapshotsCurrent);
            await EnsureSubscriptionsAsync(cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (EngineIpcException)
        {
            connected = false;
        }
        finally
        {
            sessionRefresh.Release();
        }
    }

    private async Task EnsureSubscriptionsAsync(CancellationToken cancellationToken)
    {
        var desired = new (string Stream, Subscription Contract)[]
        {
            ("configuration", Subscription.ForConfigChangedSubscribe(new ConfigurationChanges())),
            ("sync", Subscription.ForSyncStatusSubscribe(new SyncStatusChanges())),
            ("update", Subscription.ForUpdateStateSubscribe(new UpdateStateChanges())),
            ("jobs", Subscription.ForBackgroundJobStatusSubscribe(new BackgroundJobChanges())),
            ("notifications", Subscription.ForNotificationSubscribe(new Notifications())),
            ("errors", Subscription.ForErrorSubscribe(new Errors())),
        };
        foreach (var item in desired)
        {
            try
            {
                await EnsureSubscriptionAsync(item.Stream, item.Contract, cancellationToken);
            }
            catch (EngineIpcException error) when (error.Kind == EngineIpcFailureKind.SubscriptionUnsupported)
            {
                // Snapshot-backed panels remain available when an older engine omits an optional stream.
            }
        }
    }

    private async Task EnsureSubscriptionAsync(
        string stream,
        Subscription contract,
        CancellationToken cancellationToken)
    {
        if (subscriptions.ContainsKey(stream))
        {
            return;
        }

        var subscription = await engine.SubscribeAsync(contract, cancellationToken);
        subscription.ResyncRequired += () => _ = ResynchronizeAsync(stream, lifetime.Token);
        subscriptions.Add(stream, subscription);
        _ = PumpAsync(stream, subscription, lifetime.Token);
    }

    private async Task PumpAsync(
        string stream,
        IEngineSubscription subscription,
        CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in subscription.ReadAllAsync(cancellationToken))
            {
                if (!eventOrder.TryAccept(stream, delivered))
                {
                    continue;
                }

                var contract = DecodeEvent(delivered);
                dispatcher.Invoke(() => ApplyEvent(contract, delivered.OccurredAt));
                subscription.Acknowledge(delivered);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (EngineIpcException)
        {
            connected = false;
        }
    }

    private async Task ResynchronizeAsync(string stream, CancellationToken cancellationToken)
    {
        eventOrder.Reset(stream);
        dispatcher.Invoke(() => viewModel.BeginResynchronization(stream));
        if (stream is "configuration" or "sync" or "update")
        {
            await RefreshSnapshotsAsync(cancellationToken);
            dispatcher.Invoke(viewModel.MarkSnapshotsCurrent);
        }
    }

    private async Task RefreshSnapshotsAsync(CancellationToken cancellationToken)
    {
        var configuration = await engine.QueryAsync(Query.ForConfigGet(new GetConfiguration()), cancellationToken);
        var sync = await engine.QueryAsync(Query.ForSyncGetStatus(new GetSyncStatus()), cancellationToken);
        var update = await engine.QueryAsync(Query.ForUpdateGetState(new GetUpdateState()), cancellationToken);
        dispatcher.Invoke(() =>
        {
            if (configuration.Outcome.Status == CommandOutcomeStatus.Succeeded
                && configuration.Outcome.Payload.AsConfiguration() is { } configSnapshot)
            {
                viewModel.ObserveConfiguration(configSnapshot);
            }
            if (sync.Outcome.Status == CommandOutcomeStatus.Succeeded
                && sync.Outcome.Payload.AsSyncStatus() is { } syncStatus)
            {
                viewModel.ObserveSync(syncStatus);
            }
            else
            {
                viewModel.ObserveSyncUnavailable(sync.Outcome.Payload.Code);
            }
            if (update.Outcome.Status == CommandOutcomeStatus.Succeeded
                && update.Outcome.Payload.AsUpdateState() is { } updateState)
            {
                viewModel.ObserveUpdate(updateState);
            }
            else
            {
                viewModel.ObserveUpdateUnavailable(update.Outcome.Payload.Code);
            }
        });
    }

    private async Task SubmitConfigurationPatchAsync(UpdateConfiguration patch, Guid idempotencyKey)
    {
        var response = await engine.SubmitConfigurationPatchAsync(patch, idempotencyKey, lifetime.Token);
        if (response.Outcome.Status == CommandOutcomeStatus.Failed)
        {
            throw new InvalidOperationException("The Rust engine rejected the typed configuration patch.");
        }
        await RefreshConfigurationAsync(lifetime.Token);
    }

    private async Task RefreshConfigurationAsync(CancellationToken cancellationToken)
    {
        var response = await engine.QueryAsync(Query.ForConfigGet(new GetConfiguration()), cancellationToken);
        if (response.Outcome.Status == CommandOutcomeStatus.Succeeded
            && response.Outcome.Payload.AsConfiguration() is { } snapshot)
        {
            dispatcher.Invoke(() => viewModel.ObserveConfiguration(snapshot));
        }
    }

    private async Task RestartAsync()
    {
        if (launchRequest is null)
        {
            return;
        }

        await engine.StopAsync(lifetime.Token);
        connected = false;
        eventOrder.ResetAll();
        await engine.StartAsync(launchRequest, lifetime.Token);
    }

    private static Event DecodeEvent(EventEnvelope delivered)
    {
        return EngineContractCodec.DecodeEvent(delivered);
    }

    private void ApplyEvent(Event contract, long occurredAt)
    {
        if (contract.AsConfigChangedEvent() is { } configuration) viewModel.ObserveConfiguration(configuration, occurredAt);
        else if (contract.AsSyncStatusEvent() is { } sync) viewModel.ObserveSync(sync, occurredAt);
        else if (contract.AsUpdateStateEvent() is { } update) viewModel.ObserveUpdate(update, occurredAt);
        else if (contract.AsBackgroundJobStatusEvent() is { } job) viewModel.ObserveJob(job, occurredAt);
        else if (contract.AsNotificationEvent() is { } notification) viewModel.ObserveNotification(notification, occurredAt);
        else if (contract.AsErrorEvent() is { } error) viewModel.ObserveError(error, occurredAt);
    }
}
