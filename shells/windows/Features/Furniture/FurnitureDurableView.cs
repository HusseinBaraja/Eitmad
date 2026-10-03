using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using System.Windows;
using System.Windows.Controls;

namespace Eitmad.WindowsShell.Features.Furniture;

public partial class FurnitureView
{
    private FurnitureClient? client;
    private IEngineShellBridge? bridge;
    private CancellationTokenSource? refreshCancellation, reviewCancellation;
    private long sessionVersion, refreshVersion, reviewVersion, partSearchVersion;
    private CancellationTokenSource? partSearchCancellation;
    private CancellationTokenSource? editorCancellation;
    private long editorVersion;
    private bool activated;
    private SaveFurnitureCategory? pendingCategory;

    /// <summary>Connects presentation requests to the engine bridge and scoped refresh flow.</summary>
    public void Attach(IEngineShellBridge engine)
    {
        bridge = engine; CreateClient();
        ViewModel.SearchChanged += (_, _) => { if (activated) _ = RefreshAsync(debounce: true, reloadCategories: false); };
        ViewModel.PartSearchChanged += (_, _) => { if (activated) _ = RefreshPartChoicesAsync(); };
        ViewModel.ReviewRequested += (_, _) => { if (activated) _ = ReviewAsync(false); };
    }
    /// <summary>Creates a session-local client and attaches projection invalidation handlers.</summary>
    private void CreateClient()
    {
        client = new FurnitureClient(bridge!);
        client.Changed += (_, _) => { if (activated) _ = RefreshAsync(debounce: true); };
        client.PartsChanged += (_, _) => { if (activated && ViewModel.IsEditorOpen && ViewModel.CanEditFields) _ = PrepareEditorAsync(); };
        client.ProjectionInvalidated += (_, _) => ClearRestrictedData();
    }
    /// <summary>Starts subscriptions and loads the authorized Furniture list.</summary>
    public async Task ActivateAsync()
    {
        if (client is null) { ViewModel.Unavailable("بيانات الأثاث غير متاحة."); return; }
        activated = true; await client.ActivateAsync(); await RefreshAsync();
    }
    /// <summary>Cancels pending presentation work and prevents old-session results from restoring restricted data.</summary>
    private void ClearRestrictedData()
    {
        ++sessionVersion; ++refreshVersion; ++reviewVersion; ++partSearchVersion; ++editorVersion; editorCancellation?.Cancel(); partSearchCancellation?.Cancel();
        refreshCancellation?.Cancel(); reviewCancellation?.Cancel(); pendingCategory=null; ViewModel.ClearSession();
    }
    /// <summary>Removes restricted records, costs, retry input, and editor fields when authority ends.</summary>
    public void ClearSession()
    {
        activated = false; ClearRestrictedData();
        if (client is { } previous) { _ = previous.DisposeAsync(); CreateClient(); }
    }
    /// <summary>Cancels outstanding view requests and releases the Furniture subscriptions.</summary>
    public async ValueTask DisposeAsync()
    {
        refreshCancellation?.Cancel(); reviewCancellation?.Cancel(); partSearchCancellation?.Cancel();
        refreshCancellation?.Dispose(); reviewCancellation?.Dispose(); partSearchCancellation?.Dispose();
        editorCancellation?.Cancel(); editorCancellation?.Dispose();
        if (client is not null) await client.DisposeAsync();
    }
    /// <summary>Coalesces search or change requests and applies only the latest list response.</summary>
    private async Task RefreshAsync(bool debounce = false, bool reloadCategories = true)
    {
        if (client is null || !activated) return;
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose();
        var cancel = new CancellationTokenSource(); refreshCancellation = cancel; var version = ++refreshVersion;
        try
        {
            if (debounce) await Task.Delay(250, cancel.Token);
            ViewModel.IsLoading=true;
            var result = await client.LoadAsync(ViewModel.SearchText.Trim(), cancel.Token, reloadCategories);
            if (version != refreshVersion) return;
            if (result.Succeeded) {
                ViewModel.ApplyDurableData(result.Value!);
                var images=new CatalogImages.CatalogImageClient(bridge!);
                foreach(var record in result.Value!.Furniture.Where(p=>p.Image is not null)) {
                    var thumbnail=await images.LoadAsync(record.Image,96,cancel.Token);
                    if(version!=refreshVersion) return;
                    ViewModel.ApplyImage(record.Id,thumbnail);
                }
            }
            else { if (result.Failure == FurnitureFailureKind.Denied) ClearRestrictedData(); ViewModel.Unavailable(FurnitureClient.ArabicMessage(result.Failure)); }
        }
        catch (OperationCanceledException) when (cancel.IsCancellationRequested) { }
    }
    /// <summary>Resolves editor references on demand; ignores results from a cleared session.</summary>
    internal async Task<bool> PrepareEditorAsync(FurnitureListItem? item = null)
    {
        if (client is null) return true;
        if (!activated || !ViewModel.CanEditFields) return false;
        editorCancellation?.Cancel(); editorCancellation?.Dispose();
        var cancel = new CancellationTokenSource(); editorCancellation = cancel; var version = ++editorVersion; var session = sessionVersion;
        ViewModel.IsBusy = true;
        try
        {
            var result = await client.LoadEditorAsync(ViewModel.RecordFor(item), cancel.Token);
            if (session != sessionVersion || version != editorVersion) return false;
            if (result.Succeeded) { ViewModel.ApplyEditorReferences(result.Value!); return true; }
            if (result.Failure == FurnitureFailureKind.Denied) ClearRestrictedData();
            ViewModel.Fail(FurnitureClient.ArabicMessage(result.Failure));
            return false;
        }
        catch (OperationCanceledException) when (cancel.IsCancellationRequested) { return false; }
        finally { if (session == sessionVersion && version == editorVersion) ViewModel.IsBusy = false; }
    }
    /// <summary>Debounces Rust Part search and discards obsolete picker responses.</summary>
    private async Task RefreshPartChoicesAsync()
    {
        if (client is null || !activated) return;
        partSearchCancellation?.Cancel();partSearchCancellation?.Dispose();var cancel=new CancellationTokenSource();partSearchCancellation=cancel;var version=++partSearchVersion;
        try {
            await Task.Delay(250, cancel.Token);
            var result=await client.SearchPartsAsync(ViewModel.PartSearchText.Trim(),cancel.Token);
            if (version!=partSearchVersion) return;
            if (result.Succeeded) ViewModel.ApplyPartChoices(result.Value!);
            else { if (result.Failure==FurnitureFailureKind.Denied) ClearRestrictedData(); ViewModel.Fail(FurnitureClient.ArabicMessage(result.Failure)); }
        }
        catch (OperationCanceledException) when (cancel.IsCancellationRequested) { }
    }
    /// <summary>Requests Rust cost and validation results for staged editor fields.</summary>
    private async Task<bool> ReviewAsync(bool showError)
    {
        if (client is null || !ViewModel.IsEditorOpen || !ViewModel.CanEditFields) return false;
        reviewCancellation?.Cancel(); reviewCancellation?.Dispose();
        var cancel = new CancellationTokenSource(); reviewCancellation = cancel; var version = ++reviewVersion; var session = sessionVersion;
        try
        {
            var result = await client.ReviewAsync(ViewModel.SaveInput(FurnitureState.Draft), cancel.Token);
            if (session != sessionVersion || version != reviewVersion) return false;
            if (result.Succeeded) { ViewModel.ApplyReview(result.Value!); return true; }
            if (result.Failure == FurnitureFailureKind.Denied) ClearRestrictedData();
            if (showError) ViewModel.Fail(FurnitureClient.ArabicMessage(result.Failure));
        }
        catch (OperationCanceledException) when (cancel.IsCancellationRequested) { }
        catch (FurnitureViewModel.UnsavedCategoryException) { if (showError) ViewModel.Fail(FurnitureViewModel.UnsavedCategoryMessage); }
        catch (Exception error) when (error is OverflowException or FormatException) { if (showError) ViewModel.Fail("أدخل مقاسات دقيقة وكميات وأسعاراً دون كسور غير مسموحة."); }
        return false;
    }
    /// <summary>Submits staged or archived input and retains the exact request after an unknown outcome.</summary>
    private async Task SaveAsync(FurnitureState state, FurnitureListItem? archive = null)
    {
        if (client is null || !ViewModel.CanSubmit) { ViewModel.Fail("بيانات الأثاث غير متاحة."); return; }
        if (FindInvalidTextBox(this) is { } invalid) { ViewModel.Fail("صحّح المدخل غير الصالح."); invalid.Focus(); return; }
        var session = sessionVersion; ++reviewVersion; reviewCancellation?.Cancel();
        try
        {
            var input = archive is null ? ViewModel.SaveInput(state) : ViewModel.ArchiveInput(archive);
            ViewModel.IsBusy = true;
            // Freeze the exact payload before sending, including cancellation and lost-response cases.
            ViewModel.SaveUnconfirmed(input);
            var result = await client.SaveAsync(input);
            if (!activated || session != sessionVersion) return;
            if (result != FurnitureFailureKind.Unavailable) ViewModel.SaveResolved();
            if (result == FurnitureFailureKind.Denied) { ClearRestrictedData(); ViewModel.Unavailable(FurnitureClient.ArabicMessage(result)); }
            else if (result == FurnitureFailureKind.None) { ViewModel.Saved(input.State == FurnitureState.Draft); await RefreshAsync(); RestartFeedbackTimer(); }
            else ViewModel.Fail(result == FurnitureFailureKind.Unavailable ? "لم تتأكد نتيجة الحفظ. أعد محاولة الحفظ بنفس البيانات." : FurnitureClient.ArabicMessage(result));
        }
        catch (FurnitureViewModel.UnsavedCategoryException) { ViewModel.Fail(FurnitureViewModel.UnsavedCategoryMessage); }
        catch (Exception error) when (error is OverflowException or FormatException) { ViewModel.Fail("أدخل المقاسات بمنزلة عشرية واحدة والكميات والأسعار دون كسور."); }
        finally { if (session == sessionVersion) ViewModel.IsBusy = false; }
    }
    /// <summary>Saves the typed category and retains its request for an exact unavailable-outcome retry.</summary>
    private async void SaveCategoryClick(object sender, RoutedEventArgs e)
    {
        if (client is null || !ViewModel.CanSubmit) return;
        var input=pendingCategory ?? new SaveFurnitureCategory { Name=ViewModel.EditorCategory.Trim() }; var name=input.Name; var session = sessionVersion;
        ViewModel.IsBusy = true;
        try
        {
            var failure = await client.SaveAsync(input);
            if (session != sessionVersion) return;
            pendingCategory=failure==FurnitureFailureKind.Unavailable ? input : null;
            if (failure == FurnitureFailureKind.None) { await RefreshAsync(); ViewModel.EditorCategory = name; }
            else if (failure == FurnitureFailureKind.Denied) { ClearRestrictedData(); ViewModel.Unavailable(FurnitureClient.ArabicMessage(failure)); }
            else ViewModel.Fail(FurnitureClient.ArabicMessage(failure));
        }
        finally { if (session == sessionVersion) ViewModel.IsBusy = false; }
    }
}
