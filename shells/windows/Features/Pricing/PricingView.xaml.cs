using System.Windows;
using System.Windows.Controls;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Controls;
using Button = System.Windows.Controls.Button;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Pricing;

public partial class PricingView : UserControl
{
    private PricingClient? client;
    private IEngineShellBridge? engine;
    private CancellationTokenSource? refreshCancellation, reviewCancellation;
    private long refreshVersion, reviewVersion, sessionVersion;
    private bool activated;
    public PricingView()
    {
        InitializeComponent();
        ViewModel = new(); DataContext = ViewModel;
        ViewModel.SearchChanged += (_, _) => _ = RefreshAsync(true);
        ViewModel.EditorPriceChanged += (_, _) => _ = ReviewAsync();
    }
    public PricingViewModel ViewModel { get; }
    public void Attach(IEngineShellBridge bridge) { engine = bridge; CreateClient(); }
    private void CreateClient()
    {
        client = new(engine!);
        client.ProjectionInvalidated += (_, _) => ClearProjection();
        client.Changed += (_, _) => { if (activated) _ = RefreshAsync(); };
    }
    public async Task ActivateAsync()
    {
        if (client is null) { ViewModel.Unavailable("بيانات الأسعار غير متاحة."); return; }
        activated = true;
        await client.ActivateAsync(); await RefreshAsync();
    }
    public void ClearSession()
    {
        activated = false; ClearProjection();
        if (client is { } previous) { _ = previous.DisposeAsync(); CreateClient(); }
    }
    private void ClearProjection()
    {
        ++sessionVersion; ++refreshVersion; ++reviewVersion;
        refreshCancellation?.Cancel(); reviewCancellation?.Cancel(); ViewModel.ClearSession();
    }
    public async ValueTask DisposeAsync()
    {
        refreshCancellation?.Cancel(); reviewCancellation?.Cancel();
        refreshCancellation?.Dispose(); reviewCancellation?.Dispose();
        if (client is not null) await client.DisposeAsync();
    }
    private async Task RefreshAsync(bool debounce = false)
    {
        if (client is null || !activated) return;
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose();
        var cancellation = new CancellationTokenSource(); refreshCancellation = cancellation;
        var version = ++refreshVersion;
        try
        {
            if (debounce) await Task.Delay(250, cancellation.Token);
            var result = await client.LoadAsync(ViewModel.SearchText, cancellation.Token);
            if (version != refreshVersion) return;
            if (result.Succeeded) ViewModel.ApplyDurableData(result.Value!);
            else { ClearProjection(); ViewModel.Unavailable(PricingClient.ArabicMessage(result.Failure)); }
            UpdateColumns();
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
        catch (Exception)
        {
            if (version != refreshVersion || !activated) return;
            ClearProjection();
            ViewModel.Unavailable("تعذر تحميل الأسعار. أعد تحميلها للمحاولة مجدداً.");
            UpdateColumns();
        }
    }
    private void UpdateColumns()
    {
        PricingRows.Columns[2].Visibility = ViewModel.CanReadCosts ? Visibility.Visible : Visibility.Collapsed;
        PricingRows.Columns[4].Visibility = ViewModel.CanReadCosts ? Visibility.Visible : Visibility.Collapsed;
        PricingRows.Columns[6].Visibility = ViewModel.CanManage ? Visibility.Visible : Visibility.Collapsed;
        PricingRows.IsRowInvocationEnabled = ViewModel.CanManage && ViewModel.CanReadCosts;
    }
    private async Task ReviewAsync()
    {
        reviewCancellation?.Cancel(); reviewCancellation?.Dispose();
        var cancellation = new CancellationTokenSource(); reviewCancellation = cancellation;
        var version = ++reviewVersion; var editor = ViewModel.EditorVersion; var session = sessionVersion;
        if (client is null || !ViewModel.IsEditorOpen || !ViewModel.CanReadCosts || ViewModel.SavePending) return;
        var input = ViewModel.ReviewInput();
        if (input is null) return;
        try
        {
            await Task.Delay(150, cancellation.Token);
            var result = await client.ReviewAsync(input, cancellation.Token);
            if (version != reviewVersion || editor != ViewModel.EditorVersion || session != sessionVersion || !ViewModel.IsEditorOpen) return;
            if (result.Succeeded) ViewModel.ApplyReview(result.Value!);
            else if (result.Failure == PricingFailure.Denied) { ClearProjection(); _ = RefreshAsync(); }
            else ViewModel.Fail(PricingClient.ArabicMessage(result.Failure));
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
    }
    private void PriceRowInvoked(object sender, RowInvokedEventArgs e) => ViewModel.BeginEdit((PricingListItem)e.Item);
    private void EditPriceClick(object sender, RoutedEventArgs e)
    {
        if (sender is Button { DataContext: PricingListItem item }) ViewModel.BeginEdit(item);
    }
    private async void SavePriceClick(object sender, RoutedEventArgs e)
    {
        if (client is null || !ViewModel.CanSave) return;
        var input = ViewModel.SaveInput();
        if (input is null) { PriceInput.Focus(); return; }
        var session = sessionVersion; ViewModel.IsBusy = true;
        try
        {
            var result = await client.PublishAsync(input);
            if (!activated || session != sessionVersion) return;
            ViewModel.SavePending = result.Failure == PricingFailure.Unconfirmed;
            if (result.Succeeded) { ViewModel.Saved(); await RefreshAsync(); Feedback.RestartDuration(); }
            else if (result.Failure == PricingFailure.Denied) { ClearProjection(); await RefreshAsync(); }
            else { ViewModel.Fail(PricingClient.ArabicMessage(result.Failure)); PriceInput.Focus(); }
        }
        catch (Exception)
        {
            // Consume late failures as well; only the current session can show retry state.
            if (!activated || session != sessionVersion) return;
            ViewModel.SavePending = true;
            ViewModel.Fail(PricingClient.ArabicMessage(PricingFailure.Unconfirmed));
        }
        finally { if (session == sessionVersion) ViewModel.IsBusy = false; }
    }
    private void CancelPriceClick(object sender, RoutedEventArgs e) => ViewModel.CancelEditor();
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
    private async void ReloadClick(object sender, RoutedEventArgs e) => await RefreshAsync();
}
