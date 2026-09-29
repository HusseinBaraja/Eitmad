using System.Collections.ObjectModel;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.RawMaterials;

/// <summary>
/// Owns list and unsaved editor presentation state; Rust owns durable definitions.
/// </summary>
public sealed class RawMaterialsViewModel : ObservableObject
{
    public const string AllCategories = "كل الفئات";
    public const string AllStatuses = "كل الحالات";
    public const string ActiveStatus = "نشطة";
    public const string ArchivedStatus = "مؤرشفة";

    private readonly List<RawMaterialListItem> materials;
    private RawMaterialListItem? editingMaterial;
    private string searchText = string.Empty;
    private string selectedCategory = AllCategories;
    private string selectedStatus = AllStatuses;
    private bool isEditorOpen;
    private bool isCreating;
    private string editorName = string.Empty;
    private string editorCategory = "ألواح خشبية";
    private string editorUnit = "لوح";
    private decimal editorCost;
    private string editorError = string.Empty;
    private string feedbackMessage = string.Empty;
    private RawMaterialReferenceOption? editingReference;
    private bool isCategoryReference = true;
    private bool isReferenceEditorOpen;
    private bool isReferenceManagerOpen;
    private bool returnToReferenceManager;
    private string referenceName = string.Empty;
    private string referenceShortName = string.Empty;
    private string referenceError = string.Empty;
    private string availabilityMessage = string.Empty;
    private UnitDimension referenceDimension = UnitDimension.Count;
    private long referenceNumerator = 1;
    private long referenceDenominator = 1;

    public bool DurableMode { get; private set; }
    public string AvailabilityMessage { get => availabilityMessage; private set => Set(ref availabilityMessage, value); }
    public string DataStatusLabel => DurableMode ? "بيانات المواد الخام" : "بيانات تجريبية غير محفوظة";
    public string EditorHelpLabel => DurableMode ? "راجع البيانات قبل حفظها." : "يمكنك مراجعة البيانات قبل حفظها في المعاينة المحلية.";
    public event EventHandler? SearchChanged;
    public RawMaterialListItem? EditingMaterial => editingMaterial;
    public RawMaterialReferenceOption? EditingReference => editingReference;

    public RawMaterialsViewModel()
    {
        materials =
        [
            new(Guid.Parse("90e8280f-e8ce-4b57-9af5-5bb263eec885"), "لوح MDF سماكة 18 مم", "ألواح خشبية", "لوح", 25_000m),
            new(Guid.Parse("f10241bb-f60b-464a-8f43-0df9d1322c9f"), "خشب زان مجفف", "أخشاب طبيعية", "متر", 8_000m),
            new(Guid.Parse("ea15bd52-40b7-4f9c-94d4-00585c52a6e7"), "قماش كتان بيج", "أقمشة ومفروشات", "متر", 3_500m),
            new(Guid.Parse("4a34cd4c-6d5c-438c-87ab-a9ded9bd9f73"), "خشب سويدي مقاس 2×4", "أخشاب طبيعية", "متر", 5_200m, isArchived: true),
        ];

        Categories =
        [
            new("ألواح خشبية"),
            new("أخشاب طبيعية"),
            new("أقمشة ومفروشات"),
        ];
        Units =
        [
            new("لوح", "لوح"),
            new("متر", "م"),
            new("كيلوجرام", "كجم"),
            new("قطعة", "قطعة"),
        ];
        ActiveCategories = new ObservableCollection<RawMaterialReferenceOption>(Categories);
        ActiveUnits = new ObservableCollection<RawMaterialReferenceOption>(Units);
        EditorCategories = new ObservableCollection<RawMaterialReferenceOption>(ActiveCategories);
        EditorUnits = new ObservableCollection<RawMaterialReferenceOption>(ActiveUnits);
        CategoryOptions = [AllCategories, .. Categories.Select(item => item.Name)];
        StatusOptions = [AllStatuses, ActiveStatus, ArchivedStatus];
        VisibleMaterials = [];
        RefreshVisibleMaterials();
    }

    public ObservableCollection<string> CategoryOptions { get; }

    public IReadOnlyList<string> StatusOptions { get; }

    public ObservableCollection<RawMaterialReferenceOption> Categories { get; }

    public ObservableCollection<RawMaterialReferenceOption> Units { get; }

    public ObservableCollection<RawMaterialReferenceOption> ActiveCategories { get; }

    public ObservableCollection<RawMaterialReferenceOption> ActiveUnits { get; }
    public ObservableCollection<RawMaterialReferenceOption> EditorCategories { get; }
    public ObservableCollection<RawMaterialReferenceOption> EditorUnits { get; }

    public IEnumerable<RawMaterialReferenceOption> ManagedReferences =>
        IsCategoryReference ? Categories : Units;

    public ObservableCollection<RawMaterialListItem> VisibleMaterials { get; }

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                RefreshVisibleMaterials();
                if (DurableMode) SearchChanged?.Invoke(this, EventArgs.Empty);
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
                RefreshVisibleMaterials();
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
                RefreshVisibleMaterials();
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
            }
        }
    }

    public string EditorTitle => IsCreating ? "إضافة مادة خام" : "تعديل مادة خام";

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

    public string EditorUnit
    {
        get => editorUnit;
        set => Set(ref editorUnit, value ?? string.Empty);
    }

    public decimal EditorCost
    {
        get => editorCost;
        set => Set(ref editorCost, value);
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

    public bool IsReferenceEditorOpen
    {
        get => isReferenceEditorOpen;
        private set => Set(ref isReferenceEditorOpen, value);
    }

    public bool IsReferenceManagerOpen
    {
        get => isReferenceManagerOpen;
        private set => Set(ref isReferenceManagerOpen, value);
    }

    public bool IsCategoryReference
    {
        get => isCategoryReference;
        private set
        {
            if (Set(ref isCategoryReference, value))
            {
                Raise(nameof(IsUnitReference));
                Raise(nameof(ManagedReferences));
                Raise(nameof(ReferenceEditorTitle));
                Raise(nameof(ReferenceManagerTitle));
            }
        }
    }

    public bool IsUnitReference => !IsCategoryReference;

    public string ReferenceEditorTitle => editingReference is null
        ? IsCategoryReference ? "إضافة تصنيف جديد" : "إضافة وحدة جديدة"
        : IsCategoryReference ? "تعديل التصنيف" : "تعديل الوحدة";

    public string ReferenceManagerTitle => IsCategoryReference ? "إدارة التصنيفات" : "إدارة الوحدات";

    public string ReferenceName
    {
        get => referenceName;
        set => Set(ref referenceName, value ?? string.Empty);
    }

    public string ReferenceShortName
    {
        get => referenceShortName;
        set => Set(ref referenceShortName, value ?? string.Empty);
    }

    public UnitDimension ReferenceDimension
    {
        get => referenceDimension;
        set => Set(ref referenceDimension, value);
    }

    public long ReferenceNumerator
    {
        get => referenceNumerator;
        set => Set(ref referenceNumerator, value);
    }

    public long ReferenceDenominator
    {
        get => referenceDenominator;
        set => Set(ref referenceDenominator, value);
    }

    public sealed record DimensionOption(UnitDimension Value, string Label);
    public IReadOnlyList<DimensionOption> UnitDimensions { get; } =
    [
        new(UnitDimension.Count, "عدد"), new(UnitDimension.Length, "طول"),
        new(UnitDimension.Area, "مساحة"), new(UnitDimension.Volume, "حجم"),
        new(UnitDimension.Mass, "كتلة"),
    ];

    public void EnableDurableMode()
    {
        DurableMode = true;
        materials.Clear();
        Categories.Clear(); Units.Clear(); ActiveCategories.Clear(); ActiveUnits.Clear();
        EditorCategories.Clear(); EditorUnits.Clear();
        while (CategoryOptions.Count > 1) CategoryOptions.RemoveAt(CategoryOptions.Count - 1);
        EditorCategory = string.Empty; EditorUnit = string.Empty;
        FeedbackMessage = string.Empty;
        AvailabilityMessage = string.Empty;
        Raise(nameof(DataStatusLabel)); Raise(nameof(EditorHelpLabel));
        RefreshVisibleMaterials();
    }

    public void ApplyDurableData(MaterialReferences references, IReadOnlyList<Material> records)
    {
        if (!DurableMode) return;
        var priorCategory = SelectedCategory;
        AvailabilityMessage = string.Empty;
        Categories.Clear(); Units.Clear(); ActiveCategories.Clear(); ActiveUnits.Clear();
        while (CategoryOptions.Count > 1) CategoryOptions.RemoveAt(CategoryOptions.Count - 1);
        foreach (var category in references.Categories)
        {
            var option = new RawMaterialReferenceOption(category.Name, id: category.Id, revision: category.Revision)
                { IsArchived = category.Archived };
            Categories.Add(option);
            if (!option.IsArchived) { ActiveCategories.Add(option); CategoryOptions.Add(option.Name); }
        }
        foreach (var unit in references.Units)
        {
            var option = new RawMaterialReferenceOption(unit.Name, unit.Symbol, unit.Id, unit.Revision,
                unit.Dimension, unit.Numerator, unit.Denominator) { IsArchived = unit.Archived };
            Units.Add(option);
            if (!option.IsArchived) ActiveUnits.Add(option);
        }
        RefreshEditorReferences();
        materials.Clear();
        foreach (var record in records)
        {
            var category = Categories.FirstOrDefault(item => item.Id == record.CategoryId);
            var unit = Units.FirstOrDefault(item => item.Id == record.UnitId);
            materials.Add(new RawMaterialListItem(record.Id, record.Name,
                category?.Name ?? "تصنيف غير متاح", unit?.Name ?? "وحدة غير متاحة",
                record.CurrentCostYer, record.Archived, record.CategoryId, record.UnitId, record.Revision));
        }
        if (!CategoryOptions.Contains(priorCategory)) SelectedCategory = AllCategories;
        RefreshVisibleMaterials();
    }

    public void DurableSaved(string message)
    {
        IsEditorOpen = false; IsReferenceEditorOpen = false; IsReferenceManagerOpen = false;
        EditorError = string.Empty; ReferenceError = string.Empty;
        FeedbackMessage = message;
    }

    public void DurableError(string message, bool reference = false)
    {
        if (reference) ReferenceError = message;
        else EditorError = message;
    }

    public void DurableUnavailable(string message) => AvailabilityMessage = message;

    public string ReferenceError
    {
        get => referenceError;
        private set
        {
            if (Set(ref referenceError, value))
            {
                Raise(nameof(HasReferenceError));
            }
        }
    }

    public bool HasReferenceError => !string.IsNullOrEmpty(ReferenceError);

    public bool HasNoVisibleMaterials => VisibleMaterials.Count == 0;

    public string VisibleCountLabel => $"{VisibleMaterials.Count} من {materials.Count} مواد";

    public void BeginCreate()
    {
        editingMaterial = null;
        RefreshEditorReferences();
        IsCreating = true;
        EditorName = string.Empty;
        EditorCategory = DurableMode ? ActiveCategories.FirstOrDefault()?.Name ?? string.Empty : "ألواح خشبية";
        EditorUnit = DurableMode ? ActiveUnits.FirstOrDefault()?.Name ?? string.Empty : "لوح";
        EditorCost = 0m;
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    public void BeginEdit(RawMaterialListItem material)
    {
        ArgumentNullException.ThrowIfNull(material);
        editingMaterial = material;
        RefreshEditorReferences();
        IsCreating = false;
        EditorName = material.Name;
        EditorCategory = material.Category;
        EditorUnit = material.Unit;
        EditorCost = material.CurrentCost;
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    public void CancelEditor()
    {
        IsEditorOpen = false;
        EditorError = string.Empty;
    }

    private void RefreshEditorReferences()
    {
        EditorCategories.Clear(); EditorUnits.Clear();
        foreach (var item in ActiveCategories) EditorCategories.Add(item);
        foreach (var item in ActiveUnits) EditorUnits.Add(item);
        if (editingMaterial?.CategoryId is { } categoryId
            && Categories.FirstOrDefault(item => item.Id == categoryId) is { IsArchived: true } archivedCategory)
            EditorCategories.Add(archivedCategory);
        if (editingMaterial?.UnitId is { } unitId
            && Units.FirstOrDefault(item => item.Id == unitId) is { IsArchived: true } archivedUnit)
            EditorUnits.Add(archivedUnit);
    }

    public bool SaveEditor()
    {
        var normalizedName = EditorName.Trim();
        if (normalizedName.Length == 0)
        {
            EditorError = "أدخل اسم المادة الخام.";
            return false;
        }

        if (EditorCost < 0m)
        {
            EditorError = "يجب ألا تكون التكلفة سالبة.";
            return false;
        }

        if (!ActiveCategories.Any(item => item.Name == EditorCategory))
        {
            EditorError = "اختر تصنيفاً نشطاً للمادة الخام.";
            return false;
        }

        if (!ActiveUnits.Any(item => item.Name == EditorUnit))
        {
            EditorError = "اختر وحدة نشطة للمادة الخام.";
            return false;
        }

        if (editingMaterial is null)
        {
            materials.Add(new RawMaterialListItem(
                Guid.NewGuid(),
                normalizedName,
                EditorCategory,
                EditorUnit,
                EditorCost));
            FeedbackMessage = "أضيفت المادة إلى المعاينة المحلية.";
        }
        else
        {
            editingMaterial.Name = normalizedName;
            editingMaterial.Category = EditorCategory;
            editingMaterial.Unit = EditorUnit;
            editingMaterial.CurrentCost = EditorCost;
            FeedbackMessage = "حُدثت المادة في المعاينة المحلية.";
        }

        IsEditorOpen = false;
        EditorError = string.Empty;
        RefreshVisibleMaterials();
        return true;
    }

    public RawMaterialListItem Duplicate(RawMaterialListItem material)
    {
        ArgumentNullException.ThrowIfNull(material);
        if (DurableMode)
        {
            BeginCreate();
            EditorName = $"{material.Name} — نسخة";
            EditorCategory = material.Category;
            EditorUnit = material.Unit;
            EditorCost = material.CurrentCost;
            return material;
        }
        var duplicate = new RawMaterialListItem(
            Guid.NewGuid(),
            $"{material.Name} — نسخة",
            material.Category,
            material.Unit,
            material.CurrentCost);
        materials.Add(duplicate);
        FeedbackMessage = "أُنشئت نسخة محلية ويمكن تعديلها الآن.";
        RefreshVisibleMaterials();
        BeginEdit(duplicate);
        return duplicate;
    }

    public void Archive(RawMaterialListItem material)
    {
        ArgumentNullException.ThrowIfNull(material);
        if (material.IsArchived)
        {
            return;
        }

        material.IsArchived = true;
        FeedbackMessage = "أُرشفت المادة في المعاينة المحلية.";
        RefreshVisibleMaterials();
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

    public void BeginAddCategory() => BeginAddReference(isCategory: true);

    public void BeginAddUnit() => BeginAddReference(isCategory: false);

    public void BeginManageCategories() => BeginManageReferences(isCategory: true);

    public void BeginManageUnits() => BeginManageReferences(isCategory: false);

    public void BeginEditReference(RawMaterialReferenceOption reference)
    {
        ArgumentNullException.ThrowIfNull(reference);
        editingReference = reference;
        returnToReferenceManager = IsReferenceManagerOpen;
        ReferenceName = reference.Name;
        ReferenceShortName = reference.ShortName;
        ReferenceDimension = reference.Dimension;
        ReferenceNumerator = reference.Numerator;
        ReferenceDenominator = reference.Denominator;
        ReferenceError = string.Empty;
        IsReferenceManagerOpen = false;
        IsReferenceEditorOpen = true;
        Raise(nameof(ReferenceEditorTitle));
    }

    public bool SaveReferenceEditor()
    {
        var normalizedName = ReferenceName.Trim();
        var normalizedShortName = ReferenceShortName.Trim();
        if (normalizedName.Length == 0)
        {
            ReferenceError = IsCategoryReference ? "أدخل اسم التصنيف." : "أدخل اسم الوحدة.";
            return false;
        }

        if (IsUnitReference && normalizedShortName.Length == 0)
        {
            ReferenceError = "أدخل الاسم المختصر للوحدة.";
            return false;
        }

        var references = IsCategoryReference ? Categories : Units;
        if (references.Any(item => item != editingReference
            && string.Equals(item.Name, normalizedName, StringComparison.CurrentCultureIgnoreCase)))
        {
            ReferenceError = IsCategoryReference ? "اسم التصنيف مستخدم بالفعل." : "اسم الوحدة مستخدم بالفعل.";
            return false;
        }

        if (editingReference is null)
        {
            var added = new RawMaterialReferenceOption(normalizedName, IsUnitReference ? normalizedShortName : string.Empty);
            references.Add(added);
            if (IsCategoryReference)
            {
                ActiveCategories.Add(added);
                EditorCategories.Add(added);
                CategoryOptions.Add(added.Name);
                EditorCategory = added.Name;
            }
            else
            {
                ActiveUnits.Add(added);
                EditorUnits.Add(added);
                EditorUnit = added.Name;
            }
        }
        else
        {
            RenameReference(editingReference, normalizedName, IsUnitReference ? normalizedShortName : string.Empty);
        }

        ReferenceError = string.Empty;
        IsReferenceEditorOpen = false;
        if (returnToReferenceManager)
        {
            IsReferenceManagerOpen = true;
        }

        return true;
    }

    public void CancelReferenceEditor()
    {
        IsReferenceEditorOpen = false;
        ReferenceError = string.Empty;
        if (returnToReferenceManager)
        {
            IsReferenceManagerOpen = true;
        }
    }

    public void CloseReferenceManager() => IsReferenceManagerOpen = false;

    public void ArchiveReference(RawMaterialReferenceOption reference)
    {
        ArgumentNullException.ThrowIfNull(reference);
        if (reference.IsArchived)
        {
            return;
        }

        var activeReferences = IsCategoryReference ? ActiveCategories : ActiveUnits;
        if (activeReferences.Count == 1 && activeReferences.Contains(reference))
        {
            ReferenceError = IsCategoryReference
                ? "يجب إبقاء تصنيف نشط واحد على الأقل."
                : "يجب إبقاء وحدة نشطة واحدة على الأقل.";
            return;
        }

        reference.IsArchived = true;
        if (IsCategoryReference)
        {
            ActiveCategories.Remove(reference);
            EditorCategories.Remove(reference);
            if (EditorCategory == reference.Name)
            {
                EditorCategory = ActiveCategories.FirstOrDefault()?.Name ?? string.Empty;
            }
        }
        else
        {
            ActiveUnits.Remove(reference);
            EditorUnits.Remove(reference);
            if (EditorUnit == reference.Name)
            {
                EditorUnit = ActiveUnits.FirstOrDefault()?.Name ?? string.Empty;
            }
        }
    }

    private void BeginAddReference(bool isCategory)
    {
        IsCategoryReference = isCategory;
        editingReference = null;
        returnToReferenceManager = false;
        ReferenceName = string.Empty;
        ReferenceShortName = string.Empty;
        ReferenceDimension = UnitDimension.Count;
        ReferenceNumerator = 1;
        ReferenceDenominator = 1;
        ReferenceError = string.Empty;
        IsReferenceManagerOpen = false;
        IsReferenceEditorOpen = true;
        Raise(nameof(ReferenceEditorTitle));
    }

    private void BeginManageReferences(bool isCategory)
    {
        IsCategoryReference = isCategory;
        IsReferenceEditorOpen = false;
        IsReferenceManagerOpen = true;
    }

    private void RenameReference(RawMaterialReferenceOption reference, string name, string shortName)
    {
        var previousName = reference.Name;
        reference.Name = name;
        reference.ShortName = shortName;

        if (IsCategoryReference)
        {
            var filterIndex = CategoryOptions.IndexOf(previousName);
            if (filterIndex >= 0)
            {
                CategoryOptions[filterIndex] = name;
            }

            foreach (var material in materials.Where(item => item.Category == previousName))
            {
                material.Category = name;
            }

            if (EditorCategory == previousName)
            {
                EditorCategory = name;
            }

            if (SelectedCategory == previousName)
            {
                SelectedCategory = name;
            }
        }
        else
        {
            foreach (var material in materials.Where(item => item.Unit == previousName))
            {
                material.Unit = name;
            }

            if (EditorUnit == previousName)
            {
                EditorUnit = name;
            }
        }

        RefreshVisibleMaterials();
    }

    private void RefreshVisibleMaterials()
    {
        var normalizedSearch = PreviewText.NormalizeSearch(SearchText.Trim());
        var matches = materials.Where(material =>
            MatchesSearch(material, normalizedSearch)
            && (SelectedCategory == AllCategories || material.Category == SelectedCategory)
            && (SelectedStatus == AllStatuses
                || (SelectedStatus == ActiveStatus && !material.IsArchived)
                || (SelectedStatus == ArchivedStatus && material.IsArchived)));

        VisibleMaterials.Clear();
        foreach (var material in matches)
        {
            VisibleMaterials.Add(material);
        }

        Raise(nameof(HasNoVisibleMaterials));
        Raise(nameof(VisibleCountLabel));
    }

    private static bool MatchesSearch(RawMaterialListItem material, string search) =>
        search.Length == 0
        || PreviewText.NormalizeSearch(material.Name).Contains(search, StringComparison.CurrentCultureIgnoreCase)
        || PreviewText.NormalizeSearch(material.Category).Contains(search, StringComparison.CurrentCultureIgnoreCase)
        || PreviewText.NormalizeSearch(material.Unit).Contains(search, StringComparison.CurrentCultureIgnoreCase);
}
