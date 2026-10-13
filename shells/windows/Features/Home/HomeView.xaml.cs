using System.Windows;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Home;

public partial class HomeView : UserControl
{
    public static readonly DependencyProperty IsManagerDashboardProperty = DependencyProperty.Register(
        nameof(IsManagerDashboard), typeof(bool), typeof(HomeView), new PropertyMetadata(false));
    public static readonly DependencyProperty QuickRoutesProperty = DependencyProperty.Register(
        nameof(QuickRoutes), typeof(UIElement), typeof(HomeView), new PropertyMetadata(null));
    public bool IsManagerDashboard { get => (bool)GetValue(IsManagerDashboardProperty); set => SetValue(IsManagerDashboardProperty, value); }
    public UIElement? QuickRoutes { get => (UIElement?)GetValue(QuickRoutesProperty); set => SetValue(QuickRoutesProperty, value); }
    public HomeView() => InitializeComponent();
    public event Action<HomeRow>? OpenRequested;
    public event Action? ApprovalsRequested;
    private void ApprovalsClick(object sender, RoutedEventArgs e) => ApprovalsRequested?.Invoke();
    private void RefreshClick(object sender, RoutedEventArgs e) { if (DataContext is HomeViewModel home) home.Refresh(); }
    private void OpenClick(object sender, RoutedEventArgs e) { if (sender is Button { DataContext: HomeRow row }) OpenRequested?.Invoke(row); }
}
