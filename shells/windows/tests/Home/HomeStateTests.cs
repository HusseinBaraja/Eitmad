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
    public async Task SearchContextUpdatesBeforeRepliesAndResetDiscardsPendingSearch()
    {
        var data = Data(); var filtered = Data(); filtered.Quotations.Items = [];
        await using var engine = Engine(data); await using var model = new HomeViewModel(engine);
        engine.QueryHandler = query => SalesCatalogAuthorityTests.Response(QueryResult.ForHome(
            query.AsHomeRead()!.Term == "عميل" ? filtered : data));
        var queries = new List<string>();
        var pending = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        engine.QueryBarrier = query => {
            var term = query.AsHomeRead()!.Term;
            queries.Add(term);
            return term == "عميل" ? pending.Task : Task.CompletedTask;
        };
        await model.ActivateAsync(); Assert.HasCount(1, model.Activity);
        var changed = new List<string?>();
        model.PropertyChanged += (_, args) => changed.Add(args.PropertyName);
        model.Search(" عميل "); var search = model.LastLoad;
        Assert.AreEqual("عميل", model.SearchTerm); Assert.IsTrue(model.HasSearchTerm);
        Assert.AreEqual("نتائج البحث", model.ActivityTitle); Assert.HasCount(0, model.Activity);
        Assert.AreEqual("", model.ActivityMessage);
        Assert.AreEqual("جارٍ تحميل البيانات...", model.Notice);
        CollectionAssert.Contains(changed, nameof(HomeViewModel.SearchTerm));
        CollectionAssert.Contains(changed, nameof(HomeViewModel.HasSearchTerm));
        model.Search(""); await model.LastLoad;
        pending.SetResult(); await search;
        Assert.AreEqual("", model.SearchTerm); Assert.IsFalse(model.HasSearchTerm);
        Assert.AreEqual("آخر نشاط", model.ActivityTitle); Assert.HasCount(1, model.Activity);
        CollectionAssert.AreEqual(new[] { "", "عميل", "" }, queries);
        model.Search("عميل QT-2026-00001"); await model.LastLoad;
        model.Refresh(); await model.LastLoad;
        Assert.AreEqual(model.SearchTerm, queries[^1]);
        engine.QueryHandler = _ => throw new System.IO.IOException("Synthetic unavailable");
        model.Refresh(); await model.LastLoad;
        Assert.AreEqual("عميل QT-2026-00001", model.SearchTerm); Assert.IsTrue(model.HasSearchTerm);
        Assert.HasCount(0, model.Activity);
    }
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
        Assert.IsFalse(model.IsEmpty); Assert.IsFalse(model.HasNoReadyOrders);
        Assert.IsTrue(model.Notice.Contains("غير متاحة"));
        Assert.AreEqual("", model.ActivityMessage); Assert.AreEqual("", model.ReadyOrdersMessage);
    }
    [TestMethod]
    public async Task EmptyMessagesRequireObtainedCompleteCurrentSectionsAndRetryRestoresThem()
    {
        var data = new HomeSnapshot { Quotations = Section(), Orders = Section(), Approvals = Section(), Customers = Section(), Catalog = Section(), ReadyOrders = [] };
        await using var engine = Engine(data); await using var model = new HomeViewModel(engine);
        Assert.IsFalse(model.IsEmpty); Assert.IsFalse(model.HasNoReadyOrders);
        await model.ActivateAsync(); Assert.IsTrue(model.IsEmpty); Assert.IsTrue(model.HasNoReadyOrders);
        // Approvals supply a count only; omitted search sources do not qualify recent activity.
        data.Approvals.Availability = HomeAvailability.Denied; data.Customers.Availability = HomeAvailability.Unavailable;
        model.Refresh(); await model.LastLoad; Assert.IsTrue(model.IsEmpty);
        model.Search("عميل"); await model.LastLoad;
        Assert.IsFalse(model.IsEmpty); Assert.IsTrue(model.Notice.Contains("العملاء: غير متاح"));
        model.Search(""); await model.LastLoad;
        data.Orders.Complete = false; model.Refresh(); await model.LastLoad;
        Assert.IsFalse(model.IsEmpty); Assert.IsFalse(model.HasNoReadyOrders);
        Assert.IsTrue(model.Notice.Contains("الطلبات: نتائج جزئية"));
        Assert.AreEqual("", model.ActivityMessage); Assert.AreEqual("", model.ReadyOrdersMessage);
        data.Orders.Complete = true; data.Orders.ServerAvailable = false; model.Refresh(); await model.LastLoad;
        Assert.IsFalse(model.IsEmpty); Assert.IsFalse(model.HasNoReadyOrders);
        Assert.IsTrue(model.Notice.Contains("آخر بيانات محفوظة"));
        engine.QueryHandler = _ => throw new System.IO.IOException("Synthetic unavailable"); model.Refresh(); await model.LastLoad;
        Assert.IsFalse(model.IsEmpty); Assert.IsFalse(model.HasNoReadyOrders);
        data.Orders.ServerAvailable = true;
        engine.QueryHandler = _ => SalesCatalogAuthorityTests.Response(QueryResult.ForHome(data)); model.Refresh(); await model.LastLoad;
        Assert.IsTrue(model.IsEmpty); Assert.IsTrue(model.HasNoReadyOrders);
    }
    [TestMethod]
    [DataRow(HomeAvailability.Denied, "ليس لديك صلاحية")]
    [DataRow(HomeAvailability.Unavailable, "غير متاح")]
    public async Task MissingSourceDoesNotConfirmEmptyAndDoesNotRemoveOtherAvailableRows(HomeAvailability availability, string message)
    {
        var data = Data(); data.Orders.Availability = availability;
        await using var engine = Engine(data); await using var model = new HomeViewModel(engine);
        await model.ActivateAsync();
        Assert.HasCount(1, model.Activity); Assert.HasCount(0, model.ReadyOrders);
        Assert.IsFalse(model.IsEmpty); Assert.IsFalse(model.HasNoReadyOrders);
        Assert.IsTrue(model.Notice.Contains("الطلبات: " + message));
        Assert.AreEqual("", model.ActivityMessage); Assert.AreEqual("", model.ReadyOrdersMessage);
        data.Quotations.Availability = availability; model.Refresh(); await model.LastLoad;
        Assert.HasCount(0, model.Activity); Assert.IsFalse(model.IsEmpty);
        Assert.IsTrue(model.Notice.Contains("عروض الأسعار: " + message));
    }
}
