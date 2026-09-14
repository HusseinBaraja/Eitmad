using System.Windows;
using System.Windows.Media;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class CurrentQuotationView : UserControl
{
    public CurrentQuotationView() => InitializeComponent();
    private SalesCatalogViewModel Model => (SalesCatalogViewModel)DataContext;
    private void ContinueClick(object sender, RoutedEventArgs e) { Model.IsReviewingQuotation = false; }
    private void EditClick(object sender, RoutedEventArgs e)
    {
        Model.EditLine((PreviewQuotationLine)((Button)sender).DataContext);
        DependencyObject parent = this;
        while (parent is not SalesCatalogView) parent = VisualTreeHelper.GetParent(parent);
        ((SalesCatalogView)parent).FocusEditor();
    }
    private void DuplicateClick(object sender, RoutedEventArgs e) => Model.DuplicateLine((PreviewQuotationLine)((Button)sender).DataContext);
    private void RemoveClick(object sender, RoutedEventArgs e) { Model.RemoveLine((PreviewQuotationLine)((Button)sender).DataContext); ContinueButton.Focus(); }
    private void NewCustomerClick(object sender, RoutedEventArgs e) { if (!Model.IsNewCustomer) Model.BeginNewCustomer(); CustomerNameInput.Focus(); }
    private void AttachCustomerClick(object sender, RoutedEventArgs e) => Model.AttachCustomer((PreviewCustomer)((Button)sender).DataContext);
    private void SaveCustomerClick(object sender, RoutedEventArgs e) { Model.SaveNewCustomer(); CustomerNameInput.Focus(); }
    private void CancelCustomerClick(object sender, RoutedEventArgs e) { Model.CancelNewCustomer(); CustomerNameInput.Focus(); }
    private void SaveQuotationClick(object sender, RoutedEventArgs e) { if (!Model.ReviewSave()) { CustomerNameInput.BringIntoView(); CustomerNameInput.Focus(); } }
}
