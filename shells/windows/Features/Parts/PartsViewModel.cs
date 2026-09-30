using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using Eitmad.Contracts;
using Eitmad.WindowsShell.Features.RawMaterials;

namespace Eitmad.WindowsShell.Features.Parts;

/// <summary>Projects Rust parts and keeps unsaved wizard fields.</summary>
public sealed class PartsViewModel : ObservableObject
{
    public const string AllCategories = "كل الفئات", AllStatuses = "كل الحالات", ActiveStatus = "نشط", ArchivedStatus = "مؤرشف";
    private readonly Dictionary<Guid, Part> records = [];
    private readonly Dictionary<Guid, PartCost> currentCosts = [];
    private readonly Dictionary<Guid, PartCategory> categories = [];
    private readonly List<PartMaterialOption> materials = [];
    private Part? editingPart;
    private HashSet<Guid>? matchingMaterials;
    private string materialSearchMessage = "";
    private string searchText = "", selectedCategory = AllCategories, selectedStatus = AllStatuses;
    private string editorName = "", editorDescription = "", editorError = "", materialSearchText = "", feedbackMessage = "";
    private string availabilityMessage = "جار تحميل الأجزاء…", newCategoryName = "";
    private PartCategory? editorCategory;
    private bool isEditorOpen, isCreating, isMaterialPickerOpen, isBusy, savePending, archiveRequested;
    private int currentStep = 1;
    private long? totalCost;
    public event EventHandler? SearchChanged;
    public event EventHandler? MaterialSearchChanged;
    public ObservableCollection<PartListItem> VisibleParts { get; } = [];
    public ObservableCollection<PartMaterialUsage> SelectedMaterials { get; } = [];
    public ObservableCollection<PartMaterialOption> FilteredMaterials { get; } = [];
    public ObservableCollection<string> CategoryOptions { get; } = [AllCategories];
    public ObservableCollection<PartCategory> EditorCategoryOptions { get; } = [];
    public IReadOnlyList<string> StatusOptions { get; } = [AllStatuses, ActiveStatus, ArchivedStatus];
    public string SearchText { get => searchText; set { if (Set(ref searchText,value)) SearchChanged?.Invoke(this,EventArgs.Empty); } }
    public string SelectedCategory { get => selectedCategory; set { if (Set(ref selectedCategory,value ?? AllCategories)) RefreshVisible(); } }
    public string SelectedStatus { get => selectedStatus; set { if (Set(ref selectedStatus,value ?? AllStatuses)) RefreshVisible(); } }
    public bool IsEditorOpen { get => isEditorOpen; private set => Set(ref isEditorOpen,value); }
    public bool IsCreating { get => isCreating; private set { Set(ref isCreating,value); Raise(nameof(EditorTitle)); Raise(nameof(SaveButtonLabel)); } }
    public string EditorTitle => IsCreating ? "إنشاء جزء جديد" : "تعديل الجزء";
    public bool SavePending { get => savePending; set { Set(ref savePending,value); Raise(nameof(SaveButtonLabel)); Raise(nameof(CanLeaveReview)); } }
    public bool CanLeaveReview => !SavePending;
    public string SaveButtonLabel => SavePending ? "إعادة محاولة الحفظ" : IsCreating ? "حفظ الجزء" : "حفظ التعديلات";
    public int CurrentStep { get => currentStep; private set { Set(ref currentStep,value); Raise(nameof(IsStepOne)); Raise(nameof(IsStepTwo)); Raise(nameof(IsStepThree)); } }
    public bool IsStepOne => CurrentStep == 1;
    public bool IsStepTwo => CurrentStep == 2;
    public bool IsStepThree => CurrentStep == 3;
    public bool IsBusy { get => isBusy; set { Set(ref isBusy,value); Raise(nameof(CanInteract)); } }
    public bool CanInteract => !IsBusy;
    public string EditorName { get => editorName; set => Set(ref editorName,value); }
    public PartCategory? EditorCategory { get => editorCategory; set => Set(ref editorCategory,value); }
    public string EditorCategoryLabel => EditorCategory?.Name ?? "";
    public string EditorDescription { get => editorDescription; set => Set(ref editorDescription,value); }
    public string NewCategoryName { get => newCategoryName; set => Set(ref newCategoryName,value); }
    public string EditorError { get => editorError; private set { Set(ref editorError,value); Raise(nameof(HasEditorError)); } }
    public bool HasEditorError => EditorError.Length > 0;
    public string AvailabilityMessage { get => availabilityMessage; private set { Set(ref availabilityMessage,value); Raise(nameof(HasAvailabilityMessage)); Raise(nameof(PageSubtitle)); } }
    public string PageSubtitle => HasAvailabilityMessage ? AvailabilityMessage : "الأجزاء محفوظة محلياً — التكلفة من المحرك";
    public bool HasAvailabilityMessage => AvailabilityMessage.Length > 0;
    public string FeedbackMessage { get => feedbackMessage; private set { Set(ref feedbackMessage,value); Raise(nameof(HasFeedback)); } }
    public bool HasFeedback => FeedbackMessage.Length > 0;
    public bool IsMaterialPickerOpen { get => isMaterialPickerOpen; private set => Set(ref isMaterialPickerOpen,value); }
    public string MaterialSearchText { get => materialSearchText; set { if (Set(ref materialSearchText,value)) { matchingMaterials = null; materialSearchMessage = value.Length == 0 ? "" : "جار البحث…"; RefreshMaterialOptions(); MaterialSearchChanged?.Invoke(this,EventArgs.Empty); } } }
    public bool HasSelectedMaterials => SelectedMaterials.Count > 0;
    public bool HasNoMaterialOptions => FilteredMaterials.Count == 0;
    public string MaterialOptionsMessage => materialSearchMessage.Length > 0 ? materialSearchMessage : HasNoMaterialOptions ? "لا توجد مواد خام مطابقة" : "";
    public bool HasMaterialOptionsMessage => MaterialOptionsMessage.Length > 0;
    public string TotalPartCostLabel => totalCost?.ToString("N0",CultureInfo.InvariantCulture) ?? "—";
    public bool HasNoVisibleParts => VisibleParts.Count == 0 && !HasAvailabilityMessage;
    public string VisibleCountLabel => $"{VisibleParts.Count} أجزاء";

    /// <summary>Replaces Rust projections while retaining unsaved editor fields and selected category identity.</summary>
    public void ApplyDurableData(PartSnapshot data)
    {
        var selectedEditorCategory = EditorCategory;
        var filter = SelectedCategory;
        records.Clear(); currentCosts.Clear(); foreach (var projection in data.Parts) { records[projection.Part.Id] = projection.Part; currentCosts[projection.Part.Id] = projection.CurrentCost; }
        categories.Clear();
        EditorCategoryOptions.Clear();
        foreach (var name in CategoryOptions.Where(n => n != AllCategories && !data.Categories.Items.Any(c => c.Name == n)).ToArray()) CategoryOptions.Remove(name);
        foreach (var category in data.Categories.Items)
        {
            categories[category.Id] = category;
            if (!CategoryOptions.Contains(category.Name)) CategoryOptions.Add(category.Name);
            if (!category.Archived || editingPart?.CategoryId == category.Id) EditorCategoryOptions.Add(category);
        }
        // Retain the unsaved category identity across reference renames.
        if (selectedEditorCategory is { } selected) EditorCategory = EditorCategoryOptions.FirstOrDefault(c => c.Id == selected.Id) ?? selected;
        SelectedCategory = CategoryOptions.Contains(filter) ? filter : AllCategories;
        Raise(nameof(SelectedCategory));
        materials.Clear();
        materials.AddRange(ProjectMaterials(data.Materials));
        AvailabilityMessage = ""; RefreshVisible(); RefreshMaterialOptions();
    }
    /// <summary>Offers active materials and same-dimension active units; saved references are restored separately.</summary>
    private static IEnumerable<PartMaterialOption> ProjectMaterials(MaterialSnapshot data)
    {
        foreach (var material in data.Materials.Where(m => !m.Archived))
        {
            var unit = data.References.Units.FirstOrDefault(u => u.Id == material.UnitId);
            if (unit is null) continue;
            var options = data.References.Units.Where(u => !u.Archived && u.Dimension == unit.Dimension).Select(u => new PartUnitOption(u)).ToArray();
            yield return new PartMaterialOption(material,unit,options);
        }
    }
    /// <summary>Uses Rust result identities for picker matches and refreshes their reference projections.</summary>
    public void ApplyMaterialSearchResults(MaterialSnapshot data)
    {
        matchingMaterials = data.Materials.Select(m => m.Id).ToHashSet(); materialSearchMessage = "";
        foreach (var option in ProjectMaterials(data)) { materials.RemoveAll(m => m.Id == option.Id); materials.Add(option); }
        RefreshMaterialOptions();
    }
    /// <summary>Clears stale matches and displays the failed search state.</summary>
    public void FailMaterialSearch(string message)
    {
        matchingMaterials = []; materialSearchMessage = message; RefreshMaterialOptions();
    }
    /// <summary>Clears visible records when scoped data cannot be loaded without discarding unsaved input.</summary>
    public void Unavailable(string message) { AvailabilityMessage = message; records.Clear(); VisibleParts.Clear(); Raise(nameof(HasNoVisibleParts)); }
    /// <summary>Clears account-specific records, searches, editor fields, and pending-save presentation state.</summary>
    public void ClearSession()
    {
        SavePending = false; CancelEditor(); records.Clear(); currentCosts.Clear(); categories.Clear(); materials.Clear(); VisibleParts.Clear();
        ReplaceMaterials([]); EditorCategoryOptions.Clear(); CategoryOptions.Clear(); CategoryOptions.Add(AllCategories);
        EditorName = ""; EditorDescription = ""; NewCategoryName = ""; EditorCategory = null; editingPart = null;
        searchText = ""; materialSearchText = ""; matchingMaterials = null; materialSearchMessage = ""; Raise(nameof(SearchText)); Raise(nameof(MaterialSearchText));
        SelectedCategory = AllCategories; SelectedStatus = AllStatuses; ClearFeedback();
        AvailabilityMessage = "جار تحميل الأجزاء…";
    }
    /// <summary>Displays an editor failure while preserving unsaved fields.</summary>
    public void Fail(string message) => EditorError = message;
    /// <summary>Starts an unsaved part with an active category and no usages.</summary>
    public void BeginCreate()
    {
        archiveRequested = false; editingPart = null; IsCreating = true; EditorName = ""; EditorDescription = "";
        EditorCategory = EditorCategoryOptions.FirstOrDefault(c => !c.Archived);
        ReplaceMaterials([]); OpenEditor();
    }
    /// <summary>Restores a saved part revision and current cost rows, including retained archived references.</summary>
    public void BeginEdit(PartListItem row)
    {
        if (!records.TryGetValue(row.Id,out var part)) return;
        archiveRequested = false; editingPart = part; IsCreating = false; EditorName = part.Name; EditorDescription = part.Description;
        EditorCategory = categories.GetValueOrDefault(part.CategoryId);
        if (EditorCategory is { } c && !EditorCategoryOptions.Contains(c)) EditorCategoryOptions.Add(c);
        ReplaceMaterials(currentCosts[part.Id].Rows.Select(r =>
        {
            var options = materials.FirstOrDefault(m => m.Id == r.Material.Id)?.Units.ToList() ?? [];
            if (options.All(u => u.Record.Id != r.Unit.Id)) options.Add(new PartUnitOption(r.Unit));
            var usage = new PartMaterialUsage(new PartMaterialOption(r.Material,r.CostUnit,options),r.Usage.Quantity,r.Unit.Id);
            usage.SetCost(r.CostYer); return usage;
        }));
        totalCost = currentCosts[part.Id].TotalCostYer; Raise(nameof(TotalPartCostLabel)); OpenEditor();
    }
    /// <summary>Starts the information step and clears prior editor errors and picker state.</summary>
    private void OpenEditor() { EditorError = ""; CurrentStep = 1; IsMaterialPickerOpen = false; IsEditorOpen = true; Raise(nameof(EditorCategoryLabel)); }
    /// <summary>Preserves the existing composition and opens its review for an audited archive save.</summary>
    public void BeginArchive(PartListItem row)
    {
        BeginEdit(row); archiveRequested = true;
        ApplyCost(currentCosts[row.Id],review:true);
    }
    /// <summary>Copies editor fields but clears record identity so Rust creates a new part on save.</summary>
    public void Duplicate(PartListItem row) { BeginEdit(row); editingPart = null; IsCreating = true; EditorName += " — نسخة"; }
    /// <summary>Closes the wizard and picker without implying a durable save.</summary>
    public void CancelEditor() { IsEditorOpen = false; IsMaterialPickerOpen = false; EditorError = ""; }
    /// <summary>Requires visible name and category input before advancing; Rust remains the domain validator.</summary>
    public bool MoveToMaterials()
    {
        if (EditorName.Length == 0 || EditorCategory is null) { EditorError = "أدخل اسم الجزء واختر فئة."; return false; }
        EditorError = ""; CurrentStep = 2; return true;
    }
    /// <summary>Displays authoritative row and total costs and optionally advances to review.</summary>
    public void ApplyCost(PartCost cost, bool review = false)
    {
        foreach (var row in cost.Rows) SelectedMaterials.FirstOrDefault(u => u.Material.Id == row.Usage.MaterialId)?.SetCost(row.CostYer);
        totalCost = cost.TotalCostYer; Raise(nameof(TotalPartCostLabel));
        EditorError = ""; if (review) { CurrentStep = 3; Raise(nameof(EditorCategoryLabel)); }
    }
    /// <summary>Refreshes selected material references before review without discarding their selected usage units.</summary>
    public void RefreshCostReferences()
    {
        foreach (var row in SelectedMaterials)
            if (materials.FirstOrDefault(m => m.Id == row.Material.Id) is { } latest) row.RefreshReference(latest);
    }
    public Guid? EditingPartId => editingPart?.Id;
    /// <summary>Projects selected usages with exact quantity text and reviewed reference revisions.</summary>
    public PartUsage[] UsageInput() => SelectedMaterials.Select(u => u.ToInput()).ToArray();
    /// <summary>Builds the Rust save command with original expected revision and requested archive state.</summary>
    public SavePart SaveInput(bool? archived = null) => new()
    {
        Id = editingPart?.Id, ExpectedRevision = editingPart?.Revision, Name = EditorName,
        CategoryId = EditorCategory?.Id ?? Guid.Empty, Description = EditorDescription,
        Usages = UsageInput(), Archived = archived ?? (archiveRequested || editingPart?.Archived == true),
    };
    /// <summary>Closes the wizard and reports local persistence after confirmed Rust success.</summary>
    public void Saved() { SavePending = false; CancelEditor(); FeedbackMessage = "حُفظ الجزء محلياً."; }
    /// <summary>Returns one wizard step and clears the prior editor error.</summary>
    public void MoveToPreviousStep() { if (CurrentStep > 1) CurrentStep--; EditorError = ""; }
    /// <summary>Clears the prior search and opens choices without altering selected materials.</summary>
    public void OpenMaterialPicker() { MaterialSearchText = ""; RefreshMaterialOptions(); IsMaterialPickerOpen = true; }
    /// <summary>Hides choices while preserving current material usages.</summary>
    public void CloseMaterialPicker() => IsMaterialPickerOpen = false;
    /// <summary>Prevents duplicate material rows and clears reviewed costs after a new usage.</summary>
    public void AddMaterial(PartMaterialOption material)
    {
        if (SelectedMaterials.Any(u => u.Material.Id == material.Id)) return;
        AddUsage(new PartMaterialUsage(material)); InvalidateCost(); RefreshMaterialOptions(); IsMaterialPickerOpen = false;
    }
    /// <summary>Detaches usage notifications and invalidates costs after removing a row.</summary>
    public void RemoveMaterial(PartMaterialUsage usage) { usage.PropertyChanged -= UsageChanged; SelectedMaterials.Remove(usage); InvalidateCost(); RefreshMaterialOptions(); }
    /// <summary>Clears the transient confirmed-save message.</summary>
    public void ClearFeedback() => FeedbackMessage = "";
    /// <summary>Replaces usage event handlers and clears the old total before refreshing picker choices.</summary>
    private void ReplaceMaterials(IEnumerable<PartMaterialUsage> rows)
    {
        foreach (var row in SelectedMaterials) row.PropertyChanged -= UsageChanged;
        SelectedMaterials.Clear(); foreach (var row in rows) AddUsage(row); totalCost = null; Raise(nameof(TotalPartCostLabel)); Raise(nameof(HasSelectedMaterials)); RefreshMaterialOptions();
    }
    /// <summary>Attaches invalidation notifications to one unsaved material row.</summary>
    private void AddUsage(PartMaterialUsage row) { row.PropertyChanged += UsageChanged; SelectedMaterials.Add(row); }
    /// <summary>Invalidates Rust review amounts only when quantity or selected unit changes.</summary>
    private void UsageChanged(object? sender,PropertyChangedEventArgs e) { if (e.PropertyName is nameof(PartMaterialUsage.Quantity) or nameof(PartMaterialUsage.SelectedUnit)) InvalidateCost(); }
    /// <summary>Clears displayed row and total costs until Rust reviews the new input.</summary>
    private void InvalidateCost() { totalCost = null; foreach (var row in SelectedMaterials) row.SetCost(null); Raise(nameof(TotalPartCostLabel)); Raise(nameof(HasSelectedMaterials)); }
    /// <summary>Excludes selected materials and displays only the current Rust search matches.</summary>
    private void RefreshMaterialOptions()
    {
        FilteredMaterials.Clear(); foreach (var m in materials.Where(m => !SelectedMaterials.Any(u => u.Material.Id == m.Id) && (matchingMaterials is not null ? matchingMaterials.Contains(m.Id) : MaterialSearchText.Length == 0 && materialSearchMessage.Length == 0))) FilteredMaterials.Add(m);
        Raise(nameof(HasNoMaterialOptions));
        Raise(nameof(MaterialOptionsMessage)); Raise(nameof(HasMaterialOptionsMessage));
    }
    /// <summary>Applies category and archive filters to scoped Rust projections while keeping stable row IDs.</summary>
    private void RefreshVisible()
    {
        VisibleParts.Clear(); foreach (var p in records.Values)
        {
            var category = categories.GetValueOrDefault(p.CategoryId)?.Name ?? "—";
            if (SelectedCategory != AllCategories && category != SelectedCategory || SelectedStatus == ActiveStatus && p.Archived || SelectedStatus == ArchivedStatus && !p.Archived) continue;
            VisibleParts.Add(new PartListItem(p.Id,p.Name,category,currentCosts[p.Id].TotalCostYer,0,p.Archived));
        }
        Raise(nameof(HasNoVisibleParts)); Raise(nameof(VisibleCountLabel));
    }
}
