using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using MenuItem = System.Windows.Controls.MenuItem;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.RawMaterials;

public partial class RawMaterialsView : UserControl
{
    private readonly DispatcherTimer feedbackTimer;

    public RawMaterialsView()
    {
        InitializeComponent();
        ViewModel = new RawMaterialsViewModel();
        DataContext = ViewModel;
        feedbackTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2.5) };
        feedbackTimer.Tick += (_, _) =>
        {
            feedbackTimer.Stop();
            ViewModel.ClearFeedback();
        };
    }

    public RawMaterialsViewModel ViewModel { get; }

    private void AddRawMaterialClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginCreate();
        EditorNameBox.Focus();
    }

    private void RawMaterialRowClick(object sender, MouseButtonEventArgs eventArgs)
    {
        if (sender is FrameworkElement { DataContext: RawMaterialListItem material })
        {
            ViewModel.BeginEdit(material);
            EditorNameBox.Focus();
        }
    }

    private static RawMaterialListItem? MaterialFromMenuItem(object sender) =>
        sender is MenuItem { DataContext: RawMaterialListItem material } ? material : null;

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
        if (MaterialFromMenuItem(sender) is { } material)
        {
            ViewModel.BeginEdit(material);
            EditorNameBox.Focus();
        }
    }

    private void DuplicateMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (MaterialFromMenuItem(sender) is { } material)
        {
            ViewModel.Duplicate(material);
            RestartFeedbackTimer();
            EditorNameBox.Focus();
        }
    }

    private void ArchiveMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (MaterialFromMenuItem(sender) is { } material)
        {
            ViewModel.Archive(material);
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
