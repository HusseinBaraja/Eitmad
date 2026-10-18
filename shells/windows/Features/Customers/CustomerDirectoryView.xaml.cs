using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using UserControl = System.Windows.Controls.UserControl;
using Button = System.Windows.Controls.Button;
using KeyEventArgs = System.Windows.Input.KeyEventArgs;

namespace Eitmad.WindowsShell.Features.Customers;
public partial class CustomerDirectoryView : UserControl
{
    public CustomerDirectoryView() => InitializeComponent();
    public CustomerDirectoryViewModel? ViewModel => DataContext as CustomerDirectoryViewModel;
    public event EventHandler? HomeRequested;
    public void Attach(CustomerClient client, IEngineShellBridge engine) => DataContext = new CustomerDirectoryViewModel(client, engine);
    private void SearchClick(object sender, RoutedEventArgs e) => ViewModel?.Search();
    private void NextClick(object sender, RoutedEventArgs e) => ViewModel?.Next();
    private void DetailHistoryNext(object? sender, EventArgs e) => ViewModel?.NextHistory();
    private void HomeClick(object sender, RoutedEventArgs e) => HomeRequested?.Invoke(this, EventArgs.Empty);
    private async void DetailBack(object? sender, EventArgs e) { ViewModel?.Back(); await Dispatcher.BeginInvoke(SearchInput.Focus, System.Windows.Threading.DispatcherPriority.Input); }
    private void SearchKeyDown(object sender, KeyEventArgs e) { if (e.Key == Key.Enter) { ViewModel?.Search(); e.Handled = true; } }
    private async void OpenClick(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { DataContext: Customer customer } || ViewModel is not { } model) return;
        model.Open(customer); await model.LastLoad;
        if (model.HasDetail) await Dispatcher.BeginInvoke(Detail.BackButton.Focus, System.Windows.Threading.DispatcherPriority.Input);
    }
}
