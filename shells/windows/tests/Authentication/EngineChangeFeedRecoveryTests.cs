using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Authentication;

[TestClass]
public sealed class EngineChangeFeedRecoveryTests
{
    [TestMethod]
    public async Task LateSubscribeIsDisposedAndCannotRefreshAnotherAccount()
    {
        var release = new TaskCompletionSource();
        var started = new TaskCompletionSource();
        await using var engine = new FakeEngine
        {
            SubscribeBarrier = _ => { started.TrySetResult(); return release.Task; },
        };
        var changes = 0;
        await using var feed = new EngineChangeFeed(engine, ProtocolIds.Capabilities.EitmadCapabilityCustomerV1,
            Subscription.ForCustomerChangedSubscribe(new()), _ => changes++);
        var activation = feed.ActivateAsync();
        await started.Task;
        var deactivation = feed.DeactivateAsync();
        Assert.IsFalse(deactivation.IsCompleted);
        release.TrySetResult();
        await Task.WhenAll(activation, deactivation).WaitAsync(TimeSpan.FromSeconds(2));
        Assert.AreEqual(0, engine.SubscriptionCount);
        Assert.AreEqual(0, changes);
        engine.SubscribeBarrier = null;
        await feed.ActivateAsync();
        Assert.AreEqual(1, engine.SubscriptionCount);
        Assert.AreEqual(1, changes);
    }

    [TestMethod]
    public async Task QueuedOldNotificationsAreDiscardedWhileUnsubscribeRemainsPending()
    {
        var context = new QueuedContext();
        var release = new TaskCompletionSource();
        FakeSubscription? subscription = null;
        await using var engine = new FakeEngine { SubscribeHook = (_, current) => subscription = current };
        var changes = 0;
        await using var feed = new EngineChangeFeed(engine, ProtocolIds.Capabilities.EitmadCapabilityCustomerV1,
            Subscription.ForCustomerChangedSubscribe(new()), _ => changes++, refreshOnStart: false);
        var previous = SynchronizationContext.Current;
        try
        {
            SynchronizationContext.SetSynchronizationContext(context);
            await feed.ActivateAsync(); // Fake subscribe completes synchronously.
        }
        finally { SynchronizationContext.SetSynchronizationContext(previous); }
        Assert.IsNotNull(subscription);
        subscription.DisposeBarrier = () => release.Task;
        subscription.SignalResync();
        var deactivation = feed.DeactivateAsync();
        Assert.IsFalse(deactivation.IsCompleted);
        context.Drain();
        subscription.SignalResync(); context.Drain();
        Assert.AreEqual(0, changes, "Neither queued nor late notifications may reach the former account.");
        release.TrySetResult(); await deactivation.WaitAsync(TimeSpan.FromSeconds(2));
        await feed.ActivateAsync(); context.Drain();
        Assert.AreEqual(0, changes);
    }

    private sealed class QueuedContext : SynchronizationContext
    {
        private readonly Queue<Action> work = new();
        public override void Post(SendOrPostCallback callback, object? state) => work.Enqueue(() => callback(state));
        public void Drain() { while (work.TryDequeue(out var action)) action(); }
    }
}
