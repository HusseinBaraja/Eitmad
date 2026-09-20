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
    private readonly object subscriptionStateLock = new();
    private readonly HashSet<string> unsupportedStreams = [];
    private long observedGeneration = -1;
    private long sessionVersion;
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
        await sessionRefresh.WaitAsync();
        foreach (var subscription in subscriptions.Values)
        {
            await subscription.DisposeAsync();
        }
        subscriptions.Clear();
        sessionRefresh.Release();
        sessionRefresh.Dispose();
        lifetime.Dispose();
        await engine.DisposeAsync();
    }

    private void ObserveSupervision(EngineSupervisionSnapshot snapshot)
    {
        bool restore;
        bool nowConnected;
        bool invalidate;
        long version;
        lock (subscriptionStateLock)
        {
            var wasConnected = connected;
            if (snapshot.Generation != observedGeneration)
            {
                observedGeneration = snapshot.Generation;
                unsupportedStreams.Clear();
                sessionVersion++;
                connected = false;
            }
            nowConnected = snapshot.IpcHealth == EngineIpcHealthState.Connected
                && snapshot.LastLifecycle?.Ready == true;
            if (!nowConnected && connected)
            {
                sessionVersion++;
            }
            restore = nowConnected && !connected;
            invalidate = !nowConnected && wasConnected;
            connected = nowConnected;
            version = sessionVersion;
        }
        dispatcher.Invoke(() =>
        {
            viewModel.ObserveSupervision(snapshot);
            if (invalidate) viewModel.ObserveSnapshotsUnavailable();
        });
        if (restore) _ = RestoreSessionAsync(version, lifetime.Token);
    }

    private bool IsSessionCurrent(long version)
    {
        lock (subscriptionStateLock) return !disposed && connected && sessionVersion == version;
    }

    private async Task RestoreSessionAsync(long version, CancellationToken cancellationToken)
    {
        try { await sessionRefresh.WaitAsync(cancellationToken); }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { return; }
        try
        {
            if (!IsSessionCurrent(version)) return;
            foreach (var subscription in subscriptions.Values) await subscription.DisposeAsync();
            subscriptions.Clear();
            eventOrder.ResetAll();
            await EnsureSubscriptionsAsync(version, cancellationToken);
            if (!IsSessionCurrent(version)) return;
            if (await RefreshSnapshotsAsync(version, cancellationToken) && IsSessionCurrent(version))
                dispatcher.Invoke(() => { if (IsSessionCurrent(version)) viewModel.MarkSnapshotsCurrent(); });
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (EngineIpcException)
        {
            if (IsSessionCurrent(version)) dispatcher.Invoke(viewModel.ObserveSnapshotsUnavailable);
        }
        finally
        {
            sessionRefresh.Release();
        }
    }

    private async Task EnsureSubscriptionsAsync(long version, CancellationToken cancellationToken)
    {
        var desired = new (string Stream, string Capability, Subscription Contract)[]
        {
            ("configuration", ProtocolIds.Capabilities.EitmadCapabilityConfigV1, Subscription.ForConfigChangedSubscribe(new ConfigurationChanges())),
            ("sync", ProtocolIds.Capabilities.EitmadCapabilitySyncV1, Subscription.ForSyncStatusSubscribe(new SyncStatusChanges())),
            ("update", ProtocolIds.Capabilities.EitmadCapabilityUpdateV1, Subscription.ForUpdateStateSubscribe(new UpdateStateChanges())),
            ("reference-markers", ProtocolIds.Capabilities.EitmadCapabilityReferenceMarkerV1, Subscription.ForReferenceMarkerChangedSubscribe(new ReferenceMarkerChanges())),
        };
        foreach (var item in desired)
        {
            bool unsupported;
            lock (subscriptionStateLock)
            {
                unsupported = unsupportedStreams.Contains(item.Stream);
            }
            if (!engine.SupportsCapability(item.Capability) || unsupported)
            {
                continue;
            }
            try
            {
                await EnsureSubscriptionAsync(item.Stream, item.Contract, version, cancellationToken);
            }
            catch (EngineIpcException error) when (error.Kind == EngineIpcFailureKind.SubscriptionUnsupported)
            {
                lock (subscriptionStateLock)
                {
                    if (sessionVersion == version) unsupportedStreams.Add(item.Stream);
                }
            }
        }
    }

    private async Task EnsureSubscriptionAsync(
        string stream,
        Subscription contract,
        long version,
        CancellationToken cancellationToken)
    {
        if (subscriptions.ContainsKey(stream))
        {
            return;
        }

        var subscription = await engine.SubscribeAsync(contract, cancellationToken);
        if (!IsSessionCurrent(version))
        {
            await subscription.DisposeAsync();
            return;
        }
        subscription.ResyncRequired += () => _ = ResynchronizeAsync(stream, version, lifetime.Token);
        subscriptions.Add(stream, subscription);
        _ = PumpAsync(stream, subscription, version, lifetime.Token);
    }

    private async Task PumpAsync(
        string stream,
        IEngineSubscription subscription,
        long version,
        CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in subscription.ReadAllAsync(cancellationToken))
            {
                await sessionRefresh.WaitAsync(cancellationToken);
                try
                {
                    if (!IsSessionCurrent(version) || !eventOrder.TryAccept(stream, delivered)) continue;
                    var contract = DecodeEvent(delivered);
                    dispatcher.Invoke(() => ApplyEvent(contract, delivered.OccurredAt));
                    subscription.Acknowledge(delivered);
                    if (contract.AsReferenceMarkerChangedEvent() is not null)
                        await RefreshReferenceMarkersAsync(version, cancellationToken);
                }
                finally
                {
                    sessionRefresh.Release();
                }
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (EngineIpcException)
        {
            if (IsSessionCurrent(version)) dispatcher.Invoke(viewModel.ObserveSnapshotsUnavailable);
        }
    }

    private async Task ResynchronizeAsync(string stream, long version, CancellationToken cancellationToken)
    {
        try { await sessionRefresh.WaitAsync(cancellationToken); }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { return; }
        try
        {
            if (!IsSessionCurrent(version)) return;
            eventOrder.Reset(stream);
            dispatcher.Invoke(() => viewModel.BeginResynchronization(stream));
            if (stream is "configuration" or "sync" or "update" or "reference-markers")
            {
                if (await RefreshSnapshotsAsync(version, cancellationToken) && IsSessionCurrent(version))
                    dispatcher.Invoke(() => { if (IsSessionCurrent(version)) viewModel.MarkSnapshotsCurrent(); });
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (EngineIpcException)
        {
            if (IsSessionCurrent(version)) dispatcher.Invoke(viewModel.ObserveSnapshotsUnavailable);
        }
        finally { sessionRefresh.Release(); }
    }

    private async Task<bool> RefreshSnapshotsAsync(long version, CancellationToken cancellationToken)
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
        if (!IsSessionCurrent(version)) return false;
        var configuration = await configurationTask;
        var referenceMarkers = await referenceMarkersTask;
        var current = configuration.Outcome.Status == CommandOutcomeStatus.Succeeded
            && configuration.Outcome.Payload.AsConfiguration() is not null
            && referenceMarkers.Outcome.Status == CommandOutcomeStatus.Succeeded
            && referenceMarkers.Outcome.Payload.AsReferenceMarkers() is not null
            && (syncTask is null || syncTask.Result.Outcome.Status == CommandOutcomeStatus.Succeeded
                && syncTask.Result.Outcome.Payload.AsSyncStatus() is not null)
            && (updateTask is null || updateTask.Result.Outcome.Status == CommandOutcomeStatus.Succeeded
                && updateTask.Result.Outcome.Payload.AsUpdateState() is not null);
        dispatcher.Invoke(() =>
        {
            if (!IsSessionCurrent(version)) return;
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
        return current;
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

    private async Task RefreshReferenceMarkersAsync(long version, CancellationToken cancellationToken)
    {
        var response = await engine.QueryAsync(
            Query.ForReferenceMarkerList(new ListReferenceMarkers { Limit = 20 }),
            cancellationToken);
        if (response.Outcome.Status == CommandOutcomeStatus.Succeeded
            && response.Outcome.Payload.AsReferenceMarkers() is { } page)
        {
            dispatcher.Invoke(() => { if (IsSessionCurrent(version)) viewModel.ObserveReferenceMarkers(page); });
        }
        else
        {
            dispatcher.Invoke(() => { if (IsSessionCurrent(version)) viewModel.ObserveReferenceMarkersUnavailable(); });
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
