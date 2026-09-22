using System.Windows;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Customers;

public partial class CustomerDetailView : UserControl
{
    private CustomerClient? client;
    public CustomerDetailView() => InitializeComponent();
    public event EventHandler? BackRequested;
    public void Attach(CustomerClient customerClient) => client = customerClient;
    private void BackClick(object sender, RoutedEventArgs e) => BackRequested?.Invoke(this, EventArgs.Empty);
    private void EditClick(object sender, RoutedEventArgs e)
    {
        if (DataContext is CustomerPreview customer && client is not null)
            Editor.Open(customer.Customer, client, customer.Observe);
    }
}
