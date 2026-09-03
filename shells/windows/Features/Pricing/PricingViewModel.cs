using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text;

namespace Eitmad.WindowsShell.Features.Pricing;

/// <summary>
/// Owns ephemeral list and quick-edit state for the pricing preview.
/// Rust-authoritative price commands, authorization, audit, and storage are not available yet.
/// </summary>
public sealed class PricingViewModel : INotifyPropertyChanged
{
    public const string AllCategories = "كل الفئات";

    private readonly List<PricingListItem> prices;
    private PricingListItem? editingPrice;
    private string searchText = string.Empty;
    private string selectedCategory = AllCategories;
    private bool isEditorOpen;
    private string editorSellingPrice = string.Empty;
    private decimal editorMargin;
    private string editorError = string.Empty;
    private string feedbackMessage = string.Empty;

    public PricingViewModel()
    {
        prices =
        [
            new(Guid.Parse("61719ec5-ea1c-4906-a521-6016dd9c7771"), "خزانة ملابس", "صغير", "غرف النوم", 160_000m, 200_000m),
            new(Guid.Parse("80f0805d-b336-4473-83bf-1e08066462f8"), "خزانة ملابس", "كبير", "غرف النوم", 245_000m, 310_000m),
            new(Guid.Parse("59e96722-2605-4a4e-b35e-34c4a3fabf32"), "طاولة طعام", "ستة كراسي", "غرف الطعام", 285_000m, 360_000m),
            new(Guid.Parse("42951f5d-fc92-4f6d-b697-a6191894402a"), "كرسي استقبال", "قياسي", "المكاتب", 68_000m, 85_000m),
            new(Guid.Parse("9fa9b03a-4dd6-478a-a1cf-418424ac462f"), "مكتبة جدارية", "عرض 180 سم", "غرف المعيشة", 190_000m, 235_000m, isActive: false),
        ];

        Categories = [AllCategories, .. prices.Select(item => item.Category).Distinct()];
        VisiblePrices = [];
        RefreshVisiblePrices();
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public IReadOnlyList<string> Categories { get; }

    public ObservableCollection<PricingListItem> VisiblePrices { get; }

    public string SearchText
    {
        get => searchText;
        set
        {
            if (Set(ref searchText, value ?? string.Empty))
            {
                RefreshVisiblePrices();
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
                RefreshVisiblePrices();
            }
        }
    }

    public bool IsEditorOpen
    {
        get => isEditorOpen;
        private set => Set(ref isEditorOpen, value);
    }

    public string EditorProduct => editingPrice?.Product ?? string.Empty;

    public string EditorVariant => editingPrice?.Variant ?? string.Empty;

    public string EditorCost => editingPrice?.CostLabel ?? string.Empty;

    public string EditorSellingPrice
    {
        get => editorSellingPrice;
        set
        {
            if (Set(ref editorSellingPrice, value ?? string.Empty))
            {
                UpdateEditorMargin();
            }
        }
    }

    public string EditorMargin => $"{editorMargin.ToString("N0", CultureInfo.InvariantCulture)} YER";

    public bool HasNegativeEditorMargin => editorMargin < 0m;

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

    public bool HasEditorError => EditorError.Length > 0;

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

    public bool HasFeedback => FeedbackMessage.Length > 0;

    public bool HasNoVisiblePrices => VisiblePrices.Count == 0;

    public string VisibleCountLabel => $"{VisiblePrices.Count} من {prices.Count} أسعار";

    public void BeginEdit(PricingListItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        editingPrice = item;
        EditorSellingPrice = item.SellingPrice.ToString("N0", CultureInfo.InvariantCulture);
        UpdateEditorMargin();
        EditorError = string.Empty;
        IsEditorOpen = true;
        RaiseEditorDetails();
    }

    public void CancelEditor()
    {
        IsEditorOpen = false;
        EditorError = string.Empty;
        editingPrice = null;
    }

    public bool SaveEditor()
    {
        if (editingPrice is null || !TryParsePrice(EditorSellingPrice, out var sellingPrice) || sellingPrice < 0m)
        {
            EditorError = "أدخل سعر بيع صالحاً يساوي صفراً أو أكثر.";
            return false;
        }

        editingPrice.SellingPrice = decimal.Round(sellingPrice, 0, MidpointRounding.AwayFromZero);
        FeedbackMessage = "حُدث سعر البيع في المعاينة المحلية فقط.";
        IsEditorOpen = false;
        EditorError = string.Empty;
        editingPrice = null;
        return true;
    }

    public void ClearFeedback() => FeedbackMessage = string.Empty;

    private void UpdateEditorMargin()
    {
        if (editingPrice is not null && TryParsePrice(EditorSellingPrice, out var sellingPrice))
        {
            editorMargin = sellingPrice - editingPrice.Cost;
            EditorError = string.Empty;
        }
        else
        {
            editorMargin = 0m;
        }

        Raise(nameof(EditorMargin));
        Raise(nameof(HasNegativeEditorMargin));
    }

    private void RaiseEditorDetails()
    {
        Raise(nameof(EditorProduct));
        Raise(nameof(EditorVariant));
        Raise(nameof(EditorCost));
        Raise(nameof(EditorMargin));
        Raise(nameof(HasNegativeEditorMargin));
    }

    private void RefreshVisiblePrices()
    {
        var search = NormalizeSearchText(SearchText.Trim());
        var matches = prices.Where(item =>
            (search.Length == 0
             || NormalizeSearchText(item.Product).Contains(search, StringComparison.CurrentCultureIgnoreCase)
             || NormalizeSearchText(item.Variant).Contains(search, StringComparison.CurrentCultureIgnoreCase))
            && (SelectedCategory == AllCategories || item.Category == SelectedCategory));

        VisiblePrices.Clear();
        foreach (var item in matches)
        {
            VisiblePrices.Add(item);
        }

        Raise(nameof(HasNoVisiblePrices));
        Raise(nameof(VisibleCountLabel));
    }

    private static bool TryParsePrice(string value, out decimal result) =>
        decimal.TryParse(NormalizeNumericInput(value), NumberStyles.Number, CultureInfo.InvariantCulture, out result);

    private static string NormalizeNumericInput(string value)
    {
        var normalized = new StringBuilder(value.Length);
        foreach (var character in value)
        {
            normalized.Append(character switch
            {
                >= '\u0660' and <= '\u0669' => (char)('0' + character - '\u0660'),
                >= '\u06F0' and <= '\u06F9' => (char)('0' + character - '\u06F0'),
                '\u066B' => '.',
                '\u066C' => ',',
                _ => character,
            });
        }

        return normalized.ToString();
    }

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
