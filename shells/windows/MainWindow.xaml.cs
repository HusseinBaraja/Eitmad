using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Features.Operations;
using Eitmad.Platform.Windows.Shell;
using Button = System.Windows.Controls.Button;

namespace Eitmad.WindowsShell;

public partial class MainWindow : Window
{
    private readonly IDesktopSessionController? sessions;
    private readonly Features.Customers.CustomerClient? customerClient;
    private bool sessionActive;
    private bool switchingAccount;

    public static RoutedUICommand SwitchAccountCommand { get; } = new(
        "تبديل الحساب",
        nameof(SwitchAccountCommand),
        typeof(MainWindow));

    public event EventHandler<SessionEndReason?>? AccountSessionCleared;

    /// <summary>Initializes the dashboard preview and its transient interactions.</summary>
    public MainWindow(
        OperationsViewModel viewModel,
        IDesktopSessionController? sessions = null,
        bool showSignIn = true,
        IEngineShellBridge? engine = null)
    {
        InitializeComponent();
        this.sessions = sessions;
        if (engine is not null)
        {
            UsersSurface.Attach(engine);
            customerClient = new Features.Customers.CustomerClient(engine);
            ReceptionistSurface.AttachCustomerClient(customerClient);
        }
        if (sessions is not null) SignInSurface.AuthenticateAsync = sessions.SignInAsync;
        if (sessions is not null) sessions.SessionEnded += SessionEnded;
        Closed += (_, _) =>
        {
            if (this.sessions is not null) this.sessions.SessionEnded -= SessionEnded;
            if (customerClient is not null) _ = customerClient.DisposeAsync();
        };
        ReceptionistSurface.SetCatalogSources(FurnitureSurface.ViewModel, ProductsSurface.ViewModel);
        QuotationsSurface.ViewModel.UsePreviewQuotations(ReceptionistSurface.Handoffs.Quotations);
        var receptionOrders = ReceptionistSurface.PreviewOrders.ViewModel;
        OrdersSurface.ViewModel.UsePreviewOrders(receptionOrders.PreviewOrders);
        WorkOrdersSurface.ViewModel.UseOrderFixtures(receptionOrders.PreviewOrders);
        OrdersSurface.ViewModel.FindProduction = WorkOrdersSurface.ViewModel.ForOrder;
        OrdersSurface.ProductionRequested += order =>
        {
            WorkOrdersSurface.ViewModel.OpenOrderProduction(order);
            ManagerSidebar.SelectDestination("أوامر العمل");
            ShowDestination("أوامر العمل");
            Dispatcher.BeginInvoke(WorkOrdersSurface.BackToWorkOrdersButton.Focus, DispatcherPriority.Input);
        };
        WorkOrdersSurface.OrderRequested += number =>
        {
            var order = receptionOrders.PreviewOrders.FirstOrDefault(item => item.Number == number);
            if (order is null) return;
            OrdersSurface.ViewModel.OpenOrder(order);
            ManagerSidebar.SelectDestination("الطلبات");
            ShowDestination("الطلبات");
            Dispatcher.BeginInvoke(OrdersSurface.BackToOrdersButton.Focus, DispatcherPriority.Input);
        };
        WorkOrdersSurface.ViewModel.PreviewStatusChanged += work => receptionOrders.PreviewProductionStatus(work.OrderNumber,
            work.IsCompleted ? Features.Orders.OrderStatus.Ready : Features.Orders.OrderStatus.InProduction, work.Number);
        DataContext = viewModel;
        SignInSurface.Visibility = showSignIn ? Visibility.Visible : Visibility.Collapsed;
        ResponsiveRoot.Visibility = showSignIn ? Visibility.Collapsed : Visibility.Visible;
        Title = showSignIn ? "الاعتماد · تسجيل الدخول" : "الاعتماد · لوحة التحكم";
    }

    /// <summary>Shows the shell surface allowed by Rust-returned effective permissions.</summary>
    private async void SessionSignedIn(object sender, AuthenticatedSurface surface)
    {
        SignInSurface.Visibility = Visibility.Collapsed;
        sessionActive = true;
        ShowAccount(surface);
        if (surface == AuthenticatedSurface.Receptionist) await ReceptionistSurface.ActivateCustomersAsync();
    }

    /// <summary>Allows sign-out and account switching only during an active user session.</summary>
    private void CanSwitchAccount(object sender, CanExecuteRoutedEventArgs eventArgs) =>
        eventArgs.CanExecute = sessionActive && !switchingAccount && sessions is not null;

    /// <summary>Handles the Alt+K account-switch shortcut.</summary>
    private void SwitchAccountExecuted(object sender, ExecutedRoutedEventArgs eventArgs) => _ = SwitchAccountAsync();

    /// <summary>Handles the receptionist header account switch.</summary>
    private void ReceptionistAccountSwitchRequested(object? sender, EventArgs eventArgs) => _ = SwitchAccountAsync();

    private void ManagerTitleBarAccountSwitchRequested(object? sender, EventArgs eventArgs) => _ = SwitchAccountAsync();

    private void ManagerSidebarNavigationRequested(object? sender, Controls.NavigationRequestedEventArgs eventArgs) =>
        ShowDestination(eventArgs.Destination);

    private void ManagerTitleBarActionRequested(object? sender, Controls.ShellActionEventArgs eventArgs)
    {
        if (eventArgs.IsPrimary)
        {
            OpenPreviewPanel(eventArgs.Action);
            return;
        }

        ShowToast($"تم اختيار {eventArgs.Action}");
    }

    private void ManagerTitleBarSearchSubmitted(object? sender, Controls.ShellSearchEventArgs eventArgs) =>
        ShowToast($"نتائج المعاينة عن: {eventArgs.Query}");

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
            await ReceptionistSurface.DeactivateCustomersAsync();
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
            InteractionPanel.Visibility = Visibility.Collapsed;
            return;
        }

        ManagerSidebar.SelectHome();
        ShowDestination("الرئيسية");
    }

    private void SessionEnded(object? sender, SessionEndedEventArgs eventArgs) =>
        Dispatcher.Invoke(() =>
        {
            _ = ReceptionistSurface.DeactivateCustomersAsync();
            ShowSignIn(eventArgs.Reason);
        });

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
        ResponsiveRoot.Visibility = Visibility.Collapsed;
        ReceptionistSurface.Visibility = Visibility.Collapsed;
        InteractionPanel.Visibility = Visibility.Collapsed;
    }

    /// <summary>Opens the raw-material list from the dashboard shortcut.</summary>
    private void OpenRawMaterialsFromActionClick(object sender, RoutedEventArgs eventArgs)
    {
        ManagerSidebar.SelectDestination("الخامات");
        ShowDestination("الخامات");
    }

    /// <summary>Opens the parts list from the dashboard shortcut.</summary>
    private void OpenPartsFromActionClick(object sender, RoutedEventArgs eventArgs)
    {
        ManagerSidebar.SelectDestination("القطع");
        ShowDestination("القطع");
    }

    /// <summary>Switches between the dashboard preview and dedicated management pages.</summary>
    private void ShowDestination(string destination)
    {
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
        PartsSurface.Visibility = showParts ? Visibility.Visible : Visibility.Collapsed;
        FurnitureSurface.Visibility = showFurniture ? Visibility.Visible : Visibility.Collapsed;
        PricingSurface.Visibility = showPricing ? Visibility.Visible : Visibility.Collapsed;
        ProductsSurface.Visibility = showProducts ? Visibility.Visible : Visibility.Collapsed;
        QuotationsSurface.Visibility = showQuotations ? Visibility.Visible : Visibility.Collapsed;
        OrdersSurface.Visibility = showOrders ? Visibility.Visible : Visibility.Collapsed;
        WorkOrdersSurface.Visibility = showWorkOrders ? Visibility.Visible : Visibility.Collapsed;
        UsersSurface.Visibility = showUsers ? Visibility.Visible : Visibility.Collapsed;
        if (!showUsers && !showRawMaterials && !showParts && !showFurniture && !showPricing && !showProducts && !showQuotations && !showOrders && !showWorkOrders)
        {
            ManagerTitleBar.Title = destination == "الرئيسية" ? "لوحة التحكم" : destination;
            ShowToast($"تم فتح {destination} في وضع المعاينة");
        }
    }

    private void PreviewActionClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is not Button { Tag: string action }) return;
        if (action == "الموافقات")
        {
            QuotationsSurface.ViewModel.OpenApprovals();
            ManagerSidebar.SelectDestination("عروض الأسعار");
            ShowDestination("الموافقات");
            Dispatcher.BeginInvoke(QuotationsSurface.QuotationSearchBox.Focus, DispatcherPriority.Input);
            return;
        }
        ShowToast($"تم اختيار {action}");
    }

    private void OpenPreviewPanel(string title)
    {
        PreviewPanelTitle.Text = title;
        InteractionPanel.Visibility = Visibility.Visible;
        Dispatcher.BeginInvoke(CustomerNameBox.Focus, DispatcherPriority.Input);
    }

    /// <summary>Closes the quotation preview panel without saving state.</summary>
    private void ClosePreviewPanelClick(object sender, RoutedEventArgs eventArgs) =>
        InteractionPanel.Visibility = Visibility.Collapsed;

    /// <summary>Validates the preview customer name without creating a quotation.</summary>
    private void PreviewSubmitClick(object sender, RoutedEventArgs eventArgs)
    {
        if (string.IsNullOrWhiteSpace(CustomerNameBox.Text))
        {
            ShowToast("أدخل اسم العميل للمتابعة");
            CustomerNameBox.Focus();
            return;
        }

        InteractionPanel.Visibility = Visibility.Collapsed;
        ShowToast("تم فحص المسودة محلياً؛ الحفظ معطل في وضع المعاينة");
    }

    /// <summary>Shows transient preview feedback.</summary>
    private void ShowToast(string message)
    {
        InteractionToast.Message = message;
        InteractionToast.RestartDuration();
    }

    private void DismissToast(object sender, RoutedEventArgs e) => InteractionToast.Message = string.Empty;

}
