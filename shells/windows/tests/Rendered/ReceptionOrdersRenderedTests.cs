using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Reception;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class ReceptionOrdersRenderedTests
{
    [TestMethod]
    public void ReceptionNavigationFiltersReadyAndCustomerDocuments()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "OrdersAction").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var view = WpfTestHost.Descendants<OrdersView>(reception).Single();
            Assert.IsTrue(view.IsVisible);
            Assert.IsTrue(view.ViewModel.IsReceptionist);
            WpfTestHost.Capture(window, "reception-orders-list");
            view.ViewModel.SearchText = "٠٠٠٠٠٠٠٨٥";
            view.ViewModel.SelectedStatus = OrdersViewModel.ReadyStatus;
            view.ViewModel.SelectedDate = OrdersViewModel.LastSevenDays;
            var ready = view.ViewModel.VisibleOrders.Single();
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByAutomationName<Button>(view, "فتح الطلب").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "BackToOrdersButton").IsKeyboardFocusWithin);
            Assert.IsTrue(WpfTestHost.FindByName<FeedbackNotice>(view, "ReadyOrderNotice").IsVisible);
            Assert.AreEqual("الطلب جاهز", WpfTestHost.FindByName<FeedbackNotice>(view, "ReadyOrderNotice").Message);
            WpfTestHost.Capture(window, "reception-orders-ready");
            InspectDocument(WpfTestHost.FindByName<Button>(view, "PrintOrderButton"), ready.Number);
            InspectDocument(WpfTestHost.FindByName<Button>(view, "OriginalQuotationButton"), ready.OriginalQuotation!.Number);
            WpfTestHost.FindByName<Button>(view, "BackToOrdersButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.FindByName<TextBox>(view, "OrderSearchBox").IsKeyboardFocusWithin);
            Assert.AreEqual("٠٠٠٠٠٠٠٨٥", view.ViewModel.SearchText);
            view.ViewModel.SearchText = "";
            view.ViewModel.SelectedStatus = OrdersViewModel.InProductionStatus;
            var mixed = view.ViewModel.VisibleOrders.Single();
            view.ViewModel.OpenOrder(mixed);
            WpfTestHost.CompleteLayout(window);
            Assert.IsFalse(WpfTestHost.FindByName<FeedbackNotice>(view, "ReadyOrderNotice").IsVisible);
            Assert.IsTrue(mixed.Items.Any(item => !item.IsFurniture));
            WpfTestHost.Capture(window, "reception-orders-mixed-items");
            var printed = OrderCustomerDocument.Create(mixed);
            var text = new TextRange(printed.ContentStart, printed.ContentEnd).Text;
            Assert.IsTrue(text.Contains("مرتبة الراحة"));
            Assert.IsFalse(text.Contains("التكلفة"));
            Assert.IsFalse(text.Contains("المواد الخام"));
            Assert.IsFalse(text.Contains("الأجزاء"));

            void InspectDocument(Button trigger, string number)
            {
                Exception? failure = null;
                var inspected = false;
                trigger.Focus();
                window.Dispatcher.BeginInvoke(System.Windows.Threading.DispatcherPriority.ApplicationIdle, new Action(() =>
                {
                    var child = window.OwnedWindows.Cast<Window>().Single();
                    try
                    {
                        var preview = (PrintPreview)child.Content;
                        Assert.IsTrue(WpfTestHost.FindByName<Button>(preview, "PrintButton").IsKeyboardFocusWithin);
                        Assert.IsTrue(new TextRange(preview.Document.ContentStart, preview.Document.ContentEnd).Text.Contains(number));
                        inspected = true;
                    }
                    catch (Exception exception) { failure = exception; }
                    finally { child.Close(); }
                }));
                trigger.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                if (failure is not null) throw failure;
                Assert.IsTrue(inspected);
                Assert.IsTrue(trigger.IsKeyboardFocusWithin);
            }
        });
    }

    [TestMethod]
    public void CompactReadyDetailKeepsActionsAndStatusVisible()
    {
        WpfTestHost.Run(780, 745, window =>
        {
            var view = new OrdersView();
            view.ConfigureReceptionist();
            window.Content = view;
            view.ViewModel.OpenOrder(view.ViewModel.VisibleOrders.Single(row => row.IsReady));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(WpfTestHost.FindByName<FeedbackNotice>(view, "ReadyOrderNotice").IsVisible);
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "PrintOrderButton").Focus());
            Assert.IsTrue(WpfTestHost.FindByName<Button>(view, "OriginalQuotationButton").IsEnabled);
            WpfTestHost.Capture(window, "reception-orders-compact-ready");
            ControlOptions.SetHighContrast(view, true);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "reception-orders-high-contrast-ready");
        });
    }
}
