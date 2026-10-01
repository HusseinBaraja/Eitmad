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
    private bool activated;
    private SaveFurnitureCategory? pendingCategory;

    public void Attach(IEngineShellBridge engine)
    {
        bridge = engine; CreateClient();
        ViewModel.SearchChanged += (_, _) => { if (activated) _ = RefreshAsync(); };
        ViewModel.PartSearchChanged += (_, _) => { if (activated) _ = RefreshPartChoicesAsync(); };
        ViewModel.ReviewRequested += (_, _) => { if (activated) _ = ReviewAsync(false); };
    }
    private void CreateClient()
    {
        client = new FurnitureClient(bridge!);
        client.Changed += (_, _) => { if (activated) _ = RefreshAsync(); };
        client.ProjectionInvalidated += (_, _) => ClearRestrictedData();
    }
    public async Task ActivateAsync()
    {
        if (client is null) { ViewModel.Unavailable("بيانات الأثاث غير متاحة."); return; }
        activated = true; await client.ActivateAsync(); await RefreshAsync();
    }
    private void ClearRestrictedData()
    {
        ++sessionVersion; ++refreshVersion; ++reviewVersion; ++partSearchVersion; partSearchCancellation?.Cancel();
        refreshCancellation?.Cancel(); reviewCancellation?.Cancel(); pendingCategory=null; ViewModel.ClearSession();
    }
    public void ClearSession()
    {
        activated = false; ClearRestrictedData();
        if (client is { } previous) { _ = previous.DisposeAsync(); CreateClient(); }
    }
    public async ValueTask DisposeAsync()
    {
        refreshCancellation?.Cancel(); reviewCancellation?.Cancel(); partSearchCancellation?.Cancel();
        refreshCancellation?.Dispose(); reviewCancellation?.Dispose(); partSearchCancellation?.Dispose();
        if (client is not null) await client.DisposeAsync();
    }
    private async Task RefreshAsync()
    {
        if (client is null || !activated) return;
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose();
        var cancel = new CancellationTokenSource(); refreshCancellation = cancel; var version = ++refreshVersion; ViewModel.IsLoading=true;
        try
        {
            var result = await client.LoadAsync(ViewModel.SearchText.Trim(), cancel.Token);
            if (version != refreshVersion) return;
            if (result.Succeeded) ViewModel.ApplyDurableData(result.Value!);
            else { if (result.Failure == FurnitureFailureKind.Denied) ClearRestrictedData(); ViewModel.Unavailable(FurnitureClient.ArabicMessage(result.Failure)); }
        }
        catch (OperationCanceledException) when (cancel.IsCancellationRequested) { }
    }
    private async Task RefreshPartChoicesAsync()
    {
        if (client is null || !activated) return;
        partSearchCancellation?.Cancel();partSearchCancellation?.Dispose();var cancel=new CancellationTokenSource();partSearchCancellation=cancel;var version=++partSearchVersion;
        try {
            var result=await client.SearchPartsAsync(ViewModel.PartSearchText.Trim(),cancel.Token);
            if (version!=partSearchVersion) return;
            if (result.Succeeded) ViewModel.ApplyPartChoices(result.Value!);
            else { if (result.Failure==FurnitureFailureKind.Denied) ClearRestrictedData(); ViewModel.Fail(FurnitureClient.ArabicMessage(result.Failure)); }
        }
        catch (OperationCanceledException) when (cancel.IsCancellationRequested) { }
    }
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
        catch (Exception error) when (error is OverflowException or FormatException) { if (showError) ViewModel.Fail("أدخل مقاسات دقيقة وكميات وأسعاراً دون كسور غير مسموحة."); }
        return false;
    }
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
        catch (Exception error) when (error is OverflowException or FormatException) { ViewModel.Fail("أدخل المقاسات بمنزلة عشرية واحدة والكميات والأسعار دون كسور."); }
        finally { if (session == sessionVersion) ViewModel.IsBusy = false; }
    }
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
