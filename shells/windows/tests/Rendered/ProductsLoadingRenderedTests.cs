using System.Windows;
using System.Windows.Controls;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Products;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class ProductsLoadingRenderedTests
{
    [TestMethod]
    public void PendingFirstLoadHidesEmptyResultUntilProductsAndCategoriesAreReady()
    {
        var engine = Engine();
        var release = new TaskCompletionSource();
        engine.QueryBarrier = _ => release.Task;
        WpfTestHost.Run(1338, 753, window =>
        {
            var view = new ProductsView(); window.Content = view; view.Attach(engine);
            var pending = view.ActivateAsync();
            try
            {
                AssertLoading(window, view);
                release.SetResult(); Finish(pending);
                WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(view.ViewModel.IsDataReady);
                Assert.HasCount(1, view.ViewModel.VisibleProducts);
                Assert.HasCount(1, view.ViewModel.Categories);
                Assert.IsTrue(WpfTestHost.FindByAutomationName<Button>(view, "إضافة منتج").IsEnabled);
                Assert.IsTrue(WpfTestHost.FindByName<OperationsTable>(view, "ProductResults").IsVisible);
                AssertNoVisibleEmptyResult(view);
                view.ViewModel.BeginAddCategory(); Assert.IsTrue(view.ViewModel.IsCategoryEditorOpen);
            }
            finally { release.TrySetResult(); Finish(pending); Finish(view.DisposeAsync().AsTask()); }
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(false)]
    [DataRow(true)]
    public void EmptyResultRequiresSuccessAndFailureCanRecover(bool fail)
    {
        var engine = Engine(empty: true);
        engine.ThrowQueries = fail;
        if (fail) engine.QueryHandler = null;
        WpfTestHost.Run(1338, 753, window =>
        {
            var view = new ProductsView(); window.Content = view; view.Attach(engine);
            try
            {
                Finish(view.ActivateAsync()); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(!fail, view.ViewModel.IsDataReady);
                Assert.AreEqual(!fail, WpfTestHost.Descendants<EmptyState>(view).Any(s => s.IsVisible && s.Heading == "لا توجد منتجات مطابقة"));
                Assert.AreEqual(!fail, view.ViewModel.CanEdit);
                if (fail)
                {
                    engine.ThrowQueries = false; engine.QueryHandler = Engine().QueryHandler;
                    Finish(view.ActivateAsync());
                    Assert.IsTrue(view.ViewModel.IsDataReady); Assert.HasCount(1, view.ViewModel.VisibleProducts);
                }
            }
            finally { Finish(view.DisposeAsync().AsTask()); }
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(false, false)]
    [DataRow(false, true)]
    [DataRow(true, false)]
    [DataRow(true, true)]
    public void SupersededCompletionCannotEndNewLoadingOrReplaceNewData(bool latestFirst, bool oldFails)
    {
        var engine = Engine();
        var requests = new List<Query>();
        var releases = new[] { new TaskCompletionSource(), new TaskCompletionSource() };
        var success = engine.QueryHandler!;
        Task Barrier(Query query)
        {
            if (query.Kind != Query.ProductListKind) return Task.CompletedTask;
            requests.Add(query); return releases[requests.Count - 1].Task;
        }
        engine.QueryHandler = query => requests.Count > 0 && ReferenceEquals(query, requests[0])
            ? new QueryResponseEnvelope { Outcome = new QueryOutcome { Status = oldFails ? CommandOutcomeStatus.Failed : CommandOutcomeStatus.Succeeded,
                Payload = oldFails ? new QueryResult { Code = ProtocolIds.ErrorCodes.EitmadErrorAuthorizationDeniedV1 }
                    : QueryResult.ForProducts(new ProductPage { Items = [], CanManage = false }) } }
            : success(query);
        WpfTestHost.Run(1338, 753, window =>
        {
            var view = new ProductsView(); window.Content = view; view.Attach(engine);
            Finish(view.ActivateAsync()); WpfTestHost.CompleteLayout(window);
            engine.QueryBarrier = Barrier;
            var first = view.ActivateAsync(); var latest = view.ActivateAsync();
            try
            {
                Assert.HasCount(2, requests); AssertLoading(window, view);
                if (!latestFirst)
                {
                    releases[0].SetResult(); WpfTestHost.PumpDispatcher();
                    AssertLoading(window, view);
                    Assert.IsFalse(first.IsCompleted, "Activation must await the refresh that replaced its original request.");
                }
                releases[1].SetResult(); Finish(latest);
                Assert.IsTrue(view.ViewModel.IsDataReady); Assert.HasCount(1, view.ViewModel.VisibleProducts);
                if (latestFirst) releases[0].SetResult();
                Finish(first);
                Assert.IsTrue(view.ViewModel.IsDataReady); Assert.IsTrue(view.ViewModel.CanEdit);
                Assert.HasCount(1, view.ViewModel.VisibleProducts); Assert.HasCount(1, view.ViewModel.Categories);
            }
            finally { foreach (var release in releases) release.TrySetResult(); Finish(first); Finish(latest); Finish(view.DisposeAsync().AsTask()); }
        }, engine: engine);
    }

    private static void AssertLoading(MainWindow window, ProductsView view)
    {
        WpfTestHost.CompleteLayout(window);
        Assert.IsFalse(view.ViewModel.IsDataReady);
        Assert.IsTrue(WpfTestHost.FindByName<EmptyState>(view, "ProductLoadStatus").IsVisible);
        Assert.IsFalse(WpfTestHost.FindByName<OperationsTable>(view, "ProductResults").IsVisible);
        Assert.AreEqual("", view.ViewModel.VisibleCountLabel);
        Assert.IsFalse(WpfTestHost.FindByAutomationName<Button>(view, "إضافة منتج").IsEnabled);
        view.ViewModel.BeginCreate(); view.ViewModel.BeginAddCategory(); view.ViewModel.BeginManageCategories();
        Assert.IsFalse(view.ViewModel.IsEditorOpen); Assert.IsFalse(view.ViewModel.IsCategoryEditorOpen); Assert.IsFalse(view.ViewModel.IsCategoryManagerOpen);
        AssertNoVisibleEmptyResult(view);
    }

    private static void AssertNoVisibleEmptyResult(ProductsView view) =>
        Assert.IsFalse(WpfTestHost.Descendants<EmptyState>(view).Any(s => s.IsVisible && s.Heading == "لا توجد منتجات مطابقة"));

    private static FakeEngine Engine(bool empty = false)
    {
        var data = ProductsPresentationTests.Data();
        return new FakeEngine { QueryHandler = query => new QueryResponseEnvelope { Outcome = new QueryOutcome
        {
            Status = CommandOutcomeStatus.Succeeded,
            Payload = query.Kind == Query.ProductCategoryListKind ? QueryResult.ForProductCategories(data.Categories)
                : QueryResult.ForProducts(new ProductPage { Items = empty ? [] : data.Products.ToArray(), CanManage = true, CanReadCosts = true })
        } } };
    }

    private static void Finish(Task task)
    {
        var deadline = DateTime.UtcNow.AddSeconds(5);
        while (!task.IsCompleted && DateTime.UtcNow < deadline) WpfTestHost.PumpDispatcher();
        Assert.IsTrue(task.IsCompleted, "The controlled request did not complete."); task.GetAwaiter().GetResult();
    }
}
