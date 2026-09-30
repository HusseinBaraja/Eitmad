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

    public void Attach(IEngineShellBridge engine)
    {
        engineBridge = engine;
        CreateClient();
        ViewModel.SearchChanged += (_, _) => _ = RefreshAsync();
        ViewModel.MaterialSearchChanged += (_, _) => _ = RefreshMaterialChoicesAsync();
    }
    private void CreateClient()
    {
        client = new PartClient(engineBridge!);
        client.Changed += (_, _) => { if (activated) _ = RefreshAsync(); };
    }
    public async Task ActivateAsync()
    {
        if (client is null) { ViewModel.Unavailable("بيانات الأجزاء غير متاحة."); return; }
        activated = true;
        await client.ActivateAsync(); await RefreshAsync();
    }
    public void ClearSession()
    {
        activated = false; ++refreshVersion; refreshCancellation?.Cancel();
        ++materialSearchVersion; materialSearchCancellation?.Cancel();
        if (client is { } previous) { _ = previous.DisposeAsync(); CreateClient(); }
        ViewModel.ClearSession();
    }
    public async ValueTask DisposeAsync()
    {
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose();
        materialSearchCancellation?.Cancel(); materialSearchCancellation?.Dispose();
        if (client is not null) await client.DisposeAsync();
    }
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

    public PartsView()
    {
        InitializeComponent();
        ViewModel = new PartsViewModel();
        DataContext = ViewModel;
    }

    public PartsViewModel ViewModel { get; }

    private void AddPartClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginCreate();
        Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
    }

    private void PartRowInvoked(object sender, RowInvokedEventArgs eventArgs) =>
        OpenEditor((PartListItem)eventArgs.Item);

    private void OpenEditor(PartListItem part)
    {
        ViewModel.BeginEdit(part);
        Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
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
            OpenEditor(part);
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
            if (client is null || ViewModel.IsBusy) return;
            ViewModel.BeginArchive(part);
            SaveEditorClick(sender,eventArgs);
        }
    }

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

    private void NextFromInformationClick(object sender, RoutedEventArgs eventArgs)
    {
        if (!ViewModel.MoveToMaterials())
        {
            EditorNameBox.Focus();
        }
    }

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

    private void PreviousStepClick(object sender, RoutedEventArgs eventArgs) { if (!ViewModel.SavePending) ViewModel.MoveToPreviousStep(); }

    private void OpenMaterialPickerClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.OpenMaterialPicker();

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
        Feedback.RestartDuration();
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
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
}
