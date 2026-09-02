using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using Eitmad.WindowsShell.Features.Orders;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class OrdersRenderedTests
{
    [TestMethod]
    public void ManagerListAndReadOnlyDetailRenderAllRequiredInformation()
    {
        WpfTestHost.Run(1338, 753, window =>
        {
            WpfTestHost.FindByName<Button>(window, "OrdersNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<OrdersView>(window).Single();
            Assert.AreEqual(Visibility.Visible, view.Visibility);
            Assert.AreEqual("البحث برقم الطلب أو العميل", AutomationProperties.GetName(WpfTestHost.FindByName<TextBox>(view, "OrderSearchBox")));
            Assert.AreEqual(2, WpfTestHost.Descendants<ComboBox>(view).Count());
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "الطلبات"));
            Assert.IsTrue(view.ViewModel.VisibleOrders.Select(item => item.StatusLabel).Contains("قيد الإنتاج"));

            WpfTestHost.FindByAutomationName<Button>(view, "فتح الطلب")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.PumpDispatcher();
            WpfTestHost.CompleteLayout(view);

            Assert.IsTrue(view.ViewModel.IsDetailVisible);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "BackToOrdersButton").IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "الأبعاد"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "سعر البيع"));
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(view).Any(text => text.Text == "الإجمالي النهائي"));
            Assert.IsFalse(WpfTestHost.Descendants<Button>(view).Any(button =>
                button.IsVisible && AutomationProperties.GetName(button) is "تغيير حالة الطلب" or "فتح سير عمل النجّار"));
        });
    }

    [TestMethod]
    public void CompactDetailKeepsReviewContentScrollable()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            WpfTestHost.FindByName<Button>(window, "OrdersNavButton")
                .RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);

            var view = WpfTestHost.Descendants<OrdersView>(window).Single();
            view.ViewModel.OpenOrder(view.ViewModel.VisibleOrders[0]);
            WpfTestHost.CompleteLayout(view);

            Assert.AreEqual(Visibility.Visible, WpfTestHost.FindByName<Grid>(view, "DetailSurface").Visibility);
            Assert.IsTrue(WpfTestHost.Descendants<ScrollViewer>(view).Any(scroll =>
                scroll.IsVisible && scroll.HorizontalScrollBarVisibility == ScrollBarVisibility.Auto));
        });
    }
}
