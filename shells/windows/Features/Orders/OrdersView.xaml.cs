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

    public OrdersViewModel ViewModel { get; }

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
