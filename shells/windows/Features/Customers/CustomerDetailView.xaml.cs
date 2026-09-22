using System.Windows;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Customers;

public partial class CustomerDetailView : UserControl
{
    public CustomerDetailView() => InitializeComponent();
    public event EventHandler? BackRequested;
    private void BackClick(object sender, RoutedEventArgs e) => BackRequested?.Invoke(this, EventArgs.Empty);
    private void EditClick(object sender, RoutedEventArgs e)
    {
        if (DataContext is CustomerPreview customer) Editor.Open(customer.Contact, customer.ApplyPreview);
    }
}
