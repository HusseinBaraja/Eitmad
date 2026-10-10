using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Eitmad.WindowsShell.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Orders;

public partial class OrdersView : UserControl
{
    public OrdersView()
    {
        InitializeComponent();
        ViewModel = new OrdersViewModel();
        DataContext = ViewModel;
    }

    public OrdersViewModel ViewModel { get; private set; }
    public event Action<Guid>? CustomerRequested;

    private void CustomerClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.IsReceptionist && ViewModel.SelectedOrder is { } order) CustomerRequested?.Invoke(order.Id);
    }

    public void ConfigureReceptionist(bool preview = false)
    {
        ViewModel = new OrdersViewModel(true, preview);
        DataContext = ViewModel;
    }

    public void UsePreviewFixtures() { ViewModel = new OrdersViewModel(preview: true); DataContext = ViewModel; }

    public event Action<OrderListItem>? ProductionRequested;
    private void ProductionClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.SelectedOrder is { } order) ProductionRequested?.Invoke(order);
    }
    private void AcknowledgeReadyClick(object sender, RoutedEventArgs e) { ViewModel.AcknowledgeReady(); BackToOrdersButton.Focus(); }

    private async void PrintOrderClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.SelectedOrder is not { } order) return;
        if (ViewModel.IsLive) await ShowSavedDocument(false);
        else if (ViewModel.IsReceptionist) ShowDocument(OrderCustomerDocument.Create(order), "معاينة الطلب");
    }

    private async void OriginalQuotationClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.IsLive) { await ShowSavedDocument(true); return; }
        if (ViewModel.SelectedOrder?.OriginalQuotation is not { } quotation) return;
        ShowDocument(OrderCustomerDocument.CreateQuotation(quotation), ViewModel.IsLive ? "عرض السعر الأصلي" : "عرض السعر الأصلي — بيانات تجريبية");
    }

    private async Task ShowSavedDocument(bool source)
    {
        if (await ViewModel.ReadDocumentAsync(source) is not { } saved) return;
        ShowDocument(OrderCustomerDocument.CreateSaved(saved, source), source ? "عرض السعر الأصلي" : "معاينة الطلب", saved.CanPrint, source);
    }

    private void ShowDocument(System.Windows.Documents.FlowDocument document, string title, bool canPrint = false, bool source = false)
    {
        var previousFocus = System.Windows.Input.Keyboard.FocusedElement;
        var preview = new PrintPreview { Document = document, JobName = title, CanPrint = canPrint };
        var window = new Window { Title = title, Content = preview, Owner = Window.GetWindow(this),
            Width = 1000, Height = 780, MinWidth = 640, MinHeight = 480,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            FlowDirection = System.Windows.FlowDirection.RightToLeft, Language = Language };
        if (ViewModel.IsLive) preview.AuthorizePrint = async () => {
            if (await ViewModel.ReadDocumentAsync(source) is not { CanPrint: true } current) return false;
            preview.Document = OrderCustomerDocument.CreateSaved(current, source); return true;
        };
        EventHandler invalidated = (_, _) => window.Close();
        ViewModel.DocumentsInvalidated += invalidated;
        window.Closed += (_, _) => ViewModel.DocumentsInvalidated -= invalidated;
        preview.BackRequested += (_, _) => window.Close();
        window.Loaded += (_, _) => { if (preview.CanPrint) preview.PrintButton.Focus(); else preview.BackButton.Focus(); };
        window.ShowDialog();
        if (previousFocus is System.Windows.IInputElement target) System.Windows.Input.Keyboard.Focus(target);
    }

    private void OrderRowInvoked(object sender, RowInvokedEventArgs eventArgs) =>
        OpenOrder((OrderListItem)eventArgs.Item);

    private void OpenOrder(OrderListItem order)
    {
        ViewModel.OpenOrder(order);
        Dispatcher.BeginInvoke(BackToOrdersButton.Focus, DispatcherPriority.Input);
    }

    private void OpenOrderClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: OrderListItem order })
        {
            OpenOrder(order);
        }
    }

    private async void CancelOrderClick(object sender, RoutedEventArgs e) { ViewModel.CancelOrder(); await ViewModel.LastAction; BackToOrdersButton.Focus(); }
    private async void SaveFulfillmentClick(object sender, RoutedEventArgs e) { ViewModel.SaveFulfillment(); await ViewModel.LastAction; BackToOrdersButton.Focus(); }
    private async void DeliverOrderClick(object sender, RoutedEventArgs e) { ViewModel.DeliverOrder(); await ViewModel.LastAction; BackToOrdersButton.Focus(); }
    private async void StartWorkClick(object sender, RoutedEventArgs e) { ViewModel.StartWork(); await ViewModel.LastAction; BackToOrdersButton.Focus(); }
    private async void CompleteWorkClick(object sender, RoutedEventArgs e) { ViewModel.CompleteWork(); await ViewModel.LastAction; BackToOrdersButton.Focus(); }
    private async void RetryOrderClick(object sender, RoutedEventArgs e) { ViewModel.Retry(); await ViewModel.LastAction; }
    private void BackToListClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.CloseOrder();
        Dispatcher.BeginInvoke(OrderSearchBox.Focus, DispatcherPriority.Input);
    }
}
