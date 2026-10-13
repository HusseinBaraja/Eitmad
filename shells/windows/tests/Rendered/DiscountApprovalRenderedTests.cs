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
            var dpi = VisualTreeHelper.GetDpi(window);
            var displays = string.Join("; ", System.Windows.Forms.Screen.AllScreens.Select(s => $"{s.Bounds.Width}x{s.Bounds.Height} pixels"));
            Console.WriteLine($"Approval rendering: available displays {displays}; scale {dpi.DpiScaleX * 100:0}%; off-screen WPF RenderTargetBitmap at 96 DPI; synthetic server replies.");
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
                var rejection = WpfTestHost.FindByName<Expander>(view, "DiscountRejection");
                Assert.IsFalse(rejection.IsExpanded);
                Assert.IsFalse(WpfTestHost.Descendants<TextBox>(rejection).Any(t => t.IsVisible));
                var approve = WpfTestHost.FindByAutomationName<Button>(view, "الموافقة على خصم عرض السعر");
                Assert.IsTrue(approve.IsEnabled);
                var panel = WpfTestHost.FindByName<Border>(view, "ApprovalSection");
                var detailScroll = WpfTestHost.Ancestor<ScrollViewer>(panel);
                detailScroll.ScrollToVerticalOffset(detailScroll.VerticalOffset + panel.TranslatePoint(new Point(), detailScroll).Y);
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"discount-request-summary-manager-{width}");
                var expandToggle = WpfTestHost.Descendants<System.Windows.Controls.Primitives.ToggleButton>(rejection).Single();
                Assert.IsTrue(expandToggle.Focus());
                Assert.IsTrue(expandToggle.IsKeyboardFocusWithin);
                expandToggle.SetCurrentValue(System.Windows.Controls.Primitives.ToggleButton.IsCheckedProperty, true);
                WpfTestHost.CompleteLayout(window); Assert.IsTrue(rejection.IsExpanded);
                var reason = WpfTestHost.Descendants<TextBox>(view).Single(t => AutomationProperties.GetName(t) == "سبب رفض الخصم");
                Assert.IsTrue(reason.IsVisible);
                reason.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(reason.Focus()); reason.Text = "الخصم مرتفع";
                var reject = WpfTestHost.FindByAutomationName<Button>(view, "رفض خصم عرض السعر"); reject.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(reject.Focus()); Assert.IsTrue(reject.IsKeyboardFocusWithin); Assert.IsTrue(reject.IsEnabled);
                Console.WriteLine($"Approval requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%");
                WpfTestHost.Capture(window, $"discount-review-manager-{width}");
                if (width == 720) {
                    view.Resources[SystemParameters.HighContrastKey] = true; WpfTestHost.CompleteLayout(window);
                    WpfTestHost.Capture(window, "discount-review-manager-high-contrast-720");
                    view.Resources.Remove(SystemParameters.HighContrastKey); WpfTestHost.CompleteLayout(window);
                }
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
