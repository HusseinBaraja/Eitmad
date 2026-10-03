using System.Windows;
using System.Windows.Controls;
using System.Windows.Threading;
using System.Globalization;
using Eitmad.Contracts;
using Eitmad.Platform.Windows.Shell;
using Eitmad.WindowsShell.Controls;
using Button = System.Windows.Controls.Button;
using ComboBox = System.Windows.Controls.ComboBox;
using MenuItem = System.Windows.Controls.MenuItem;
using UserControl = System.Windows.Controls.UserControl;

namespace Eitmad.WindowsShell.Features.RawMaterials;

public partial class RawMaterialsView : UserControl
{
    private MaterialClient? client;
    private CancellationTokenSource? refreshCancellation;
    private long refreshVersion;

    public RawMaterialsView()
    {
        InitializeComponent();
        ViewModel = new RawMaterialsViewModel();
        DataContext = ViewModel;
    }

    public RawMaterialsViewModel ViewModel { get; }

    public void Attach(IEngineShellBridge engine)
    {
        client = new MaterialClient(engine);
        client.Changed += (_, _) => _ = RefreshAsync();
        ViewModel.SearchChanged += (_, _) => _ = RefreshAsync();
    }

    public async Task ActivateAsync()
    {
        if (client is null) return;
        await client.ActivateAsync();
        await RefreshAsync();
    }

    public async ValueTask DisposeAsync()
    {
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose();
        if (client is not null) await client.DisposeAsync();
    }

    private async Task RefreshAsync()
    {
        if (client is null) return;
        refreshCancellation?.Cancel(); refreshCancellation?.Dispose();
        var cancellation = new CancellationTokenSource();
        refreshCancellation = cancellation;
        var version = ++refreshVersion;
        try
        {
            var result = await client.LoadAsync(ViewModel.SearchText.Trim(), cancellation.Token);
            if (version != refreshVersion) return;
            if (result.Succeeded) ViewModel.ApplyDurableData(result.Value!.References, result.Value.Materials);
            else ViewModel.Unavailable(MaterialClient.ArabicMessage(result.Failure));
        }
        catch (OperationCanceledException) when (cancellation.IsCancellationRequested) { }
    }

    private void AddRawMaterialClick(object sender, RoutedEventArgs eventArgs)
    {
        ViewModel.BeginCreate();
        Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
    }

    private void RawMaterialRowInvoked(object sender, RowInvokedEventArgs eventArgs) =>
        OpenEditor((RawMaterialListItem)eventArgs.Item);

    private void OpenEditor(RawMaterialListItem material)
    {
        ViewModel.BeginEdit(material);
        Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
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
            OpenEditor(material);
        }
    }

    private void DuplicateMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (MaterialFromMenuItem(sender) is { } material)
        {
            ViewModel.Duplicate(material);
            Dispatcher.BeginInvoke(EditorNameBox.Focus, DispatcherPriority.Input);
        }
    }

    private async void ArchiveMenuItemClick(object sender, RoutedEventArgs eventArgs)
    {
        if (MaterialFromMenuItem(sender) is { } material)
        {
            if (client is null) { ViewModel.Unavailable(MaterialClient.ArabicMessage(MaterialFailureKind.Unavailable)); return; }
            var failure = await client.SaveAsync(new SaveMaterial
            {
                Id = material.Id, ExpectedRevision = material.Revision, Name = material.Name,
                CategoryId = material.CategoryId, UnitId = material.UnitId, CurrentCostYer = (long)material.CurrentCost,
                Archived = true,
            });
            if (failure == MaterialFailureKind.None)
            { ViewModel.Saved("أُرشفت المادة الخام."); await RefreshAsync(); RestartFeedbackTimer(); }
            else ViewModel.Unavailable(MaterialClient.ArabicMessage(failure));
        }
    }

    private async void SaveEditorClick(object sender, RoutedEventArgs eventArgs)
    {
        if (client is null) { ViewModel.Fail(MaterialClient.ArabicMessage(MaterialFailureKind.Unavailable)); return; }

        var category = ViewModel.EditorCategories.FirstOrDefault(item => item.Id == ViewModel.EditorCategoryId);
        var unit = ViewModel.EditorUnits.FirstOrDefault(item => item.Id == ViewModel.EditorUnitId);
        if (category?.Id is not { } categoryId || unit?.Id is not { } unitId
            || !TryParseWholeCost(EditorCostBox.Text, out var cost))
        { ViewModel.Fail("اختر تصنيفاً ووحدة، وأدخل تكلفة صحيحة بالريال اليمني."); return; }
        ViewModel.EditorCost = cost;
        var failure = await client.SaveAsync(new SaveMaterial
        {
            Id = ViewModel.EditingMaterial?.Id,
            ExpectedRevision = ViewModel.EditingMaterial?.Revision,
            Name = ViewModel.EditorName.Trim(), CategoryId = categoryId, UnitId = unitId,
            CurrentCostYer = cost,
            Archived = ViewModel.EditingMaterial?.IsArchived ?? false,
        });
        if (failure == MaterialFailureKind.None)
        { ViewModel.Saved("حُفظت المادة الخام."); await RefreshAsync(); RestartFeedbackTimer(); }
        else ViewModel.Fail(MaterialClient.ArabicMessage(failure));
    }

    private static bool TryParseWholeCost(string input, out long cost)
    {
        var digits = new string(input.Trim().Select(c => c switch
        {
            >= '٠' and <= '٩' => (char)('0' + c - '٠'),
            >= '۰' and <= '۹' => (char)('0' + c - '۰'),
            _ => c,
        }).ToArray());
        return long.TryParse(digits, NumberStyles.None, CultureInfo.InvariantCulture, out cost);
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
        if (button.DataContext is ComboBox comboBox)
        {
            comboBox.IsDropDownOpen = false;
        }
    }

    private void EditReferenceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: RawMaterialReferenceOption reference })
        {
            ViewModel.BeginEditReference(reference);

        }
    }

    private void AddManagedReferenceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (ViewModel.IsCategoryReference) ViewModel.BeginAddCategory();
        else ViewModel.BeginAddUnit();
    }

    private async void ArchiveReferenceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (sender is Button { DataContext: RawMaterialReferenceOption reference })
        {
            if (client is null) { ViewModel.Fail(MaterialClient.ArabicMessage(MaterialFailureKind.Unavailable), reference: true); return; }
            var failure = ViewModel.IsCategoryReference
                ? await client.SaveAsync(new SaveMaterialCategory
                    { Id = reference.Id, ExpectedRevision = reference.Revision, Name = reference.Name, Archived = true })
                : await client.SaveAsync(new SaveMaterialUnit
                    { Id = reference.Id, ExpectedRevision = reference.Revision, Name = reference.Name, Symbol = reference.ShortName,
                      Dimension = reference.Dimension, Numerator = reference.Numerator,
                      Denominator = reference.Denominator, Archived = true });
            if (failure == MaterialFailureKind.None)
            {
                ViewModel.Saved("أُرشف المرجع.");
                await RefreshAsync();
                RestartFeedbackTimer();
            }
            else ViewModel.Fail(MaterialClient.ArabicMessage(failure), reference: true);
        }
    }

    private async void SaveReferenceClick(object sender, RoutedEventArgs eventArgs)
    {
        if (client is null) { ViewModel.Fail(MaterialClient.ArabicMessage(MaterialFailureKind.Unavailable), reference: true); return; }

        MaterialFailureKind failure;
        var selected = ViewModel.EditingReference;
        if (ViewModel.IsCategoryReference)
            failure = await client.SaveAsync(new SaveMaterialCategory
                { Id = selected?.Id, ExpectedRevision = selected?.Revision,
                  Name = ViewModel.ReferenceName.Trim(), Archived = selected?.IsArchived ?? false });
        else
        {
            if (!long.TryParse(UnitNumeratorBox.Text, NumberStyles.None, CultureInfo.InvariantCulture, out var numerator)
                || !long.TryParse(UnitDenominatorBox.Text, NumberStyles.None, CultureInfo.InvariantCulture, out var denominator)
                || numerator <= 0 || denominator <= 0)
            { ViewModel.Fail("أدخل بسطاً ومقاماً صحيحين أكبر من صفر.", reference: true); return; }
            failure = await client.SaveAsync(new SaveMaterialUnit
                { Id = selected?.Id, ExpectedRevision = selected?.Revision,
                  Name = ViewModel.ReferenceName.Trim(), Symbol = ViewModel.ReferenceShortName.Trim(),
                  Dimension = ViewModel.ReferenceDimension, Numerator = numerator,
                  Denominator = denominator, Archived = selected?.IsArchived ?? false });
        }
        if (failure == MaterialFailureKind.None)
        {
            var name = ViewModel.ReferenceName.Trim();
            var isCategory = ViewModel.IsCategoryReference;
            ViewModel.Saved("حُفظ المرجع.");
            await RefreshAsync();
            if (isCategory)
            {
                if (selected?.Id is { } id) ViewModel.EditorCategoryId = id;
                else ViewModel.EditorCategory = name;
            }
            else
            {
                if (selected?.Id is { } id) ViewModel.EditorUnitId = id;
                else ViewModel.EditorUnit = name;
            }
        }
        else ViewModel.Fail(MaterialClient.ArabicMessage(failure), reference: true);
    }

    private void CancelReferenceClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CancelReferenceEditor();

    private void CloseReferenceManagerClick(object sender, RoutedEventArgs eventArgs) => ViewModel.CloseReferenceManager();

    private void RestartFeedbackTimer()
    {
        Feedback.RestartDuration();
    }
    private void FeedbackDismissed(object sender, RoutedEventArgs e) => ViewModel.ClearFeedback();
}
