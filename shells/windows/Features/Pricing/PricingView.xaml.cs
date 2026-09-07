using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Pricing;

public partial class PricingView : UserControl
{
    public PricingView()
    {
        InitializeComponent();
        ViewModel = new PricingViewModel();
        DataContext = ViewModel;
    }

    public PricingViewModel ViewModel { get; }

    private void EditPriceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: PricingListItem item })
        {
            ViewModel.BeginEdit(item);
            Dispatcher.BeginInvoke(PriceInput.Focus, DispatcherPriority.Input);
        }
    }

    private void SavePriceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ViewModel.SaveEditor())
        {

            Feedback.RestartDuration();
        }
        else
        {
            PriceInput.Focus();
        }
    }

    private void CancelPriceClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
}
