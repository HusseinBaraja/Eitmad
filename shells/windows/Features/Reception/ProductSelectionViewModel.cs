using System.Globalization;

namespace Eitmad.WindowsShell.Features.Reception;

// Sales-only snapshots for the temporary quotation preview.
public sealed record SalesProductVariant(Guid Id, string Name, decimal Price)
{
    public Eitmad.Contracts.CatalogEntry? Entry { get; init; }
    public string PriceLabel => FurnitureSelectionViewModel.Money(Price);
}

public sealed class ProductSelectionViewModel : ObservableObject
{
    private Eitmad.Contracts.SalesConfiguration? configuration;
    private string validationMessage = "";
    public event EventHandler? Changed;
    public Eitmad.Contracts.SalesConfiguration? Configuration => configuration;
    public void Apply(Eitmad.Contracts.SalesConfiguration value) { configuration = value; validationMessage = SalesCatalogViewModel.Availability(value.ServerAvailable); RaiseState(); }
    public void Fail(string message) { configuration = null; validationMessage = message; RaiseState(); }
    public Eitmad.Contracts.CheckSalesConfiguration? ConfigurationInput() => SelectedVariant?.Entry is { } e ? new()
    { Selection = new() { Target = e.Price.Target, PriceRevision = e.Price.Revision, Quantity = Quantity }, Dimensions = null! } : null;
    private SalesProductVariant? selectedVariant;
    private int quantity = 1;

    public ProductSelectionViewModel(SalesCatalogItem item, IReadOnlyList<SalesProductVariant> variants)
    {
        Item = item;
        Variants = variants;
    }

    public bool IsEditing { get; init; }
    public string ActionLabel => IsEditing ? "حفظ التعديلات" : "إضافة إلى عرض السعر";
    public string BackLabel => IsEditing ? "إلغاء" : "العودة إلى المنتجات";
    public SalesCatalogItem Item { get; }
    public string PersistenceNotice => Item.Entry is null ? "الاختيار وعرض السعر مؤقتان ولا يتم حفظهما" : "الاختيار محلي. راجع عرض السعر ثم استخدم حفظ كمسودة لتأكيد الحفظ.";
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
    public decimal UnitPrice => Item.Entry is not null ? configuration?.Price.UnitPriceYer ?? 0 : HasVariants ? SelectedVariant?.Price ?? 0 : Item.Price;
    public decimal LineTotal => Item.Entry is not null ? configuration?.Price.TotalYer ?? 0 : TryTotal(out var total) ? total : 0;
    public bool CanAdd => Item.Entry is not null ? configuration is not null : (!HasVariants || SelectedVariant is not null) && TryTotal(out _);
    public string UnitPriceLabel => CanAdd ? UnitPrice.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public string LineTotalLabel => CanAdd ? LineTotal.ToString("N0", CultureInfo.InvariantCulture) : "—";
    public string Guidance => Item.Entry is not null ? validationMessage : HasVariants && SelectedVariant is null ? "اختر النوع / المقاس لإضافة المنتج" : CanAdd ? string.Empty : "السعر غير متاح";
    private bool TryTotal(out decimal total)
    {
        total = 0;
        try { total = checked(UnitPrice * Quantity); return true; }
        catch (OverflowException) { return false; }
    }
    /// <summary>Invalidates checked totals after input changes and requests a new Rust validation result.</summary>
    private void Refresh()
    {
        if (Item.Entry is not null) { configuration = null; validationMessage = "اختر النوع / المقاس للتحقق من المنتج."; }
        RaiseState(); Changed?.Invoke(this, EventArgs.Empty);
    }
    private void RaiseState()
    {
        foreach (var name in new[] { nameof(UnitPrice), nameof(LineTotal), nameof(CanAdd), nameof(UnitPriceLabel), nameof(LineTotalLabel), nameof(Guidance) }) Raise(name);
    }
}
