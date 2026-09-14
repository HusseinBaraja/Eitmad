using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using Eitmad.WindowsShell.Features.Authentication;
using Eitmad.WindowsShell.Features.Operations;
using Button = System.Windows.Controls.Button;

namespace Eitmad.WindowsShell;

public partial class MainWindow : Window
{
    private PreviewAccountRole currentRole = PreviewAccountRole.Manager;

    public static RoutedUICommand SwitchAccountCommand { get; } = new(
        "تبديل الحساب التجريبي",
        nameof(SwitchAccountCommand),
        typeof(MainWindow));

    /// <summary>Initializes the dashboard preview and its transient interactions.</summary>
    public MainWindow(OperationsViewModel viewModel, bool showSignIn = true)
    {
        InitializeComponent();
        ReceptionistSurface.SetCatalogSources(FurnitureSurface.ViewModel, ProductsSurface.ViewModel);
        DataContext = viewModel;
        SignInSurface.Visibility = showSignIn ? Visibility.Visible : Visibility.Collapsed;
        ResponsiveRoot.Visibility = showSignIn ? Visibility.Collapsed : Visibility.Visible;
        Title = showSignIn ? "الاعتماد · تسجيل الدخول" : "الاعتماد · لوحة التحكم";
    }

    /// <summary>Shows the shell surface that belongs to the selected preview account.</summary>
    private void PreviewSignedIn(object sender, PreviewSignedInEventArgs eventArgs)
    {
        SignInSurface.Visibility = Visibility.Collapsed;
        ShowAccount(eventArgs.Role);
    }

    /// <summary>Allows the development-only account switch after preview sign-in.</summary>
    private void CanSwitchAccount(object sender, CanExecuteRoutedEventArgs eventArgs) =>
        eventArgs.CanExecute = SignInSurface.Visibility != Visibility.Visible;

    /// <summary>Handles the Alt+K development shortcut.</summary>
    private void SwitchAccountExecuted(object sender, ExecutedRoutedEventArgs eventArgs) => SwitchAccount();

    /// <summary>Handles the receptionist header account switch.</summary>
    private void ReceptionistAccountSwitchRequested(object? sender, EventArgs eventArgs) => SwitchAccount();

    private void ManagerTitleBarAccountSwitchRequested(object? sender, EventArgs eventArgs) => SwitchAccount();

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

    private void SwitchAccount()
    {
        if (SignInSurface.Visibility == Visibility.Visible)
        {
            return;
        }

        ShowAccount(currentRole == PreviewAccountRole.Manager
            ? PreviewAccountRole.Receptionist
            : PreviewAccountRole.Manager);
    }

    private void ShowAccount(PreviewAccountRole role)
    {
        currentRole = role;
        var showManager = role == PreviewAccountRole.Manager;
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
        var showRawMaterials = destination == "الخامات";
        var showParts = destination == "القطع";
        var showFurniture = destination == "الأثاث";
        var showPricing = destination == "التسعير";
        var showProducts = destination == "المنتجات";
        var showQuotations = destination == "عروض الأسعار";
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
        if (sender is Button { Tag: string action }) ShowToast($"تم اختيار {action}");
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
