using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using MenuItem = System.Windows.Controls.MenuItem;
using OpenFileDialog = Microsoft.Win32.OpenFileDialog;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Furniture;

public partial class FurnitureView : UserControl
{
    private readonly DispatcherTimer feedbackTimer;

    public FurnitureView()
    {
        InitializeComponent();
        ViewModel = new FurnitureViewModel();
        DataContext = ViewModel;
        feedbackTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(3) };
        feedbackTimer.Tick += (_, _) =>
        {
            feedbackTimer.Stop();
            ViewModel.ClearFeedback();
        };
    }

    public FurnitureViewModel ViewModel { get; }

    private void AddFurnitureClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginCreate();
        Dispatcher.BeginInvoke(FurnitureNameBox.Focus, DispatcherPriority.Input);
    }

    private void ChooseImageClick(object sender, RoutedEventArgs eventArgs)
    {
        var dialog = new OpenFileDialog
        {
            Title = "اختر صورة المنتج",
            Filter = "ملفات الصور|*.png;*.jpg;*.jpeg;*.webp;*.bmp|كل الملفات|*.*",
            CheckFileExists = true,
            Multiselect = false,
        };

        if (dialog.ShowDialog() != true)
        {
            return;
        }

        var image = new BitmapImage();
        image.BeginInit();
        image.CacheOption = BitmapCacheOption.OnLoad;
        image.UriSource = new System.Uri(dialog.FileName, System.UriKind.Absolute);
        image.EndInit();
        image.Freeze();
        ViewModel.SetProductImage(image, Path.GetFileName(dialog.FileName));
    }

    private void OpenRowMenuClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { ContextMenu: { } menu } button)
        {
            menu.PlacementTarget = button;
            menu.IsOpen = true;
            eventArgs.Handled = true;
        }
    }

    private static FurnitureListItem? FurnitureFromMenuItem(object sender) =>
        sender is MenuItem { DataContext: FurnitureListItem item } ? item : null;

    private void EditFurnitureClick(object sender, RoutedEventArgs eventArgs)
    {
        if (FurnitureFromMenuItem(sender) is { } item)
        {
            ViewModel.BeginEdit(item);
            Dispatcher.BeginInvoke(FurnitureNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void DuplicateFurnitureClick(object sender, RoutedEventArgs eventArgs)
    {
        if (FurnitureFromMenuItem(sender) is { } item)
        {
            ViewModel.DuplicateFurniture(item);
            RestartFeedbackTimer();
            Dispatcher.BeginInvoke(FurnitureNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void ArchiveFurnitureClick(object sender, RoutedEventArgs eventArgs)
    {
        if (FurnitureFromMenuItem(sender) is { } item)
        {
            ViewModel.ArchiveFurniture(item);
            RestartFeedbackTimer();
        }
    }

    private void NextStepClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ViewModel.IsStepOne)
        {
            if (!ViewModel.MoveToParts())
            {
                FurnitureNameBox.Focus();
            }

            return;
        }

        if (ViewModel.IsStepTwo)
        {
            ViewModel.MoveToVariants();
            return;
        }

        if (ViewModel.IsStepThree)
        {
            ViewModel.MoveToOptions();
            return;
        }

        ViewModel.RequestNextFromOptions();
        RestartFeedbackTimer();
    }

    private void PreviousStepClick(object sender, RoutedEventArgs eventArgs) => ViewModel.MoveToPreviousStep();

    private void CancelEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();

    private void OpenPartPickerClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.OpenPartPicker();
        Dispatcher.BeginInvoke(PartSearchBox.Focus, DispatcherPriority.Input);
    }

    private void ClosePartPickerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.ClosePartPicker();

    private void SelectPartClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurniturePartOption part })
        {
            ViewModel.AddPart(part);
        }
    }

    private void RemovePartClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurniturePartUsage usage })
        {
            ViewModel.RemovePart(usage);
        }
    }

    private void AddVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginAddVariant();
        Dispatcher.BeginInvoke(VariantNameBox.Focus, DispatcherPriority.Input);
    }

    private void EditVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureVariant variant })
        {
            ViewModel.BeginEditVariant(variant);
            Dispatcher.BeginInvoke(VariantNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void DuplicateVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureVariant variant })
        {
            ViewModel.DuplicateVariant(variant);
            RestartFeedbackTimer();
        }
    }

    private void RemoveVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureVariant variant })
        {
            ViewModel.RemoveVariant(variant);
        }
    }

    private void SaveVariantClick(object sender, RoutedEventArgs eventArgs) => ViewModel.SaveVariant();

    private void CancelVariantClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelVariantEditor();

    private void AddColorClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginAddColor();
        Dispatcher.BeginInvoke(ColorNameBox.Focus, DispatcherPriority.Input);
    }

    private void SaveColorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.SaveColor();

    private void CancelColorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelColorEditor();

    private void ToggleColorClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureColorOption color })
        {
            ViewModel.ToggleColor(color);
        }
    }

    private void AddHandleClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginAddHandle();
        Dispatcher.BeginInvoke(HandleNameBox.Focus, DispatcherPriority.Input);
    }

    private void SaveHandleClick(object sender, RoutedEventArgs eventArgs) => ViewModel.SaveHandle();

    private void CancelHandleClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelHandleEditor();

    private void ToggleHandleClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: FurnitureHandleOption handle })
        {
            ViewModel.ToggleHandle(handle);
        }
    }

    private void RestartFeedbackTimer()
    {
        feedbackTimer.Stop();
        feedbackTimer.Start();
    }
}
