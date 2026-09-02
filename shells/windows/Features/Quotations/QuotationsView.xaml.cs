using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
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

    public QuotationsViewModel ViewModel { get; }

    private void OpenQuotationClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: QuotationListItem quotation })
        {
            ViewModel.OpenQuotation(quotation);
            Dispatcher.BeginInvoke(BackToQuotationsButton.Focus, DispatcherPriority.Input);
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
