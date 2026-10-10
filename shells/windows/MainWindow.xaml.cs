using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.Platform.Windows.Shell;
using Button = System.Windows.Controls.Button;

namespace Eitmad.WindowsShell;

public partial class MainWindow : Window
{
    private readonly IDesktopSessionController? sessions;
    private readonly Features.Customers.CustomerClient? customerClient;
    private readonly Features.Quotations.QuotationDraftClient? quotationDraftClient;
    private readonly Features.Orders.OrderClient? orderClient;
    private readonly Features.Home.HomeViewModel? home;
    private long accountGeneration;
    private bool sessionActive;
    private bool switchingAccount;

    public static RoutedUICommand SwitchAccountCommand { get; } = new(
        "تبديل الحساب",
        nameof(SwitchAccountCommand),
        typeof(MainWindow));

    public event EventHandler<SessionEndReason?>? AccountSessionCleared;

    public MainWindow(
        IDesktopSessionController? sessions = null,
        bool showSignIn = true,
        IEngineShellBridge? engine = null,
        bool preview = false)
    {
        InitializeComponent();
        if (preview && engine is null) {
            QuotationsSurface.UsePreviewFixtures(); OrdersSurface.UsePreviewFixtures(); WorkOrdersSurface.UsePreviewFixtures();
        }
        this.sessions = sessions;
        if (engine is not null)
        {
            UsersSurface.Attach(engine);
            RawMaterialsSurface.Attach(engine);
            PartsSurface.Attach(engine);
            FurnitureSurface.Attach(engine);
            ProductsSurface.Attach(engine);
            PricingSurface.Attach(engine);
            customerClient = new Features.Customers.CustomerClient(engine);
            ReceptionistSurface.AttachCustomerClient(customerClient);
            home = new(engine);
            ManagerHome.DataContext = home;
            ReceptionistSurface.AttachHome(home);
            ManagerHome.OpenRequested += row => _ = OpenHomeItemAsync(row);
            ManagerTitleBar.SetBinding(Controls.ShellTitleBar.ApprovalCountProperty,
                new System.Windows.Data.Binding(nameof(Features.Home.HomeViewModel.ApprovalCount)) { Source = home });
        }
        if (sessions is not null) SignInSurface.AuthenticateAsync = sessions.SignInAsync;
        if (sessions is not null) sessions.SessionEnded += SessionEnded;
        Closed += (_, _) =>
        {
            if (this.sessions is not null) this.sessions.SessionEnded -= SessionEnded;
            if (customerClient is not null) _ = customerClient.DisposeAsync();
            _ = RawMaterialsSurface.DisposeAsync();
            _ = PartsSurface.DisposeAsync();
            _ = FurnitureSurface.DisposeAsync();
            _ = ProductsSurface.DisposeAsync();
            _ = PricingSurface.DisposeAsync();
            _ = ReceptionistSurface.DisposeCatalogAsync();
            if (quotationDraftClient is not null) _ = quotationDraftClient.DisposeAsync();
            if (orderClient is not null) _ = orderClient.DisposeAsync();
            if (home is not null) _ = home.DisposeAsync();
        };
        ReceptionistSurface.SetCatalogSources(FurnitureSurface.ViewModel, ProductsSurface.ViewModel, preview && engine is null);
        if (engine is not null) ReceptionistSurface.AttachCatalog(engine);
        if (engine is null && preview) QuotationsSurface.ViewModel.UsePreviewQuotations(ReceptionistSurface.Handoffs.Quotations);
        else if (engine is not null)
        {
            quotationDraftClient = new(engine);
            ReceptionistSurface.AttachDraftClient(quotationDraftClient);
            QuotationsSurface.ViewModel.AttachDraftClient(quotationDraftClient);
        }
        var receptionOrders = ReceptionistSurface.PreviewOrders.ViewModel;
        if (engine is null && preview) OrdersSurface.ViewModel.UsePreviewOrders(receptionOrders.PreviewOrders);
        if (engine is not null) {
            orderClient = new(engine);
            OrdersSurface.ViewModel.Attach(orderClient); receptionOrders.Attach(orderClient); WorkOrdersSurface.ViewModel.Attach(orderClient);
            ReceptionistSurface.ReceptionQuotations.ViewModel.AttachOrders(orderClient);
            ReceptionistSurface.ReceptionQuotations.ViewModel.OrderConfirmed += order => ReceptionistSurface.OpenConfirmedOrder(order);
        }
        if (engine is null && preview) WorkOrdersSurface.ViewModel.UseOrderFixtures(receptionOrders.PreviewOrders);
        OrdersSurface.ViewModel.FindProduction = WorkOrdersSurface.ViewModel.ForOrder;
        OrdersSurface.ProductionRequested += async order =>
        {
            WorkOrdersSurface.ViewModel.OpenOrderProduction(order);
            ManagerSidebar.SelectDestination("أوامر العمل");
            ShowDestination("أوامر العمل");
            await WorkOrdersSurface.ViewModel.LastLoad;
            if (WorkOrdersSurface.IsVisible && WorkOrdersSurface.ViewModel.IsDetailVisible)
                await Dispatcher.InvokeAsync(WorkOrdersSurface.BackToWorkOrdersButton.Focus, DispatcherPriority.Input);
        };
        WorkOrdersSurface.OrderRequested += async number =>
        {
            if (engine is not null && WorkOrdersSurface.ViewModel.SelectedWorkOrder?.Record is { } record) {
                OpenManagerDestination("الطلبات");
                await OrdersSurface.ViewModel.OpenByIdAsync(record.OrderId);
                if (OrdersSurface.IsVisible) OrdersSurface.BackToOrdersButton.Focus();
                return;
            }
            var order = (engine is null ? receptionOrders : OrdersSurface.ViewModel).PreviewOrders.FirstOrDefault(item => item.Number == number);
            if (order is null) return;
            OrdersSurface.ViewModel.OpenOrder(order);
            ManagerSidebar.SelectDestination("الطلبات");
            ShowDestination("الطلبات");
            _ = Dispatcher.BeginInvoke(OrdersSurface.BackToOrdersButton.Focus, DispatcherPriority.Input);
        };
        if (preview && engine is null) WorkOrdersSurface.ViewModel.PreviewStatusChanged += work => receptionOrders.PreviewProductionStatus(work.OrderNumber,
            work.IsCompleted ? Features.Orders.OrderStatus.Ready : Features.Orders.OrderStatus.InProduction, work.Number);
        SignInSurface.Visibility = showSignIn ? Visibility.Visible : Visibility.Collapsed;
        ResponsiveRoot.Visibility = showSignIn ? Visibility.Collapsed : Visibility.Visible;
        Title = showSignIn ? "الاعتماد · تسجيل الدخول" : "الاعتماد · لوحة التحكم";
    }

    private async void SessionSignedIn(object sender, AuthenticatedSurface surface)
    {
        SignInSurface.Visibility = Visibility.Collapsed;
        sessionActive = true;
        var session = ++accountGeneration;
        ShowAccount(surface);
        if (home is not null) await home.ActivateAsync();
        if (session != accountGeneration) return;
        if (surface == AuthenticatedSurface.Manager) await QuotationsSurface.ViewModel.ActivateDraftsAsync();
        else await ReceptionistSurface.ReceptionQuotations.ViewModel.ActivateDraftsAsync();
        if (session != accountGeneration) return;
        if (surface == AuthenticatedSurface.Manager) { await OrdersSurface.ViewModel.ActivateAsync(); if (session != accountGeneration) return; await WorkOrdersSurface.ViewModel.ActivateAsync(); }
        else await ReceptionistSurface.PreviewOrders.ViewModel.ActivateAsync();
        if (session != accountGeneration) return;
        if (surface != AuthenticatedSurface.Receptionist) return;
        try
        {
            await ReceptionistSurface.ActivateCustomersAsync();
            if (session != accountGeneration) return;
            await ReceptionistSurface.ActivateCatalogAsync();
        }
        catch (Exception error) when (error is Eitmad.Platform.Windows.LocalIpc.EngineIpcException
            or IOException or ObjectDisposedException)
        {
            ShowToast(Features.Customers.CustomerClient.ArabicMessage(Features.Customers.CustomerFailureKind.Unavailable));
        }
    }

    private void CanSwitchAccount(object sender, CanExecuteRoutedEventArgs eventArgs) =>
        eventArgs.CanExecute = sessionActive && !switchingAccount && sessions is not null;

    private void SwitchAccountExecuted(object sender, ExecutedRoutedEventArgs eventArgs) => _ = SwitchAccountAsync();

    private void ReceptionistAccountSwitchRequested(object? sender, EventArgs eventArgs) => _ = SwitchAccountAsync();

    private void ManagerTitleBarAccountSwitchRequested(object? sender, EventArgs eventArgs) => _ = SwitchAccountAsync();

    private void ManagerSidebarNavigationRequested(object? sender, Controls.NavigationRequestedEventArgs eventArgs) =>
        ShowDestination(eventArgs.Destination);

    private void ManagerTitleBarActionRequested(object? sender, Controls.ShellActionEventArgs eventArgs)
    {
        if (eventArgs.IsPrimary || eventArgs.Action == "التنبيهات") OpenManagerDestination("الموافقات");
        else ShowToast("الرسائل غير متاحة ضمن سير العمل الحالي.");
    }

    private void ManagerTitleBarSearchSubmitted(object? sender, Controls.ShellSearchEventArgs eventArgs)
    {
        OpenManagerDestination("الرئيسية"); home?.Search(eventArgs.Query);
    }

    /// <summary>Clears receptionist projections and stops session adapters before switching accounts.</summary>
    private async Task SwitchAccountAsync()
    {
        if (!sessionActive || switchingAccount || sessions is null)
        {
            return;
        }
        switchingAccount = true;
        CommandManager.InvalidateRequerySuggested();
        HideAccountSurfaces();
        try
        {
            if (home is not null) await home.DeactivateAsync();
            await ReceptionistSurface.DeactivateCustomersAsync();
            await ReceptionistSurface.DeactivateCatalogAsync();
            if (quotationDraftClient is not null) await quotationDraftClient.DeactivateAsync();
            await sessions.SignOutAsync();
            ShowSignIn();
        }
        catch (Eitmad.Platform.Windows.LocalIpc.EngineIpcException)
        {
            ShowSignIn(SessionEndReason.ConnectionLost);
        }
        finally
        {
            switchingAccount = false;
            CommandManager.InvalidateRequerySuggested();
        }
    }

    private void ShowAccount(AuthenticatedSurface surface)
    {
        var showManager = surface == AuthenticatedSurface.Manager;
        ResponsiveRoot.Visibility = showManager ? Visibility.Visible : Visibility.Collapsed;
        ReceptionistSurface.Visibility = showManager ? Visibility.Collapsed : Visibility.Visible;
        Title = showManager ? "الاعتماد · لوحة التحكم" : "الاعتماد · الرئيسية";

        if (!showManager)
        {
            return;
        }

        ManagerSidebar.SelectHome();
        ShowDestination("الرئيسية");
    }

    private void SessionEnded(object? sender, SessionEndedEventArgs eventArgs) =>
        Dispatcher.Invoke(() => _ = CompleteSessionEndAsync(eventArgs.Reason));

    private async Task CompleteSessionEndAsync(SessionEndReason reason)
    {
        SignInSurface.IsEnabled = false;
        try
        {
            HideAccountSurfaces();
            if (home is not null) await home.DeactivateAsync();
            await ReceptionistSurface.DeactivateCustomersAsync();
            await ReceptionistSurface.DeactivateCatalogAsync();
            if (quotationDraftClient is not null) await quotationDraftClient.DeactivateAsync();
        }
        finally
        {
            ShowSignIn(reason);
            SignInSurface.IsEnabled = true;
        }
    }

    private void ShowSignIn(SessionEndReason? reason = null)
    {
        sessionActive = false;
        HideAccountSurfaces();
        SignInSurface.Reset(reason);
        SignInSurface.Visibility = Visibility.Visible;
        Title = "الاعتماد · تسجيل الدخول";
        AccountSessionCleared?.Invoke(this, reason);
    }

    private void HideAccountSurfaces()
    {
        ++accountGeneration;
        home?.Clear();
        ManagerTitleBar.SearchBox.Clear(); ReceptionistSurface.ClearHomeSession();
        WorkOrdersSurface.ViewModel.ClearWorkOrders();
        OrdersSurface.ViewModel.ClearOrders(); ReceptionistSurface.PreviewOrders.ViewModel.ClearOrders();
        if (orderClient is not null) _ = orderClient.DeactivateAsync();
        QuotationsSurface.ViewModel.ClearDrafts();
        ReceptionistSurface.ReceptionQuotations.ViewModel.ClearDrafts();
        PartsSurface.ClearSession();
        FurnitureSurface.ClearSession();
        ProductsSurface.ClearSession();
        PricingSurface.ClearSession();
        ResponsiveRoot.Visibility = Visibility.Collapsed;
        ReceptionistSurface.Visibility = Visibility.Collapsed;
    }

    private void OpenRawMaterialsFromActionClick(object sender, RoutedEventArgs eventArgs)
    {
        ManagerSidebar.SelectDestination("الخامات");
        ShowDestination("الخامات");
    }

    private void OpenPartsFromActionClick(object sender, RoutedEventArgs eventArgs)
    {
        ManagerSidebar.SelectDestination("القطع");
        ShowDestination("القطع");
    }

    private void ShowDestination(string destination)
    {
        if (destination is not ("الرئيسية" or "الخامات" or "القطع" or "الأثاث" or "التسعير" or "المنتجات" or "عروض الأسعار" or "الموافقات" or "الطلبات" or "المستخدمون" or "أوامر العمل"))
        { ShowToast("هذه الميزة غير متاحة ضمن سير العمل الحالي."); return; }
        if (destination == "عروض الأسعار") QuotationsSurface.ViewModel.ApprovalsOnly = false;
        var showRawMaterials = destination == "الخامات";
        var showParts = destination == "القطع";
        var showFurniture = destination == "الأثاث";
        var showPricing = destination == "التسعير";
        var showProducts = destination == "المنتجات";
        var showQuotations = destination is "عروض الأسعار" or "الموافقات";
        var showOrders = destination == "الطلبات";
        var showUsers = destination == "المستخدمون";
        var showWorkOrders = destination == "أوامر العمل";
        DashboardSurface.Visibility = showRawMaterials || showParts || showFurniture || showPricing || showProducts || showQuotations || showOrders || showWorkOrders || showUsers ? Visibility.Collapsed : Visibility.Visible;
        RawMaterialsSurface.Visibility = showRawMaterials ? Visibility.Visible : Visibility.Collapsed;
        if (showRawMaterials) _ = RawMaterialsSurface.ActivateAsync();
        PartsSurface.Visibility = showParts ? Visibility.Visible : Visibility.Collapsed;
        if (showParts) _ = PartsSurface.ActivateAsync();
        FurnitureSurface.Visibility = showFurniture ? Visibility.Visible : Visibility.Collapsed;
        if (showFurniture) _ = FurnitureSurface.ActivateAsync();
        PricingSurface.Visibility = showPricing ? Visibility.Visible : Visibility.Collapsed;
        if (showPricing) _ = PricingSurface.ActivateAsync();
        ProductsSurface.Visibility = showProducts ? Visibility.Visible : Visibility.Collapsed;
        if (showProducts) _ = ProductsSurface.ActivateAsync();
        QuotationsSurface.Visibility = showQuotations ? Visibility.Visible : Visibility.Collapsed;
        OrdersSurface.Visibility = showOrders ? Visibility.Visible : Visibility.Collapsed;
        WorkOrdersSurface.Visibility = showWorkOrders ? Visibility.Visible : Visibility.Collapsed;
        UsersSurface.Visibility = showUsers ? Visibility.Visible : Visibility.Collapsed;
        if (!showUsers && !showRawMaterials && !showParts && !showFurniture && !showPricing && !showProducts && !showQuotations && !showOrders && !showWorkOrders)
        {
            ManagerTitleBar.Title = destination == "الرئيسية" ? "لوحة التحكم" : destination;
            home?.Search("");
        }
    }

    private void HomeActionClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is not Button { Tag: string action }) return;
        OpenManagerDestination(action);
    }

    private void OpenManagerDestination(string destination)
    {
        if (destination == "الموافقات") QuotationsSurface.ViewModel.OpenApprovals();
        ManagerSidebar.SelectDestination(destination == "الموافقات" ? "عروض الأسعار" : destination);
        ShowDestination(destination);
        if (destination is "عروض الأسعار" or "الموافقات") Dispatcher.BeginInvoke(QuotationsSurface.QuotationSearchBox.Focus, DispatcherPriority.Input);
    }

    private async Task OpenHomeItemAsync(Features.Home.HomeRow row)
    {
        var session = accountGeneration;
        switch (row.Item.Destination) {
            case Eitmad.Contracts.HomeDestination.Quotation:
                OpenManagerDestination("عروض الأسعار");
                await QuotationsSurface.ViewModel.ActivateDraftsAsync();
                if (session != accountGeneration) return;
                var quotation = QuotationsSurface.ViewModel.PreviewQuotations.FirstOrDefault(q => q.Id == row.Item.Id);
                if (quotation is null) { ShowToast("عرض السعر غير متاح. أعد تحميل البيانات."); return; }
                QuotationsSurface.ViewModel.OpenQuotation(quotation);
                QuotationsSurface.BackToQuotationsButton.Focus();
                break;
            case Eitmad.Contracts.HomeDestination.Order:
                OpenManagerDestination("الطلبات");
                await OrdersSurface.ViewModel.OpenByIdAsync(row.Item.Id);
                if (session == accountGeneration) OrdersSurface.BackToOrdersButton.Focus();
                break;
            case Eitmad.Contracts.HomeDestination.Customer:
                ShowToast("البحث الشامل في عملاء المؤسسة غير متاح حالياً.");
                break;
            case Eitmad.Contracts.HomeDestination.Catalog:
                OpenManagerDestination("التسعير"); PricingSurface.ViewModel.SearchText = row.Title;
                break;
        }
    }

    private void ShowToast(string message)
    {
        InteractionToast.Message = message;
        InteractionToast.RestartDuration();
    }

    private void DismissToast(object sender, RoutedEventArgs e) => InteractionToast.Message = string.Empty;

}
