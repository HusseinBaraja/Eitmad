using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Button = System.Windows.Controls.Button;
using ComboBox = System.Windows.Controls.ComboBox;
using MenuItem = System.Windows.Controls.MenuItem;
using OpenFileDialog = Microsoft.Win32.OpenFileDialog;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Products;

public partial class ProductsView : UserControl
{
    private readonly DispatcherTimer feedbackTimer;

    public ProductsView()
    {
        InitializeComponent();
        ViewModel = new ProductsViewModel();
        DataContext = ViewModel;
        feedbackTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(2.5) };
        feedbackTimer.Tick += (_, _) =>
        {
            feedbackTimer.Stop();
            ViewModel.ClearFeedback();
        };
    }

    public ProductsViewModel ViewModel { get; }

    private void AddProductClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginCreate();
        Dispatcher.BeginInvoke(ProductNameBox.Focus, DispatcherPriority.Input);
    }

    private void EditProductClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ProductFromMenuItem(sender) is { } product)
        {
            ViewModel.BeginEdit(product);
            Dispatcher.BeginInvoke(ProductNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void DuplicateProductClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ProductFromMenuItem(sender) is { } product)
        {
            ViewModel.BeginDuplicate(product);
            Dispatcher.BeginInvoke(ProductNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void ArchiveProductClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ProductFromMenuItem(sender) is { } product)
        {
            ViewModel.RequestArchive(product);
        }
    }

    private static ProductListItem? ProductFromMenuItem(object sender) =>
        sender is MenuItem { DataContext: ProductListItem product } ? product : null;

    private void OpenRowMenuClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { ContextMenu: { } menu } button)
        {
            menu.PlacementTarget = button;
            menu.IsOpen = true;
            eventArgs.Handled = true;
        }
    }

    private void ChooseImageClick(object sender, RoutedEventArgs eventArgs)
    {
        var dialog = new OpenFileDialog
        {
            Title = "اختر صورة المنتج",
            Filter = "Image files|*.png;*.jpg;*.jpeg;*.webp;*.bmp|All files|*.*",
            Multiselect = false,
        };

        if (dialog.ShowDialog() != true)
        {
            return;
        }

        var bitmap = new BitmapImage();
        bitmap.BeginInit();
        bitmap.CacheOption = BitmapCacheOption.OnLoad;
        bitmap.UriSource = new System.Uri(dialog.FileName, System.UriKind.Absolute);
        bitmap.EndInit();
        bitmap.Freeze();
        ViewModel.ProductImage = bitmap;
        ViewModel.ProductImageName = System.IO.Path.GetFileName(dialog.FileName);
    }

    private void AddVariantClick(object sender, RoutedEventArgs eventArgs) => ViewModel.AddVariant();

    private void RemoveVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: ProductVariant variant })
        {
            ViewModel.RemoveVariant(variant);
        }
    }

    private void SaveProductClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ViewModel.SaveEditor())
        {
            RestartFeedbackTimer();
        }
        else
        {
            ProductNameBox.Focus();
        }
    }

    private void CancelEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();

    private void ArchiveFromEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.RequestArchiveFromEditor();

    private void ConfirmArchiveClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.ConfirmArchive();
        RestartFeedbackTimer();
    }

    private void CancelArchiveClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelArchive();

    private void AddCategoryFromDropdownClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button button)
        {
            CloseOwningDropdown(button);
            ViewModel.BeginAddCategory();
            Dispatcher.BeginInvoke(CategoryNameBox.Focus, DispatcherPriority.Input);
            eventArgs.Handled = true;
        }
    }

    private void ManageCategoriesFromDropdownClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button button)
        {
            CloseOwningDropdown(button);
            ViewModel.BeginManageCategories();
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

    private void EditCategoryClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: ProductCategoryOption category })
        {
            ViewModel.BeginEditCategory(category);
            Dispatcher.BeginInvoke(CategoryNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private void ArchiveCategoryClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: ProductCategoryOption category })
        {
            ViewModel.ArchiveCategory(category);
        }
    }

    private void SaveCategoryClick(object sender, RoutedEventArgs eventArgs)
    {
        if (!ViewModel.SaveCategory())
        {
            CategoryNameBox.Focus();
        }
    }

    private void CancelCategoryClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelCategoryEditor();

    private void CloseCategoryManagerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CloseCategoryManager();

    private void RestartFeedbackTimer()
    {
        feedbackTimer.Stop();
        feedbackTimer.Start();
    }
}
