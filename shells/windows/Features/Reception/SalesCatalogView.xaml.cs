using System.Windows;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class SalesCatalogView : UserControl
{
    public SalesCatalogView() => InitializeComponent();
    private void ClearClick(object sender, RoutedEventArgs e)
    {
        ((SalesCatalogViewModel)DataContext).ClearFilters();
        CatalogSearch.Focus();
    }
    private void SelectClick(object sender, RoutedEventArgs e)
    {
        if (sender is Button { DataContext: SalesCatalogItem item })
            ((SalesCatalogViewModel)DataContext).Select(item);
    }
}
