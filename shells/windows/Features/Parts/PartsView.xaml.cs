using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using MenuItem = System.Windows.Controls.MenuItem;
using TextBox = System.Windows.Controls.TextBox;
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
        Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
    }

    private void PartRowClick(object sender, MouseButtonEventArgs eventArgs)
    {
        if (sender is FrameworkElement { DataContext: PartListItem part })
        {
            ViewModel.BeginEdit(part);
            Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
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
            Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void DuplicateMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (PartFromMenuItem(sender) is { } part)
        {
            ViewModel.Duplicate(part);
            RestartFeedbackTimer();
            Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
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
    }

    private void NextFromInformationClick(object sender, RoutedEventArgs eventArgs)
    {
        if (!ViewModel.MoveToMaterials())
        {
            EditorNameBox.Focus();
        }
    }

    private void NextFromMaterialsClick(object sender, RoutedEventArgs eventArgs)
    {
        var invalidQuantity = VisualDescendants<TextBox>(MaterialRows)
            .FirstOrDefault(Validation.GetHasError);
        if (invalidQuantity is not null)
        {
            invalidQuantity.Focus();
            return;
        }

        ViewModel.MoveToReview();
    }

    private void PreviousStepClick(object sender, RoutedEventArgs eventArgs) => ViewModel.MoveToPreviousStep();

    private void OpenMaterialPickerClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.OpenMaterialPicker();
        Dispatcher.BeginInvoke(MaterialSearchBox.Focus, DispatcherPriority.Input);
    }

    private void CloseMaterialPickerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CloseMaterialPicker();

    private void SelectMaterialClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: PartMaterialOption material })
        {
            ViewModel.AddMaterial(material);
        }
    }

    private void RemoveMaterialClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: PartMaterialUsage usage })
        {
            ViewModel.RemoveMaterial(usage);
        }
    }

    private void CancelEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();

    private void RestartFeedbackTimer()
    {
        feedbackTimer.Stop();
        feedbackTimer.Start();
    }

    private static IEnumerable<T> VisualDescendants<T>(DependencyObject parent) where T : DependencyObject
    {
        for (var index = 0; index < VisualTreeHelper.GetChildrenCount(parent); index++)
        {
            var child = VisualTreeHelper.GetChild(parent, index);
            if (child is T match)
            {
                yield return match;
            }

            foreach (var descendant in VisualDescendants<T>(child))
            {
                yield return descendant;
            }
        }
    }
}
