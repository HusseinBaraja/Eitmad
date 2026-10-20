using System.Windows;
using System.Windows.Controls;
using System.Windows.Automation;
using System.Windows.Input;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.Customers;
using Eitmad.WindowsShell.Tests.Customers;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class ManagerCustomersRenderedTests
{
    [TestMethod]
    public void ManagerHomeOpensCustomerBranchAndAccountSwitchClearsDirectory()
    {
        var customer = CustomerDirectoryTests.Contact(Guid.NewGuid());
        var home = Home.HomeStateTests.Data(); home.Quotations.Items = []; home.ReadyOrders = [];
        home.Customers = Home.HomeStateTests.Section(1, new HomeItem { Id = customer.Id, BranchId = customer.Scope.Id, Destination = HomeDestination.Customer, Title = customer.Name, State = "عميل" });
        var engine = new FakeEngine { SupportedCapabilities = new HashSet<string> {
            ProtocolIds.Capabilities.EitmadCapabilityHomeV1, ProtocolIds.Capabilities.EitmadCapabilityCustomerV1,
        } }; engine.Connect();
        engine.QueryHandler = query => CustomerDirectoryTests.Result(query.Kind switch {
            Query.HomeReadKind => QueryResult.ForHome(home),
            Query.QuotationDraftListKind => QueryResult.ForQuotationDrafts(new() { Items = [] }),
            Query.QuotationListKind => QueryResult.ForQuotations(new() { Items = [], ServerAvailable = true }),
            Query.OrderListKind => QueryResult.ForOrders(new() { Items = [], Pending = [], ServerAvailable = true }),
            Query.CustomerSearchKind => QueryResult.ForCustomers(new() { Items = [customer] }),
            Query.CustomerGetKind when query.AsCustomerGet()!.BranchId == customer.Scope.Id => QueryResult.ForCustomer(customer),
            Query.CustomerHistoryKind => QueryResult.ForCustomerHistory(new() { Quotations = [], Orders = [], ServerAvailable = true }),
            _ => throw new InvalidOperationException("Unexpected query"),
        });
        WpfTestHost.Run(1338, 753, window => {
            WpfTestHost.SignIn(window, "manager"); Finish(window.AccountActivation);
            var model = (Features.Home.HomeViewModel)window.ManagerHome.DataContext;
            model.Search("عميل"); Finish(model.LastLoad); WpfTestHost.CompleteLayout(window);
            var open = WpfTestHost.Descendants<Button>(window.ManagerHome).First(b => b.DataContext is Features.Home.HomeRow row && row.Item.Destination == HomeDestination.Customer);
            open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            var directory = window.CustomersSurface.ViewModel!;
            var deadline = DateTime.UtcNow.AddSeconds(3);
            while (!directory.HasDetail && DateTime.UtcNow < deadline) WpfTestHost.PumpDispatcher();
            Assert.IsTrue(directory.HasDetail); Assert.AreEqual(customer.Scope.Id.ToString(), directory.Selected!.BranchId);
            directory.Term = "عميل";
            MainWindow.SwitchAccountCommand.Execute(null, window);
            window.ConfirmAccountSwitchButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(window.AccountTransition);
            Assert.IsNull(directory.Selected); Assert.HasCount(0, directory.Rows); Assert.AreEqual("", directory.Term);
            Assert.AreEqual(Visibility.Collapsed, window.CustomersSurface.Visibility); Assert.AreEqual(0, engine.SubscriptionCount);
        }, showSignIn: true, engine: engine);
        Finish(engine.DisposeAsync().AsTask());
    }
    [TestMethod]
    [DataRow(1536, 864)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ManagerFindsBothBranchesAndOpensReadOnlyDetail(int width, int height)
    {
        var first = CustomerDirectoryTests.Contact(Guid.NewGuid(), "مؤسسة الاعتماد للأثاث — عميل تجريبي");
        var second = CustomerDirectoryTests.Contact(Guid.NewGuid(), "إعتماد القيسي — الفرع الثاني");
        var engine = new FakeEngine();
        engine.QueryHandler = query => CustomerDirectoryTests.Result(query.Kind switch {
            Query.CustomerSearchKind => QueryResult.ForCustomers(new() { Items = [first, second] }),
            Query.CustomerGetKind => QueryResult.ForCustomer(second),
            Query.CustomerHistoryKind => QueryResult.ForCustomerHistory(new() { Quotations = [], Orders = [], ServerAvailable = true }),
            _ => throw new InvalidOperationException("Unexpected query"),
        });
        WpfTestHost.Run(width, height, window => {
            window.CustomersNavButtonForTest();
            var view = WpfTestHost.FindByName<CustomerDirectoryView>(window, "CustomersSurface");
            var model = view.ViewModel!; Finish(model.LastLoad);
            WpfTestHost.CompleteLayout(window);
            Assert.AreEqual(Visibility.Collapsed, view.Detail.Visibility);
            Assert.IsTrue(WpfTestHost.FindByName<ListBox>(view, "Results").IsVisible);
            Assert.HasCount(2, model.Rows); Assert.AreNotEqual(model.Rows[0].Scope.Id, model.Rows[1].Scope.Id);
            var search = WpfTestHost.FindByName<TextBox>(view, "SearchInput");
            search.Focus(); search.Text = "اعتماد";
            search.RaiseEvent(new KeyEventArgs(Keyboard.PrimaryDevice, PresentationSource.FromVisual(search), 0, Key.Enter) { RoutedEvent = Keyboard.KeyDownEvent });
            Finish(model.LastLoad); WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(AutomationProperties.GetName(search).Contains("بحث"));
            Assert.IsTrue(search.IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, $"manager-customers-list-{width}x{height}");
            var open = WpfTestHost.FindByAutomationName<Button>(view, second.Name);
            open.Focus(); open.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(model.LastLoad);
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(model.HasDetail); Assert.IsFalse(model.Selected!.CanEdit);
            Assert.AreEqual(Visibility.Collapsed, WpfTestHost.FindByName<Button>(view.Detail, "EditButton").Visibility);
            Assert.IsTrue(view.Detail.BackButton.IsKeyboardFocusWithin);
            WpfTestHost.Capture(window, $"manager-customers-detail-{width}x{height}");
            view.Detail.BackButton.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); Finish(model.LastLoad);
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(search.IsKeyboardFocusWithin);
            if (width == 1338) {
                view.Resources[SystemParameters.HighContrastKey] = true;
                foreach (var control in WpfTestHost.Descendants<Control>(view)) Eitmad.WindowsShell.Controls.ControlOptions.SetHighContrast(control, true);
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "manager-customers-high-contrast");
                foreach (var control in WpfTestHost.Descendants<Control>(view)) Eitmad.WindowsShell.Controls.ControlOptions.SetHighContrast(control, false);
                view.Resources.Remove(SystemParameters.HighContrastKey);
            }
            model.Clear(); Assert.IsNull(model.Selected); Assert.HasCount(0, model.Rows);
            Assert.AreEqual("", model.Term);
        }, engine: engine);
        Finish(engine.DisposeAsync().AsTask());
    }
    private static void Finish(Task task) { while (!task.IsCompleted) WpfTestHost.PumpDispatcher(); task.GetAwaiter().GetResult(); }
}

internal static class ManagerCustomerNavigation
{
    internal static void CustomersNavButtonForTest(this MainWindow window) =>
        WpfTestHost.FindByName<Button>(window, "CustomersNavButton").RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
}
