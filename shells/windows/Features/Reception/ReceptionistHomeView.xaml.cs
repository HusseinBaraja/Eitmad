using System.Windows;
using System.Windows.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class ReceptionistHomeView : UserControl
{
    public ReceptionistHomeView() => InitializeComponent();

    public event EventHandler? AccountSwitchRequested;

    public void SetCatalogSources(Features.Furniture.FurnitureViewModel furniture, Features.Products.ProductsViewModel products)
    {
        var catalog = new SalesCatalogViewModel(furniture, products);
        CatalogContent.DataContext = catalog;
        ((Button)ReceptionistSidebar.FindName("QuotationsNavButton")).Visibility = Visibility.Visible;
        ReceptionQuotations.ConfigureReceptionist(quotation => QuotationPreviewProjection.Create(quotation, furniture, products));
        ReceptionistTitleBar.PrimaryActionButton.Width = 220;
        System.Windows.Automation.AutomationProperties.SetName(ReceptionistTitleBar.PrimaryActionButton, "فتح عرض السعر");
        ReceptionistTitleBar.SetBinding(Controls.ShellTitleBar.PrimaryActionLabelProperty,
            new System.Windows.Data.Binding(nameof(SalesCatalogViewModel.QuotationLabel)) { Source = catalog });
    }

    private void SharedAccountSwitchRequested(object? sender, EventArgs eventArgs) =>
        AccountSwitchRequested?.Invoke(this, EventArgs.Empty);

    private void TitleBarActionRequested(object? sender, Controls.ShellActionEventArgs eventArgs)
    {
        if (eventArgs.IsPrimary)
        {
            if (!CatalogContent.IsVisible) Navigate("المنتجات");
            ((SalesCatalogViewModel)CatalogContent.DataContext).IsReviewingQuotation = true;
            return;
        }
        ShowPreviewFeedback(eventArgs.Action);
    }

    private void TitleBarSearchSubmitted(object? sender, Controls.ShellSearchEventArgs eventArgs) =>
        ShowNotice($"نتائج المعاينة عن: {eventArgs.Query}");

    private void SidebarNavigationRequested(object? sender, Controls.NavigationRequestedEventArgs eventArgs) =>
        Navigate(eventArgs.Destination);

    private void Navigate(string destination)
    {
        if (destination is not ("المنتجات" or "الرئيسية" or "عروض الأسعار"))
        {
            ShowNotice($"تم اختيار {destination} في وضع المعاينة");
            return;
        }
        var catalog = destination == "المنتجات";
        if (catalog) ((SalesCatalogViewModel)CatalogContent.DataContext).Reload();
        CatalogContent.Visibility = catalog ? Visibility.Visible : Visibility.Collapsed;
        ReceptionQuotations.Visibility = destination == "عروض الأسعار" ? Visibility.Visible : Visibility.Collapsed;
        HomeContent.Visibility = destination == "الرئيسية" ? Visibility.Visible : Visibility.Collapsed;
        ReceptionistTitleBar.Title = destination;
        ReceptionistSidebar.SelectDestination(destination);
    }

    private void ActionClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is not Button { Tag: string action })
        {
            return;
        }

        ShowPreviewFeedback(action);
    }

    private void ShowPreviewFeedback(string action)
    {
        if (action is "المنتجات" or "عروض الأسعار") { Navigate(action); return; }
        if (action == "عرض سعر جديد") { Navigate("المنتجات"); ((SalesCatalogViewModel)CatalogContent.DataContext).IsReviewingQuotation = true; CatalogContent.RestoreQuotationFocus(); return; }
        ShowNotice($"تم اختيار {action} في وضع المعاينة");
    }

    private void ShowNotice(string message)
    {
        ReceptionistNotice.Message = message;
        ReceptionistNotice.RestartDuration();
    }

    private void DismissNotice(object sender, RoutedEventArgs eventArgs) => ReceptionistNotice.Message = string.Empty;
}

