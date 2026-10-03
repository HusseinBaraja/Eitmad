using System.Collections.Concurrent;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Customers;

[TestClass]
public sealed class CustomerFeedTests
{
    [TestMethod]
    public async Task DeactivationDiscardsQueuedEventsBeforeAnotherActivation()
    {
        await using var engine = new FakeEngine();
        await using var client = new CustomerClient(engine);
        var context = new QueuedContext();
        var changed = new List<Guid?>();
        client.Changed += (_, id) => changed.Add(id);
        var previous = SynchronizationContext.Current;
        try
        {
            SynchronizationContext.SetSynchronizationContext(context);
            await client.ActivateAsync();
        }
        finally { SynchronizationContext.SetSynchronizationContext(previous); }

        Publish(engine, Guid.NewGuid());
        await context.WaitForPostAsync();
        await client.DeactivateAsync();
        Assert.AreEqual(0, engine.SubscriptionCount);
        await client.ActivateAsync();
        context.Drain();
        Assert.HasCount(0, changed);

        var currentCustomer = Guid.NewGuid();
        Publish(engine, currentCustomer);
        var deadline = DateTime.UtcNow.AddSeconds(3);
        while (changed.Count == 0 && DateTime.UtcNow < deadline) await Task.Delay(5);
        CollectionAssert.AreEqual(new Guid?[] { currentCustomer }, changed);
        Assert.AreEqual(1, engine.SubscriptionCount);
    }

    private static void Publish(FakeEngine engine, Guid customerId) =>
        engine.Publish(Subscription.CustomerChangedSubscribeKind, new EventEnvelope
        {
            Event = new Dictionary<string, object>
            {
                ["kind"] = Event.CustomerChangedEventKind,
                ["payload"] = new CustomerChangeNotice
                {
                    CustomerId = customerId, Scope = engine.CustomerBranch, Revision = 1,
                    ChangeId = Guid.NewGuid(), ChangedAt = 1,
                },
            },
        });

    private sealed class QueuedContext : SynchronizationContext
    {
        private readonly ConcurrentQueue<Action> actions = new();
        private readonly TaskCompletionSource posted = new(TaskCreationOptions.RunContinuationsAsynchronously);
        public override void Post(SendOrPostCallback callback, object? state)
        {
            actions.Enqueue(() => callback(state));
            posted.TrySetResult();
        }
        public Task WaitForPostAsync() => posted.Task.WaitAsync(TimeSpan.FromSeconds(3));
        public void Drain() { while (actions.TryDequeue(out var action)) action(); }
    }
}
