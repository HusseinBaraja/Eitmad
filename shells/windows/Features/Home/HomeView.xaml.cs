using System.Windows;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Home;

public partial class HomeView : UserControl
{
    public HomeView() => InitializeComponent();
    public event Action<HomeRow>? OpenRequested;
    public event Action? ApprovalsRequested;
    public event Action<string>? OrdersRequested;
    public event Action? SearchCleared;
    public event Action? QuotationsRequested;
    private void ClearSearchClick(object sender, RoutedEventArgs e)
    {
        if (DataContext is not HomeViewModel home) return;
        home.Search("");
        SearchCleared?.Invoke();
    }
    private void ApprovalsClick(object sender, RoutedEventArgs e) => ApprovalsRequested?.Invoke();
    private void OrdersClick(object sender, RoutedEventArgs e) => OrdersRequested?.Invoke(Orders.OrdersViewModel.AllStatuses);
    private void ReadyOrdersClick(object sender, RoutedEventArgs e) => OrdersRequested?.Invoke(Orders.OrdersViewModel.ReadyStatus);
    private void QuotationsClick(object sender, RoutedEventArgs e) => QuotationsRequested?.Invoke();
    private void RefreshClick(object sender, RoutedEventArgs e) { if (DataContext is HomeViewModel home) home.Refresh(); }
    private void OpenClick(object sender, RoutedEventArgs e) { if (sender is Button { DataContext: HomeRow row }) OpenRequested?.Invoke(row); }
}
