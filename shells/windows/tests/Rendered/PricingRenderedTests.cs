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
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void LongSyncIssueNamesKeepRecoveryActionsInsideTheViewport(int width, int height)
    {
        WpfTestHost.Run(width, height, window =>
        {
            if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
            var view = new PricingView();
            window.Content = view;
            var page = PricingPresentationTests.Data();
            page.CatalogSyncIssues = [new CatalogSyncIssue { Kind = "furniture", Id = Guid.NewGuid(), Revision = 1,
                Conflicted = true, Name = "خزانة ملابس كبيرة بأبواب متعددة وأدراج داخلية وتشطيب خشبي للاستخدام في غرف النوم الرئيسية" }];
            view.ViewModel.ApplyDurableData(page);
            WpfTestHost.CompleteLayout(window);
            var review = WpfTestHost.FindByName<Expander>(view, "CatalogSyncReview");
            review.IsExpanded = true;
            WpfTestHost.CompleteLayout(window);
            var button = WpfTestHost.Descendants<Button>(review).Single(control => control.DataContext is CatalogSyncIssue);
            var viewport = WpfTestHost.Ancestor<ScrollContentPresenter>(review);
            var bounds = button.TransformToAncestor(viewport).TransformBounds(new Rect(button.RenderSize));
            Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= viewport.ActualWidth && bounds.Top >= 0 && bounds.Bottom <= viewport.ActualHeight,
                $"Recovery action {bounds} must fit the viewport {viewport.RenderSize}.");
            StringAssert.Contains(AutomationProperties.GetName(button), page.CatalogSyncIssues[0].Name);
            var header = WpfTestHost.Descendants<System.Windows.Controls.Primitives.ToggleButton>(review).Single();
            Assert.IsTrue(header.Focus());
            Assert.IsTrue(header.MoveFocus(new System.Windows.Input.TraversalRequest(System.Windows.Input.FocusNavigationDirection.Next)));
            Assert.IsTrue(button.IsKeyboardFocusWithin);
            var dpi = System.Windows.Media.VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Pricing recovery: display {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY} px; actual window {window.ActualWidth}x{window.ActualHeight} DIP; scaling {dpi.DpiScaleX * 100}%.");
            WpfTestHost.Capture(window, $"pricing-recovery-{width}x{height}");
            if (width == 720)
            {
                review.FontSize *= 1.5;
                button.FontSize *= 1.5;
                WpfTestHost.CompleteLayout(window);
                bounds = button.TransformToAncestor(viewport).TransformBounds(new Rect(button.RenderSize));
                Assert.IsTrue(bounds.Left >= 0 && bounds.Right <= viewport.ActualWidth && bounds.Top >= 0 && bounds.Bottom <= viewport.ActualHeight,
                    $"Enlarged recovery action {bounds} must fit the viewport {viewport.RenderSize}.");
                WpfTestHost.Capture(window, "pricing-recovery-enlarged-text-720x560");
            }
        });
    }

    [TestMethod]
    public void SyncReviewOpensDefinitionPageAndRetainsRejectedAndConflictedStates()
    {
        var page = PricingPresentationTests.Data();
        page.CatalogSyncIssues = [new CatalogSyncIssue { Kind = "product-category", Id = Guid.NewGuid(), Revision = 1, Name = "فئة اختبار", Conflicted = true },
            new CatalogSyncIssue { Kind = "furniture", Id = Guid.NewGuid(), Revision = 2, Name = "أثاث اختبار" }];
        var engine = new FakeEngine();
        engine.QueryHandler = _ => new QueryResponseEnvelope { Outcome = new QueryOutcome
            { Status = CommandOutcomeStatus.Succeeded, Payload = QueryResult.ForPrices(page) } };
        WpfTestHost.Run(1920, 1080, window =>
        {
            window.WindowState = WindowState.Maximized;
            WpfTestHost.FindByName<Button>(window, "PricingNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            VerifyDefinitionReview(window, WpfTestHost.Descendants<PricingView>(window).Single());
        }, engine: engine);
    }

    internal static void VerifyDefinitionReview(MainWindow window, PricingView view)
    {
        var review = WpfTestHost.FindByName<Expander>(view, "CatalogSyncReview");
        Assert.IsTrue(review.IsVisible);
        review.IsExpanded = true;
        WpfTestHost.CompleteLayout(window);
        var issues = view.ViewModel.SyncIssues;
        var buttons = WpfTestHost.Descendants<Button>(review).Where(button => button.DataContext is CatalogSyncIssue).ToArray();
        Assert.HasCount(issues.Count, buttons);
        foreach (var issue in issues)
        {
            var button = buttons.Single(button => ReferenceEquals(button.DataContext, issue));
            StringAssert.Contains(AutomationProperties.GetName(button), issue.Name);
            var badge = WpfTestHost.Descendants<StatusBadge>(review).Single(badge => ReferenceEquals(badge.DataContext, issue));
            Assert.AreEqual(issue.Conflicted ? "تعارض" : "مرفوض", badge.Text);
            Assert.AreEqual(PresentationTone.Danger, badge.Tone);
        }
        var header = WpfTestHost.Descendants<System.Windows.Controls.Primitives.ToggleButton>(review).Single();
        Assert.IsTrue(header.Focus());
        header.MoveFocus(new System.Windows.Input.TraversalRequest(System.Windows.Input.FocusNavigationDirection.Next));
        Assert.IsTrue(buttons[0].IsKeyboardFocusWithin);
        buttons[0].RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        WpfTestHost.CompleteLayout(window);
        Assert.IsTrue(WpfTestHost.FindByName<FrameworkElement>(window, "ProductsSurface").IsVisible);
        Assert.IsFalse(view.IsVisible);
        WpfTestHost.FindByName<Button>(window, "PricingNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
        WpfTestHost.CompleteLayout(window);
        view.ViewModel.ClearSession();
        WpfTestHost.CompleteLayout(view);
        Assert.IsFalse(review.IsVisible);
        Assert.IsEmpty(WpfTestHost.Descendants<Button>(review).Where(button => button.DataContext is CatalogSyncIssue));
    }

    [TestMethod]
    public void CategorySelectionMatchesVisibleFilterAfterLoadingAndReloading()
    {
        var page = PricingPresentationTests.Data();
        var engine = new FakeEngine();
        engine.QueryHandler = _ => new QueryResponseEnvelope { Outcome = new QueryOutcome
        {
            Status = CommandOutcomeStatus.Succeeded, Payload = QueryResult.ForPrices(page),
        } };
        WpfTestHost.Run(1920, 1080, window =>
        {
            window.WindowState = WindowState.Maximized;
            WpfTestHost.FindByName<Button>(window, "PricingNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            VerifyCategoryFilter(WpfTestHost.Descendants<PricingView>(window).Single(), replacement => page = replacement);
        }, engine: engine);
    }

    internal static void VerifyCategoryFilter(PricingView view, Action<PricePage> replacePage)
    {
        var model = view.ViewModel;
        var filter = WpfTestHost.FindByAutomationName<ComboBox>(view, "تصفية حسب الفئة");
        var reload = WpfTestHost.FindByAutomationName<Button>(view, "إعادة تحميل الأسعار");

        void CheckSelection(string expected, int count)
        {
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(expected, model.SelectedCategory);
            Assert.AreEqual(expected, filter.SelectedItem);
            Assert.AreEqual(expected, filter.SelectionBoxItem);
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(filter).Any(text => text.IsVisible && text.Text == expected));
            Assert.HasCount(count, model.VisiblePrices);
            Assert.HasCount(count, WpfTestHost.FindByName<OperationsTable>(view, "PricingRows").Items);
        }

        void Reload(PricePage page)
        {
            replacePage(page);
            reload.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
        }

        CheckSelection(PricingViewModel.AllCategories, 2);
        Reload(PricingPresentationTests.Data());
        CheckSelection(PricingViewModel.AllCategories, 2);

        filter.SelectedItem = "غرف الطعام";
        CheckSelection("غرف الطعام", 1);
        Assert.AreEqual("طاولة طعام", model.VisiblePrices.Single().Product);
        Reload(PricingPresentationTests.Data());
        CheckSelection("غرف الطعام", 1);
        Assert.AreEqual("طاولة طعام", model.VisiblePrices.Single().Product);

        var remaining = PricingPresentationTests.Data();
        remaining.Items = [remaining.Items[0]];
        Reload(remaining);
        CheckSelection(PricingViewModel.AllCategories, 1);
        Reload(new PricePage { Items = [], CanManage = true, CanReadCosts = true });
        CheckSelection(PricingViewModel.AllCategories, 0);
        Reload(PricingPresentationTests.Data());
        CheckSelection(PricingViewModel.AllCategories, 2);

        filter.SelectedItem = "غرف الطعام";
        view.ClearSession();
        CheckSelection(PricingViewModel.AllCategories, 0);
    }

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
            Payload = query.AsPricingList() is not null ? QueryResult.ForPrices(page)
                : QueryResult.ForPriceReview(query.AsPricingReview()!.SellingPriceYer == 150_000
                    ? new PriceReview { CostYer = 160_000, MarginYer = -10_000, BelowCost = true }
                    : new PriceReview { CostYer = 160_000, MarginYer = 60_000 }) } };
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
            Console.WriteLine($"Available display: {SystemParameters.PrimaryScreenWidth} x {SystemParameters.PrimaryScreenHeight} DIP; work area {SystemParameters.WorkArea.Width} x {SystemParameters.WorkArea.Height} DIP");
            Console.WriteLine($"Pricing render: {window.ActualWidth} x {window.ActualHeight} DIP; display scaling {dpi.DpiScaleX * 100}%");

            WpfTestHost.Capture(window, $"pricing-{Math.Round(window.ActualWidth)}x{Math.Round(window.ActualHeight)}-{dpi.DpiScaleX * 100}percent");
            WpfTestHost.FindByAutomationName<Button>(view, "تعديل سعر البيع").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            var input = WpfTestHost.FindByName<TextBox>(view, "PriceInput");
            Assert.IsTrue(input.IsKeyboardFocusWithin);
            input.Text = "150000";
            view.ViewModel.ApplyReview(150_000, new PriceReview { CostYer = 160_000, MarginYer = -10_000, BelowCost = true });
            WpfTestHost.CompleteLayout(view);
            var warning = WpfTestHost.FindByAutomationName<CheckBox>(view, "تأكيد نشر سعر أقل من التكلفة");
            var save = WpfTestHost.FindByAutomationName<Button>(view, "حفظ سعر البيع");
            Assert.IsTrue(warning.IsVisible);
            Assert.IsFalse(save.IsEnabled);
            input.Focus();
            input.MoveFocus(new System.Windows.Input.TraversalRequest(System.Windows.Input.FocusNavigationDirection.Next));
            Assert.IsTrue(warning.IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, $"pricing-warning-{Math.Round(window.ActualWidth)}x{Math.Round(window.ActualHeight)}-{dpi.DpiScaleX * 100}percent");
            if (width == 720)
            {
                warning.FontSize *= 1.5;
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "pricing-warning-enlarged-text-720x560");
                warning.ClearValue(Control.FontSizeProperty);
            }
            warning.IsChecked = true;
            Assert.IsTrue(save.IsEnabled);
            input.Text = "220000.5";
            view.ViewModel.ApplyReview(150_000, new PriceReview { CostYer = 160_000, MarginYer = -10_000, BelowCost = true });
            WpfTestHost.CompleteLayout(view);
            Assert.IsTrue(view.ViewModel.HasEditorError);
            Assert.IsFalse(warning.IsVisible);
            Assert.IsFalse(view.ViewModel.ConfirmBelowCost);
            Assert.IsFalse(save.IsEnabled);
            Assert.AreEqual("—", view.ViewModel.EditorMargin);
            var margin = WpfTestHost.Descendants<TextBlock>(view).Single(text =>
                System.Windows.Data.BindingOperations.GetBinding(text, TextBlock.TextProperty)?.Path.Path == nameof(PricingViewModel.EditorMargin));
            Assert.AreEqual("—", margin.Text);
            Assert.IsTrue(margin.IsVisible);
            WpfTestHost.Capture(window, $"ui-26-invalid-price-{Math.Round(window.ActualWidth)}x{Math.Round(window.ActualHeight)}-{dpi.DpiScaleX * 100}percent");
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
