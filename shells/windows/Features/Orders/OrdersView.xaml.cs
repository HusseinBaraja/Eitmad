using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
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

    private void OpenOrderClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: OrderListItem order })
        {
            ViewModel.OpenOrder(order);
            Dispatcher.BeginInvoke(BackToOrdersButton.Focus, DispatcherPriority.Input);
        }
    }

    private void BackToListClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.CloseOrder();
        Dispatcher.BeginInvoke(OrderSearchBox.Focus, DispatcherPriority.Input);
    }
}
