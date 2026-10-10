using System.Windows;
using System.Windows.Media;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Reception;

public partial class CurrentQuotationView : UserControl
{
    public CurrentQuotationView() => InitializeComponent();
    private SalesCatalogViewModel Model => (SalesCatalogViewModel)DataContext;
    private void ContinueClick(object sender, RoutedEventArgs e)
    {
        Model.IsReviewingQuotation = false;
        DependencyObject? parent = VisualTreeHelper.GetParent(this);
        while (parent is not null && parent is not SalesCatalogView) parent = VisualTreeHelper.GetParent(parent);
        (parent as SalesCatalogView)?.RestoreSelectionFocus();
    }
    private async void EditClick(object sender, RoutedEventArgs e)
    {
        Model.EditLine((PreviewQuotationLine)((Button)sender).DataContext);
        await Model.LastCatalogOperation;
        DependencyObject? parent = this;
        while (parent is not null && parent is not SalesCatalogView) parent = VisualTreeHelper.GetParent(parent);
        (parent as SalesCatalogView)?.FocusEditor();
    }
    private void DuplicateClick(object sender, RoutedEventArgs e) => Model.DuplicateLine((PreviewQuotationLine)((Button)sender).DataContext);
    private void RemoveClick(object sender, RoutedEventArgs e) { Model.QuotationLines.Remove((PreviewQuotationLine)((Button)sender).DataContext); ContinueButton.Focus(); }
    private void NewCustomerClick(object sender, RoutedEventArgs e) { if (!Model.IsNewCustomer) Model.BeginNewCustomer(); CustomerNameInput.Focus(); }
    private void AttachCustomerClick(object sender, RoutedEventArgs e) => Model.AttachCustomer((PreviewCustomer)((Button)sender).DataContext);
    private async void SaveCustomerClick(object sender, RoutedEventArgs e)
    {
        if (await Model.SaveNewCustomerAsync()) CustomerNameInput.Focus();
        else (Model.CustomerNameError.Length > 0 ? CustomerNameInput : PhoneInput).Focus();
    }
    private void CancelCustomerClick(object sender, RoutedEventArgs e) { Model.CancelNewCustomer(); CustomerNameInput.Focus(); }
    private async void RequestApprovalClick(object sender, RoutedEventArgs e) { Model.RequestDiscountApproval(); await Model.LastApprovalRequest; if (Model.IsDiscountPending) SaveDraftButton.Focus(); else FocusMissingField(); }
    private async void SaveDraftClick(object sender, RoutedEventArgs e) { if (!await Model.SaveDraftAsync()) FocusMissingField(); }
    private async void ReloadDraftClick(object sender, RoutedEventArgs e)
    {
        if (System.Windows.MessageBox.Show(Window.GetWindow(this), "ستُستبدل التعديلات غير المحفوظة بالنسخة المحفوظة. هل تريد المتابعة؟", "إعادة فتح المسودة", MessageBoxButton.YesNo, MessageBoxImage.Question, MessageBoxResult.No) != MessageBoxResult.Yes) return;
        await Model.ReloadDraftAsync(); ContinueButton.Focus();
    }
    private async void SaveQuotationClick(object sender, RoutedEventArgs e) { if (!await Model.IssueQuotationAsync()) FocusMissingField(); }
    private void FocusMissingField()
    {
        FrameworkElement target = Model.IsQuotationEmpty ? ContinueButton : Model.CustomerNameError.Length > 0 ? CustomerNameInput : Model.PhoneError.Length > 0 ? PhoneInput : DiscountInput;
        target.BringIntoView(); target.Focus();
    }
    private async void PrintPreviewClick(object sender, RoutedEventArgs e)
    {
        if (!Model.CanPreviewCustomer) return;
        var model = Model;
        var saved = model.IsLiveQuotation ? await model.ReadDocumentAsync() : null;
        if (model.IsLiveQuotation && saved is null) return;
        if (!model.IsLiveQuotation && !model.CheckRequiredFields()) { FocusMissingField(); return; }
        var preview = new Controls.PrintPreview { Document = saved is not null ? QuotationCustomerDocument.CreateSaved(saved) : QuotationCustomerDocument.Create(model, DateTime.Today), CanPrint = saved?.CanPrint == true };
        if (model.IsLiveQuotation) preview.AuthorizePrint = async () => {
            if (await model.ReadDocumentAsync() is not { CanPrint: true } current) return false;
            preview.Document = QuotationCustomerDocument.CreateSaved(current); return true;
        };
        var window = new Window
        {
            Title = "معاينة عرض السعر", Content = preview, Owner = Window.GetWindow(this),
            Width = 900, Height = 850, MinWidth = 640, MinHeight = 480,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            FlowDirection = System.Windows.FlowDirection.RightToLeft, Language = Language,
        };
        EventHandler invalidated = (_, _) => window.Close();
        model.DocumentInvalidated += invalidated;
        window.Closed += (_, _) => model.DocumentInvalidated -= invalidated;
        preview.BackRequested += (_, _) => window.Close();
        window.Loaded += (_, _) => { if (preview.CanPrint) preview.PrintButton.Focus(); else preview.BackButton.Focus(); };
        window.Closed += (_, _) => { PrintPreviewButton.BringIntoView(); PrintPreviewButton.Focus(); };
        window.ShowDialog();
    }
}
