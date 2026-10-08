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
public sealed class DiscountApprovalRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ConfirmedApprovalControlsRenderAndRejectThroughTheServerReply(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            var authority = new DiscountApprovalTests.ConfirmedAuthority(); var reception = authority.Engine(); var manager = authority.Engine();
            var catalog = new SalesCatalogClient(reception); var receptionDrafts = new QuotationDraftClient(reception); var managerDrafts = new QuotationDraftClient(manager);
            SalesCatalogViewModel? editor = null; QuotationsView? view = null;
            try {
                Finish(receptionDrafts.ActivateAsync()); var preparation = QuotationDraftTests.Editor(catalog, receptionDrafts); Finish(preparation); editor = preparation.Result;
                editor.DiscountInput = "6"; Finish(editor.LastQuotationEvaluation); editor.IsReviewingQuotation = true;
                var quotation = new CurrentQuotationView { DataContext = editor }; window.Content = quotation;
                if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
                WpfTestHost.CompleteLayout(window); var scroll = WpfTestHost.Descendants<ScrollViewer>(quotation).First(); scroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                var request = WpfTestHost.FindByAutomationName<Button>(quotation, "طلب موافقة"); Assert.IsTrue(request.IsEnabled); Assert.IsTrue(request.Focus()); Assert.IsTrue(request.IsKeyboardFocusWithin);
                request.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(editor.LastApprovalRequest); Assert.IsTrue(editor.IsDiscountPending);
                WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, $"discount-pending-reception-{width}");
                view = new QuotationsView(); view.ViewModel.AttachDraftClient(managerDrafts); Finish(view.ViewModel.ActivateDraftsAsync()); view.ViewModel.OpenApprovals(); window.Content = view;
                WpfTestHost.CompleteLayout(window); Assert.HasCount(1, view.ViewModel.VisibleQuotations); WpfTestHost.Capture(window, $"discount-inbox-manager-{width}");
                view.ViewModel.OpenQuotation(view.ViewModel.VisibleQuotations.Single()); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
                var reason = WpfTestHost.Descendants<TextBox>(view).Single(t => AutomationProperties.GetName(t) == "سبب رفض الخصم");
                reason.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(reason.Focus()); reason.Text = "الخصم مرتفع";
                var reject = WpfTestHost.FindByAutomationName<Button>(view, "رفض خصم عرض السعر"); reject.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(reject.Focus()); Assert.IsTrue(reject.IsKeyboardFocusWithin); Assert.IsTrue(reject.IsEnabled);
                var dpi = VisualTreeHelper.GetDpi(window); Console.WriteLine($"Approval requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%");
                WpfTestHost.Capture(window, $"discount-review-manager-{width}");
                reject.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(view.ViewModel.LastApprovalDecision);
                Assert.IsFalse(view.ViewModel.CanDecideApproval); authority.Publish(reception); Finish(QuotationDraftTests.WaitFor(() => editor.IsDiscountRejected));
                window.Content = quotation; WpfTestHost.CompleteLayout(window); scroll.ScrollToBottom(); WpfTestHost.CompleteLayout(window);
                Assert.IsFalse(editor.CanIssueQuotation); WpfTestHost.Capture(window, $"discount-rejected-reception-{width}");
            } finally {
                if (view is not null) Finish(view.ViewModel.DeactivateDraftsAsync()); if (editor is not null) Finish(editor.DeactivateCatalogAsync());
                Finish(receptionDrafts.DisposeAsync().AsTask()); Finish(managerDrafts.DisposeAsync().AsTask()); Finish(catalog.DisposeAsync().AsTask()); Finish(reception.DisposeAsync().AsTask()); Finish(manager.DisposeAsync().AsTask());
            }
        });
    }
    private static void Finish(Task task)
    {
        task = task.WaitAsync(TimeSpan.FromSeconds(15));
        if (!task.IsCompleted) { var frame = new DispatcherFrame(); var dispatcher = Dispatcher.CurrentDispatcher;
            _ = task.ContinueWith(_ => dispatcher.BeginInvoke(new Action(() => frame.Continue = false)), TaskScheduler.Default); Dispatcher.PushFrame(frame); }
        task.GetAwaiter().GetResult();
    }
}
