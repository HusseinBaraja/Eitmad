using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.WorkOrders;

public partial class WorkOrdersView : UserControl
{
    private readonly DispatcherTimer feedbackTimer;

    public WorkOrdersView()
    {
        InitializeComponent();
        ViewModel = new WorkOrdersViewModel();
        DataContext = ViewModel;
        feedbackTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(3) };
        feedbackTimer.Tick += (_, _) =>
        {
            feedbackTimer.Stop();
            ViewModel.ClearFeedback();
        };
    }

    public WorkOrdersViewModel ViewModel { get; }

    private void OpenWorkOrderClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: WorkOrderListItem workOrder })
        {
            ViewModel.OpenWorkOrder(workOrder);
            Dispatcher.BeginInvoke(BackToWorkOrdersButton.Focus, DispatcherPriority.Input);
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

        feedbackTimer.Stop();
        feedbackTimer.Start();
    }
}
