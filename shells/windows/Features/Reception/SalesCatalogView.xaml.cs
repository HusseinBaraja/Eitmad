using System.Windows;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class SalesCatalogView : UserControl
{
    public static readonly DependencyProperty ShowQuotationHeaderProperty = DependencyProperty.Register(
        nameof(ShowQuotationHeader), typeof(bool), typeof(SalesCatalogView), new PropertyMetadata(false));
    public bool ShowQuotationHeader { get => (bool)GetValue(ShowQuotationHeaderProperty); set => SetValue(ShowQuotationHeaderProperty, value); }
    public SalesCatalogView() => InitializeComponent();
    private void ClearClick(object sender, RoutedEventArgs e)
    {
        ((SalesCatalogViewModel)DataContext).ClearFilters();
        CatalogSearch.Focus();
    }
    private Button? lastSelectionButton;
    public void RestoreCatalogFocus() => Dispatcher.BeginInvoke(new Action(() => CatalogSearch.Focus()));
    public void RestoreQuotationFocus() => Dispatcher.BeginInvoke(new Action(() => QuotationView.ContinueButton.Focus()));
    public void RestoreSelectionFocus()
    {
        if (((SalesCatalogViewModel)DataContext).IsReviewingQuotation) RestoreQuotationFocus();
        else Dispatcher.BeginInvoke(new Action(() =>
        {
            var model = (SalesCatalogViewModel)DataContext;
            if (model.IsSelecting) (model.IsSelectingProduct ? ProductSelectionView.BackButton : SelectionView.BackButton).Focus();
            else if (lastSelectionButton?.IsVisible == true) lastSelectionButton.Focus();
            else CatalogSearch.Focus();
        }));
    }
    public void FocusEditor() => Dispatcher.BeginInvoke(new Action(() => (((SalesCatalogViewModel)DataContext).IsSelectingProduct ? ProductSelectionView.BackButton : SelectionView.BackButton).Focus()));
    private void OpenQuotationClick(object sender, RoutedEventArgs e)
    {
        ((SalesCatalogViewModel)DataContext).IsReviewingQuotation = true;
        RestoreQuotationFocus();
    }
    private void SelectClick(object sender, RoutedEventArgs e)
    {
        if (sender is Button { DataContext: SalesCatalogItem item })
        {
            lastSelectionButton = (Button)sender;
            ((SalesCatalogViewModel)DataContext).Select(item);
            if (((SalesCatalogViewModel)DataContext).IsSelecting)
                FocusEditor();
        }
    }
}
