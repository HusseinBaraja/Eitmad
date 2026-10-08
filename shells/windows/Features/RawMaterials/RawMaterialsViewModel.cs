using System.Collections.ObjectModel;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.RawMaterials;

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
    private string editorCategory = string.Empty;
    private string editorUnit = string.Empty;
    private Guid? editorCategoryId;
    private Guid? editorUnitId;
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

    public string AvailabilityMessage { get => availabilityMessage; private set => Set(ref availabilityMessage, value); }
    public string DataStatusLabel => "بيانات المواد الخام";
    public string EditorHelpLabel => "راجع البيانات قبل حفظها.";
    public event EventHandler? SearchChanged;
    public RawMaterialListItem? EditingMaterial => editingMaterial;
    public RawMaterialReferenceOption? EditingReference => editingReference;

    public RawMaterialsViewModel()
    {
        materials = [];
        Categories = []; Units = []; ActiveCategories = []; ActiveUnits = [];
        EditorCategories = []; EditorUnits = [];
        CategoryOptions = [AllCategories];
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
                SearchChanged?.Invoke(this, EventArgs.Empty);
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
        set
        {
            Set(ref editorCategory, value ?? string.Empty);
            Set(ref editorCategoryId, EditorCategories.FirstOrDefault(item => item.Name == editorCategory)?.Id,
                nameof(EditorCategoryId));
        }
    }

    public string EditorUnit
    {
        get => editorUnit;
        set
        {
            Set(ref editorUnit, value ?? string.Empty);
            Set(ref editorUnitId, EditorUnits.FirstOrDefault(item => item.Name == editorUnit)?.Id,
                nameof(EditorUnitId));
        }
    }

    public Guid? EditorCategoryId
    {
        get => editorCategoryId;
        set
        {
            Set(ref editorCategoryId, value);
            Set(ref editorCategory, EditorCategories.FirstOrDefault(item => item.Id == value)?.Name ?? string.Empty,
                nameof(EditorCategory));
        }
    }

    public Guid? EditorUnitId
    {
        get => editorUnitId;
        set
        {
            Set(ref editorUnitId, value);
            Set(ref editorUnit, EditorUnits.FirstOrDefault(item => item.Id == value)?.Name ?? string.Empty,
                nameof(EditorUnit));
        }
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

    public void ApplyDurableData(MaterialReferences references, IReadOnlyList<Material> records)
    {
        var priorCategory = SelectedCategory;
        var priorEditorCategoryId = EditorCategoryId;
        var priorEditorUnitId = EditorUnitId;
        AvailabilityMessage = string.Empty;
        Categories.Clear(); Units.Clear(); ActiveCategories.Clear(); ActiveUnits.Clear();
        while (CategoryOptions.Count > 1) CategoryOptions.RemoveAt(CategoryOptions.Count - 1);
        foreach (var category in references.Categories)
        {
            var option = new RawMaterialReferenceOption(category.Name, category.Id, category.Revision, IsArchived: category.Archived);
            Categories.Add(option);
            if (!option.IsArchived) { ActiveCategories.Add(option); CategoryOptions.Add(option.Name); }
        }
        foreach (var unit in references.Units)
        {
            var option = new RawMaterialReferenceOption(unit.Name, unit.Id, unit.Revision, unit.Symbol,
                unit.Dimension, unit.Numerator, unit.Denominator, unit.Archived);
            Units.Add(option);
            if (!option.IsArchived) ActiveUnits.Add(option);
        }
        RefreshEditorReferences();
        if (IsEditorOpen)
        {
            EditorCategoryId = priorEditorCategoryId;
            EditorUnitId = priorEditorUnitId;
            if (!EditorCategories.Any(item => item.Id == priorEditorCategoryId)
                || !EditorUnits.Any(item => item.Id == priorEditorUnitId))
                EditorError = "تغير التصنيف أو الوحدة. اختر مرجعاً متاحاً قبل الحفظ.";
        }
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

    public void Saved(string message)
    {
        IsEditorOpen = false; IsReferenceEditorOpen = false; IsReferenceManagerOpen = false;
        EditorError = string.Empty; ReferenceError = string.Empty;
        FeedbackMessage = message;
    }

    public void Fail(string message, bool reference = false)
    {
        if (reference) ReferenceError = message;
        else EditorError = message;
    }

    public void Unavailable(string message) => AvailabilityMessage = message;

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
        EditorCategory = ActiveCategories.FirstOrDefault()?.Name ?? string.Empty;
        EditorUnit = ActiveUnits.FirstOrDefault()?.Name ?? string.Empty;
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
        EditorCategoryId = material.CategoryId;
        EditorUnitId = material.UnitId;
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

    public void Duplicate(RawMaterialListItem material)
    {
        BeginCreate();
        EditorName = $"{material.Name} — نسخة";
        EditorCategoryId = material.CategoryId;
        EditorUnitId = material.UnitId;
        EditorCost = material.CurrentCost;
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

    private void RefreshVisibleMaterials()
    {
        var matches = materials.Where(material =>
            (SelectedCategory == AllCategories || material.Category == SelectedCategory)
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

}
