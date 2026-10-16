using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Home;
using Eitmad.WindowsShell.Tests.Home;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Tests.TestDoubles;
using Eitmad.WindowsShell.Tests.Products;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class HomeSearchRenderedTests
{
    [TestMethod]
    [DataRow(true, 1920, 1080)]
    [DataRow(true, 1338, 753)]
    [DataRow(true, 720, 560)]
    [DataRow(false, 1920, 1080)]
    [DataRow(false, 1338, 753)]
    [DataRow(false, 720, 560)]
    public void SignedInRoleOffersWorkingSearchAndReceptionistOpensContactAndSalesHistory(bool manager, int width, int height)
    {
        var order = new Orders.OrderAuthorityTests.Fixture().Order;
        var contact = order.Source.Quotation.Evaluation.Customer!;
        contact.Name = "عميل تجريبي للبحث"; contact.Phone = "000000010";
        var customer = new Customer { Id = contact.Id, Scope = order.Scope, Name = contact.Name, Phone = contact.Phone,
            Address = "عدن، عنوان تجريبي", Notes = "سجل تجريبي للتحقق", Revision = 1, Status = CustomerStatus.Active, SyncState = SyncState.Confirmed };
        var data = HomeStateTests.Data(); data.Quotations.Items = []; data.ReadyOrders = [];
        data.Customers = HomeStateTests.Section(0, new HomeItem { Id = customer.Id, Destination = HomeDestination.Customer, Title = customer.Name, State = "عميل" });
        if (manager) data.Customers.Availability = HomeAvailability.Unavailable;
        var engine = new FakeEngine { SupportedCapabilities = new HashSet<string> {
            ProtocolIds.Capabilities.EitmadCapabilityHomeV1, ProtocolIds.Capabilities.EitmadCapabilityCustomerV1,
            ProtocolIds.Capabilities.EitmadCapabilityOrdersV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1,
            ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1,
        } };
        engine.QueryHandler = query => SalesCatalogAuthorityTests.Response(query.Kind switch {
            Query.HomeReadKind => QueryResult.ForHome(data),
            Query.QuotationDraftListKind => QueryResult.ForQuotationDrafts(new() { Items = [] }),
            Query.QuotationListKind => QueryResult.ForQuotations(new() { Items = [order.Source], ServerAvailable = true }),
            Query.OrderListKind => QueryResult.ForOrders(new() { Items = [order], Pending = [], ServerAvailable = true }),
            Query.CustomerGetKind when query.AsCustomerGet()!.CustomerId == customer.Id => QueryResult.ForCustomer(customer),
            _ => throw new InvalidOperationException("Unexpected role search query."),
        });
        try {
            WpfTestHost.Run(width, height, window => {
                WpfTestHost.SignIn(window, manager ? "manager" : "receptionist"); Finish(window.AccountActivation);
                if (width == 1920) { window.Left = 0; window.Top = 0; window.WindowState = WindowState.Maximized; }
                var model = (HomeViewModel)window.ManagerHome.DataContext;
                var toolbar = manager ? window.ManagerTitleBar : window.ReceptionistSurface.ReceptionistTitleBar;
                var home = manager ? window.ManagerHome : WpfTestHost.FindByName<HomeView>(window.ReceptionistSurface, "ReceptionHome");
                WpfTestHost.CompleteLayout(window);
                var dpi = VisualTreeHelper.GetDpi(window);
                Console.WriteLine($"Role {(manager ? "Manager" : "Receptionist")}: display {SystemParameters.PrimaryScreenWidth * dpi.DpiScaleX:0}x{SystemParameters.PrimaryScreenHeight * dpi.DpiScaleY:0} px; requested {width}x{height}; actual {window.ActualWidth:0.0}x{window.ActualHeight:0.0} DIP; scaling {dpi.DpiScaleX * 100:0}%");
                WpfTestHost.Capture(window, $"role-search-hint-{(manager ? "manager" : "receptionist")}-{width}");
                Assert.AreEqual(!manager, toolbar.SearchPlaceholder.Contains("عميل") || toolbar.SearchPlaceholder.Contains("العملاء"));
                toolbar.SearchBox.Text = customer.Name;
                toolbar.SearchBox.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(toolbar.SearchBox), 0, Key.Enter) { RoutedEvent = Keyboard.KeyDownEvent });
                Finish(model.LastLoad); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, $"role-search-{(manager ? "manager" : "receptionist")}-{width}");
                if (manager) {
                    Assert.HasCount(0, model.Activity); Assert.IsTrue(model.IsEmpty);
                    Assert.IsFalse(model.Notice.Contains("العملاء"));
                    Assert.IsFalse(engine.WasQueried(Query.CustomerGetKind));
                    return;
                }
                var open = WpfTestHost.Descendants<Button>(home).Single(b => b.DataContext is HomeRow row && row.Item.Destination == HomeDestination.Customer);
                Assert.IsTrue(open.Focus());
                open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.CompleteLayout(window);
                var detail = WpfTestHost.FindByName<CustomerDetailView>(window.ReceptionistSurface, "CustomerDetail");
                Assert.IsTrue(detail.IsVisible);
                var shown = (CustomerPreview)detail.DataContext;
                Assert.AreEqual(customer.Id, shown.Id); Assert.AreEqual(customer.Phone, shown.Phone);
                Assert.AreEqual(customer.Address, shown.Address); Assert.AreEqual(customer.Notes, shown.Notes);
                Assert.AreEqual(order.Source.Number, shown.Quotations.Single().Number);
                Assert.AreEqual(order.Number, shown.Orders.Single().Number);
                Assert.HasCount(1, detail.QuotationsTable.Items); Assert.HasCount(1, detail.OrdersTable.Items);
                Assert.IsTrue(detail.BackButton.IsKeyboardFocusWithin);
                WpfTestHost.Capture(window, $"role-search-contact-history-{width}");
            }, showSignIn: true, engine: engine);
        }
        finally { Finish(engine.DisposeAsync().AsTask()); }
    }

    [TestMethod]
    [DataRow(true)]
    [DataRow(false)]
    public void SubmittedSearchAndClearUseTheVisibleRoleToolbar(bool manager)
    {
        var engine = HomeStateTests.Engine(HomeStateTests.Data());
        var queries = new List<string>();
        engine.QueryBarrier = query => { queries.Add(query.AsHomeRead()!.Term); return Task.CompletedTask; };
        try
        {
            WpfTestHost.Run(1338, 753, window => {
                var model = (HomeViewModel)window.ManagerHome.DataContext;
                Finish(model.ActivateAsync(includeCustomerSearch: !manager));
                window.ResponsiveRoot.Visibility = manager ? Visibility.Visible : Visibility.Collapsed;
                window.ReceptionistSurface.Visibility = manager ? Visibility.Collapsed : Visibility.Visible;
                WpfTestHost.CompleteLayout(window);
                var home = manager ? window.ManagerHome : WpfTestHost.FindByName<HomeView>(window.ReceptionistSurface, "ReceptionHome");
                var toolbar = manager ? window.ManagerTitleBar : window.ReceptionistSurface.ReceptionistTitleBar;
                toolbar.SearchBox.Text = "عميل QT-2026-00001";
                toolbar.SearchBox.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(toolbar.SearchBox), 0, Key.Enter) { RoutedEvent = Keyboard.KeyDownEvent });
                Finish(model.LastLoad);
                Assert.AreEqual(toolbar.SearchBox.Text, queries[^1]);
                WpfTestHost.FindByName<Button>(home, "ClearSearchButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
                Finish(model.LastLoad); WpfTestHost.CompleteLayout(window);
                Assert.AreEqual("", queries[^1]);
                Assert.AreEqual("", toolbar.SearchBox.Text);
                Assert.IsTrue(toolbar.SearchBox.IsKeyboardFocusWithin);
            }, engine: engine);
        }
        finally { Finish(engine.DisposeAsync().AsTask()); }
    }
    private static void Finish(Task task) { while (!task.IsCompleted) WpfTestHost.PumpDispatcher(); task.GetAwaiter().GetResult(); }
}
