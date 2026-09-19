using System.Windows;
using System.Windows.Controls;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Quotations;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class CustomersRenderedTests
{
    [TestMethod]
    public void RepublishedQuotationKeepsCustomerIdentityAndOrderHistory()
    {
        var directory = new CustomerPreviewDirectory();
        var original = new OrdersViewModel(true).VisibleOrders.First(order => order.OriginalQuotation is not null)
            .OriginalQuotation!;
        var customer = directory.ForQuotation(original.Id);
        Assert.IsTrue(customer.Orders.Count > 0);
        var replacement = new QuotationListItem(original.Id, original.Number, original.Customer,
            original.Date, original.Status, original.Discount, original.Items, phone: "000000099");

        directory.IncludeQuotation(replacement);

        Assert.AreSame(customer, directory.ForQuotation(original.Id));
        Assert.IsTrue(customer.Orders.Count > 0);
        Assert.AreEqual("000000099", customer.Phone);
        Assert.HasCount(1, customer.Quotations.Where(item => item.Number == original.Number));
    }

    [TestMethod]
    public void ReceptionRecordsOpenSameCustomerAndRestoreFocus()
    {
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "OrdersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var orders = WpfTestHost.Descendants<OrdersView>(reception).Single();
            var order = orders.ViewModel.VisibleOrders.First();
            orders.ViewModel.OpenOrder(order);
            WpfTestHost.CompleteLayout(window);
            var open = WpfTestHost.FindByAutomationName<Button>(orders, "فتح تفاصيل عميل الطلب");
            open.Focus();
            open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var detail = WpfTestHost.FindByName<CustomerDetailView>(reception, "CustomerDetail");
            var customer = (CustomerPreview)detail.DataContext;
            Assert.IsTrue(customer.Orders.Any(row => row.Number == order.Number));
            Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, "customers-reception-detail");
            WpfTestHost.FindByName<Button>(detail, "BackButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(open.IsKeyboardFocusWithin);
            Assert.AreSame(order, orders.ViewModel.SelectedOrder);
            WpfTestHost.FindByName<Button>(reception, "QuotationsNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var quotations = WpfTestHost.Descendants<QuotationsView>(reception).Single();
            quotations.ViewModel.OpenQuotation(quotations.ViewModel.VisibleQuotations.Single(row => row.Id == order.OriginalQuotation!.Id));
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByAutomationName<Button>(quotations, "فتح تفاصيل عميل عرض السعر").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreSame(customer, detail.DataContext);
        });
    }

    [TestMethod]
    public void DetailEditKeepsHistoryAndCancelDiscardsDraft()
    {
        WpfTestHost.Run(1100, 850, window =>
        {
            var customer = new CustomerPreview(new("منزل عائلة الصبري", "000000084", "عدن، المنصورة — عنوان تجريبي", "الاتصال قبل توصيل الأثاث."),
                [new("QUO-2026-0084", new(2026, 9, 12), "سرير وادي ظهر، مرتبة الراحة", "محوّل", "345,000 YER")],
                [new("ORD-2026-0084", new(2026, 9, 13), "سرير وادي ظهر، مرتبة الراحة", "قيد الإنتاج", "345,000 YER")]);
            var view = new CustomerDetailView { DataContext = customer };
            window.Content = view;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "customers-detail");
            var edit = WpfTestHost.FindByName<Button>(view, "EditButton");
            edit.Focus();
            edit.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var name = WpfTestHost.FindByName<TextBox>(view, "NameInput");
            Assert.IsTrue(name.IsKeyboardFocusWithin);
            name.Text = "اسم مؤقت";
            WpfTestHost.FindByAutomationName<Button>(view, "إلغاء تعديل العميل").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("منزل عائلة الصبري", customer.Name);
            Assert.IsTrue(edit.IsKeyboardFocusWithin);
            edit.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            name.Text = "";
            WpfTestHost.FindByName<Button>(view, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            Assert.IsTrue(WpfTestHost.FindByName<DialogHost>(view, "Dialog").IsOpen);
            name.Text = "عائلة الصبري — بيانات تجريبية";
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "customers-editor");
            WpfTestHost.FindByName<Button>(view, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("عائلة الصبري — بيانات تجريبية", customer.Name);
            Assert.AreEqual("ORD-2026-0084", customer.Orders.Single().Number);
            Assert.AreEqual("QUO-2026-0084", customer.Quotations.Single().Number);
            window.Width = 780;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "customers-compact-detail");
            foreach (var table in WpfTestHost.Descendants<OperationsTable>(view)) ControlOptions.SetHighContrast(table, true);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, "customers-contrast-detail");
        });
    }
}

