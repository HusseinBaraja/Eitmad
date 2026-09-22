using System.Windows;
using System.Windows.Controls;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Features.Orders;
using Eitmad.WindowsShell.Features.Quotations;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class CustomersRenderedTests
{
    [TestMethod]
    public void HistoryProjectionUsesRustIdentityAcrossRepublishedQuotation()
    {
        var orders = new OrdersViewModel(true).VisibleOrders;
        var sourceOrder = orders.First(order => order.OriginalQuotation is not null);
        var original = sourceOrder.OriginalQuotation!;
        var customer = ContractCustomer(sourceOrder.Customer, sourceOrder.Phone);
        var replacement = new QuotationListItem(original.Id, original.Number, original.Customer,
            original.Date, original.Status, original.Discount, original.Items, phone: sourceOrder.Phone)
        {
            CustomerId = customer.Id,
        };

        var projection = CustomerHistoryProjection.Create(customer, [replacement], orders);

        Assert.AreEqual(customer.Id, projection.Id);
        Assert.IsTrue(projection.Orders.Count > 0);
        Assert.HasCount(1, projection.Quotations.Where(item => item.Number == original.Number));
    }

    [TestMethod]
    public void ReceptionRecordsOpenRustCustomerAndRestoreFocus()
    {
        var order = new OrdersViewModel(true).VisibleOrders.First();
        var engine = new FakeEngine();
        engine.Customers.Add(ContractCustomer(order.Customer, order.Phone));
        WpfTestHost.Run(1338, 900, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<Button>(reception, "OrdersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var orders = WpfTestHost.Descendants<OrdersView>(reception).Single();
            orders.ViewModel.OpenOrder(orders.ViewModel.VisibleOrders.Single(item => item.Id == order.Id));
            WpfTestHost.CompleteLayout(window);
            var open = WpfTestHost.FindByAutomationName<Button>(orders, "فتح تفاصيل عميل الطلب");
            open.Focus();
            open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            var detail = WpfTestHost.FindByName<CustomerDetailView>(reception, "CustomerDetail");
            var customer = (CustomerPreview)detail.DataContext;
            Assert.AreEqual(engine.Customers[0].Id, customer.Id);
            Assert.IsTrue(customer.Orders.Any(row => row.Number == order.Number));
            Assert.IsTrue(WpfTestHost.FindByName<Button>(detail, "BackButton").IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, "customers-reception-detail");
            WpfTestHost.FindByName<Button>(detail, "BackButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(open.IsKeyboardFocusWithin);
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ReceptionCustomerSelectionRendersRustSearchResults(int width, int height)
    {
        var engine = new FakeEngine();
        engine.Customers.Add(ContractCustomer("مؤسسة الاعتماد للأثاث", "+967 777 123 456", "صنعاء", ""));
        WpfTestHost.Run(width, height, window =>
        {
            var reception = WpfTestHost.FindByName<ReceptionistHomeView>(window, "ReceptionistSurface");
            reception.Visibility = Visibility.Visible;
            WpfTestHost.FindByName<Grid>(window, "ResponsiveRoot").Visibility = Visibility.Collapsed;
            var catalog = (SalesCatalogView)reception.FindName("CatalogContent");
            catalog.Visibility = Visibility.Visible;
            ((SalesCatalogViewModel)catalog.DataContext).IsReviewingQuotation = true;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.FindByName<TextBox>(catalog, "CustomerNameInput").Text = "الاعتماد";
            WpfTestHost.CompleteLayout(window);
            Assert.IsNotNull(WpfTestHost.FindByAutomationName<Button>(catalog, "اختيار العميل مؤسسة الاعتماد للأثاث"));
            WpfTestHost.Capture(window, $"customers-selection-{width}x{height}");
        }, engine: engine);
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void DetailEditKeepsHistoryAndCancelDiscardsDraft(int width, int height)
    {
        var engine = new FakeEngine();
        var contract = ContractCustomer("منزل عائلة الصبري", "000000084",
            "عدن، المنصورة — عنوان تجريبي", "الاتصال قبل توصيل الأثاث.");
        engine.Customers.Add(contract);
        WpfTestHost.Run(width, height, window =>
        {
            var customer = new CustomerPreview(contract,
                [new("QUO-2026-0084", new(2026, 9, 12), "سرير وادي ظهر، مرتبة الراحة", "محوّل", "345,000 YER")],
                [new("ORD-2026-0084", new(2026, 9, 13), "سرير وادي ظهر، مرتبة الراحة", "قيد الإنتاج", "345,000 YER")]);
            var view = new CustomerDetailView { DataContext = customer };
            view.Attach(new CustomerClient(engine));
            window.Content = view;
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"customers-detail-{width}x{height}");
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
            name.Text = "عائلة الصبري";
            WpfTestHost.Capture(window, $"customers-editor-{width}x{height}");
            WpfTestHost.FindByName<Button>(view, "ApplyButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual("عائلة الصبري", customer.Name);
            Assert.AreEqual("ORD-2026-0084", customer.Orders.Single().Number);
            Assert.AreEqual("QUO-2026-0084", customer.Quotations.Single().Number);
            foreach (var table in WpfTestHost.Descendants<OperationsTable>(view)) ControlOptions.SetHighContrast(table, true);
            WpfTestHost.CompleteLayout(window);
            WpfTestHost.Capture(window, $"customers-contrast-detail-{width}x{height}");
        }, engine: engine);
    }

    private static Customer ContractCustomer(string name, string phone, string? address = null, string? notes = null) => new()
    {
        Id = Guid.NewGuid(),
        Scope = new ScopeRef { Kind = "branch", Id = Guid.NewGuid() },
        Name = name,
        Phone = phone,
        Address = address!,
        Notes = notes!,
        Status = CustomerStatus.Active,
        Revision = 1,
        SyncState = ErSyncState.Pending,
        UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
    };
}
