using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Pricing;
using Eitmad.WindowsShell.Tests.Pricing;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class PricingRenderedTests
{
    [TestMethod]
    [DataRow(false)]
    [DataRow(true)]
    public void UnexpectedPublicationFailureKeepsExactRetryAndIgnoresEndedSession(bool endSession)
    {
        var engine = new FakeEngine();
        engine.QueryHandler = query => new QueryResponseEnvelope { Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded,
            Payload = query.AsPricingList() is not null ? QueryResult.ForPrices(PricingPresentationTests.Data()) : QueryResult.ForPriceReview(new PriceReview { CostYer = 160_000, MarginYer = 40_000 }) } };
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PricingNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PricingView>(window).Single();
            WpfTestHost.FindByAutomationName<Button>(view, "تعديل سعر البيع").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            engine.CommandHandler = _ =>
            {
                if (endSession) view.ClearSession();
                throw new System.Text.Json.JsonException("Synthetic malformed reply");
            };
            var save = WpfTestHost.FindByAutomationName<Button>(view, "حفظ سعر البيع");
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            Assert.IsFalse(view.ViewModel.IsBusy);
            if (endSession)
            {
                Assert.IsFalse(view.ViewModel.SavePending);
                Assert.IsFalse(view.ViewModel.IsEditorOpen);
                Assert.IsFalse(view.ViewModel.HasEditorError);
                return;
            }
            Assert.IsTrue(view.ViewModel.SavePending);
            Assert.IsTrue(view.ViewModel.IsEditorOpen);
            Assert.IsFalse(view.ViewModel.CanEdit);
            Assert.IsTrue(view.ViewModel.CanSave);
            Assert.AreEqual(PricingClient.ArabicMessage(PricingFailure.Unconfirmed), view.ViewModel.EditorError);
            var originalKey = engine.LastIdempotencyKey;
            engine.CommandHandler = command =>
            {
                var input = command.AsPricingPublish()!;
                Assert.AreEqual(200_000L, input.SellingPriceYer);
                return new CommandResponseEnvelope { Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded,
                    Payload = CommandResult.ForPricePublished(new PublishedPrice { Target = input.Target, Currency = "YER", Revision = 2, SellingPriceYer = input.SellingPriceYer, Colors = [], Handles = [] }) } };
            };
            save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(originalKey, engine.LastIdempotencyKey);
            Assert.IsFalse(view.ViewModel.SavePending);
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
        }, engine: engine);
    }

    [TestMethod]
    public void ConfirmedPublicationWithFailedReloadDoesNotBecomeAnUnconfirmedRetry()
    {
        var engine = new FakeEngine();
        var published = false;
        engine.QueryHandler = query =>
        {
            if (published && query.AsPricingList() is not null) throw new System.Text.Json.JsonException("Synthetic malformed list");
            return new QueryResponseEnvelope { Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded,
                Payload = query.AsPricingList() is not null ? QueryResult.ForPrices(PricingPresentationTests.Data()) : QueryResult.ForPriceReview(new PriceReview { CostYer = 160_000, MarginYer = 40_000 }) } };
        };
        engine.CommandHandler = command =>
        {
            published = true;
            var input = command.AsPricingPublish()!;
            return new CommandResponseEnvelope { Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded,
                Payload = CommandResult.ForPricePublished(new PublishedPrice { Target = input.Target, Currency = "YER", Revision = 2, SellingPriceYer = input.SellingPriceYer, Colors = [], Handles = [] }) } };
        };
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PricingNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PricingView>(window).Single();
            WpfTestHost.FindByAutomationName<Button>(view, "تعديل سعر البيع").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ سعر البيع").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(published);
            Assert.IsFalse(view.ViewModel.IsBusy);
            Assert.IsFalse(view.ViewModel.SavePending);
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            Assert.IsEmpty(view.ViewModel.VisiblePrices);
            StringAssert.Contains(view.ViewModel.AvailabilityMessage, "تعذر تحميل الأسعار");
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void PricingListEditorAndBelowCostWarningRenderAtBaselineSizes(int width, int height)
    {
        var page = PricingPresentationTests.Data();
        page.CatalogSyncIssues = [new CatalogSyncIssue { Kind = "product-category", Id = Guid.NewGuid(), Revision = 1, Name = "فئة المنتجات الجاهزة" }];
        var engine = new FakeEngine();
        engine.QueryHandler = query => new QueryResponseEnvelope { Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded,
            Payload = query.AsPricingList() is not null ? QueryResult.ForPrices(page) : QueryResult.ForPriceReview(new PriceReview { CostYer = 160_000, MarginYer = -10_000, BelowCost = true }) } };
        engine.CommandHandler = command =>
        {
            var input = command.AsPricingPublish()!;
            page.Items[0].Published = new PriceSummary { Currency = "YER", Revision = 2, SellingPriceYer = input.SellingPriceYer };
            page.Items[0].MarginYer = 60_000;
            return new CommandResponseEnvelope { Outcome = new CommandOutcome { Status = CommandOutcomeStatus.Succeeded,
                Payload = CommandResult.ForPricePublished(new PublishedPrice { Target = input.Target, Currency = "YER", SellingPriceYer = input.SellingPriceYer, Revision = 2, Colors = [], Handles = [] }) } };
        };
        WpfTestHost.Run(width, height, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PricingNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PricingView>(window).Single();
            Assert.HasCount(2, view.ViewModel.VisiblePrices);
            Assert.IsTrue(WpfTestHost.FindByName<TextBlock>(view, "CatalogSyncIssueNotice").IsVisible);
            var dpi = System.Windows.Media.VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Pricing render: {window.ActualWidth} x {window.ActualHeight} DIP; display scaling {dpi.DpiScaleX * 100}%");

            WpfTestHost.Capture(window, $"pricing-{Math.Round(window.ActualWidth)}x{Math.Round(window.ActualHeight)}-{dpi.DpiScaleX * 100}percent");
            WpfTestHost.FindByAutomationName<Button>(view, "تعديل سعر البيع").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            var input = WpfTestHost.FindByName<TextBox>(view, "PriceInput");
            Assert.IsTrue(input.IsKeyboardFocusWithin);
            input.Text = "220000.5";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ سعر البيع").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsTrue(view.ViewModel.HasEditorError);
            view.ViewModel.ApplyReview(new PriceReview { CostYer = 160_000, MarginYer = -10_000, BelowCost = true });
            WpfTestHost.CompleteLayout(view);
            var warning = WpfTestHost.FindByAutomationName<CheckBox>(view, "تأكيد نشر سعر أقل من التكلفة");
            Assert.IsTrue(warning.IsVisible);
            input.Focus();
            input.MoveFocus(new System.Windows.Input.TraversalRequest(System.Windows.Input.FocusNavigationDirection.Next));
            Assert.IsTrue(warning.IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, $"pricing-warning-{Math.Round(window.ActualWidth)}x{Math.Round(window.ActualHeight)}-{dpi.DpiScaleX * 100}percent");
            input.Text = "220000";
            WpfTestHost.FindByAutomationName<Button>(view, "حفظ سعر البيع").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.IsFalse(view.ViewModel.IsEditorOpen);
            Assert.AreEqual(220_000L, view.ViewModel.VisiblePrices[0].SellingPrice);
            StringAssert.Contains(view.ViewModel.FeedbackMessage, "بتأكيد الخادم");
        }, engine: engine);
    }
    [TestMethod]
    public void ReceptionistPricingColumnsAndEditorAreWithheld()
    {
        var engine = new FakeEngine();
        engine.QueryHandler = _ => new QueryResponseEnvelope { Outcome = new QueryOutcome { Status = CommandOutcomeStatus.Succeeded, Payload = QueryResult.ForPrices(PricingPresentationTests.Data(false)) } };
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "PricingNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<PricingView>(window).Single();
            var table = WpfTestHost.FindByName<OperationsTable>(view, "PricingRows");
            Assert.AreEqual(Visibility.Collapsed, table.Columns[2].Visibility);
            Assert.AreEqual(Visibility.Collapsed, table.Columns[4].Visibility);
            Assert.AreEqual(Visibility.Collapsed, table.Columns[6].Visibility);
            Assert.IsFalse(table.IsRowInvocationEnabled);
            Assert.IsTrue(view.ViewModel.VisiblePrices.All(row => row.Cost is null && row.Margin is null));
            WpfTestHost.Capture(window, "pricing-receptionist-1338x753-125percent");
        }, engine: engine);
    }
}
