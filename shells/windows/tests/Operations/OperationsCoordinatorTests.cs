using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Operations;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Operations;

[TestClass]
public sealed class OperationsCoordinatorTests
{
    [TestMethod]
    public async Task SaveReferenceMarkerUpdatesOnlyReturnedItem()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        var otherMarkerId = Guid.Parse("9c9bc2bc-842c-57f6-a37b-4c6c3f9f7421");
        model.ObserveReferenceMarkers(new ReferenceMarkerPage
        {
            Items =
            [
                new ReferenceMarker
                {
                    Id = Guid.Parse("8b8ab1ab-731b-46f5-926a-3b5b2f8f6310"), Label = "مرجع قديم", Revision = 1,
                    Scope = TestData.Scope(), SyncState = ReferenceMarkerSyncState.Confirmed, UpdatedAt = 1_800_000_000_000,
                },
                new ReferenceMarker
                {
                    Id = otherMarkerId, Label = "مرجع محفوظ", Revision = 3,
                    Scope = TestData.Scope(), SyncState = ReferenceMarkerSyncState.Confirmed, UpdatedAt = 1_800_000_000_000,
                },
            ],
        });
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        model.ReferenceMarkerLabel = "مرجع محدّث";

        model.SaveReferenceMarkerCommand.Execute(null);

        await TestData.Eventually(() => model.ReferenceMarkers.Any(marker => marker.Label == "مرجع محدّث"));
        Assert.HasCount(2, model.ReferenceMarkers);
        Assert.AreEqual("مرجع محفوظ", model.ReferenceMarkers.Single(marker => marker.Id == otherMarkerId).Label);
    }

    [TestMethod]
    public void EventOrderingRejectsDuplicatesAndStaleSequences()
    {
        var gate = new EventOrderGate();
        var subscription = Guid.NewGuid();

        Assert.IsTrue(gate.TryAccept("sync", TestData.Event(subscription, 3)));
        Assert.IsFalse(gate.TryAccept("sync", TestData.Event(subscription, 3)));
        Assert.IsFalse(gate.TryAccept("sync", TestData.Event(subscription, 2)));
        Assert.IsTrue(gate.TryAccept("sync", TestData.Event(subscription, 4)));
        Assert.IsTrue(gate.TryAccept("sync", TestData.Event(Guid.NewGuid(), 1)));
    }

    [TestMethod]
    public void EventOrderingSerializesConcurrentDelivery()
    {
        var gate = new EventOrderGate();
        var subscription = Guid.NewGuid();

        Parallel.For(1, 1_001, sequence => gate.TryAccept("sync", TestData.Event(subscription, sequence)));

        Assert.IsFalse(gate.TryAccept("sync", TestData.Event(subscription, 1_000)));
    }

    [TestMethod]
    public async Task ReconnectionRefreshesWithoutDuplicateSubscriptions()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();

        engine.Connect();
        await TestData.Eventually(() => engine.QueryCount >= 4 && engine.SubscriptionCount == 4);
        engine.Disconnect();
        engine.Connect();
        await TestData.Eventually(() => engine.QueryCount >= 8);

        Assert.AreEqual(4, engine.SubscriptionCount);
        Assert.IsFalse(model.ShowConnectionBanner);
    }

    [TestMethod]
    public async Task ResyncRefreshesRustSnapshots()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();
        engine.Connect();
        await TestData.Eventually(() => engine.QueryCount >= 4);

        engine.SignalResync(ProtocolIds.Subscriptions.EitmadSyncStatusSubscribeV1);

        await TestData.Eventually(() => engine.QueryCount >= 8);
        Assert.IsFalse(model.ShowConnectionBanner);
    }

    [TestMethod]
    public async Task UnsupportedCapabilitiesAvoidRequestTraffic()
    {
        var engine = new FakeEngine
        {
            SupportedCapabilities = new HashSet<string>
            {
                ProtocolIds.Capabilities.EitmadCapabilityConfigV1,
                ProtocolIds.Capabilities.EitmadCapabilityReferenceMarkerV1,
            },
        };
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();

        engine.Connect();

        await TestData.Eventually(() => engine.QueryCount == 2 && engine.SubscriptionCount == 2);
        Assert.IsFalse(engine.WasQueried(Query.SyncGetStatusKind));
        Assert.IsFalse(engine.WasQueried(Query.UpdateGetStateKind));
        Assert.AreEqual("غير متاحة", model.SyncCard.Value);
        Assert.AreEqual("غير متاحة", model.UpdateCard.Value);
    }

    [TestMethod]
    public async Task ConfigurationQueryFailureClearsStaleState()
    {
        var engine = new FakeEngine { FailConfigurationQuery = true };
        var model = new OperationsViewModel();
        model.ObserveConfiguration(TestData.Configuration(4, "en-US"));
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();

        engine.Connect();

        await TestData.Eventually(() => engine.QueryCount >= 4);
        Assert.AreEqual(-1L, model.ConfigRevision);
        Assert.IsEmpty(model.Configuration);
        Assert.AreEqual("غير متاح", model.ConfigurationRevisionLabel);
    }

    [TestMethod]
    public async Task ShutdownStopsEngineCleanly()
    {
        var engine = new FakeEngine();
        var model = new OperationsViewModel();
        await using var coordinator = new OperationsCoordinator(engine, model, new ImmediateDispatcher());
        await coordinator.StartAsync();

        await coordinator.StopAsync();

        Assert.AreEqual(1, engine.StopCount);
    }
}

internal static class TestData
{
    public static ScopeRef Scope() => new() { Kind = "organization", Id = Guid.NewGuid() };

    public static ConfigSnapshot Configuration(long revision, string locale) => new()
    {
        Revision = revision,
        SchemaVersion = 1,
        Scope = Scope(),
        Entries =
        [
            new ConfigEntry
            {
                Key = ProtocolIds.ConfigKeys.EitmadConfigLocalePrimaryV1,
                Sensitivity = ConfigSensitivity.Public,
                RestartRequirement = RestartRequirement.None,
                Value = new ConfigReadValue { Kind = ConfigReadValueKind.Text, Value = locale },
            },
        ],
    };

    public static EventEnvelope Event(Guid subscriptionId, long sequence) => new()
    {
        SubscriptionId = subscriptionId,
        CorrelationId = Guid.NewGuid(),
        Cursor = Guid.NewGuid(),
        Sequence = sequence,
        OccurredAt = sequence,
        Event = [],
    };

    public static async Task Eventually(Func<bool> condition)
    {
        var deadline = DateTime.UtcNow + TimeSpan.FromSeconds(3);
        while (!condition())
        {
            if (DateTime.UtcNow >= deadline)
            {
                Assert.Fail("Expected asynchronous shell condition was not reached.");
            }

            await Task.Delay(5);
        }
    }
}
