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
    public event Action<Guid>? CustomerRequested;

    private void CustomerClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.IsReceptionist && ViewModel.SelectedQuotation is { } quotation) CustomerRequested?.Invoke(quotation.Id);
    }
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
            ShowPreviewWindow(new Features.Reception.SalesCatalogView { DataContext = createPreview(null), ShowQuotationHeader = true }, "عرض سعر جديد — معاينة فقط");
    }

    private void EditQuotationClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { CanEdit: true } quotation || createPreview is null) return;
        ShowPreviewWindow(new Features.Reception.SalesCatalogView { DataContext = createPreview(quotation), ShowQuotationHeader = true }, "تعديل عرض السعر — معاينة فقط");
    }

    private void PrintQuotationClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { CanPrint: true } quotation || createPreview is null) return;
        var preview = new PrintPreview { Document = Features.Reception.QuotationCustomerDocument.CreateExistingPreview(createPreview(quotation), quotation.Date.ToDateTime(TimeOnly.MinValue)) };
        ShowPreviewWindow(preview, "معاينة عرض السعر");
    }

    private void ConvertQuotationClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { CanEdit: true } quotation) return;
        ConversionDialog.DataContext = quotation;
        ConversionDialog.IsOpen = true;
    }

    private void CancelConversionClick(object sender, RoutedEventArgs e) => ConversionDialog.IsOpen = false;

    private void ConfirmConversionClick(object sender, RoutedEventArgs e)
    {
        if (!ConversionDialog.IsOpen || ConversionDialog.DataContext is not QuotationListItem { CanEdit: true } quotation) return;
        ConversionDialog.IsOpen = false;
        var view = new Features.Orders.OrdersView();
        view.ConfigureReceptionist();
        view.ViewModel.OpenOrder(new(Guid.NewGuid(), "معاينة غير محفوظة", quotation.Customer,
            DateOnly.FromDateTime(DateTime.Today), Features.Orders.OrderStatus.New, quotation.Discount,
            quotation.Items.Select(line => new Features.Orders.OrderLineItem(line.FurnitureName, line.Variant, "—", line.Color, line.Handle, line.Quantity, line.UnitPrice)).ToArray(), quotation.Phone, quotation));
        ShowPreviewWindow(view, "تفاصيل الطلب — معاينة فقط، لم يتم حفظ طلب");
    }

    private void OpenLinkedOrderClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { IsConverted: true } quotation) return;
        var view = new Features.Orders.OrdersView();
        view.ConfigureReceptionist();
        view.ViewModel.OpenOrder(view.ViewModel.VisibleOrders.Single(order => order.OriginalQuotation?.Id == quotation.Id));
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
            if (content is Features.Reception.SalesCatalogView catalog)
            {
                if (((Features.Reception.SalesCatalogViewModel)catalog.DataContext).IsReviewingQuotation) catalog.RestoreQuotationFocus();
                else catalog.RestoreCatalogFocus();
            }
            if (content is Features.Orders.OrdersView orders) orders.BackToOrdersButton.Focus();
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
