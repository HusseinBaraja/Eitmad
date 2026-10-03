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
    private CancellationToken lifetimeToken;
    private IEngineSubscription? subscription;
    private SynchronizationContext? context;
    private bool ready;
    private long generation;
    private bool disposed;

    public async Task ActivateAsync(CancellationToken cancellationToken = default)
    {
        ObjectDisposedException.ThrowIf(disposed, this);
        await gate.WaitAsync(cancellationToken);
        try
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            if (lifetime is not null) return;
            context = SynchronizationContext.Current;
            lifetime = new CancellationTokenSource();
            lifetimeToken = lifetime.Token;
            ready = IsReady(engine.Snapshot);
            generation = engine.Snapshot.Generation;
            engine.StateChanged += ObserveEngine;
        }
        finally { gate.Release(); }
        await SubscribeAsync(cancellationToken);
    }

    private void ObserveEngine(EngineSupervisionSnapshot snapshot)
    {
        if (lifetime is null || disposed) return;
        var wasReady = ready;
        ready = IsReady(snapshot);
        if (!ready)
        {
            if (wasReady && notifyUnavailable) SignalRefresh();
            return;
        }
        if (subscription is null) _ = SubscribeAsync(lifetimeToken);
        else if (!wasReady || generation != snapshot.Generation) SignalRefresh();
        generation = snapshot.Generation;
    }

    private static bool IsReady(EngineSupervisionSnapshot snapshot) =>
        snapshot.IpcHealth == EngineIpcHealthState.Connected && snapshot.LastLifecycle?.Ready == true;

    private async Task SubscribeAsync(CancellationToken cancellationToken)
    {
        if (!engine.SupportsCapability(capability)) return;
        try
        {
            await gate.WaitAsync(cancellationToken);
            try
            {
                if (disposed || lifetime is null || subscription is not null) return;
                var current = await engine.SubscribeAsync(contract, cancellationToken);
                subscription = current;
                current.ResyncRequired += SignalRefresh;
                _ = PumpAsync(current, lifetime.Token);
                if (refreshOnStart) SignalRefresh();
            }
            finally { gate.Release(); }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested) { }
        catch (Exception error) when (error is EngineIpcException or IOException or ObjectDisposedException)
        { SignalRefresh(); }
    }

    private async Task PumpAsync(IEngineSubscription current, CancellationToken cancellationToken)
    {
        try
        {
            await foreach (var delivered in current.ReadAllAsync(cancellationToken))
            {
                var notice = EngineContractCodec.DecodeEvent(delivered);
                Post(() => changed(notice), cancellationToken);
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
                if (!ReferenceEquals(subscription, current)) return;
                if (error is EngineIpcException { Kind: EngineIpcFailureKind.SessionChanged }) invalidated?.Invoke();
                await DropAsync();
            }
            finally { gate.Release(); }
            SignalRefresh();
            foreach (var delay in new[] { 0, 250, 500, 1_000, 2_000 })
            {
                if (cancellationToken.IsCancellationRequested) return;
                try { await Task.Delay(delay, cancellationToken); }
                catch (OperationCanceledException) { return; }
                await SubscribeAsync(cancellationToken);
                if (subscription is not null) return;
            }
        }
    }

    private void SignalRefresh() => Post(() => changed(null));

    private void Post(Action action, CancellationToken cancellationToken = default)
    {
        var observedLifetime = lifetime;
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
        current.ResyncRequired -= SignalRefresh;
        await current.DisposeAsync();
    }

    public async Task DeactivateAsync()
    {
        await gate.WaitAsync();
        try
        {
            engine.StateChanged -= ObserveEngine;
            lifetime?.Cancel();
            lifetime?.Dispose();
            lifetime = null;
            await DropAsync();
        }
        finally { gate.Release(); }
    }

    public async ValueTask DisposeAsync()
    {
        if (disposed) return;
        disposed = true;
        await DeactivateAsync();
    }
}
