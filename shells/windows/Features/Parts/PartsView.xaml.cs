using System.Windows;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Features.RawMaterials;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using Eitmad.WindowsShell.Controls;
using Button = System.Windows.Controls.Button;
using MenuItem = System.Windows.Controls.MenuItem;
using TextBox = System.Windows.Controls.TextBox;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.Parts;

public partial class PartsView : UserControl
{
    private PartClient? client;
    private IEngineShellBridge? engineBridge;
    private CancellationTokenSource? refreshCancellation;
    private CancellationTokenSource? materialSearchCancellation;
    private long materialSearchVersion;
    private long refreshVersion;
    private bool activated;

    /// <summary>Connects the view to typed IPC and refreshes when part or material search text changes.</summary>
    public void Attach(IEngineShellBridge engine)
    {
        engineBridge = engine;
        CreateClient();
        ViewModel.SearchChanged += (_, _) => _ = RefreshAsync();
        ViewModel.MaterialSearchChanged += (_, _) => _ = RefreshMaterialChoicesAsync();
    }
    /// <summary>Creates session-local retry state and refreshes an active view on Rust invalidation.</summary>
    private void CreateClient()
    {
        client = new PartClient(engineBridge!);
        client.Changed += (_, _) => { if (activated) _ = RefreshAsync(); };
    }
    /// <summary>Starts subscriptions and loads scoped data, or displays unavailable state without a bridge.</summary>
    public async Task ActivateAsync()
    {
        if (client is null) { ViewModel.Unavailable("بيانات الأجزاء غير متاحة."); return; }
        activated = true;
        await client.ActivateAsync(); await RefreshAsync();
    }
    /// <summary>Cancels stale reads and replaces retry state before clearing account-specific presentation data.</summary>
    public void ClearSession()
    {
        activated = false; ++refreshVersion; refreshCancellation?.Cancel();
        ++materialSearchVersion; materialSearchCancellation?.Cancel();
        if (client is { } previous) { _ = previous.DisposeAsync(); CreateClient(); }
        ViewModel.ClearSession();
    }
    /// <summary>Cancels pending reads and releases the typed client when the view closes.</summary>
    public async ValueTask DisposeAsync()
    {
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose();
        materialSearchCancellation?.Cancel(); materialSearchCancellation?.Dispose();
        if (client is not null) await client.DisposeAsync();
    }
    /// <summary>Cancels superseded loads and applies only the latest result without overwriting unsaved editor fields.</summary>
    private async Task RefreshAsync()
    {
        if (client is null) return;
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose();
        var cancellation = new CancellationTokenSource(); refreshCancellation = cancellation;
        var version = ++refreshVersion;
        try
        {
            var result = await client.LoadAsync(ViewModel.SearchText.Trim(),cancellation.Token);
            if (version != refreshVersion) return;
            if (result.Succeeded) { ViewModel.ApplyDurableData(result.Value!); if (ViewModel.IsMaterialPickerOpen) await RefreshMaterialChoicesAsync(); }
            else ViewModel.Unavailable(PartClient.ArabicMessage(result.Failure));
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
    }
    /// <summary>Applies only the latest Rust search result so stale responses cannot replace current picker matches.</summary>
    private async Task RefreshMaterialChoicesAsync()
    {
        if (client is null) return;
        materialSearchCancellation?.Cancel(); materialSearchCancellation?.Dispose();
        var cancellation = new CancellationTokenSource(); materialSearchCancellation = cancellation;
        var version = ++materialSearchVersion;
        try
        {
            var result = await client.SearchMaterialsAsync(ViewModel.MaterialSearchText.Trim(),cancellation.Token);
            if (version != materialSearchVersion) return;
            if (result.Succeeded) ViewModel.ApplyMaterialSearchResults(result.Value!);
            else ViewModel.FailMaterialSearch(PartClient.ArabicMessage(result.Failure));
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
    }

    /// <summary>Creates the native surface and its presentation state before binding controls.</summary>
    public PartsView()
    {
        InitializeComponent();
        ViewModel = new PartsViewModel();
        DataContext = ViewModel;
    }

    public PartsViewModel ViewModel { get; }

    /// <summary>Opens an unsaved part and moves keyboard focus to its name.</summary>
    private void AddPartClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginCreate();
        Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
    }

    /// <summary>Opens the part identified by native table activation.</summary>
    private void PartRowInvoked(object sender, RowInvokedEventArgs eventArgs) =>
        OpenEditor((PartListItem)eventArgs.Item);

    /// <summary>Restores the selected composition and focuses its name for editing.</summary>
    private void OpenEditor(PartListItem part)
    {
        ViewModel.BeginEdit(part);
        Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
    }

    /// <summary>Resolves the row attached to the action menu instead of relying on table selection.</summary>
    private static PartListItem? PartFromMenuItem(object sender) =>
        sender is MenuItem { DataContext: PartListItem part } ? part : null;

    /// <summary>Binds the popup to its invoking row before native placement opens it.</summary>
    private void OpenRowMenuClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { ContextMenu: { } menu })
        {
            menu.PlacementTarget = (Button)sender;
            menu.IsOpen = true;
            eventArgs.Handled = true;
        }
    }

    /// <summary>Opens the record owned by the selected menu action.</summary>
    private void EditMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (PartFromMenuItem(sender) is { } part)
        {
            OpenEditor(part);
        }
    }

    /// <summary>Creates an unsaved copy and focuses its name without submitting a mutation.</summary>
    private void DuplicateMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (PartFromMenuItem(sender) is { } part)
        {
            ViewModel.Duplicate(part);
            RestartFeedbackTimer();
            Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
        }
    }

    /// <summary>Submits an archive through the normal reviewed save path when the client is idle.</summary>
    private void ArchiveMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (PartFromMenuItem(sender) is { } part)
        {
            if (client is null || ViewModel.IsBusy) return;
            ViewModel.BeginArchive(part);
            SaveEditorClick(sender,eventArgs);
        }
    }

    /// <summary>Keeps an unknown save outcome frozen for retry and closes the editor only after Rust success.</summary>
    private async void SaveEditorClick(object sender, RoutedEventArgs eventArgs)
    {
        if (client is null || ViewModel.IsBusy || !ViewModel.IsStepThree) return;
        ViewModel.IsBusy = true;
        try
        {
            var failure = await client.SaveAsync(ViewModel.SaveInput());
            ViewModel.SavePending = failure == MaterialFailureKind.Unavailable;
            if (failure == MaterialFailureKind.None) { ViewModel.Saved(); await RefreshAsync(); RestartFeedbackTimer(); }
            else ViewModel.Fail(ViewModel.SavePending ? "لم تتأكد نتيجة الحفظ. أعد المحاولة بنفس البيانات." : PartClient.ArabicMessage(failure));
        }
        finally { ViewModel.IsBusy = false; }
    }

    /// <summary>Saves a separate part category and selects it after refreshing authoritative references.</summary>
    private async void SaveCategoryClick(object sender, RoutedEventArgs eventArgs)
    {
        if (client is null || ViewModel.IsBusy) return;
        ViewModel.IsBusy = true;
        try
        {
            var name = ViewModel.NewCategoryName;
            var failure = await client.SaveAsync(new SavePartCategory { Name = name });
            if (failure == MaterialFailureKind.None)
            {
                await RefreshAsync();
                ViewModel.EditorCategory = ViewModel.EditorCategoryOptions.FirstOrDefault(c => c.Name == name);
                ViewModel.NewCategoryName = "";
            }
            else ViewModel.Fail(PartClient.ArabicMessage(failure));
        }
        finally { ViewModel.IsBusy = false; }
    }

    /// <summary>Moves to material input or restores focus to the missing information.</summary>
    private void NextFromInformationClick(object sender, RoutedEventArgs eventArgs)
    {
        if (!ViewModel.MoveToMaterials())
        {
            EditorNameBox.Focus();
        }
    }

    /// <summary>Checks input controls and requests Rust cost review before opening the final step.</summary>
    private async void NextFromMaterialsClick(object sender, RoutedEventArgs eventArgs)
    {
        var invalidQuantity = VisualDescendants<TextBox>(MaterialRows)
            .FirstOrDefault(Validation.GetHasError);
        if (invalidQuantity is not null)
        {
            invalidQuantity.Focus();
            return;
        }

        if (client is null || ViewModel.IsBusy) return;
        ViewModel.IsBusy = true;
        try
        {
            ViewModel.RefreshCostReferences();
            var result = await client.CostAsync(ViewModel.UsageInput(),ViewModel.EditingPartId);
            if (result.Succeeded) ViewModel.ApplyCost(result.Value!, review: true);
            else ViewModel.Fail(PartClient.ArabicMessage(result.Failure));
        }
        finally { ViewModel.IsBusy = false; }
    }

    /// <summary>Prevents leaving the reviewed payload while a part save outcome is unknown.</summary>
    private void PreviousStepClick(object sender, RoutedEventArgs eventArgs) { if (!ViewModel.SavePending) ViewModel.MoveToPreviousStep(); }

    /// <summary>Opens the picker with a fresh search state.</summary>
    private void OpenMaterialPickerClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.OpenMaterialPicker();

    }

    /// <summary>Closes the picker without changing selected usages.</summary>
    private void CloseMaterialPickerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CloseMaterialPicker();

    /// <summary>Adds the material identified by the invoking picker row.</summary>
    private void SelectMaterialClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: PartMaterialOption material })
        {
            ViewModel.AddMaterial(material);
        }
    }

    /// <summary>Removes the selected usage and invalidates the reviewed cost.</summary>
    private void RemoveMaterialClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: PartMaterialUsage usage })
        {
            ViewModel.RemoveMaterial(usage);
        }
    }

    /// <summary>Closes unsaved presentation state without a product mutation.</summary>
    private void CancelEditorClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelEditor();

    /// <summary>Restarts the visible success message duration after confirmed save.</summary>
    private void RestartFeedbackTimer()
    {
        Feedback.RestartDuration();
    }

    /// <summary>Enumerates rendered descendants to find invalid material inputs before cost review.</summary>
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
    /// <summary>Clears the presentation message after native dismissal.</summary>
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
}
