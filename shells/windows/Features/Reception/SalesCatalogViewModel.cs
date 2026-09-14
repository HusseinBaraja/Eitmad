using System.Collections.ObjectModel;
using System.Globalization;
using System.Windows.Media;
using Eitmad.WindowsShell.Features.Furniture;
using Eitmad.WindowsShell.Features.Products;

namespace Eitmad.WindowsShell.Features.Reception;

/// <summary>A sales-only projection of transient manager preview data, not an IPC contract.</summary>
public sealed record SalesCatalogItem(Guid Id, string Name, string Category, string Description,
    string VariantSummary, decimal Price, bool HasStartingPrice, string ThumbnailKind, ImageSource? Image)
{
    public string PriceLabel => Price.ToString("N0", CultureInfo.InvariantCulture) + " YER";
    public string PricePrefix => HasStartingPrice ? "ابتداءً من" : "السعر";
    public string SelectionName => "اختيار " + Name;
}

/// <summary>Filters preview presentation state. Production catalog queries remain Rust-owned.</summary>
public sealed class SalesCatalogViewModel : ObservableObject
{
    private readonly FurnitureViewModel furniture;
    private readonly ProductsViewModel products;
    private List<SalesCatalogItem> items = [];
    private string searchText = string.Empty;
    private string selectedCategory = "الكل";
    private string selectionNotice = string.Empty;

    public SalesCatalogViewModel(FurnitureViewModel furniture, ProductsViewModel products)
    {
        this.furniture = furniture;
        this.products = products;
        Reload();
    }

    public ObservableCollection<SalesCatalogItem> VisibleItems { get; } = [];
    public ObservableCollection<string> Categories { get; } = [];
    public bool IsEmpty => VisibleItems.Count == 0;
    public string SelectionNotice { get => selectionNotice; private set => Set(ref selectionNotice, value); }
    public string SearchText
    {
        get => searchText;
        set { if (Set(ref searchText, value ?? string.Empty)) Refresh(); }
    }
    public string SelectedCategory
    {
        get => selectedCategory;
        set { if (Set(ref selectedCategory, value ?? "الكل")) Refresh(); }
    }

    public void Reload()
    {
        items = [.. furniture.GetSalesCatalogItems(), .. products.GetSalesCatalogItems()];
        var category = selectedCategory;
        Categories.Clear();
        Categories.Add("الكل");
        foreach (var name in furniture.EditorCategoryOptions.Concat(products.ActiveCategories.Select(value => value.Name)).Distinct())
            Categories.Add(name);
        SelectedCategory = Categories.Contains(category) ? category : "الكل";
        Raise(nameof(SelectedCategory));
        Refresh();
    }

    public void ClearFilters()
    {
        SearchText = string.Empty;
        SelectedCategory = "الكل";
    }

    private bool isReviewingQuotation;
    public bool IsReviewingQuotation { get => isReviewingQuotation; set => Set(ref isReviewingQuotation, value); }
    private FurnitureSelectionViewModel? selection;
    public FurnitureSelectionViewModel? Selection { get => selection; private set { Set(ref selection, value); Raise(nameof(IsSelecting)); } }
    public bool IsSelecting => Selection is not null;
    public ObservableCollection<PreviewQuotationLine> QuotationLines { get; } = [];
    public string QuotationLabel => $"عرض السعر · {QuotationLines.Count} عناصر";
    public void CloseSelection() => Selection = null;
    public bool AddSelection()
    {
        if (Selection is not { CanAdd: true } current) return false;
        QuotationLines.Add(new(current.Item.Name, current.SelectedSize!.Name, current.SelectedColor?.Name,
            current.SelectedHandle?.Name, current.Quantity, current.UnitPrice, current.LineTotal));
        Raise(nameof(QuotationLabel));
        return true;
    }

    public void Select(SalesCatalogItem item)
    {
        if (!VisibleItems.Contains(item)) return;
        Selection = furniture.GetSalesSelection(item.Id);
        if (Selection is null)
            SelectionNotice = $"تم اختيار {item.Name} للمعاينة فقط. إعداد الصنف غير متاح بعد.";
    }

    private void Refresh()
    {
        SelectionNotice = string.Empty;
        var query = PreviewText.NormalizeSearch(SearchText.Trim());
        VisibleItems.Clear();
        foreach (var item in items.Where(item =>
                     (SelectedCategory == "الكل" || item.Category == SelectedCategory) &&
                     (PreviewText.NormalizeSearch(item.Name).Contains(query, StringComparison.OrdinalIgnoreCase) ||
                      PreviewText.NormalizeSearch(item.Category).Contains(query, StringComparison.OrdinalIgnoreCase))))
            VisibleItems.Add(item);
        Raise(nameof(IsEmpty));
    }
}
