using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Tests.Quotations;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class QuotationLifecycleRenderedTests
{
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ConfirmedQuotationActionsRenderForBothRoles(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            var fixture = new QuotationLifecycleTests.Fixture(); var client = new QuotationDraftClient(fixture.Engine);
            try {
                var view = new QuotationsView(); view.ConfigureReceptionist(_ => throw new InvalidOperationException("Synthetic editor is not used")); view.ViewModel.AttachDraftClient(client); Finish(view.ViewModel.ActivateDraftsAsync());
                view.ViewModel.OpenQuotation(view.ViewModel.VisibleQuotations.Single()); window.Content = view;
                if (width == 1920) { window.Left = 0; window.WindowState = WindowState.Maximized; }
                WpfTestHost.CompleteLayout(window);
                var issue = WpfTestHost.FindByAutomationName<Button>(view, "إصدار عرض السعر"); issue.BringIntoView(); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(issue.IsVisible); Assert.IsTrue(issue.IsEnabled); Assert.IsTrue(issue.Focus()); Assert.IsTrue(issue.IsKeyboardFocusWithin);
                Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
                WpfTestHost.Capture(window, $"quotation-issue-reception-{width}");
                issue.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(view.ViewModel.LastLifecycleAction);
                Assert.AreEqual("QT-2026-00001", view.ViewModel.SelectedQuotation!.Number); Assert.IsFalse(view.ViewModel.SelectedQuotation.CanConvert);
                Finish(view.ViewModel.DeactivateDraftsAsync());
                fixture.Record.PermittedActions = [QuotationPermittedAction.Accept, QuotationPermittedAction.Print];
                Finish(view.ViewModel.ActivateDraftsAsync()); view.ViewModel.OpenQuotation(view.ViewModel.VisibleQuotations.Single());
                WpfTestHost.CompleteLayout(window);
                var acceptance = WpfTestHost.FindByAutomationName<ComboBox>(view, "طريقة قبول عرض السعر");
                acceptance.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(acceptance.Focus());
                acceptance.IsDropDownOpen = true; WpfTestHost.CompleteLayout(window); Assert.IsTrue(acceptance.IsDropDownOpen); acceptance.IsDropDownOpen = false;
                var acceptanceNote = WpfTestHost.FindByAutomationName<TextBox>(view, "ملاحظة قبول العميل"); Assert.IsTrue(acceptanceNote.Focus());
                var accept = WpfTestHost.FindByAutomationName<Button>(view, "تأكيد قبول العميل لعرض السعر");
                accept.BringIntoView(); WpfTestHost.CompleteLayout(window); Assert.IsTrue(accept.IsVisible); Assert.IsTrue(accept.Focus());
                WpfTestHost.Capture(window, $"quotation-accept-reception-{width}");
                Finish(view.ViewModel.DeactivateDraftsAsync());
                fixture.Record.PermittedActions = [QuotationPermittedAction.Revise, QuotationPermittedAction.Cancel, QuotationPermittedAction.Print];
                view = new QuotationsView(); view.ViewModel.AttachDraftClient(client); Finish(view.ViewModel.ActivateDraftsAsync());
                view.ViewModel.OpenQuotation(view.ViewModel.VisibleQuotations.Single()); window.Content = view; WpfTestHost.CompleteLayout(window);
                var revise = WpfTestHost.FindByAutomationName<Button>(view, "إنشاء إصدار جديد من عرض السعر"); revise.BringIntoView(); WpfTestHost.CompleteLayout(window);
                Assert.IsTrue(revise.IsVisible); Assert.IsTrue(revise.Focus()); Assert.IsTrue(revise.IsKeyboardFocusWithin);
                var reason = WpfTestHost.FindByAutomationName<TextBox>(view, "سبب إلغاء عرض السعر"); Assert.IsTrue(reason.IsVisible); Assert.IsTrue(reason.Focus()); reason.Text = "إلغاء تجريبي مع حفظ سجل العرض";
                var dpi = VisualTreeHelper.GetDpi(window); Console.WriteLine($"Quotation requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; scale {dpi.DpiScaleX * 100:0}%");
                WpfTestHost.Capture(window, $"quotation-issued-manager-{width}");
                if (width == 720) { foreach (var control in WpfTestHost.Descendants<Control>(view)) Eitmad.WindowsShell.Controls.ControlOptions.SetHighContrast(control, true); WpfTestHost.CompleteLayout(window); WpfTestHost.Capture(window, "quotation-issued-manager-high-contrast-720"); }
                Finish(view.ViewModel.DeactivateDraftsAsync());
            } finally { Finish(client.DisposeAsync().AsTask()); Finish(fixture.Engine.DisposeAsync().AsTask()); }
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
