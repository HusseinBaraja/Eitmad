using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace Eitmad.WindowsShell.Features.RawMaterials;

/// <summary>
/// Owns ephemeral list-page state for the raw-materials preview.
/// Durable validation, authorization, and storage remain unavailable until a Rust vertical exists.
/// </summary>
public sealed class RawMaterialsViewModel : INotifyPropertyChanged
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

    public RawMaterialsViewModel()
    {
        materials =
        [
            new(Guid.Parse("90e8280f-e8ce-4b57-9af5-5bb263eec885"), "MDF 18mm", "ألواح خشبية", "لوح", 25_000m),
            new(Guid.Parse("f10241bb-f60b-464a-8f43-0df9d1322c9f"), "Beech Wood", "أخشاب", "متر", 8_000m),
            new(Guid.Parse("ea15bd52-40b7-4f9c-94d4-00585c52a6e7"), "قماش كتان Linen", "أقمشة", "متر", 3_500m),
            new(Guid.Parse("4a34cd4c-6d5c-438c-87ab-a9ded9bd9f73"), "خشب سويدي 2×4", "أخشاب", "متر", 5_200m, isArchived: true),
        ];

        CategoryOptions = [AllCategories, "ألواح خشبية", "أخشاب", "أقمشة"];
        StatusOptions = [AllStatuses, ActiveStatus, ArchivedStatus];
        UnitOptions = ["لوح", "متر", "كيلوجرام", "قطعة"];
        VisibleMaterials = [];
        RefreshVisibleMaterials();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public IReadOnlyList<string> CategoryOptions { get; }

    public IReadOnlyList<string> StatusOptions { get; }

    public IReadOnlyList<string> UnitOptions { get; }

    public ObservableCollection<RawMaterialListItem> VisibleMaterials { get; }

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                RefreshVisibleMaterials();
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

    public bool HasNoVisibleMaterials => VisibleMaterials.Count == 0;

    public string VisibleCountLabel => $"{VisibleMaterials.Count} من {materials.Count} مواد";

    public void BeginCreate()
    {
        editingMaterial = null;
        IsCreating = true;
        EditorName = string.Empty;
        EditorCategory = "ألواح خشبية";
        EditorUnit = "لوح";
        EditorCost = 0m;
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    public void BeginEdit(RawMaterialListItem material)
    {
        ArgumentNullException.ThrowIfNull(material);
        editingMaterial = material;
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

    private void RefreshVisibleMaterials()
    {
        var normalizedSearch = SearchText.Trim();
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
        || material.Name.Contains(search, StringComparison.CurrentCultureIgnoreCase)
        || material.Category.Contains(search, StringComparison.CurrentCultureIgnoreCase)
        || material.Unit.Contains(search, StringComparison.CurrentCultureIgnoreCase);

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
