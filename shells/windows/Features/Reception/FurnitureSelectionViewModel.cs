using System.Globalization;
using Brush = System.Windows.Media.Brush;

namespace Eitmad.WindowsShell.Features.Reception;

// Public presentation values and unsaved choices. Rust validates live configurations and prices.
public sealed record SalesSize(Guid Id, string Name, string DimensionsLabel, decimal Price)
{
    public Eitmad.Contracts.CatalogEntry? Entry { get; init; }
    public string PriceLabel => FurnitureSelectionViewModel.Money(Price);
}
public sealed record SalesOption(Guid Id, string Name, decimal Price, Brush SwatchBrush)
{
    public string PriceLabel => Price == 0 ? "مشمول" : "+" + FurnitureSelectionViewModel.Money(Price);
}
public sealed class FurnitureSelectionViewModel : ObservableObject
{
    private Eitmad.Contracts.SalesConfiguration? configuration;
    private string validationMessage = "";
    private string widthCm = "", heightCm = "", depthCm = "";
    public event EventHandler? Changed;
    public Eitmad.Contracts.SalesConfiguration? Configuration => configuration;
    public void Apply(Eitmad.Contracts.SalesConfiguration value) { configuration = value; validationMessage = SalesCatalogViewModel.Availability(value.ServerAvailable); RaiseState(); }
    public void Fail(string message) { configuration = null; validationMessage = message; RaiseState(); }
    public bool CanCustomize => SelectedSize?.Entry?.Customization is not null;
    public string WidthCm { get => widthCm; set { if (Set(ref widthCm, value)) Refresh(); } }
    public string HeightCm { get => heightCm; set { if (Set(ref heightCm, value)) Refresh(); } }
    public string DepthCm { get => depthCm; set { if (Set(ref depthCm, value)) Refresh(); } }
    public string MinimumDimensionsLabel => SelectedSize?.Entry?.Customization is { } c ? SalesCatalogViewModel.DimensionsLabel(c.Minimum) : "";
    public string MaximumDimensionsLabel => SelectedSize?.Entry?.Customization is { } c ? SalesCatalogViewModel.DimensionsLabel(c.Maximum) : "";
    public Eitmad.Contracts.CheckSalesConfiguration? ConfigurationInput()
    {
        if (SelectedSize?.Entry is not { } e) return null;
        if (!Millimetres(WidthCm, out var w) || !Millimetres(HeightCm, out var h) || !Millimetres(DepthCm, out var d)) { Fail("أدخل المقاسات بالسنتيمتر، بمنزلة عشرية واحدة كحد أقصى."); return null; }
        return new() { Selection = new() { Target = e.Price.Target, PriceRevision = e.Price.Revision, ColorId = SelectedColor?.Id, HandleId = SelectedHandle?.Id, Quantity = Quantity }, Dimensions = new() { WidthMm = w, HeightMm = h, DepthMm = d } };
    }
    private static bool Millimetres(string text, out long value)
    {
        value = 0;
        if (!decimal.TryParse(PreviewText.NormalizeNumericInput(text), NumberStyles.AllowDecimalPoint, CultureInfo.InvariantCulture, out var cm) || cm > uint.MaxValue / 10m || cm * 10 != decimal.Truncate(cm * 10)) return false;
        value = (long)(cm * 10); return true;
    }
    private SalesSize? selectedSize;
    private SalesOption? selectedColor;
    private SalesOption? selectedHandle;
    private int quantity = 1;
    public FurnitureSelectionViewModel(SalesCatalogItem item, IReadOnlyList<SalesSize> sizes,
        IReadOnlyList<SalesOption> colors, IReadOnlyList<SalesOption> handles)
    {
        Item = item; Sizes = sizes; Colors = colors; Handles = handles;
    }
    public bool IsEditing { get; init; }
    public string ActionLabel => IsEditing ? "حفظ التعديلات" : "إضافة إلى عرض السعر";
    public string BackLabel => IsEditing ? "إلغاء" : "العودة إلى المنتجات";
    public SalesCatalogItem Item { get; }
    public IReadOnlyList<SalesSize> Sizes { get; }
    public IReadOnlyList<SalesOption> Colors { get; private set; }
    public IReadOnlyList<SalesOption> Handles { get; private set; }
    public bool HasColors => Colors.Count > 0;
    public bool HasHandles => Handles.Count > 0;
    public SalesSize? SelectedSize { get => selectedSize; set {
        if (!(value is null || Sizes.Contains(value)) || !Set(ref selectedSize, value)) return;
        if (value?.Entry is { } e) {
            static SalesOption Color(Eitmad.Contracts.FurnitureOption o) {
                Brush brush = System.Windows.SystemColors.ControlBrush;
                try { brush = new System.Windows.Media.SolidColorBrush((System.Windows.Media.Color)System.Windows.Media.ColorConverter.ConvertFromString(o.Visual)); }
                catch (Exception error) when (error is FormatException or NotSupportedException) { }
                return new(o.Id, o.Name, o.PriceAdjustmentYer, brush);
            }
            Colors = e.Colors.Select(Color).ToArray(); Handles = e.Handles.Select(o => new SalesOption(o.Id, o.Name, o.PriceAdjustmentYer, new Features.Furniture.FurnitureHandleOption(o.Id, o.Name, o.Visual, o.PriceAdjustmentYer).HandleBrush)).ToArray(); selectedColor = selectedHandle = null;
            widthCm = (e.Dimensions!.WidthMm / 10m).ToString("0.#", CultureInfo.InvariantCulture);
            heightCm = (e.Dimensions.HeightMm / 10m).ToString("0.#", CultureInfo.InvariantCulture);
            depthCm = (e.Dimensions.DepthMm / 10m).ToString("0.#", CultureInfo.InvariantCulture);
            foreach (var name in new[] { nameof(Colors), nameof(Handles), nameof(HasColors), nameof(HasHandles), nameof(SelectedColor), nameof(SelectedHandle), nameof(CanCustomize), nameof(MinimumDimensionsLabel), nameof(MaximumDimensionsLabel), nameof(WidthCm), nameof(HeightCm), nameof(DepthCm) }) Raise(name);
        }
        Refresh();
    } }
    public SalesOption? SelectedColor { get => selectedColor; set { if ((value is null || Colors.Contains(value)) && Set(ref selectedColor, value)) Refresh(); } }
    public SalesOption? SelectedHandle { get => selectedHandle; set { if ((value is null || Handles.Contains(value)) && Set(ref selectedHandle, value)) Refresh(); } }
    public int Quantity { get => quantity; set { if (value >= 1 && value <= 999 && Set(ref quantity, value)) Refresh(); } }
    public bool CanAdd => Item.Entry is not null ? configuration is not null : SelectedSize is not null && (!HasColors || SelectedColor is not null)
        && (!HasHandles || SelectedHandle is not null) && TryPrice(out _, out _);
    public string Guidance => Item.Entry is not null ? validationMessage : Sizes.Count == 0 ? "لا توجد مقاسات متاحة لهذا الأثاث" : CanAdd ? "" : "اختر المقاس والخيارات المتاحة لإضافة الأثاث";
    public decimal UnitPrice => Item.Entry is not null ? configuration?.Price.UnitPriceYer ?? 0 : TryPrice(out var unit, out _) ? unit : 0;
    public decimal LineTotal => Item.Entry is not null ? configuration?.Price.TotalYer ?? 0 : TryPrice(out _, out var total) ? total : 0;
    public string BasePriceLabel => SelectedSize?.PriceLabel ?? "—";
    public string AdditionsLabel => Item.Entry is not null ? configuration is null ? "—" : Money(configuration.AdditionsYer) : TryPrice(out _, out _) ? Money((SelectedColor?.Price ?? 0) + (SelectedHandle?.Price ?? 0)) : "—";
    public string UnitPriceLabel => CanAdd ? Money(UnitPrice) : "—";
    public string LineTotalLabel => CanAdd ? Money(LineTotal) : "—";
    private bool TryPrice(out decimal unit, out decimal total)
    {
        unit = total = 0;
        try { unit = checked((SelectedSize?.Price ?? 0) + (SelectedColor?.Price ?? 0) + (SelectedHandle?.Price ?? 0)); total = checked(unit * Quantity); return true; }
        catch (OverflowException) { return false; }
    }
    internal static string Money(decimal value) => value.ToString("N0", CultureInfo.InvariantCulture) + " ر.ي";
    private void Refresh()
    {
        if (Item.Entry is not null) { configuration = null; validationMessage = "اختر المقاس والخيارات للتحقق من الأثاث."; }
        RaiseState(); Changed?.Invoke(this, EventArgs.Empty);
    }
    private void RaiseState()
    {
        foreach (var name in new[] { nameof(CanAdd), nameof(Guidance), nameof(BasePriceLabel), nameof(AdditionsLabel), nameof(UnitPrice), nameof(LineTotal), nameof(UnitPriceLabel), nameof(LineTotalLabel) }) Raise(name);
    }
}

