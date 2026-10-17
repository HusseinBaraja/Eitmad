using Eitmad.Contracts;
using Eitmad.Platform.Windows.LocalIpc;
using Eitmad.Platform.Windows.ProcessSupervision;

namespace Eitmad.Platform.Windows.Shell;

// Owns a feature's event reader. The supervisor owns transport reattachment.
public sealed class EngineChangeFeed(
    IEngineShellBridge engine,
    string capability,
    Subscription contract,
    Action<Event?> changed,
    Action? invalidated = null,
    bool refreshOnStart = true,
    bool notifyUnavailable = false) : IAsyncDisposable
{
    private readonly SemaphoreSlim gate = new(1, 1);
    private CancellationTokenSource? lifetime;
    private IEngineSubscription? subscription;
    private Action? subscriptionRefresh;
    private SynchronizationContext? context;
    private bool ready;
    private long generation;
    private bool disposed;

    public async Task ActivateAsync(CancellationToken cancellationToken = default)
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        await gate.WaitAsync(cancellationToken);
        CancellationTokenSource owner;
        try
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            if (lifetime is not null) return;
            context = SynchronizationContext.Current;
            owner = lifetime = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            ready = IsReady(engine.Snapshot);
            generation = engine.Snapshot.Generation;
            engine.StateChanged += ObserveEngine;
        }
        finally { gate.Release(); }
        await SubscribeAsync(owner, owner.Token);
    }

    private void ObserveEngine(EngineSupervisionSnapshot snapshot)
    {
        var owner = lifetime;
        if (owner is null || owner.IsCancellationRequested || disposed) return;
        var wasReady = ready;
        ready = IsReady(snapshot);
        if (!ready)
        {
            if (wasReady && notifyUnavailable) SignalRefresh();
            return;
        }
        if (subscription is null) _ = SubscribeAsync(owner, owner.Token);
        else if (!wasReady || generation != snapshot.Generation) SignalRefresh();
        generation = snapshot.Generation;
    }

    private static bool IsReady(EngineSupervisionSnapshot snapshot) =>
        snapshot.IpcHealth == EngineIpcHealthState.Connected && snapshot.LastLifecycle?.Ready == true;

    private async Task SubscribeAsync(CancellationTokenSource owner, CancellationToken cancellationToken)
    {
        if (!engine.SupportsCapability(capability)) return;
        try
        {
            await gate.WaitAsync(cancellationToken);
            try
            {
                if (disposed || !ReferenceEquals(lifetime, owner) || subscription is not null) return;
                var current = await engine.SubscribeAsync(contract, cancellationToken);
                if (!ReferenceEquals(lifetime, owner) || cancellationToken.IsCancellationRequested)
                {
                    await current.DisposeAsync();
                    return;
                }
                subscription = current;
                subscriptionRefresh = () => Post(() => changed(null), cancellationToken, owner);
                current.ResyncRequired += subscriptionRefresh;
                _ = PumpAsync(current, owner, cancellationToken);
                if (refreshOnStart) Post(() => changed(null), cancellationToken, owner);
            }
            finally { gate.Release(); }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { }
        catch (Exception error) when (error is EngineIpcException or IOException or ObjectDisposedException)
        { Post(() => changed(null), cancellationToken, owner); }
    }

    private async Task PumpAsync(IEngineSubscription current, CancellationTokenSource owner, CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in current.ReadAllAsync(cancellationToken))
            {
                var notice = EngineContractCodec.DecodeEvent(delivered);
                Post(() => changed(notice), cancellationToken, owner);
                current.Acknowledge(delivered);
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { return; }
        catch (ObjectDisposedException) when (cancellationToken.IsCancellationRequested) { return; }
        catch (Exception error) when (error is EngineIpcException or IOException or InvalidDataException)
        {
            if (cancellationToken.IsCancellationRequested) return;
            try { await gate.WaitAsync(cancellationToken); }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { return; }
            try
            {
                if (!ReferenceEquals(subscription, current) || !ReferenceEquals(lifetime, owner)
                    || cancellationToken.IsCancellationRequested) return;
                if (error is EngineIpcException { Kind: EngineIpcFailureKind.SessionChanged })
                    Post(() => invalidated?.Invoke(), cancellationToken, owner);
                await DropAsync();
            }
            finally { gate.Release(); }
            Post(() => changed(null), cancellationToken, owner);
            foreach (var delay in new[] { 0, 250, 500, 1_000, 2_000 })
            {
                if (cancellationToken.IsCancellationRequested) return;
                try { await Task.Delay(delay, cancellationToken); }
                catch (OperationCanceledException) { return; }
                await SubscribeAsync(owner, cancellationToken);
                if (subscription is not null) return;
            }
        }
    }

    private void SignalRefresh() => Post(() => changed(null));

    private void Post(Action action, CancellationToken cancellationToken = default, CancellationTokenSource? owner = null)
    {
        var observedLifetime = owner ?? lifetime;
        void Notify()
        {
            if (!disposed && observedLifetime is not null && ReferenceEquals(lifetime, observedLifetime)
                && !observedLifetime.IsCancellationRequested && !cancellationToken.IsCancellationRequested) action();
        }
        if (context is null) Notify();
        else context.Post(_ => Notify(), null);
    }

    private async Task DropAsync()
    {
        if (subscription is not { } current) return;
        subscription = null;
        current.ResyncRequired -= subscriptionRefresh;
        subscriptionRefresh = null;
        await current.DisposeAsync();
    }

    public async Task DeactivateAsync()
    {
        // Stop delivery and an in-flight subscribe before waiting for its gate or IPC cleanup.
        var owner = Interlocked.Exchange(ref lifetime, null);
        engine.StateChanged -= ObserveEngine;
        owner?.Cancel();
        await gate.WaitAsync();
        try
        {
            await DropAsync();
        }
        finally { gate.Release(); owner?.Dispose(); }
    }

    public async ValueTask DisposeAsync()
    {
        if (disposed) return;
        disposed = true;
        await DeactivateAsync();
    }
}
