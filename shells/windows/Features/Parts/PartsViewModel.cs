using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text;

namespace Eitmad.WindowsShell.Features.Parts;

/// <summary>Owns transient list, filter, and guided-editor state for the Parts preview.</summary>
public sealed class PartsViewModel : INotifyPropertyChanged
{
    public const string AllCategories = "كل الفئات";
    public const string AllStatuses = "كل الحالات";
    public const string ActiveStatus = "نشط";
    public const string ArchivedStatus = "مؤرشف";

    private readonly List<PartListItem> parts;
    private readonly Dictionary<Guid, string> descriptions = [];
    private readonly Dictionary<Guid, List<PartMaterialUsage>> materialUsages = [];
    private readonly List<PartMaterialOption> availableMaterials;
    private PartListItem? editingPart;
    private string searchText = string.Empty;
    private string selectedCategory = AllCategories;
    private string selectedStatus = AllStatuses;
    private bool isEditorOpen;
    private bool isCreating;
    private int currentStep = 1;
    private string editorName = string.Empty;
    private string editorCategory = "خزانة ملابس";
    private string editorDescription = string.Empty;
    private string editorError = string.Empty;
    private string feedbackMessage = string.Empty;
    private bool isMaterialPickerOpen;
    private string materialSearchText = string.Empty;

    public PartsViewModel()
    {
        parts =
        [
            new(Guid.Parse("660f159e-89bb-4970-b2bd-9d69cae3e84b"), "Wardrobe Side Panel", "خزانة ملابس", 9_450m, 3),
            new(Guid.Parse("51ec39a0-ef63-4bc6-92e3-3de5316bf8e7"), "رف داخلي قابل للتعديل", "رفوف", 3_500m, 5),
            new(Guid.Parse("1c37959e-3937-4666-a549-e9c77a5af4dd"), "واجهة باب بإطار", "أبواب", 6_200m, 2),
            new(Guid.Parse("6970881f-96f8-413f-8dca-bd6f4e122812"), "باب جرار مزخرف", "أبواب", 7_800m, 1, isArchived: true),
        ];

        availableMaterials =
        [
            new(Guid.Parse("c82cf130-539f-4fc3-9d07-dbc5c1da7e4e"), "MDF 18mm", "m²", 7_250m),
            new(Guid.Parse("eb969e93-ad20-41f5-8c56-bbb83bba5d9c"), "Edge Band", "m", 250m),
            new(Guid.Parse("09b5cc44-8c98-4697-bcd0-39e90f7611ed"), "خشب زان مجفف", "m", 8_000m),
            new(Guid.Parse("84c316a4-f65d-4d4e-9ec3-2adfcaedfc6f"), "قماش كتان بيج", "m", 3_500m),
        ];

        CategoryOptions = [AllCategories, "خزانة ملابس", "رفوف", "أبواب", "أدراج"];
        EditorCategoryOptions = ["خزانة ملابس", "رفوف", "أبواب", "أدراج"];
        StatusOptions = [AllStatuses, ActiveStatus, ArchivedStatus];
        VisibleParts = [];
        SelectedMaterials = [];
        FilteredMaterials = [];
        RefreshVisibleParts();
        RefreshMaterialOptions();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public IReadOnlyList<string> CategoryOptions { get; }

    public IReadOnlyList<string> EditorCategoryOptions { get; }

    public IReadOnlyList<string> StatusOptions { get; }

    public ObservableCollection<PartListItem> VisibleParts { get; }

    public ObservableCollection<PartMaterialUsage> SelectedMaterials { get; }

    public ObservableCollection<PartMaterialOption> FilteredMaterials { get; }

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                RefreshVisibleParts();
            }
        }
    }

    public string SelectedCategory
    {
        get => selectedCategory;
        set
        {
            if (Set(ref selectedCategory, value ?? AllCategories))
            {
                RefreshVisibleParts();
            }
        }
    }

    public string SelectedStatus
    {
        get => selectedStatus;
        set
        {
            if (Set(ref selectedStatus, value ?? AllStatuses))
            {
                RefreshVisibleParts();
            }
        }
    }

    public bool IsEditorOpen
    {
        get => isEditorOpen;
        private set => Set(ref isEditorOpen, value);
    }

    public bool IsCreating
    {
        get => isCreating;
        private set
        {
            if (Set(ref isCreating, value))
            {
                Raise(nameof(EditorTitle));
                Raise(nameof(SaveButtonLabel));
            }
        }
    }

    public string EditorTitle => IsCreating ? "إنشاء جزء جديد" : "تعديل الجزء";

    public string SaveButtonLabel => IsCreating ? "حفظ الجزء" : "حفظ التعديلات";

    public int CurrentStep
    {
        get => currentStep;
        private set
        {
            if (Set(ref currentStep, value))
            {
                Raise(nameof(IsStepOne));
                Raise(nameof(IsStepTwo));
                Raise(nameof(IsStepThree));
            }
        }
    }

    public bool IsStepOne => CurrentStep == 1;

    public bool IsStepTwo => CurrentStep == 2;

    public bool IsStepThree => CurrentStep == 3;

    public string EditorName
    {
        get => editorName;
        set => Set(ref editorName, value ?? string.Empty);
    }

    public string EditorCategory
    {
        get => editorCategory;
        set => Set(ref editorCategory, value ?? string.Empty);
    }

    public string EditorDescription
    {
        get => editorDescription;
        set => Set(ref editorDescription, value ?? string.Empty);
    }

    public string EditorError
    {
        get => editorError;
        private set
        {
            if (Set(ref editorError, value))
            {
                Raise(nameof(HasEditorError));
            }
        }
    }

    public bool HasEditorError => !string.IsNullOrEmpty(EditorError);

    public string FeedbackMessage
    {
        get => feedbackMessage;
        private set
        {
            if (Set(ref feedbackMessage, value))
            {
                Raise(nameof(HasFeedback));
            }
        }
    }

    public bool HasFeedback => !string.IsNullOrEmpty(FeedbackMessage);

    public bool IsMaterialPickerOpen
    {
        get => isMaterialPickerOpen;
        private set => Set(ref isMaterialPickerOpen, value);
    }

    public string MaterialSearchText
    {
        get => materialSearchText;
        set
        {
            if (Set(ref materialSearchText, value ?? string.Empty))
            {
                RefreshMaterialOptions();
            }
        }
    }

    public bool HasSelectedMaterials => SelectedMaterials.Count > 0;

    public bool HasNoMaterialOptions => FilteredMaterials.Count == 0;

    public decimal TotalPartCost => TryCalculateTotalPartCost(out var totalCost) ? totalCost : 0m;

    public string TotalPartCostLabel => TryCalculateTotalPartCost(out var totalCost)
        ? totalCost.ToString("N0", CultureInfo.InvariantCulture)
        : "—";

    public bool HasNoVisibleParts => VisibleParts.Count == 0;

    public string VisibleCountLabel => $"{VisibleParts.Count} من {parts.Count} أجزاء";

    public void BeginCreate()
    {
        editingPart = null;
        IsCreating = true;
        EditorName = string.Empty;
        EditorCategory = "خزانة ملابس";
        EditorDescription = string.Empty;
        ReplaceSelectedMaterials([]);
        EditorError = string.Empty;
        CurrentStep = 1;
        IsMaterialPickerOpen = false;
        IsEditorOpen = true;
    }

    public void BeginEdit(PartListItem part)
    {
        ArgumentNullException.ThrowIfNull(part);
        editingPart = part;
        IsCreating = false;
        EditorName = part.Name;
        EditorCategory = part.Category;
        EditorDescription = descriptions.GetValueOrDefault(part.Id, string.Empty);
        ReplaceSelectedMaterials(materialUsages.TryGetValue(part.Id, out var usages)
            ? usages.Select(item => item.Copy())
            : []);
        EditorError = string.Empty;
        CurrentStep = 1;
        IsMaterialPickerOpen = false;
        IsEditorOpen = true;
    }

    public void CancelEditor()
    {
        IsEditorOpen = false;
        IsMaterialPickerOpen = false;
        EditorError = string.Empty;
    }

    public bool MoveToMaterials()
    {
        if (EditorName.Trim().Length == 0)
        {
            EditorError = "أدخل اسم الجزء.";
            return false;
        }

        EditorError = string.Empty;
        CurrentStep = 2;
        return true;
    }

    public bool MoveToReview()
    {
        if (SelectedMaterials.Any(item => item.Quantity <= 0m))
        {
            EditorError = "أدخل كمية أكبر من صفر لكل مادة خام.";
            return false;
        }

        if (!TryCalculateTotalPartCost(out _))
        {
            EditorError = "الكمية كبيرة جداً لحساب تكلفة الجزء.";
            return false;
        }

        if (IsCreating && SelectedMaterials.Count == 0)
        {
            EditorError = "أضف مادة خام واحدة على الأقل للمتابعة.";
            return false;
        }

        EditorError = string.Empty;
        CurrentStep = 3;
        return true;
    }

    public void MoveToPreviousStep()
    {
        if (CurrentStep > 1)
        {
            CurrentStep--;
            EditorError = string.Empty;
        }
    }

    public void OpenMaterialPicker()
    {
        MaterialSearchText = string.Empty;
        RefreshMaterialOptions();
        IsMaterialPickerOpen = true;
    }

    public void CloseMaterialPicker() => IsMaterialPickerOpen = false;

    public void AddMaterial(PartMaterialOption material)
    {
        ArgumentNullException.ThrowIfNull(material);
        if (SelectedMaterials.Any(item => item.Material.Id == material.Id))
        {
            return;
        }

        AddSelectedMaterial(new PartMaterialUsage(material));
        RefreshMaterialOptions();
        IsMaterialPickerOpen = false;
        EditorError = string.Empty;
    }

    public void RemoveMaterial(PartMaterialUsage usage)
    {
        ArgumentNullException.ThrowIfNull(usage);
        usage.PropertyChanged -= SelectedMaterialChanged;
        SelectedMaterials.Remove(usage);
        RefreshMaterialState();
        RefreshMaterialOptions();
    }

    public bool SaveEditor()
    {
        if (CurrentStep != 3 || EditorName.Trim().Length == 0)
        {
            return false;
        }

        var normalizedName = EditorName.Trim();
        var cost = SelectedMaterials.Count == 0 && editingPart is not null
            ? editingPart.Cost
            : TotalPartCost;
        var target = editingPart;
        if (target is null)
        {
            target = new PartListItem(Guid.NewGuid(), normalizedName, EditorCategory, cost, 0);
            parts.Add(target);
            FeedbackMessage = "أضيف الجزء إلى المعاينة المحلية.";
        }
        else
        {
            target.Name = normalizedName;
            target.Category = EditorCategory;
            target.Cost = cost;
            FeedbackMessage = "حُدث الجزء في المعاينة المحلية.";
        }

        descriptions[target.Id] = EditorDescription.Trim();
        materialUsages[target.Id] = SelectedMaterials.Select(item => item.Copy()).ToList();
        IsEditorOpen = false;
        IsMaterialPickerOpen = false;
        EditorError = string.Empty;
        RefreshVisibleParts();
        return true;
    }

    public PartListItem Duplicate(PartListItem part)
    {
        ArgumentNullException.ThrowIfNull(part);
        var duplicate = new PartListItem(Guid.NewGuid(), $"{part.Name} — نسخة", part.Category, part.Cost, part.UsedInCount);
        parts.Add(duplicate);
        if (descriptions.TryGetValue(part.Id, out var description))
        {
            descriptions[duplicate.Id] = description;
        }

        if (materialUsages.TryGetValue(part.Id, out var usages))
        {
            materialUsages[duplicate.Id] = usages.Select(item => item.Copy()).ToList();
        }

        FeedbackMessage = "أُنشئت نسخة محلية ويمكن تعديلها الآن.";
        RefreshVisibleParts();
        BeginEdit(duplicate);
        return duplicate;
    }

    public void Archive(PartListItem part)
    {
        ArgumentNullException.ThrowIfNull(part);
        if (part.IsArchived)
        {
            return;
        }

        part.IsArchived = true;
        FeedbackMessage = "أُرشف الجزء في المعاينة المحلية.";
        RefreshVisibleParts();
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

    private void ReplaceSelectedMaterials(IEnumerable<PartMaterialUsage> usages)
    {
        foreach (var existing in SelectedMaterials)
        {
            existing.PropertyChanged -= SelectedMaterialChanged;
        }

        SelectedMaterials.Clear();
        foreach (var usage in usages)
        {
            AddSelectedMaterial(usage);
        }

        RefreshMaterialState();
        RefreshMaterialOptions();
    }

    private void AddSelectedMaterial(PartMaterialUsage usage)
    {
        usage.PropertyChanged += SelectedMaterialChanged;
        SelectedMaterials.Add(usage);
        RefreshMaterialState();
    }

    private void SelectedMaterialChanged(object? sender, PropertyChangedEventArgs eventArgs)
    {
        if (eventArgs.PropertyName is nameof(PartMaterialUsage.Quantity) or nameof(PartMaterialUsage.TotalCost))
        {
            Raise(nameof(TotalPartCost));
            Raise(nameof(TotalPartCostLabel));
        }
    }

    private void RefreshMaterialState()
    {
        Raise(nameof(HasSelectedMaterials));
        Raise(nameof(TotalPartCost));
        Raise(nameof(TotalPartCostLabel));
    }

    private bool TryCalculateTotalPartCost(out decimal totalCost)
    {
        totalCost = 0m;
        try
        {
            foreach (var usage in SelectedMaterials)
            {
                if (!usage.TryCalculateTotalCost(out var rowCost))
                {
                    totalCost = 0m;
                    return false;
                }

                totalCost = checked(totalCost + rowCost);
            }

            return true;
        }
        catch (OverflowException)
        {
            totalCost = 0m;
            return false;
        }
    }

    private void RefreshMaterialOptions()
    {
        var normalizedSearch = NormalizeSearchText(MaterialSearchText.Trim());
        var selectedIds = SelectedMaterials.Select(item => item.Material.Id).ToHashSet();
        FilteredMaterials.Clear();
        foreach (var material in availableMaterials.Where(item =>
                     !selectedIds.Contains(item.Id)
                     && (normalizedSearch.Length == 0
                         || NormalizeSearchText(item.Name).Contains(normalizedSearch, StringComparison.CurrentCultureIgnoreCase))))
        {
            FilteredMaterials.Add(material);
        }

        Raise(nameof(HasNoMaterialOptions));
    }

    private void RefreshVisibleParts()
    {
        var normalizedSearch = NormalizeSearchText(SearchText.Trim());
        var matches = parts.Where(part =>
            MatchesSearch(part, normalizedSearch)
            && (SelectedCategory == AllCategories || part.Category == SelectedCategory)
            && (SelectedStatus == AllStatuses
                || (SelectedStatus == ActiveStatus && !part.IsArchived)
                || (SelectedStatus == ArchivedStatus && part.IsArchived)));

        VisibleParts.Clear();
        foreach (var part in matches)
        {
            VisibleParts.Add(part);
        }

        Raise(nameof(HasNoVisibleParts));
        Raise(nameof(VisibleCountLabel));
    }

    private static bool MatchesSearch(PartListItem part, string search) =>
        search.Length == 0
        || NormalizeSearchText(part.Name).Contains(search, StringComparison.CurrentCultureIgnoreCase)
        || NormalizeSearchText(part.Category).Contains(search, StringComparison.CurrentCultureIgnoreCase);

    private static string NormalizeSearchText(string value)
    {
        var decomposed = value.Normalize(NormalizationForm.FormD);
        var normalized = new StringBuilder(decomposed.Length);

        foreach (var character in decomposed)
        {
            var category = CharUnicodeInfo.GetUnicodeCategory(character);
            if (character == '\u0640'
                || category is UnicodeCategory.NonSpacingMark
                    or UnicodeCategory.SpacingCombiningMark
                    or UnicodeCategory.EnclosingMark)
            {
                continue;
            }

            normalized.Append(character switch
            {
                '\u0622' or '\u0623' or '\u0625' or '\u0671' => '\u0627',
                '\u0649' => '\u064A',
                '\u0629' => '\u0647',
                _ => character,
            });
        }

        return normalized.ToString();
    }

    private bool Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
        {
            return false;
        }

        field = value;
        Raise(propertyName);
        return true;
    }

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}
