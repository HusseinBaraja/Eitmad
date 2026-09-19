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

    public void ConfigureReceptionist()
    {
        ViewModel = new OrdersViewModel(true);
        DataContext = ViewModel;
    }

    public event Action<OrderListItem>? ProductionRequested;
    private void ProductionClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.SelectedOrder is { } order) ProductionRequested?.Invoke(order);
    }
    private void AcknowledgeReadyClick(object sender, RoutedEventArgs e) { ViewModel.AcknowledgeReady(); BackToOrdersButton.Focus(); }

    private void PrintOrderClick(object sender, RoutedEventArgs e)
    {
        if (!ViewModel.IsReceptionist || ViewModel.SelectedOrder is not { } order) return;
        ShowDocument(OrderCustomerDocument.Create(order), "معاينة الطلب");
    }

    private void OriginalQuotationClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.SelectedOrder?.OriginalQuotation is not { } quotation) return;
        ShowDocument(OrderCustomerDocument.CreateQuotation(quotation), "عرض السعر الأصلي — بيانات تجريبية");
    }

    private void ShowDocument(System.Windows.Documents.FlowDocument document, string title)
    {
        var previousFocus = System.Windows.Input.Keyboard.FocusedElement;
        var preview = new PrintPreview { Document = document, JobName = title };
        var window = new Window { Title = title, Content = preview, Owner = Window.GetWindow(this),
            Width = 1000, Height = 780, MinWidth = 640, MinHeight = 480,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            FlowDirection = System.Windows.FlowDirection.RightToLeft, Language = Language };
        preview.BackRequested += (_, _) => window.Close();
        window.Loaded += (_, _) => preview.PrintButton.Focus();
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

    private void BackToListClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.CloseOrder();
        Dispatcher.BeginInvoke(OrderSearchBox.Focus, DispatcherPriority.Input);
    }
}
