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
    private readonly CatalogImages.CatalogImageThumbnails thumbnails = new();
    /// <summary>Connects Rust-owned product data and subscribes to presentation search changes.</summary>
    public void Attach(IEngineShellBridge engine) { engineBridge = engine; CreateClient(); ViewModel.SearchChanged += (_, _) => _ = RefreshAsync(debounce: true); }
    /// <summary>Connects invalidation before refresh so restricted cached fields are removed immediately.</summary>
    private void CreateClient()
    {
        client = new ProductClient(engineBridge!);
        client.ProjectionInvalidated += (_, _) => ClearRestrictedData();
        client.Changed += (_, _) => { if (activated) _ = RefreshAsync(); };
    }
    /// <summary>Starts the scoped subscription before loading the page to avoid a policy-change gap.</summary>
    public async Task ActivateAsync() { if (client is null) { ViewModel.Unavailable("بيانات المنتجات غير متاحة."); return; } activated = true; await client.ActivateAsync(); await RefreshAsync(); }
    /// <summary>Invalidates late completions and replaces the client so retry payloads cannot cross account sessions.</summary>
    public void ClearSession() { pendingCategoryInput = null; activated = false; ClearRestrictedData(); if (client is { } previous) { _ = previous.DisposeAsync(); CreateClient(); } }
    /// <summary>Cancels page queries and releases the scoped subscription when its host closes.</summary>
    public async ValueTask DisposeAsync() { refreshCancellation?.Cancel(); refreshCancellation?.Dispose(); thumbnails.Clear(); if (client is not null) await client.DisposeAsync(); }
    /// <summary>Applies only the latest query result; canceled or invalidated results cannot restore an old projection.</summary>
    private async Task RefreshAsync(bool debounce = false)
    {
        if (client is null || !activated) return;
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose(); var cancellation = new CancellationTokenSource(); refreshCancellation = cancellation; var version = ++refreshVersion;
        try { if (debounce) await Task.Delay(250, cancellation.Token); var result = await client.LoadAsync(ViewModel.SearchText, cancellation.Token); if (version != refreshVersion) return; if (result.Succeeded) {
                ViewModel.ApplyDurableData(result.Value!);
                var images = new CatalogImages.CatalogImageClient(engineBridge!);
                await thumbnails.ApplyAsync(images,
                    result.Value!.Products.Where(p => p.Image is not null).Select(p => (p.Id, p.Image!)),
                    (id, image) => { if (version == refreshVersion) ViewModel.ApplyImage(id, image); }, cancellation.Token);
            } else { if (result.Failure == ProductFailureKind.Denied) ClearRestrictedData(); ViewModel.Unavailable(ProductClient.ArabicMessage(result.Failure)); } }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
    }
    /// <summary>Submits staged fields and rejects a completion from an invalidated session or policy projection.</summary>
    private async Task LoadEditorImageAsync() {
        if(engineBridge is null) return;
        var session=sessionVersion; var editor=ViewModel.ImageEditVersion; var reference=ViewModel.EditorImageReference;
        var image=await new CatalogImages.CatalogImageClient(engineBridge).LoadAsync(reference,2048);
        if(session==sessionVersion && editor==ViewModel.ImageEditVersion && ViewModel.IsEditorOpen) ViewModel.SetImportedImage(reference,image);
    }
    /// <summary>Imports through Rust and ignores results after the editor or authorized session changes.</summary>
    private async void ChooseImageClick(object sender, RoutedEventArgs args) {
        if(engineBridge is null || !ViewModel.CanEdit || !ViewModel.IsEditorOpen) return;
        var pickerSession=sessionVersion; var pickerEditor=ViewModel.ImageEditVersion;
        var dialog=new Microsoft.Win32.OpenFileDialog { Title="اختر صورة المنتج", Filter="صور PNG وJPEG|*.png;*.jpg;*.jpeg", CheckFileExists=true, Multiselect=false };
        if(dialog.ShowDialog(Window.GetWindow(this)) != true) return;
        if(pickerSession!=sessionVersion || pickerEditor!=ViewModel.ImageEditVersion || !ViewModel.IsEditorOpen) return;
        ViewModel.InvalidateImageLoad();
        var session=sessionVersion; var editor=ViewModel.ImageEditVersion; ViewModel.IsBusy=true;
        try {
            var images=new CatalogImages.CatalogImageClient(engineBridge);
            var reference=await images.ImportAsync(CatalogImageKind.Product,dialog.FileName);
            var image=await images.LoadAsync(reference,2048);
            if(session!=sessionVersion || editor!=ViewModel.ImageEditVersion || !ViewModel.IsEditorOpen) return;
            if(reference is null) ViewModel.Fail("تعذر استيراد الصورة. اختر صورة PNG أو JPEG صالحة.");
            else ViewModel.SetImportedImage(reference,image);
        }
        catch(Exception e) when(e is Eitmad.Platform.Windows.LocalIpc.EngineIpcException or IOException or InvalidOperationException or ObjectDisposedException) { if(session==sessionVersion) ViewModel.Fail("تعذر استيراد الصورة. حاول مرة أخرى."); }
        finally { if(session==sessionVersion) ViewModel.IsBusy=false; }
    }
    /// <summary>Stages a null reference without deleting bytes retained by previous revisions.</summary>
    private void RemoveImageClick(object sender, RoutedEventArgs args) { if(ViewModel.CanEdit) { ViewModel.InvalidateImageLoad(); ViewModel.SetImportedImage(null,null); } }

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
    /// <summary>Retains the exact request after an unknown outcome and selects only a successfully created category.</summary>
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
            if (failure == ProductFailureKind.None) { await RefreshAsync(); if (session == sessionVersion) ViewModel.CategorySaved(input); }
            else ViewModel.FailCategory(ViewModel.SavePending ? "لم تتأكد نتيجة الحفظ. أعد المحاولة بنفس البيانات." : ProductClient.ArabicMessage(failure));
        }
        finally { if (session == sessionVersion) ViewModel.IsBusy = false; }
    }

    /// <summary>Clears all retained product fields and invalidates pending loads and saves before reauthorization.</summary>
    private void ClearRestrictedData()
    {
        ++sessionVersion;
        ++refreshVersion;
        refreshCancellation?.Cancel();
        thumbnails.Clear();
        pendingCategoryInput = null;
        ViewModel.ClearSession();
    }

    /// <summary>Finds rendered inputs so invalid purchase-cost fields can receive focus before submission.</summary>
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
        _ = LoadEditorImageAsync();
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
            _ = LoadEditorImageAsync();
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

    /// <summary>Submits the current editor through the asynchronous product save path.</summary>
    private async void SaveProductClick(object sender, RoutedEventArgs args) => await SaveAsync(false);

    private void CancelEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();

    private void ArchiveFromEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.RequestArchiveFromEditor();

    /// <summary>Submits the retained product revision after archive confirmation.</summary>
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

    /// <summary>Submits the selected category revision through the category retry path.</summary>
    private async void ArchiveCategoryClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: ProductCategoryOption category })
        {
            ViewModel.BeginEditCategory(category);
            await SaveCategoryAsync(ViewModel.ArchiveCategoryInput(category));
        }
    }

    /// <summary>Submits staged category fields through the asynchronous category save path.</summary>
    private async void SaveCategoryClick(object sender, RoutedEventArgs args) => await SaveCategoryAsync(ViewModel.CategoryInput());

    private void CancelCategoryClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelCategoryEditor();

    private void CloseCategoryManagerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CloseCategoryManager();

    private void RestartFeedbackTimer()
    {
        Feedback.RestartDuration();
    }
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
}
