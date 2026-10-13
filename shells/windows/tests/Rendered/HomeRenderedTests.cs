using System.Windows;
using System.Windows.Automation;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
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
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void ReceptionDailyWorkIsVisibleWithoutScrolling(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            using var state = new HomeScope();
            state.Load();
            if (width == 1920) window.WindowState = WindowState.Maximized;
            window.ResponsiveRoot.Visibility = Visibility.Collapsed;
            var reception = window.ReceptionistSurface;
            reception.Visibility = Visibility.Visible;
            reception.AttachHome(state.Model);
            WpfTestHost.CompleteLayout(window);
            var scroll = WpfTestHost.FindByName<ScrollViewer>(reception, "HomeContent");
            scroll.ScrollToTop(); WpfTestHost.CompleteLayout(window);
            var dpi = VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"UI-14: requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100}%.");
            WpfTestHost.Capture(window, $"ui14-reception-top-{width}");
            var home = WpfTestHost.FindByName<HomeView>(reception, "ReceptionHome");
            var metrics = WpfTestHost.FindByName<UniformGrid>(home, "MetricsGrid");
            var ready = WpfTestHost.Descendants<Button>(WpfTestHost.FindByName<Border>(home, "ReadyOrdersPanel"))
                .Single(b => b.DataContext is HomeRow);
            foreach (var element in new FrameworkElement[] { reception.NewQuotationAction, reception.ReceptionistActionsGrid, metrics, ready })
            {
                var bounds = element.TransformToAncestor(scroll).TransformBounds(new Rect(element.RenderSize));
                Assert.IsTrue(bounds.Top >= 0 && bounds.Bottom <= scroll.ViewportHeight,
                    $"{element.Name}: {bounds}; viewport height {scroll.ViewportHeight}. Daily work must be visible before scrolling.");
            }
            if (width == 1338)
            {
                foreach (var control in WpfTestHost.Descendants<Control>(scroll)) ControlOptions.SetHighContrast(control, true);
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "ui14-reception-system-colors");
            }
            if (width == 720)
            {
                foreach (var text in WpfTestHost.Descendants<TextBlock>(scroll)) text.FontSize *= 1.25;
                WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "ui14-reception-enlarged-text-top-720");
                ready.BringIntoView(); WpfTestHost.CompleteLayout(window);
                WpfTestHost.Capture(window, "ui14-reception-enlarged-text-ready-720");
            }
        });
    }

    [TestMethod]
    [DataRow(1920, 1080)]
    [DataRow(1338, 753)]
    [DataRow(720, 560)]
    public void HomeSectionResultsRenderWithoutFalseEmptyMessagesForBothRoles(int width, int height)
    {
        WpfTestHost.Run(width, height, window => {
            var data = HomeStateTests.Data();
            var engine = HomeStateTests.Engine(data);
            var model = new HomeViewModel(engine);
            try
            {
                Finish(model.ActivateAsync());
                if (width == 1920) window.WindowState = WindowState.Maximized;
                window.ManagerHome.DataContext = model;
                window.ReceptionistSurface.AttachHome(model);
                WpfTestHost.CompleteLayout(window);
                var dpi = VisualTreeHelper.GetDpi(window);
                Console.WriteLine($"UI-12: requested {width}x{height}; actual {window.ActualWidth}x{window.ActualHeight} DIP; display {SystemParameters.PrimaryScreenWidth}x{SystemParameters.PrimaryScreenHeight} DIP; scaling {dpi.DpiScaleX * 100}%.");
                foreach (var role in new[] { "manager", "reception" })
                {
                    var manager = role == "manager";
                    window.ResponsiveRoot.Visibility = manager ? Visibility.Visible : Visibility.Collapsed;
                    window.ReceptionistSurface.Visibility = manager ? Visibility.Collapsed : Visibility.Visible;
                    WpfTestHost.CompleteLayout(window);
                    var home = manager ? window.ManagerHome : WpfTestHost.FindByName<HomeView>(window.ReceptionistSurface, "ReceptionHome");
                    foreach (var result in new[] { "empty", "denied", "partial-denied", "unavailable" })
                    {
                        if (result != "unavailable") { data.Quotations = HomeStateTests.Section(); data.Orders = HomeStateTests.Section(); data.ReadyOrders = []; }
                        if (result == "denied") { data.Quotations.Availability = HomeAvailability.Denied; data.Orders.Availability = HomeAvailability.Denied; }
                        if (result == "partial-denied")
                        {
                            data.Quotations.Availability = HomeAvailability.Denied;
                            data.Orders = HomeStateTests.Data().Orders; data.Orders.Complete = false; data.Orders.ServerAvailable = false;
                            data.Orders.Items = HomeStateTests.Data().ReadyOrders; data.ReadyOrders = data.Orders.Items;
                        }
                        // First obtain rows, then fail the query, as in the reported defect.
                        engine.QueryHandler = _ => SalesCatalogAuthorityTests.Response(QueryResult.ForHome(data));
                        model.Refresh(); Finish(model.LastLoad);
                        if (result == "unavailable")
                        {
                            engine.QueryHandler = _ => throw new System.IO.IOException("Synthetic unavailable");
                            model.Refresh(); Finish(model.LastLoad);
                        }
                        WpfTestHost.CompleteLayout(window);
                        var activity = WpfTestHost.FindByName<TextBlock>(home, "ActivityMessage");
                        var ready = WpfTestHost.FindByName<TextBlock>(home, "ReadyOrdersMessage");
                        Assert.AreEqual(model.ActivityMessage, activity.Text); Assert.AreEqual(model.ReadyOrdersMessage, ready.Text);
                        Assert.AreEqual(result == "empty", activity.Text.Contains("لا توجد"));
                        Assert.AreEqual(result == "empty", ready.Text.Contains("لا توجد"));
                        Assert.AreEqual(result == "partial-denied" ? 1 : 0, model.ReadyOrders.Count);
                        Assert.IsTrue(WpfTestHost.Descendants<Button>(home).Any(b => Equals(b.Content, "إعادة تحميل البيانات") && b.IsVisible));
                        ready.BringIntoView(); WpfTestHost.CompleteLayout(window);
                        WpfTestHost.Capture(window, $"ui12-home-{role}-{result}-{width}-ready");
                        activity.BringIntoView(); WpfTestHost.CompleteLayout(window);
                        WpfTestHost.Capture(window, $"ui12-home-{role}-{result}-{width}-activity");
                    }
                }
            }
            finally { Finish(model.DisposeAsync().AsTask()); Finish(engine.DisposeAsync().AsTask()); }
        });
    }
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
            var dpi = VisualTreeHelper.GetDpi(window);
            Console.WriteLine($"Manager capture: requested {width} x {height}; actual {window.ActualWidth} x {window.ActualHeight}; display {SystemParameters.PrimaryScreenWidth} x {SystemParameters.PrimaryScreenHeight}; scaling {dpi.DpiScaleX * 100}%");
            WpfTestHost.Capture(window, $"home-manager-top-{width}");
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
        Console.WriteLine($"{role}: baseline {baseline}; window {window.ActualWidth} x {window.ActualHeight}; display {SystemParameters.PrimaryScreenWidth} x {SystemParameters.PrimaryScreenHeight}; scaling {dpi.DpiScaleX * 100}%");
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
