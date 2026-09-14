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
    private Button? lastSelectionButton;
    public void RestoreSelectionFocus() => Dispatcher.BeginInvoke(new Action(() => lastSelectionButton?.Focus()));
    private void CloseQuotationClick(object sender, RoutedEventArgs e) => ((SalesCatalogViewModel)DataContext).IsReviewingQuotation = false;
    private void SelectClick(object sender, RoutedEventArgs e)
    {
        if (sender is Button { DataContext: SalesCatalogItem item })
        {
            lastSelectionButton = (Button)sender;
            ((SalesCatalogViewModel)DataContext).Select(item);
            if (((SalesCatalogViewModel)DataContext).IsSelecting)
                Dispatcher.BeginInvoke(new Action(() => SelectionView.BackButton.Focus()));
        }
    }
}
