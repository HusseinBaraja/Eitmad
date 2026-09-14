using System.Windows;
using System.Windows.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class ReceptionistHomeView : UserControl
{
    public ReceptionistHomeView() => InitializeComponent();

    public event EventHandler? AccountSwitchRequested;

    public void SetCatalogSources(Features.Furniture.FurnitureViewModel furniture, Features.Products.ProductsViewModel products) =>
        CatalogContent.DataContext = new SalesCatalogViewModel(furniture, products);

    private void SharedAccountSwitchRequested(object? sender, EventArgs eventArgs) =>
        AccountSwitchRequested?.Invoke(this, EventArgs.Empty);

    private void TitleBarActionRequested(object? sender, Controls.ShellActionEventArgs eventArgs) =>
        ShowPreviewFeedback(eventArgs.Action);

    private void TitleBarSearchSubmitted(object? sender, Controls.ShellSearchEventArgs eventArgs) =>
        ShowNotice($"نتائج المعاينة عن: {eventArgs.Query}");

    private void SidebarNavigationRequested(object? sender, Controls.NavigationRequestedEventArgs eventArgs) =>
        Navigate(eventArgs.Destination);

    private void Navigate(string destination)
    {
        if (destination is not ("المنتجات" or "الرئيسية"))
        {
            ShowNotice($"تم اختيار {destination} في وضع المعاينة");
            return;
        }
        var catalog = destination == "المنتجات";
        if (catalog) ((SalesCatalogViewModel)CatalogContent.DataContext).Reload();
        CatalogContent.Visibility = catalog ? Visibility.Visible : Visibility.Collapsed;
        HomeContent.Visibility = catalog ? Visibility.Collapsed : Visibility.Visible;
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
        if (action == "المنتجات") { Navigate(action); return; }
        ShowNotice(action == "عرض سعر جديد"
            ? "واجهة إنشاء عرض السعر ستُربط في المرحلة التالية"
            : $"تم اختيار {action} في وضع المعاينة");
    }

    private void ShowNotice(string message)
    {
        ReceptionistNotice.Message = message;
        ReceptionistNotice.RestartDuration();
    }

    private void DismissNotice(object sender, RoutedEventArgs eventArgs) => ReceptionistNotice.Message = string.Empty;
}
