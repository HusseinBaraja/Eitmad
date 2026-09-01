using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text;

namespace Eitmad.WindowsShell.Features.Parts;

/// <summary>Owns transient list, filter, and editor state for the parts preview.</summary>
public sealed class PartsViewModel : INotifyPropertyChanged
{
    public const string AllCategories = "كل الفئات";
    public const string AllStatuses = "كل الحالات";
    public const string ActiveStatus = "نشط";
    public const string ArchivedStatus = "مؤرشف";

    private readonly List<PartListItem> parts;
    private PartListItem? editingPart;
    private string searchText = string.Empty;
    private string selectedCategory = AllCategories;
    private string selectedStatus = AllStatuses;
    private bool isEditorOpen;
    private bool isCreating;
    private string editorName = string.Empty;
    private string editorCategory = "خزانة ملابس";
    private decimal editorCost;
    private int editorUsedInCount;
    private string editorError = string.Empty;
    private string feedbackMessage = string.Empty;

    public PartsViewModel()
    {
        parts =
        [
            new(Guid.Parse("660f159e-89bb-4970-b2bd-9d69cae3e84b"), "Wardrobe Side Panel", "خزانة ملابس", 9_450m, 3),
            new(Guid.Parse("51ec39a0-ef63-4bc6-92e3-3de5316bf8e7"), "رف داخلي قابل للتعديل", "رفوف", 3_500m, 5),
            new(Guid.Parse("1c37959e-3937-4666-a549-e9c77a5af4dd"), "واجهة باب بإطار", "أبواب", 6_200m, 2),
            new(Guid.Parse("6970881f-96f8-413f-8dca-bd6f4e122812"), "باب جرار مزخرف", "أبواب", 7_800m, 1, isArchived: true),
        ];

        CategoryOptions = [AllCategories, "خزانة ملابس", "رفوف", "أبواب", "أدراج"];
        EditorCategoryOptions = ["خزانة ملابس", "رفوف", "أبواب", "أدراج"];
        StatusOptions = [AllStatuses, ActiveStatus, ArchivedStatus];
        VisibleParts = [];
        RefreshVisibleParts();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public IReadOnlyList<string> CategoryOptions { get; }

    public IReadOnlyList<string> EditorCategoryOptions { get; }

    public IReadOnlyList<string> StatusOptions { get; }

    public ObservableCollection<PartListItem> VisibleParts { get; }

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
            }
        }
    }

    public string EditorTitle => IsCreating ? "إضافة جزء" : "تعديل جزء";

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

    public decimal EditorCost
    {
        get => editorCost;
        set => Set(ref editorCost, value);
    }

    public int EditorUsedInCount
    {
        get => editorUsedInCount;
        set => Set(ref editorUsedInCount, value);
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

    public bool HasNoVisibleParts => VisibleParts.Count == 0;

    public string VisibleCountLabel => $"{VisibleParts.Count} من {parts.Count} أجزاء";

    public void BeginCreate()
    {
        editingPart = null;
        IsCreating = true;
        EditorName = string.Empty;
        EditorCategory = "خزانة ملابس";
        EditorCost = 0m;
        EditorUsedInCount = 0;
        EditorError = string.Empty;
        IsEditorOpen = true;
    }

    public void BeginEdit(PartListItem part)
    {
        ArgumentNullException.ThrowIfNull(part);
        editingPart = part;
        IsCreating = false;
        EditorName = part.Name;
        EditorCategory = part.Category;
        EditorCost = part.Cost;
        EditorUsedInCount = part.UsedInCount;
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
            EditorError = "أدخل اسم الجزء.";
            return false;
        }

        if (EditorCost < 0m || EditorUsedInCount < 0)
        {
            EditorError = "يجب ألا تكون التكلفة أو عدد المنتجات سالباً.";
            return false;
        }

        if (editingPart is null)
        {
            parts.Add(new PartListItem(
                Guid.NewGuid(),
                normalizedName,
                EditorCategory,
                EditorCost,
                EditorUsedInCount));
            FeedbackMessage = "أضيف الجزء إلى المعاينة المحلية.";
        }
        else
        {
            editingPart.Name = normalizedName;
            editingPart.Category = EditorCategory;
            editingPart.Cost = EditorCost;
            editingPart.UsedInCount = EditorUsedInCount;
            FeedbackMessage = "حُدث الجزء في المعاينة المحلية.";
        }

        IsEditorOpen = false;
        EditorError = string.Empty;
        RefreshVisibleParts();
        return true;
    }

    public PartListItem Duplicate(PartListItem part)
    {
        ArgumentNullException.ThrowIfNull(part);
        var duplicate = new PartListItem(
            Guid.NewGuid(),
            $"{part.Name} — نسخة",
            part.Category,
            part.Cost,
            part.UsedInCount);
        parts.Add(duplicate);
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
