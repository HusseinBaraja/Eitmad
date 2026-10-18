using System.Windows;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Customers;

public partial class CustomerDetailView : UserControl
{
    private CustomerClient? client;
    public CustomerDetailView() { InitializeComponent(); DataContextChanged += (_, _) => Resolution.ClearSession(); }
    public event EventHandler? BackRequested;
    public event EventHandler? HistoryNextRequested;
    private void HistoryNextClick(object sender, RoutedEventArgs e) => HistoryNextRequested?.Invoke(this, EventArgs.Empty);
    public void Attach(CustomerClient customerClient) => client = customerClient;
    public void EnableResolution() => ResolveButton.Visibility = Visibility.Visible;
    private async void ResolveClick(object sender, RoutedEventArgs e)
    {
        if (DataContext is CustomerPreview customer && client is not null) {
            await Resolution.OpenAsync(customer.Customer, client);
        }
    }
    private void BackClick(object sender, RoutedEventArgs e) => BackRequested?.Invoke(this, EventArgs.Empty);
    private void EditClick(object sender, RoutedEventArgs e)
    {
        if (DataContext is CustomerPreview customer && client is not null)
            Editor.Open(customer.Customer, client, customer.Observe);
    }
}
