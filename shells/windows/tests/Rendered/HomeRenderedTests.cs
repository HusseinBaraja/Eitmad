using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using Eitmad.WindowsShell.Controls;
using Eitmad.WindowsShell.Features.Home;
using Eitmad.WindowsShell.Features.Reception;
using Eitmad.WindowsShell.Tests.Home;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Tests.Orders;
using Eitmad.WindowsShell.Tests.Quotations;
using Eitmad.WindowsShell.Tests.Products;
using Eitmad.WindowsShell.Tests.TestDoubles;

namespace Eitmad.WindowsShell.Tests.Rendered;

[TestClass]
public sealed class HomeRenderedTests
{
    [TestMethod]
    public void HomeRowsOpenRustQuotationAndOrderDetailsWithoutSharedPreviewCollections()
    {
        var quote = new QuotationLifecycleTests.Fixture();
        var order = new OrderAuthorityTests.Fixture().Order;
        var data = HomeStateTests.Data();
        data.Quotations.Items[0].Id = quote.Record.Quotation.Id;
        data.ReadyOrders[0].Id = order.Id;
        using var engine = new EngineScope();
        engine.Value.QueryHandler = query => SalesCatalogAuthorityTests.Response(query.Kind switch {
            Query.HomeReadKind => QueryResult.ForHome(data),
            Query.QuotationDraftListKind => QueryResult.ForQuotationDrafts(new() { Items = [quote.Draft] }),
            Query.QuotationListKind => QueryResult.ForQuotations(new() { Items = [quote.Record], ServerAvailable = true }),
            Query.OrderGetKind or Query.OrderListKind => QueryResult.ForOrders(new() { Items = [order], Pending = [], ServerAvailable = true }),
            _ => throw new InvalidOperationException("Unexpected home destination query"),
        });
        WpfTestHost.Run(1338, 753, window => {
            var model = (HomeViewModel)window.ManagerHome.DataContext;
            Finish(model.ActivateAsync()); WpfTestHost.CompleteLayout(window);
            Open(window.ManagerHome, HomeDestination.Quotation);
            WaitFor(() => window.QuotationsSurface.ViewModel.SelectedQuotation is not null,
                () => $"{window.QuotationsSurface.ViewModel.ListState}; {window.InteractionToast.Message}; rows {string.Join(',', window.QuotationsSurface.ViewModel.PreviewQuotations.Select(q => q.Id))}; requested {quote.Record.Quotation.Id}");
            Assert.AreEqual(quote.Record.Quotation.Id, window.QuotationsSurface.ViewModel.SelectedQuotation!.Id);
            Assert.IsTrue(window.QuotationsSurface.IsVisible);
            Open(window.ManagerHome, HomeDestination.Order);
            WaitFor(() => window.OrdersSurface.ViewModel.SelectedOrder is not null);
            Assert.AreEqual(order.Id, window.OrdersSurface.ViewModel.SelectedOrder!.Id);
            Assert.IsTrue(window.OrdersSurface.IsVisible);
            Assert.IsFalse(ReferenceEquals(window.OrdersSurface.ViewModel.PreviewOrders, window.ReceptionistSurface.PreviewOrders.ViewModel.PreviewOrders));
            window.ResponsiveRoot.Visibility = Visibility.Collapsed;
            window.ReceptionistSurface.Visibility = Visibility.Visible;
            WpfTestHost.CompleteLayout(window);
            var receptionHome = WpfTestHost.FindByName<HomeView>(window.ReceptionistSurface, "ReceptionHome");
            Open(receptionHome, HomeDestination.Quotation);
            WaitFor(() => window.ReceptionistSurface.ReceptionQuotations.ViewModel.SelectedQuotation is not null);
            Assert.AreEqual(quote.Record.Quotation.Id, window.ReceptionistSurface.ReceptionQuotations.ViewModel.SelectedQuotation!.Id);
            Open(receptionHome, HomeDestination.Order);
            WaitFor(() => window.ReceptionistSurface.PreviewOrders.ViewModel.SelectedOrder is not null);
            Assert.AreEqual(order.Id, window.ReceptionistSurface.PreviewOrders.ViewModel.SelectedOrder!.Id);
            Assert.IsTrue(window.ReceptionistSurface.PreviewOrders.IsVisible);
        }, engine: engine.Value);
    }
    private static void Open(HomeView home, HomeDestination destination)
    {
        var button = WpfTestHost.Descendants<Button>(home).First(b => b.DataContext is HomeRow row && row.Item.Destination == destination);
        button.RaiseEvent(new RoutedEventArgs(Button.ClickEvent)); WpfTestHost.PumpDispatcher();
    }
    private static void Finish(Task task) { while (!task.IsCompleted) WpfTestHost.PumpDispatcher(); task.GetAwaiter().GetResult(); }
    private static void WaitFor(Func<bool> complete, Func<string>? details = null)
    {
        var deadline = DateTime.UtcNow.AddSeconds(3);
        while (!complete() && DateTime.UtcNow < deadline) WpfTestHost.PumpDispatcher();
        Assert.IsTrue(complete(), "The home destination did not open the saved record. " + details?.Invoke());
    }
    private sealed class EngineScope : IDisposable
    {
        public FakeEngine Value { get; } = new() { SupportedCapabilities = new HashSet<string> {
            ProtocolIds.Capabilities.EitmadCapabilityHomeV1, ProtocolIds.Capabilities.EitmadCapabilityOrdersV1,
            ProtocolIds.Capabilities.EitmadCapabilityQuotationDraftV1, ProtocolIds.Capabilities.EitmadCapabilityQuotationLifecycleV1,
        } };
        public void Dispose() => Finish(Value.DisposeAsync().AsTask());
    }
    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ScopedHomeRendersForBothRolesAndActionsHaveKeyboardNames(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            using var state = new HomeScope();
            state.Load();
            if (width == 1920) { window.Left = 0; window.Top = 0; window.WindowState = WindowState.Maximized; }
            window.ManagerHome.DataContext = state.Model;
            window.ManagerTitleBar.ApprovalCount = state.Model.ApprovalCount;
            WpfTestHost.CompleteLayout(window);
            Verify(window, window.ManagerHome, "manager", width);
            if (width == 1338) {
                ControlOptions.SetHighContrast(window.DashboardSurface, true); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "home-manager-high-contrast");
                ControlOptions.SetHighContrast(window.DashboardSurface, false);
            }
            var action = WpfTestHost.Descendants<Button>(window).First(b => Equals(b.Tag, "الموافقات") && b.IsVisible);
            Assert.IsTrue(action.Focus()); action.RaiseEvent(new RoutedEventArgs(Button.ClickEvent));
            WpfTestHost.CompleteLayout(window);
            Assert.IsTrue(window.QuotationsSurface.IsVisible); Assert.IsTrue(window.QuotationsSurface.ViewModel.ApprovalsOnly);
            window.ManagerSidebar.SelectHome();
            window.QuotationsSurface.Visibility = Visibility.Collapsed;
            window.ResponsiveRoot.Visibility = Visibility.Collapsed;
            window.ReceptionistSurface.Visibility = Visibility.Visible;
            var reception = window.ReceptionistSurface;
            reception.AttachHome(state.Model);
            WpfTestHost.CompleteLayout(window);
            var receptionHome = WpfTestHost.FindByName<HomeView>(reception, "ReceptionHome");
            Verify(window, receptionHome, "reception", width);
            ControlOptions.SetHighContrast(reception, true); WpfTestHost.CompleteLayout(window);
            if (width == 1338) WpfTestHost.Capture(window, "home-reception-high-contrast");
            Assert.IsTrue(WpfTestHost.Descendants<TextBlock>(receptionHome).Any(t => t.Text == "3" || t.Text == "2"));
        });
    }
    private static void Verify(MainWindow window, HomeView view, string role, int baseline)
    {
        var dpi = VisualTreeHelper.GetDpi(window);
        Console.WriteLine($"{role}: baseline {baseline}; window {window.ActualWidth} x {window.ActualHeight}; scaling {dpi.DpiScaleX * 100}%");
        view.BringIntoView(); WpfTestHost.CompleteLayout(window);
        Assert.AreEqual(FlowDirection.RightToLeft, view.FlowDirection);
        var button = WpfTestHost.Descendants<Button>(view).First(b => b.DataContext is HomeRow);
        button.BringIntoView(); WpfTestHost.CompleteLayout(window);
        Assert.IsTrue(button.Focus()); Assert.IsTrue(button.IsKeyboardFocusWithin);
        Assert.IsFalse(string.IsNullOrWhiteSpace(AutomationProperties.GetName(button)));
        Assert.IsTrue(view.ActualWidth <= window.ActualWidth);
        WpfTestHost.Capture(window, $"home-{role}-{baseline}");
    }
    private sealed class HomeScope : IDisposable
    {
        private readonly TestDoubles.FakeEngine engine = HomeStateTests.Engine(HomeStateTests.Data());
        public HomeViewModel Model { get; }
        public HomeScope() => Model = new(engine);
        public void Load() { var task = Model.ActivateAsync(); while (!task.IsCompleted) WpfTestHost.PumpDispatcher(); task.GetAwaiter().GetResult(); }
        public void Dispose() { var task = Model.DisposeAsync().AsTask(); while (!task.IsCompleted) WpfTestHost.PumpDispatcher(); task.GetAwaiter().GetResult(); engine.DisposeAsync().AsTask().GetAwaiter().GetResult(); }
    }
}
