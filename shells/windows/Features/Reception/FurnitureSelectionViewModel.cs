using System.Globalization;
using Brush = System.Windows.Media.Brush;

namespace Eitmad.WindowsShell.Features.Reception;

// Sales-only snapshots of manager fixtures. No durable data or production pricing authority.
public sealed record SalesSize(Guid Id, string Name, string DimensionsLabel, decimal Price)
{
    public string PriceLabel => FurnitureSelectionViewModel.Money(Price);
}
public sealed record SalesOption(Guid Id, string Name, decimal Price, Brush SwatchBrush)
{
    public string PriceLabel => Price == 0 ? "مشمول" : "+" + FurnitureSelectionViewModel.Money(Price);
}
public sealed record PreviewQuotationLine(string Name, string Variant, string? Color, string? Handle,
    int Quantity, decimal UnitPrice, decimal LineTotal);

public sealed class FurnitureSelectionViewModel : ObservableObject
{
    private SalesSize? selectedSize;
    private SalesOption? selectedColor;
    private SalesOption? selectedHandle;
    private int quantity = 1;
    public FurnitureSelectionViewModel(SalesCatalogItem item, IReadOnlyList<SalesSize> sizes,
        IReadOnlyList<SalesOption> colors, IReadOnlyList<SalesOption> handles)
    {
        Item = item; Sizes = sizes; Colors = colors; Handles = handles;
    }
    public SalesCatalogItem Item { get; }
    public IReadOnlyList<SalesSize> Sizes { get; }
    public IReadOnlyList<SalesOption> Colors { get; }
    public IReadOnlyList<SalesOption> Handles { get; }
    public bool HasColors => Colors.Count > 0;
    public bool HasHandles => Handles.Count > 0;
    public SalesSize? SelectedSize { get => selectedSize; set { if ((value is null || Sizes.Contains(value)) && Set(ref selectedSize, value)) Refresh(); } }
    public SalesOption? SelectedColor { get => selectedColor; set { if ((value is null || Colors.Contains(value)) && Set(ref selectedColor, value)) Refresh(); } }
    public SalesOption? SelectedHandle { get => selectedHandle; set { if ((value is null || Handles.Contains(value)) && Set(ref selectedHandle, value)) Refresh(); } }
    public int Quantity { get => quantity; set { if (value >= 1 && value <= 999 && Set(ref quantity, value)) Refresh(); } }
    public bool CanAdd => SelectedSize is not null && (!HasColors || SelectedColor is not null)
        && (!HasHandles || SelectedHandle is not null) && TryPrice(out _, out _);
    public string Guidance => Sizes.Count == 0 ? "لا توجد مقاسات متاحة لهذا الأثاث" : CanAdd ? "" : "اختر المقاس والخيارات المتاحة لإضافة الأثاث";
    public decimal UnitPrice => TryPrice(out var unit, out _) ? unit : 0;
    public decimal LineTotal => TryPrice(out _, out var total) ? total : 0;
    public string BasePriceLabel => SelectedSize?.PriceLabel ?? "—";
    public string AdditionsLabel => TryPrice(out _, out _) ? Money((SelectedColor?.Price ?? 0) + (SelectedHandle?.Price ?? 0)) : "—";
    public string UnitPriceLabel => CanAdd ? Money(UnitPrice) : "—";
    public string LineTotalLabel => CanAdd ? Money(LineTotal) : "—";
    private bool TryPrice(out decimal unit, out decimal total)
    {
        unit = total = 0;
        try { unit = checked((SelectedSize?.Price ?? 0) + (SelectedColor?.Price ?? 0) + (SelectedHandle?.Price ?? 0)); total = checked(unit * Quantity); return true; }
        catch (OverflowException) { return false; }
    }
    internal static string Money(decimal value) => value.ToString("N0", CultureInfo.InvariantCulture) + " YER";
    private void Refresh()
    {
        foreach (var name in new[] { nameof(CanAdd), nameof(Guidance), nameof(BasePriceLabel), nameof(AdditionsLabel), nameof(UnitPrice), nameof(LineTotal), nameof(UnitPriceLabel), nameof(LineTotalLabel) }) Raise(name);
    }
}

