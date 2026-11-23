using System.Windows;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Home;

public partial class HomeView : UserControl
{
    public HomeView() => InitializeComponent();
    public event Action<HomeRow>? OpenRequested;
    public event Action? ApprovalsRequested;
    public event Action? SearchCleared;
    // A pipeline stage asks the host to open its destination; the host owns navigation.
    public event Action<string>? StageRequested;
    private void ClearSearchClick(object sender, RoutedEventArgs e)
    {
        if (DataContext is not HomeViewModel home) return;
        home.Search("");
        SearchCleared?.Invoke();
    }
    private void ApprovalsClick(object sender, RoutedEventArgs e) => ApprovalsRequested?.Invoke();
    private void StageClick(object sender, RoutedEventArgs e) { if (sender is Button { Tag: string destination }) StageRequested?.Invoke(destination); }
    private void RefreshClick(object sender, RoutedEventArgs e) { if (DataContext is HomeViewModel home) home.Refresh(); }
    private void OpenClick(object sender, RoutedEventArgs e) { if (sender is Button { DataContext: HomeRow row }) OpenRequested?.Invoke(row); }
}
