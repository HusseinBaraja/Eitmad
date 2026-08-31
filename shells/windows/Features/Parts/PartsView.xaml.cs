using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using MenuItem = System.Windows.Controls.MenuItem;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Parts;

public partial class PartsView : UserControl
{
    private readonly DispatcherTimer feedbackTimer;

    public PartsView()
    {
        InitializeComponent();
        ViewModel = new PartsViewModel();
        DataContext = ViewModel;
        feedbackTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2.5) };
        feedbackTimer.Tick += (_, _) =>
        {
            feedbackTimer.Stop();
            ViewModel.ClearFeedback();
        };
    }

    public PartsViewModel ViewModel { get; }

    private void AddPartClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginCreate();
        EditorNameBox.Focus();
    }

    private void PartRowClick(object sender, MouseButtonEventArgs eventArgs)
    {
        if (sender is FrameworkElement { DataContext: PartListItem part })
        {
            ViewModel.BeginEdit(part);
            EditorNameBox.Focus();
        }
    }

    private static PartListItem? PartFromMenuItem(object sender) =>
        sender is MenuItem { DataContext: PartListItem part } ? part : null;

    private void OpenRowMenuClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { ContextMenu: { } menu })
        {
            menu.PlacementTarget = (Button)sender;
            menu.IsOpen = true;
            eventArgs.Handled = true;
        }
    }

    private void EditMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (PartFromMenuItem(sender) is { } part)
        {
            ViewModel.BeginEdit(part);
            EditorNameBox.Focus();
        }
    }

    private void DuplicateMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (PartFromMenuItem(sender) is { } part)
        {
            ViewModel.Duplicate(part);
            RestartFeedbackTimer();
            EditorNameBox.Focus();
        }
    }

    private void ArchiveMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (PartFromMenuItem(sender) is { } part)
        {
            ViewModel.Archive(part);
            RestartFeedbackTimer();
        }
    }

    private void SaveEditorClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ViewModel.SaveEditor())
        {
            RestartFeedbackTimer();
        }
        else
        {
            EditorNameBox.Focus();
        }
    }

    private void CancelEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();

    private void RestartFeedbackTimer()
    {
        feedbackTimer.Stop();
        feedbackTimer.Start();
    }
}
