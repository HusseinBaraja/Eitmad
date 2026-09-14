using System.Globalization;

namespace Eitmad.WindowsShell.Features.Reception;

// Sales-only snapshots for the temporary quotation preview.
public sealed record SalesProductVariant(Guid Id, string Name, decimal Price)
{
    public string PriceLabel => FurnitureSelectionViewModel.Money(Price);
}

public sealed class ProductSelectionViewModel : ObservableObject
{
    private SalesProductVariant? selectedVariant;
    private int quantity = 1;

    public ProductSelectionViewModel(SalesCatalogItem item, IReadOnlyList<SalesProductVariant> variants)
    {
        Item = item;
        Variants = variants;
    }

    public SalesCatalogItem Item { get; }
    public IReadOnlyList<SalesProductVariant> Variants { get; }
    public bool HasVariants => Variants.Count > 0;
    public bool HasNoVariants => !HasVariants;
    public SalesProductVariant? SelectedVariant
    {
        get => selectedVariant;
        set { if ((value is null || Variants.Contains(value)) && Set(ref selectedVariant, value)) Refresh(); }
    }
    public int Quantity
    {
        get => quantity;
        set { if (value is >= 1 and <= 999 && Set(ref quantity, value)) Refresh(); }
    }
    public decimal UnitPrice => HasVariants ? SelectedVariant?.Price ?? 0 : Item.Price;
    public decimal LineTotal => TryTotal(out var total) ? total : 0;
    public bool CanAdd => (!HasVariants || SelectedVariant is not null) && TryTotal(out _);
    public string UnitPriceLabel => CanAdd ? UnitPrice.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public string LineTotalLabel => CanAdd ? LineTotal.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public string Guidance => HasVariants && SelectedVariant is null ? "اختر النوع / المقاس لإضافة المنتج" : CanAdd ? string.Empty : "السعر غير متاح";
    private bool TryTotal(out decimal total)
    {
        total = 0;
        try { total = checked(UnitPrice * Quantity); return true; }
        catch (OverflowException) { return false; }
    }
    private void Refresh()
    {
        foreach (var name in new[] { nameof(UnitPrice), nameof(LineTotal), nameof(CanAdd), nameof(UnitPriceLabel), nameof(LineTotalLabel), nameof(Guidance) }) Raise(name);
    }
}
