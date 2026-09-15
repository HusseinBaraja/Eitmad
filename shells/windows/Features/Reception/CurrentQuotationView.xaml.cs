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
    private void RequestApprovalClick(object sender, RoutedEventArgs e) { Model.RequestDiscountApproval(); SaveDraftButton.Focus(); }
    private void SaveDraftClick(object sender, RoutedEventArgs e) => Model.ReviewDraftSave();
    private void SaveQuotationClick(object sender, RoutedEventArgs e) { if (!Model.ReviewSave()) FocusMissingField(); }
    private void FocusMissingField()
    {
        FrameworkElement target = Model.IsQuotationEmpty ? ContinueButton : Model.CustomerNameError.Length > 0 ? CustomerNameInput : Model.PhoneError.Length > 0 ? PhoneInput : DiscountInput;
        target.BringIntoView(); target.Focus();
    }
    private void PrintPreviewClick(object sender, RoutedEventArgs e)
    {
        if (!Model.CanPreviewCustomer) return;
        if (!Model.CheckRequiredFields()) { FocusMissingField(); return; }
        var preview = new Controls.PrintPreview { Document = QuotationCustomerDocument.Create(Model, DateTime.Today) };
        var window = new Window
        {
            Title = "معاينة عرض السعر", Content = preview, Owner = Window.GetWindow(this),
            Width = 900, Height = 850, MinWidth = 640, MinHeight = 480,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            FlowDirection = System.Windows.FlowDirection.RightToLeft, Language = Language,
        };
        preview.BackRequested += (_, _) => window.Close();
        window.Loaded += (_, _) => preview.PrintButton.Focus();
        window.Closed += (_, _) => { PrintPreviewButton.BringIntoView(); PrintPreviewButton.Focus(); };
        window.ShowDialog();
    }
}
