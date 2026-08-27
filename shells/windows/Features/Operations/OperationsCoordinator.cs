using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;
using Eitmad.Platform.Windows.Shell;

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
    private readonly HashSet<string> unsupportedStreams = [];
    private long observedGeneration = -1;
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
        viewModel.SubmitReferenceMarker = SubmitReferenceMarkerAsync;
        viewModel.RestartEngine = RestartAsync;
    }

    public async Task StartAsync(CancellationToken cancellationToken = default)
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        engine.StateChanged += ObserveSupervision;
        await engine.StartAsync(cancellationToken);
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
        if (snapshot.Generation != observedGeneration)
        {
            observedGeneration = snapshot.Generation;
            unsupportedStreams.Clear();
        }
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
        var desired = new (string Stream, string Capability, Subscription Contract)[]
        {
            ("configuration", ProtocolIds.Capabilities.EitmadCapabilityConfigV1, Subscription.ForConfigChangedSubscribe(new ConfigurationChanges())),
            ("sync", ProtocolIds.Capabilities.EitmadCapabilitySyncV1, Subscription.ForSyncStatusSubscribe(new SyncStatusChanges())),
            ("update", ProtocolIds.Capabilities.EitmadCapabilityUpdateV1, Subscription.ForUpdateStateSubscribe(new UpdateStateChanges())),
            ("reference-markers", ProtocolIds.Capabilities.EitmadCapabilityReferenceMarkerV1, Subscription.ForReferenceMarkerChangedSubscribe(new ReferenceMarkerChanges())),
        };
        foreach (var item in desired.Where(item =>
                     engine.SupportsCapability(item.Capability)
                     && !unsupportedStreams.Contains(item.Stream)))
        {
            try
            {
                await EnsureSubscriptionAsync(item.Stream, item.Contract, cancellationToken);
            }
            catch (EngineIpcException error) when (error.Kind == EngineIpcFailureKind.SubscriptionUnsupported)
            {
                unsupportedStreams.Add(item.Stream);
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
                if (contract.AsReferenceMarkerChangedEvent() is not null)
                {
                    await RefreshReferenceMarkersAsync(cancellationToken);
                }
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
        try
        {
            if (stream is "configuration" or "sync" or "update" or "reference-markers")
            {
                await RefreshSnapshotsAsync(cancellationToken);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return;
        }
        catch (EngineIpcException)
        {
            connected = false;
        }

        dispatcher.Invoke(viewModel.MarkSnapshotsCurrent);
    }

    private async Task RefreshSnapshotsAsync(CancellationToken cancellationToken)
    {
        var configurationTask = engine.QueryAsync(Query.ForConfigGet(new GetConfiguration()), cancellationToken);
        var syncTask = engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilitySyncV1)
            ? engine.QueryAsync(Query.ForSyncGetStatus(new GetSyncStatus()), cancellationToken)
            : null;
        var updateTask = engine.SupportsCapability(ProtocolIds.Capabilities.EitmadCapabilityUpdateV1)
            ? engine.QueryAsync(Query.ForUpdateGetState(new GetUpdateState()), cancellationToken)
            : null;
        var referenceMarkersTask = engine.QueryAsync(
            Query.ForReferenceMarkerList(new ListReferenceMarkers { Limit = 20 }),
            cancellationToken);
        var pending = new List<Task> { configurationTask, referenceMarkersTask };
        if (syncTask is not null) pending.Add(syncTask);
        if (updateTask is not null) pending.Add(updateTask);
        await Task.WhenAll(pending);
        var configuration = await configurationTask;
        var referenceMarkers = await referenceMarkersTask;
        dispatcher.Invoke(() =>
        {
            if (configuration.Outcome.Status == CommandOutcomeStatus.Succeeded
                && configuration.Outcome.Payload.AsConfiguration() is { } configSnapshot)
            {
                viewModel.ObserveConfiguration(configSnapshot);
            }
            else
            {
                viewModel.ObserveConfigurationUnavailable();
            }
            if (syncTask is not null
                && syncTask.Result.Outcome.Status == CommandOutcomeStatus.Succeeded
                && syncTask.Result.Outcome.Payload.AsSyncStatus() is { } syncStatus)
            {
                viewModel.ObserveSync(syncStatus);
            }
            else
            {
                viewModel.ObserveSyncUnavailable(
                    syncTask?.Result.Outcome.Payload.Code
                    ?? ProtocolIds.ErrorCodes.EitmadErrorIpcSubscriptionUnsupportedV1);
            }
            if (updateTask is not null
                && updateTask.Result.Outcome.Status == CommandOutcomeStatus.Succeeded
                && updateTask.Result.Outcome.Payload.AsUpdateState() is { } updateState)
            {
                viewModel.ObserveUpdate(updateState);
            }
            else
            {
                viewModel.ObserveUpdateUnavailable(
                    updateTask?.Result.Outcome.Payload.Code
                    ?? ProtocolIds.ErrorCodes.EitmadErrorIpcSubscriptionUnsupportedV1);
            }
            if (referenceMarkers.Outcome.Status == CommandOutcomeStatus.Succeeded
                && referenceMarkers.Outcome.Payload.AsReferenceMarkers() is { } page)
            {
                viewModel.ObserveReferenceMarkers(page);
            }
            else
            {
                viewModel.ObserveReferenceMarkersUnavailable();
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
        if (response.Outcome.Payload.Kind == PurpleKind.ConfigurationUpdated
            && response.Outcome.Payload.Payload is { } payload)
        {
            dispatcher.Invoke(() => viewModel.ObserveConfiguration(new ConfigSnapshot
            {
                SchemaVersion = payload.SchemaVersion ?? 0,
                Revision = payload.Revision ?? 0,
                Scope = payload.Scope,
                Entries = payload.Entries ?? [],
            }));
        }
    }

    private async Task SubmitReferenceMarkerAsync(UpsertReferenceMarker marker, Guid idempotencyKey)
    {
        var response = await engine.SubmitReferenceMarkerAsync(marker, idempotencyKey, lifetime.Token);
        if (response.Outcome.Status == CommandOutcomeStatus.Failed)
        {
            throw new InvalidOperationException("The Rust engine rejected the typed reference marker.");
        }
        if (response.Outcome.Payload.Kind == PurpleKind.ReferenceMarkerUpserted
            && response.Outcome.Payload.Payload is { } payload
            && payload.Id is { } id
            && payload.Revision is { } revision
            && payload.UpdatedAt is { } updatedAt
            && payload.SyncState is { } syncState)
        {
            dispatcher.Invoke(() => viewModel.ObserveReferenceMarker(new ReferenceMarker
            {
                Id = id,
                Label = payload.Label,
                Revision = revision,
                Scope = payload.Scope,
                SyncState = syncState,
                UpdatedAt = updatedAt,
            }));
        }
    }

    private async Task RefreshReferenceMarkersAsync(CancellationToken cancellationToken)
    {
        var response = await engine.QueryAsync(
            Query.ForReferenceMarkerList(new ListReferenceMarkers { Limit = 20 }),
            cancellationToken);
        if (response.Outcome.Status == CommandOutcomeStatus.Succeeded
            && response.Outcome.Payload.AsReferenceMarkers() is { } page)
        {
            dispatcher.Invoke(() => viewModel.ObserveReferenceMarkers(page));
        }
        else
        {
            dispatcher.Invoke(viewModel.ObserveReferenceMarkersUnavailable);
        }
    }

    private async Task RestartAsync()
    {
        await engine.StopAsync(lifetime.Token);
        connected = false;
        eventOrder.ResetAll();
        await engine.StartAsync(lifetime.Token);
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
        else if (contract.AsReferenceMarkerChangedEvent() is { } marker) viewModel.ObserveReferenceMarkerChanged(marker);
    }
}
