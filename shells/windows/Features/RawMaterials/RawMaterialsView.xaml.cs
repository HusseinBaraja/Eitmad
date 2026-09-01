using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using ComboBox = System.Windows.Controls.ComboBox;
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
        Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
    }

    private void RawMaterialRowClick(object sender, MouseButtonEventArgs eventArgs)
    {
        if (sender is FrameworkElement { DataContext: RawMaterialListItem material })
        {
            ViewModel.BeginEdit(material);
            Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
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
            Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void DuplicateMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (MaterialFromMenuItem(sender) is { } material)
        {
            ViewModel.Duplicate(material);
            RestartFeedbackTimer();
            Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
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

    private void AddReferenceFromDropdownClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { Tag: string kind } button)
        {
            CloseOwningDropdown(button);
            if (kind == "unit")
            {
                ViewModel.BeginAddUnit();
            }
            else
            {
                ViewModel.BeginAddCategory();
            }

            Dispatcher.BeginInvoke(ReferenceNameBox.Focus, DispatcherPriority.Input);
            eventArgs.Handled = true;
        }
    }

    private void ManageReferencesFromDropdownClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { Tag: string kind } button)
        {
            CloseOwningDropdown(button);
            if (kind == "unit")
            {
                ViewModel.BeginManageUnits();
            }
            else
            {
                ViewModel.BeginManageCategories();
            }

            eventArgs.Handled = true;
        }
    }

    private static void CloseOwningDropdown(Button button)
    {
        if (button.TemplatedParent is ComboBox comboBox)
        {
            comboBox.IsDropDownOpen = false;
        }
    }

    private void EditReferenceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: RawMaterialReferenceOption reference })
        {
            ViewModel.BeginEditReference(reference);
            Dispatcher.BeginInvoke(ReferenceNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void ArchiveReferenceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: RawMaterialReferenceOption reference })
        {
            ViewModel.ArchiveReference(reference);
        }
    }

    private void SaveReferenceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (!ViewModel.SaveReferenceEditor())
        {
            ReferenceNameBox.Focus();
        }
    }

    private void CancelReferenceClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelReferenceEditor();

    private void CloseReferenceManagerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CloseReferenceManager();

    private void RestartFeedbackTimer()
    {
        feedbackTimer.Stop();
        feedbackTimer.Start();
    }
}
