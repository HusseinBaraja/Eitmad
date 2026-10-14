using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.Quotations;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class QuotationDraftRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void MixedDraftSaveReopenAndFurnitureEditingUseTheRenderedControls(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            var authority = new QuotationDraftTests.Authority(); var engine = authority.Engine();
            var catalog = new SalesCatalogClient(engine); var drafts = new QuotationDraftClient(engine);
            SalesCatalogViewModel? model = null;
            try
            {
                var preparation = QuotationDraftTests.Editor(catalog, drafts); Finish(preparation); model = preparation.Result;
                var view = new SalesCatalogView { DataContext = model, ShowQuotationHeader = true };
                window.Content = view; model.IsReviewingQuotation = true;
                if (width == 1920) window.WindowState = WindowState.Maximized;
                WpfTestHost.CompleteLayout(window);
                var dpi = VisualTreeHelper.GetDpi(window);
                var geometry = $"Draft requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; display {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX:0}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY:0} px; scale {dpi.DpiScaleX * 100:0}%; WPF RenderTargetBitmap at 96 DPI; synthetic Rust replies.";
                Console.WriteLine(geometry);
                if (Environment.GetEnvironmentVariable("EITMAD_UI_CAPTURE_DIR") is { Length: > 0 } directory) {
                    System.IO.Directory.CreateDirectory(directory);
                    System.IO.File.WriteAllText(System.IO.Path.Combine(directory, $"ui32-{width}.txt"), geometry);
                }
                Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
                WpfTestHost.Capture(window, $"quotation-draft-lines-{width}");
                var review = WpfTestHost.Descendants<CurrentQuotationView>(view).Single();
                var header = WpfTestHost.FindByName<Controls.PageHeader>(view, "WorkflowHeader");
                Assert.AreEqual(model.QuotationHeading, header.Title);
                Assert.IsFalse(header.ContainsButton);
                Assert.IsTrue(header.ShowSubtitle);
                view.RestoreQuotationFocus(); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(review.ContinueButton.IsKeyboardFocused);
                review.ContinueButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                Assert.IsFalse(model.IsReviewingQuotation);
                Assert.IsTrue(header.ContainsButton);
                Assert.AreEqual(model.QuotationLabel, header.ButtonText);
                WpfTestHost.FindByName<Button>(header, "HeaderButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(model.IsReviewingQuotation);
                Assert.AreEqual(model.QuotationHeading, header.Title);
                Assert.IsFalse(header.ContainsButton);
                var scroll = WpfTestHost.Descendants<ScrollViewer>(review).First(); scroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                var save = WpfTestHost.FindByName<Button>(review, "SaveDraftButton");
                Assert.IsTrue(save.IsEnabled); Assert.AreEqual("حفظ كمسودة", AutomationProperties.GetName(save));
                save.Focus(); Assert.IsTrue(save.IsKeyboardFocused);
                save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(QuotationDraftTests.WaitFor(() => !model.IsDraftBusy));
                Assert.IsNotNull(authority.Draft); Assert.IsTrue(model.DraftState.Contains("محلياً"));
                Assert.IsTrue(WpfTestHost.FindByName<Button>(review, "PrintPreviewButton").IsEnabled);
                WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, $"quotation-draft-saved-{width}");
                var snapshot = authority.Draft.Snapshot;
                model.DiscountInput = "6"; Finish(model.LastQuotationEvaluation); Finish(model.OpenDraftAsync(snapshot.Id));
                Assert.AreEqual("5", model.DiscountInput); Assert.AreEqual(48553m, model.FinalTotal);
                scroll.ScrollToTop(); WpfTestHost.CompleteLayout(window);
                var furniture = model.QuotationLines.Single(line => line.IsFurniture);
                WpfTestHost.FindByAutomationName<Button>(review, "تعديل " + furniture.Name).RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(model.LastCatalogOperation);
                WpfTestHost.CompleteLayout(window);
                Assert.IsFalse(model.IsReviewingQuotation); Assert.AreEqual("120", model.Selection!.WidthCm); Assert.AreEqual(furniture.Quantity, model.Selection.Quantity);
                Assert.IsTrue(model.Selection.CanAdd); Assert.AreEqual(2, snapshot.Intent.Lines.Length);
                WpfTestHost.Capture(window, $"quotation-draft-edit-furniture-{width}");
                model.CloseSelection(); WpfTestHost.CompleteLayout(window); scroll.ScrollToBottom();
                var commandHandler = engine.CommandHandler;
                engine.CommandHandler = _ => throw new System.IO.IOException("Synthetic save failure");
                model.DiscountInput = "4"; Finish(model.LastQuotationEvaluation);
                save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(QuotationDraftTests.WaitFor(() => !model.IsDraftBusy));
                Assert.AreEqual("4", model.DiscountInput); Assert.AreEqual(500, snapshot.Intent.DiscountBasisPoints);
                Assert.IsTrue(model.HasDraftSaveFailure); Assert.IsTrue(model.DraftEditState.Contains("احتُفظ")); WpfTestHost.CompleteLayout(window);
                scroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                var failure = WpfTestHost.FindByName<Controls.FeedbackNotice>(review, "DraftSaveFailureNotice");
                Assert.IsTrue(failure.IsVisible); Assert.IsTrue(save.IsEnabled); Assert.AreEqual("إعادة محاولة الحفظ", save.Content);
                Assert.IsFalse(model.CanIssueQuotation); Assert.IsTrue(WpfTestHost.FindByName<Button>(review, "PrintPreviewButton").IsEnabled);
                foreach (var element in new FrameworkElement[] { failure, save }) {
                    var saveScroll = WpfTestHost.Ancestor<ScrollViewer>(save);
                    var bounds = element.TransformToAncestor(saveScroll).TransformBounds(new Rect(element.RenderSize));
                    Assert.IsTrue(new Rect(saveScroll.RenderSize).Contains(bounds), $"{element.Name} is outside the save viewport: {bounds}");
                }
                WpfTestHost.Capture(window, $"quotation-draft-failed-save-{width}");
                if (width == 720) {
                    review.Resources[SystemParameters.HighContrastKey] = true; WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Capture(window, "quotation-draft-failed-save-high-contrast-720");
                    review.Resources.Remove(SystemParameters.HighContrastKey);
                }
                engine.CommandHandler = commandHandler;
                save.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(QuotationDraftTests.WaitFor(() => !model.IsDraftBusy));
                Assert.IsFalse(model.HasDraftSaveFailure); Assert.AreEqual("4", model.DiscountInput);
                Assert.AreEqual(400, authority.Draft!.Snapshot.Intent.DiscountBasisPoints);
                var manager = new QuotationsView(); manager.ViewModel.AttachDraftClient(drafts); Finish(manager.ViewModel.ActivateDraftsAsync());
                window.Content = manager; WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(snapshot.Id, manager.ViewModel.VisibleQuotations.Single().Id);
                WpfTestHost.Capture(window, $"quotation-draft-manager-list-{width}");
                manager.ViewModel.OpenQuotation(manager.ViewModel.VisibleQuotations.Single()); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(WpfTestHost.FindByName<Controls.FeedbackNotice>(manager, "DraftNotice").IsVisible);
                Assert.IsFalse(manager.ViewModel.ShowManagerApproval); WpfTestHost.Capture(window, $"quotation-draft-manager-detail-{width}");
                Finish(manager.ViewModel.DeactivateDraftsAsync());
            }
            finally { if (model is not null) Finish(model.DeactivateCatalogAsync()); Finish(catalog.DisposeAsync().AsTask()); Finish(drafts.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask()); }
        });
    }
    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(10));
        if (!task.IsCompleted) {
            var frame = new DispatcherFrame(); var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default);
            Dispatcher.PushFrame(frame);
        }
        task.GetAwaiter().GetResult();
    }
}
