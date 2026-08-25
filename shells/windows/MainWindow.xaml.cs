using System.Windows;
using System.Windows.Input;
using Eitmad.WindowsShell.Features.Operations;

namespace Eitmad.WindowsShell;

public partial class MainWindow : Window
{
    public MainWindow(OperationsViewModel viewModel)
    {
        InitializeComponent();
        DataContext = viewModel;
    }

    private void TitleBarMouseDown(object sender, MouseButtonEventArgs eventArgs)
    {
        if (eventArgs.ChangedButton == MouseButton.Left)
        {
            if (eventArgs.ClickCount == 2)
            {
                ToggleMaximize();
            }
            else
            {
                DragMove();
            }
        }
    }

    private void MinimizeClick(object sender, RoutedEventArgs eventArgs) => WindowState = WindowState.Minimized;

    private void MaximizeClick(object sender, RoutedEventArgs eventArgs) => ToggleMaximize();

    private void CloseClick(object sender, RoutedEventArgs eventArgs) => Close();

    private void ToggleMaximize() => WindowState = WindowState == WindowState.Maximized
        ? WindowState.Normal
        : WindowState.Maximized;
}
