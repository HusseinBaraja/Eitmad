using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Threading;
using System.Windows.Media;
using Eitmad.WindowsShell.Controls;
using Button = System.Windows.Controls.Button;
using ComboBox = System.Windows.Controls.ComboBox;
using MenuItem = System.Windows.Controls.MenuItem;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Products;

public partial class ProductsView : UserControl
{
    private ProductClient? client;
    private IEngineShellBridge? engineBridge;
    private CancellationTokenSource? refreshCancellation;
    private long refreshVersion, sessionVersion;
    private bool activated;
    private SaveProductCategory? pendingCategoryInput;
    public void Attach(IEngineShellBridge engine) { engineBridge = engine; CreateClient(); ViewModel.SearchChanged += (_, _) => _ = RefreshAsync(); }
    private void CreateClient() { client = new ProductClient(engineBridge!); client.Changed += (_, _) => { if (activated) _ = RefreshAsync(); }; }
    public async Task ActivateAsync() { if (client is null) { ViewModel.Unavailable("بيانات المنتجات غير متاحة."); return; } activated = true; await client.ActivateAsync(); await RefreshAsync(); }
    public void ClearSession() { pendingCategoryInput = null; activated = false; ++sessionVersion; ++refreshVersion; refreshCancellation?.Cancel(); if (client is { } previous) { _ = previous.DisposeAsync(); CreateClient(); } ViewModel.ClearSession(); }
    public async ValueTask DisposeAsync() { refreshCancellation?.Cancel(); refreshCancellation?.Dispose(); if (client is not null) await client.DisposeAsync(); }
    private async Task RefreshAsync()
    {
        if (client is null || !activated) return;
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose(); var cancellation = new CancellationTokenSource(); refreshCancellation = cancellation; var version = ++refreshVersion;
        try { var result = await client.LoadAsync(ViewModel.SearchText, cancellation.Token); if (version != refreshVersion) return; if (result.Succeeded) ViewModel.ApplyDurableData(result.Value!); else { if (result.Failure == ProductFailureKind.Denied) ClearRestrictedData(); ViewModel.Unavailable(ProductClient.ArabicMessage(result.Failure)); } }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
    }
    private async Task SaveAsync(bool archive)
    {
        if (client is null || ViewModel.IsBusy || !ViewModel.CanManage) { ViewModel.Fail("بيانات المنتجات غير متاحة."); return; }
        if (!archive && VisualDescendants<System.Windows.Controls.TextBox>(this).FirstOrDefault(input => input.IsVisible && Validation.GetHasError(input)) is { } invalid)
        {
            ViewModel.Fail("أدخل تكلفة شراء صحيحة بالريال اليمني دون كسور.");
            invalid.Focus();
            return;
        }
        var session = sessionVersion; ViewModel.IsBusy = true;
        try
        {
            var input = archive ? ViewModel.ArchiveInput() : ViewModel.SaveInput();
            var failure = await client.SaveAsync(input);
            if (!activated || session != sessionVersion) return;
            ViewModel.SavePending = failure == ProductFailureKind.Unavailable;
            if (failure == ProductFailureKind.Denied) { ClearRestrictedData(); ViewModel.Unavailable(ProductClient.ArabicMessage(failure)); return; }
            if (failure == ProductFailureKind.None) { ViewModel.Saved(); await RefreshAsync(); RestartFeedbackTimer(); }
            else ViewModel.Fail(ViewModel.SavePending ? "لم تتأكد نتيجة الحفظ. أعد المحاولة بنفس البيانات." : ProductClient.ArabicMessage(failure));
        }
        catch (Exception e) when (e is OverflowException or FormatException) { ViewModel.Fail("أدخل تكلفة الشراء بالريال اليمني دون كسور."); }
        finally { if (session == sessionVersion) ViewModel.IsBusy = false; }
    }
    private async Task SaveCategoryAsync(SaveProductCategory input)
    {
        if (client is null || ViewModel.IsBusy || !ViewModel.CanManage) return;
        var session = sessionVersion; ViewModel.IsBusy = true;
        input = pendingCategoryInput ?? input;
        try
        {
            var failure = await client.SaveAsync(input); if (!activated || session != sessionVersion) return;
            ViewModel.SavePending = failure == ProductFailureKind.Unavailable;
            pendingCategoryInput = ViewModel.SavePending ? input : null;
            if (failure == ProductFailureKind.Denied) { ClearRestrictedData(); ViewModel.Unavailable(ProductClient.ArabicMessage(failure)); return; }
            if (failure == ProductFailureKind.None) { await RefreshAsync(); ViewModel.CategorySaved(input.Name); }
            else ViewModel.FailCategory(ViewModel.SavePending ? "لم تتأكد نتيجة الحفظ. أعد المحاولة بنفس البيانات." : ProductClient.ArabicMessage(failure));
        }
        finally { if (session == sessionVersion) ViewModel.IsBusy = false; }
    }

    private void ClearRestrictedData()
    {
        ++refreshVersion;
        refreshCancellation?.Cancel();
        pendingCategoryInput = null;
        ViewModel.ClearSession();
    }

    private static IEnumerable<T> VisualDescendants<T>(DependencyObject parent) where T : DependencyObject
    {
        for (var index = 0; index < VisualTreeHelper.GetChildrenCount(parent); index++)
        {
            var child = VisualTreeHelper.GetChild(parent, index);
            if (child is T match) yield return match;
            foreach (var descendant in VisualDescendants<T>(child)) yield return descendant;
        }
    }

    public ProductsView()
    {
        InitializeComponent();
        ViewModel = new ProductsViewModel();
        DataContext = ViewModel;
    }

    public ProductsViewModel ViewModel { get; }

    private void ProductRowInvoked(object sender, RowInvokedEventArgs eventArgs) =>
        OpenEditor((ProductListItem)eventArgs.Item);

    private void OpenEditor(ProductListItem product)
    {
        ViewModel.BeginEdit(product);
        Dispatcher.BeginInvoke(ProductNameBox.Focus, DispatcherPriority.Input);
    }

    private void AddProductClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginCreate();
        Dispatcher.BeginInvoke(ProductNameBox.Focus, DispatcherPriority.Input);
    }

    private void EditProductClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ProductFromMenuItem(sender) is { } product)
        {
            OpenEditor(product);
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

    private void AddVariantClick(object sender, RoutedEventArgs eventArgs) => ViewModel.AddVariant();

    private void RemoveVariantClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: ProductVariant variant })
        {
            ViewModel.RemoveVariant(variant);
        }
    }

    private async void SaveProductClick(object sender, RoutedEventArgs args) => await SaveAsync(false);

    private void CancelEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();

    private void ArchiveFromEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.RequestArchiveFromEditor();

    private async void ConfirmArchiveClick(object sender, RoutedEventArgs args) => await SaveAsync(true);

    private void CancelArchiveClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelArchive();

    private void AddCategoryFromDropdownClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button button)
        {
            CloseOwningDropdown(button);
            ViewModel.BeginAddCategory();

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
        if (button.DataContext is ComboBox comboBox)
        {
            comboBox.IsDropDownOpen = false;
        }
    }

    private void EditCategoryClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: ProductCategoryOption category })
        {
            ViewModel.BeginEditCategory(category);

        }
    }

    private async void ArchiveCategoryClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: ProductCategoryOption category })
        {
            ViewModel.BeginEditCategory(category);
            await SaveCategoryAsync(ViewModel.ArchiveCategoryInput(category));
        }
    }

    private async void SaveCategoryClick(object sender, RoutedEventArgs args) => await SaveCategoryAsync(ViewModel.CategoryInput());

    private void CancelCategoryClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelCategoryEditor();

    private void CloseCategoryManagerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CloseCategoryManager();

    private void RestartFeedbackTimer()
    {
        Feedback.RestartDuration();
    }
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
}
