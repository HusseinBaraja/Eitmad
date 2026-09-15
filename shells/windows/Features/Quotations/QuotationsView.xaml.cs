using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Eitmad.WindowsShell.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Quotations;

public partial class QuotationsView : UserControl
{
    public QuotationsView()
    {
        InitializeComponent();
        ViewModel = new QuotationsViewModel();
        DataContext = ViewModel;
    }

    public QuotationsViewModel ViewModel { get; private set; }
    private Func<QuotationListItem?, Features.Reception.SalesCatalogViewModel>? createPreview;

    public void ConfigureReceptionist(Func<QuotationListItem?, Features.Reception.SalesCatalogViewModel> factory)
    {
        createPreview = factory;
        DetailStatusBadge.HorizontalAlignment = System.Windows.HorizontalAlignment.Left;
        QuotationTable.Columns.Single(column => (string)column.Header == "الخصم").Visibility = Visibility.Collapsed;
        ViewModel = new QuotationsViewModel(true);
        DataContext = ViewModel;
    }

    private void NewQuotationClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.IsReceptionist && createPreview is not null)
            ShowPreviewWindow(new Features.Reception.SalesCatalogView { DataContext = createPreview(null) }, "عرض سعر جديد — معاينة فقط");
    }

    private void EditQuotationClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { CanEdit: true } quotation || createPreview is null) return;
        ShowPreviewWindow(new Features.Reception.SalesCatalogView { DataContext = createPreview(quotation) }, "تعديل عرض السعر — معاينة فقط");
    }

    private void PrintQuotationClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { CanPrint: true } quotation || createPreview is null) return;
        var preview = new PrintPreview { Document = Features.Reception.QuotationCustomerDocument.CreateExistingPreview(createPreview(quotation), quotation.Date.ToDateTime(TimeOnly.MinValue)) };
        ShowPreviewWindow(preview, "معاينة عرض السعر");
    }

    private void ConvertQuotationClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { CanEdit: true }) return;
        ReceptionActionNotice.Text = "التحويل إلى طلب غير متاح في المعاينة — لم يتم إنشاء طلب أو تغيير عرض السعر";
    }

    private void OpenLinkedOrderClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { IsConverted: true } quotation) return;
        var view = new Features.Orders.OrdersView();
        view.ViewModel.OpenOrder(new(Guid.Parse("6374de1e-f4db-49f5-a9d0-01b37a588280"), "ORD-2026-0079", quotation.Customer,
            quotation.Date, Features.Orders.OrderStatus.New, quotation.Discount,
            quotation.Items.Select(line => new Features.Orders.OrderLineItem(line.FurnitureName, line.Variant, "—", line.Color, line.Handle, line.Quantity, line.UnitPrice)).ToArray()));
        ShowPreviewWindow(view, "الطلب المرتبط — بيانات تجريبية");
    }

    private void ShowPreviewWindow(UserControl content, string title)
    {
        var window = new Window { Title = title, Content = content, Owner = Window.GetWindow(this), Width = 1050, Height = 780,
            MinWidth = 640, MinHeight = 480, WindowStartupLocation = WindowStartupLocation.CenterOwner,
            FlowDirection = System.Windows.FlowDirection.RightToLeft, Language = Language };
        if (content is PrintPreview print) print.BackRequested += (_, _) => window.Close();
        window.Loaded += (_, _) =>
        {
            if (content is PrintPreview printContent) printContent.PrintButton.Focus();
            if (content is Features.Reception.SalesCatalogView catalog) catalog.RestoreQuotationFocus();
        };
        window.Closed += (_, _) => { if (ViewModel.IsListVisible) QuotationSearchBox.Focus(); else BackToQuotationsButton.Focus(); };
        window.ShowDialog();
    }

    private void QuotationRowInvoked(object sender, RowInvokedEventArgs eventArgs) =>
        OpenQuotation((QuotationListItem)eventArgs.Item);

    private void OpenQuotation(QuotationListItem quotation)
    {
        ReceptionActionNotice.Text = string.Empty;
        ViewModel.OpenQuotation(quotation);
        Dispatcher.BeginInvoke(BackToQuotationsButton.Focus, DispatcherPriority.Input);
    }

    private void OpenQuotationClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: QuotationListItem quotation })
        {
            OpenQuotation(quotation);
        }
    }

    private void BackToListClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.CloseQuotation();
        Dispatcher.BeginInvoke(QuotationSearchBox.Focus, DispatcherPriority.Input);
    }

    private void ApproveDiscountClick(object sender, RoutedEventArgs eventArgs) => ViewModel.ApproveDiscount();

    private void RejectDiscountClick(object sender, RoutedEventArgs eventArgs) => ViewModel.RejectDiscount();
}
