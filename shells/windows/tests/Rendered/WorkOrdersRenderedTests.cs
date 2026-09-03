using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using Eitmad.WindowsShell.Features.WorkOrders;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class WorkOrdersRenderedTests
{
    [TestMethod]
    public void ManagerListAndDetailRenderManufacturingInformationWithoutCosting()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "WorkOrdersNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<WorkOrdersView>(window).Single();
            Assert.AreEqual(Visibility.Visible, view.Visibility);
            Assert.AreEqual("البحث في أوامر العمل", AutomationProperties.GetName(WpfTestHost.FindByName<TextBox>(view, "WorkOrderSearchBox")));
            Assert.AreEqual(2, WpfTestHost.Descendants<ComboBox>(view).Count());
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "الأثاث"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "مسند إلى"));
            Assert.IsFalse(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text.Contains("السعر", StringComparison.Ordinal) || text.Text.Contains("الربح", StringComparison.Ordinal)));

            WpfTestHost.FindByAutomationName<Button>(view, "فتح أمر العمل")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            WpfTestHost.CompleteLayout(view);

            Assert.IsTrue(view.ViewModel.IsDetailVisible);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "BackToWorkOrdersButton").IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "أمر عمل"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "#024" && text.FlowDirection == FlowDirection.LeftToRight));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "الأجزاء المطلوبة"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "ملاحظات الطلب"));
            Assert.IsGreaterThan(0, WpfTestHost.Descendants<Canvas>(view).Count(canvas => canvas.IsVisible));

            var statusAction = WpfTestHost.FindByName<Button>(view, "AdvanceStatusButton");
            Assert.AreEqual("تغيير حالة أمر العمل", AutomationProperties.GetName(statusAction));
            statusAction.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(WorkOrderStatus.InProgress, view.ViewModel.SelectedWorkOrder?.Status);
        });
    }

    [TestMethod]
    public void CompactWorkOrderListKeepsWideTableScrollable()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            WpfTestHost.FindByName<Button>(window, "WorkOrdersNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<WorkOrdersView>(window).Single();
            Assert.IsTrue(WpfTestHost.Descendants<ScrollViewer>(view).Any(scroll =>
                scroll.IsVisible && scroll.HorizontalScrollBarVisibility == ScrollBarVisibility.Auto));

            view.ViewModel.OpenWorkOrder(view.ViewModel.VisibleWorkOrders[0]);
            WpfTestHost.CompleteLayout(view);
            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Grid>(view, "DetailSurface").Visibility);
        });
    }
}
