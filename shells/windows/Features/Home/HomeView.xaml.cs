using System.Windows;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Home;

public partial class HomeView : UserControl
{
    public HomeView() => InitializeComponent();
    public event Action<HomeRow>? OpenRequested;
    private void RefreshClick(object sender, RoutedEventArgs e) { if (DataContext is HomeViewModel home) home.Refresh(); }
    private void OpenClick(object sender, RoutedEventArgs e) { if (sender is Button { DataContext: HomeRow row }) OpenRequested?.Invoke(row); }
}
