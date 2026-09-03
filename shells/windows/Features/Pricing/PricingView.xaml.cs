using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Pricing;

public partial class PricingView : UserControl
{
    private readonly DispatcherTimer feedbackTimer;

    public PricingView()
    {
        InitializeComponent();
        ViewModel = new PricingViewModel();
        DataContext = ViewModel;
        feedbackTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(3) };
        feedbackTimer.Tick += (_, _) =>
        {
            feedbackTimer.Stop();
            ViewModel.ClearFeedback();
        };
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
            feedbackTimer.Stop();
            feedbackTimer.Start();
        }
        else
        {
            PriceInput.Focus();
        }
    }

    private void CancelPriceClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();
}
