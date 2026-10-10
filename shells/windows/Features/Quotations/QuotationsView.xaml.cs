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
        ViewModel.DocumentsInvalidated += (_, _) => CloseEditors();
        DataContext = ViewModel;
    }

    public QuotationsViewModel ViewModel { get; private set; }
    public event Action<Guid>? CustomerRequested;

    private void CustomerClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.IsReceptionist && ViewModel.SelectedQuotation is { } quotation) CustomerRequested?.Invoke(quotation.Id);
    }
    private Func<QuotationListItem?, Features.Reception.SalesCatalogViewModel>? createPreview;
    public Func<QuotationListItem?, Task<Features.Reception.SalesCatalogViewModel?>>? LiveEditorFactory { get; set; }
    private readonly List<Window> editorWindows = [];
    private bool openingEditor;
    public void CloseEditors() { foreach (var window in editorWindows.ToArray()) window.Close(); }
    private async Task OpenLiveEditorAsync(QuotationListItem? row)
    {
        if (LiveEditorFactory is null || openingEditor) return;
        openingEditor = true;
        try
        {
            var editor = await LiveEditorFactory(row);
            if (editor is null) return;
            try { ShowPreviewWindow(new Features.Reception.SalesCatalogView { DataContext = editor, ShowQuotationHeader = true }, row is null ? "مسودة عرض سعر جديدة" : "تعديل مسودة عرض السعر"); }
            finally { await editor.DisposeEditorAsync(); }
        }
        finally { openingEditor = false; }
    }

    public void ConfigureReceptionist(Func<QuotationListItem?, Features.Reception.SalesCatalogViewModel> factory)
    {
        createPreview = factory;
        DetailStatusBadge.HorizontalAlignment = System.Windows.HorizontalAlignment.Left;
        QuotationTable.Columns.Single(column => (string)column.Header == "الخصم").Visibility = Visibility.Collapsed;
        ViewModel = new QuotationsViewModel(true);
        ViewModel.DocumentsInvalidated += (_, _) => CloseEditors();
        DataContext = ViewModel;
    }

    private async void NewQuotationClick(object sender, RoutedEventArgs e)
    {
        if (LiveEditorFactory is not null) { await OpenLiveEditorAsync(null); return; }
        if (ViewModel.IsReceptionist && createPreview is not null)
            ShowPreviewWindow(new Features.Reception.SalesCatalogView { DataContext = createPreview(null), ShowQuotationHeader = true }, "عرض سعر جديد — معاينة فقط");
    }

    private async void EditQuotationClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { CanEdit: true } quotation || createPreview is null) return;
        if (LiveEditorFactory is not null) { await OpenLiveEditorAsync(quotation); return; }
        ShowPreviewWindow(new Features.Reception.SalesCatalogView { DataContext = createPreview(quotation), ShowQuotationHeader = true }, "تعديل عرض السعر — معاينة فقط");
    }

    private async void PrintQuotationClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.SelectedQuotation is not { CanPrint: true } quotation) return;
        if (ViewModel.HasDocumentAuthority) {
            if (await ViewModel.ReadDocumentAsync() is not { } saved) return;
            var preview = new PrintPreview { Document = Features.Reception.QuotationCustomerDocument.CreateSaved(saved), CanPrint = saved.CanPrint };
            preview.AuthorizePrint = async () => {
                if (await ViewModel.ReadDocumentAsync() is not { CanPrint: true } current) return false;
                preview.Document = Features.Reception.QuotationCustomerDocument.CreateSaved(current); return true;
            };
            ShowPreviewWindow(preview, "معاينة عرض السعر");
        } else if (createPreview is not null) {
            ShowPreviewWindow(new PrintPreview { Document = Features.Reception.QuotationCustomerDocument.CreateExistingPreview(createPreview(quotation), quotation.Date.ToDateTime(TimeOnly.MinValue)) }, "معاينة عرض السعر");
        }
    }
    private async void AcceptClick(object sender, RoutedEventArgs e) { ViewModel.Accept(); await ViewModel.LastLifecycleAction; BackToQuotationsButton.Focus(); }
    private async void IssueClick(object sender, RoutedEventArgs e) { ViewModel.Issue(); await ViewModel.LastLifecycleAction; BackToQuotationsButton.Focus(); }
    private async void ValidityClick(object sender, RoutedEventArgs e) { ViewModel.SetValidity(); await ViewModel.LastLifecycleAction; BackToQuotationsButton.Focus(); }
    private async void ReviseClick(object sender, RoutedEventArgs e) { ViewModel.Revise(); await ViewModel.LastLifecycleAction; BackToQuotationsButton.Focus(); }
    private async void CancelQuotationClick(object sender, RoutedEventArgs e) { ViewModel.Cancel(); await ViewModel.LastLifecycleAction; BackToQuotationsButton.Focus(); }
    private async void RetryLifecycleClick(object sender, RoutedEventArgs e) { ViewModel.RetryLifecycle(); await ViewModel.LastLifecycleAction; BackToQuotationsButton.Focus(); }

    private void ConvertQuotationClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { CanConvert: true } quotation) return;
        ConversionDialog.DataContext = quotation;
        ConversionDialog.IsOpen = true;
    }

    private void CancelConversionClick(object sender, RoutedEventArgs e) => ConversionDialog.IsOpen = false;

    private async void ConfirmConversionClick(object sender, RoutedEventArgs e)
    {
        if (!ConversionDialog.IsOpen || ConversionDialog.DataContext is not QuotationListItem { CanConvert: true } quotation) return;
        ConversionDialog.IsOpen = false;
        if (quotation.Lifecycle is not null) { await ViewModel.ConvertAsync(); BackToQuotationsButton.Focus(); return; }
        var view = new Features.Orders.OrdersView();
        view.ConfigureReceptionist();
        view.CustomerRequested += _ => CustomerRequested?.Invoke(quotation.Id);
        view.ViewModel.OpenOrder(new(Guid.NewGuid(), "معاينة غير محفوظة", quotation.Customer,
            DateOnly.FromDateTime(DateTime.Today), Features.Orders.OrderStatus.New, quotation.Discount,
            quotation.Items.Select(line => new Features.Orders.OrderLineItem(line.FurnitureName, line.Variant, line.Dimensions, line.Color, line.Handle, line.Quantity, line.UnitPrice, line.IsFurniture, line.ThumbnailKind, line.Image)).ToArray(), quotation.Phone, quotation));
        ShowPreviewWindow(view, "تفاصيل الطلب — معاينة فقط، لم يتم حفظ طلب");
    }

    private async void OpenLinkedOrderClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedQuotation is not { IsConverted: true } quotation) return;
        if (quotation.Lifecycle is not null) { await ViewModel.OpenLinkedOrderAsync(); return; }
        var view = new Features.Orders.OrdersView();
        view.ConfigureReceptionist();
        view.CustomerRequested += _ => CustomerRequested?.Invoke(quotation.Id);
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
            if (content is PrintPreview printContent) if (printContent.CanPrint) printContent.PrintButton.Focus(); else printContent.BackButton.Focus();
            if (content is Features.Reception.SalesCatalogView catalog)
            {
                if (((Features.Reception.SalesCatalogViewModel)catalog.DataContext).IsReviewingQuotation) catalog.RestoreQuotationFocus();
                else catalog.RestoreCatalogFocus();
            }
            if (content is Features.Orders.OrdersView orders) orders.BackToOrdersButton.Focus();
        };
        window.Closed += (_, _) => { if (ViewModel.IsListVisible) QuotationSearchBox.Focus(); else BackToQuotationsButton.Focus(); };
        editorWindows.Add(window);
        try { window.ShowDialog(); }
        finally { editorWindows.Remove(window); }
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

    private async void ApproveDiscountClick(object sender, RoutedEventArgs eventArgs) { ViewModel.ApproveDiscount(); await ViewModel.LastApprovalDecision; BackToQuotationsButton.Focus(); }

    private async void RejectDiscountClick(object sender, RoutedEventArgs eventArgs) { ViewModel.RejectDiscount(); await ViewModel.LastApprovalDecision; BackToQuotationsButton.Focus(); }
}
