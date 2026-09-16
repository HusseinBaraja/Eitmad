using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Eitmad.WindowsShell.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.WorkOrders;

public partial class WorkOrdersView : UserControl
{
    public WorkOrdersView()
    {
        InitializeComponent();
        ViewModel = new WorkOrdersViewModel();
        DataContext = ViewModel;
    }

    public WorkOrdersViewModel ViewModel { get; }

    public event Action<string>? OrderRequested;
    private void OpenOrderClick(object sender, RoutedEventArgs e)
    {
        if (ViewModel.SelectedWorkOrder is { } workOrder) OrderRequested?.Invoke(workOrder.OrderNumber);
    }

    private void WorkOrderRowInvoked(object sender, RowInvokedEventArgs eventArgs) =>
        OpenWorkOrder((WorkOrderListItem)eventArgs.Item);

    private void OpenWorkOrder(WorkOrderListItem workOrder)
    {
        ViewModel.OpenWorkOrder(workOrder);
        Dispatcher.BeginInvoke(BackToWorkOrdersButton.Focus, DispatcherPriority.Input);
    }

    private void OpenWorkOrderClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: WorkOrderListItem workOrder })
        {
            OpenWorkOrder(workOrder);
        }
    }

    private void BackToListClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.CloseWorkOrder();
        Dispatcher.BeginInvoke(WorkOrderSearchBox.Focus, DispatcherPriority.Input);
    }

    private void AdvanceStatusClick(object sender, RoutedEventArgs eventArgs)
    {
        if (!ViewModel.AdvanceSelectedStatus())
        {
            return;
        }


        Feedback.RestartDuration();
    }
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
}
