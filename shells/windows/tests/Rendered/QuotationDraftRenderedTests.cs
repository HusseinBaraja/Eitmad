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
                Console.WriteLine($"Draft requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%");
                Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
                WpfTestHost.Capture(window, $"quotation-draft-lines-{width}");
                var review = WpfTestHost.Descendants<CurrentQuotationView>(view).Single();
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
                Assert.IsTrue(model.QuotationNotice.Contains("احتُفظ")); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"quotation-draft-failed-save-{width}");
                engine.CommandHandler = commandHandler;
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
