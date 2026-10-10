using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Home;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Home;

[TestClass]
public sealed class HomeStateTests
{
    internal static HomeSection Section(uint count = 0, params HomeItem[] items) => new() { Availability = HomeAvailability.Available, Complete = true, ServerAvailable = true, Count = count, Items = items };
    internal static HomeSnapshot Data() => new() {
        Quotations = Section(2, new HomeItem { Id = Guid.NewGuid(), Destination = HomeDestination.Quotation, Number = "QT-2026-00001", Title = "عميل تجريبي A-12", State = "صادر", ChangedAt = 1791626400000 }),
        Orders = new() { Availability = HomeAvailability.Available, Complete = true, ServerAvailable = true, Count = 1, SecondaryCount = 1, Items = [] }, Approvals = Section(1), Customers = Section(), Catalog = Section(),
        ReadyOrders = [new() { Id = Guid.NewGuid(), Destination = HomeDestination.Order, Number = "OR-2026-00001", Title = "عميل تجريبي B-7", State = "جاهز", ChangedAt = 1791626400000 }],
    };
    internal static FakeEngine Engine(HomeSnapshot data) => new() {
        SupportedCapabilities = new HashSet<string> { ProtocolIds.Capabilities.EitmadCapabilityHomeV1, ProtocolIds.Capabilities.EitmadCapabilityOrdersV1 },
        QueryHandler = q => { Assert.IsNotNull(q.AsHomeRead()); return SalesCatalogAuthorityTests.Response(QueryResult.ForHome(data)); },
    };
    [TestMethod]
    public async Task SubscriptionAndReconnectRefreshRustStateAndReconstructionReadsAgain()
    {
        var data = Data(); await using var engine = Engine(data); engine.Connect();
        await using (var model = new HomeViewModel(engine)) {
            await model.ActivateAsync(); Assert.AreEqual("2", model.OpenCount); Assert.HasCount(1, model.ReadyOrders);
            data.Quotations.Count = 3; data.ReadyOrders = [];
            engine.SignalResync(ProtocolIds.Subscriptions.EitmadOrderChangedSubscribeV1);
            await model.LastLoad; Assert.AreEqual("3", model.OpenCount); Assert.HasCount(0, model.ReadyOrders);
            engine.Disconnect(); engine.Connect(); await model.LastLoad;
            model.Search("عميل"); await model.LastLoad;
            Assert.AreEqual("نتائج البحث", model.ActivityTitle);
            await model.DeactivateAsync(); Assert.AreEqual("—", model.OpenCount); Assert.HasCount(0, model.Activity);
        }
        await using var restarted = new HomeViewModel(engine); await restarted.ActivateAsync(); Assert.AreEqual("3", restarted.OpenCount);
    }
    [TestMethod]
    public async Task DenialAndUnavailableReadsRemoveProtectedRowsAndLateRepliesCannotRepopulate()
    {
        var data = Data(); await using var engine = Engine(data); await using var model = new HomeViewModel(engine);
        await model.ActivateAsync(); Assert.HasCount(1, model.Activity);
        data.Quotations.Availability = HomeAvailability.Denied; data.Orders.Availability = HomeAvailability.Unavailable;
        model.Refresh(); await model.LastLoad;
        Assert.HasCount(0, model.Activity); Assert.HasCount(0, model.ReadyOrders); Assert.AreEqual("—", model.OpenCount);
        var barrier = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.QueryBarrier = _ => barrier.Task;
        model.Refresh(); var pending = model.LastLoad; model.Clear(); barrier.SetResult(); await pending;
        Assert.HasCount(0, model.Activity); Assert.AreEqual("—", model.ApprovalCount);
    }
    [TestMethod]
    public async Task PartialCountsAndCachedDataAreExplicitAndFailedQueriesDoNotShowZero()
    {
        var data = Data(); data.Quotations.Complete = false; data.Orders.ServerAvailable = false;
        await using var engine = Engine(data); await using var model = new HomeViewModel(engine);
        await model.ActivateAsync(); Assert.AreEqual("≥ 2", model.OpenCount); Assert.IsTrue(model.Notice.Contains("نتائج جزئية")); Assert.IsTrue(model.Notice.Contains("غير متصل"));
        engine.QueryHandler = _ => throw new System.IO.IOException("Synthetic unavailable"); model.Refresh(); await model.LastLoad;
        Assert.AreEqual("—", model.OpenCount); Assert.HasCount(0, model.ReadyOrders); Assert.HasCount(0, model.Activity);
    }
}
