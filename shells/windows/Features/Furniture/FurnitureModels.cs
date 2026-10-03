using System.Globalization;
using System.Windows.Media;
using MediaColor = System.Windows.Media.Color;
using MediaColorConverter = System.Windows.Media.ColorConverter;

namespace Eitmad.WindowsShell.Features.Furniture;

public sealed record FurnitureListItem(
    Guid Id,
    string Name,
    string Category,
    int VariantCount,
    decimal SellingPrice,
    string ThumbnailKind,
    bool IsArchived,
    bool IsDraft)
{
    public bool CanArchive => !IsArchived;

    public string VariantCountLabel => VariantCount switch
    {
        1 => "مقاس واحد",
        2 => "مقاسان",
        _ => $"{VariantCount} مقاسات",
    };

    public string SellingPriceAmountLabel => SellingPrice.ToString("N0", CultureInfo.InvariantCulture);

    public string StatusLabel => IsArchived ? "مؤرشف" : IsDraft ? "مسودة" : "نشط";
}

/// <summary>Describes one selectable furniture part in the transient picker.</summary>
public sealed record FurniturePartOption(Guid Id, string Name, string Category, decimal UnitCost)
{
    public Eitmad.Contracts.CompositionReference? Reference { get; init; }
    public string UnitCostLabel => UnitCost.ToString("N0", CultureInfo.InvariantCulture);
}

/// <summary>Owns the local quantity and calculated row total for a selected part.</summary>
public sealed class FurniturePartUsage : ObservableObject
{
    private decimal quantity;

    public FurniturePartUsage(FurniturePartOption part, decimal quantity = 1m)
    {
        Part = part;
        this.quantity = quantity;
    }

    public FurniturePartOption Part { get; }

    public decimal Quantity
    {
        get => quantity;
        set
        {
            if (quantity == value)
            {
                return;
            }

            quantity = value;
            Raise();
            Raise(nameof(TotalCost));
            Raise(nameof(TotalCostLabel));
        }
    }

    public decimal TotalCost { get; private set; }
    /// <summary>Displays a row cost supplied by Rust and updates bound labels.</summary>
    public void ApplyRowCost(decimal value) { TotalCost = value; Raise(nameof(TotalCost)); Raise(nameof(TotalCostLabel)); }

    public string UnitCostLabel => Part.UnitCost.ToString("N0", CultureInfo.InvariantCulture);

    public string TotalCostLabel => TotalCost.ToString("N0", CultureInfo.InvariantCulture);

}

/// <summary>Represents one fixed manager-defined furniture size in the preview.</summary>
public sealed class FurnitureVariant : ObservableObject
{
    private decimal sellingPrice;
    private decimal? reviewedMargin;
    public Eitmad.Contracts.FurnitureCustomization? Customization { get; set; }
    public Guid[] ColorIds { get; set; } = [];
    public Guid[] HandleIds { get; set; } = [];
    public bool IsArchived { get; set; }
    /// <summary>Displays Rust cost and margin results without calculating domain values.</summary>
    public void ApplyReview(decimal cost, decimal margin) { CalculatedCost=cost; reviewedMargin=margin; Raise(nameof(CalculatedCostLabel)); Raise(nameof(MarginLabel)); Raise(nameof(HasNegativeMargin)); Raise(nameof(MarginCaption)); }

    public FurnitureVariant(
        Guid id,
        string name,
        decimal width,
        decimal height,
        decimal depth,
        decimal calculatedCost,
        decimal? sellingPrice = null)
    {
        Id = id;
        Name = name;
        Width = width;
        Height = height;
        Depth = depth;
        CalculatedCost = calculatedCost;
        this.sellingPrice = sellingPrice ?? calculatedCost;
    }

    public Guid Id { get; }

    public string Name { get; set; }

    public decimal Width { get; set; }

    public decimal Height { get; set; }

    public decimal Depth { get; set; }

    public decimal CalculatedCost { get; set; }

    public decimal SellingPrice
    {
        get => sellingPrice;
        set
        {
            if (sellingPrice == value)
            {
                return;
            }

            sellingPrice = value;
            reviewedMargin = null;
            Raise();
            Raise(nameof(SellingPriceLabel));
            Raise(nameof(Margin));
            Raise(nameof(MarginLabel));
            Raise(nameof(MarginCaption));
            Raise(nameof(HasNegativeMargin));
        }
    }

    public string SellingPriceInput
    {
        get => SellingPrice.ToString("N0", CultureInfo.InvariantCulture);
        set
        {
            if (!decimal.TryParse(PreviewText.NormalizeNumericInput(value), NumberStyles.Number, CultureInfo.InvariantCulture, out var parsed)
                || parsed < 0m || parsed != decimal.Truncate(parsed) || parsed > long.MaxValue)
            {
                throw new FormatException("أدخل سعر بيع صالحاً يساوي صفراً أو أكثر.");
            }

            SellingPrice = parsed;
        }
    }

    public decimal Margin => reviewedMargin ?? 0m;

    public bool HasNegativeMargin => Margin < 0m;

    public string DimensionsLabel => $"{Format(Width)} × {Format(Height)} × {Format(Depth)} سم";

    public string CalculatedCostLabel => CalculatedCost.ToString("N0", CultureInfo.InvariantCulture);

    public string SellingPriceLabel => SellingPrice.ToString("N0", CultureInfo.InvariantCulture);

    public string MarginLabel => reviewedMargin?.ToString("N0", CultureInfo.InvariantCulture) ?? "—";

    public string MarginCaption => HasNegativeMargin ? "خسارة متوقعة" : "هامش الربح";

    public FurnitureVariant Copy(string name) =>
        new(Guid.NewGuid(), name, Width, Height, Depth, CalculatedCost, SellingPrice) { Customization=Customization,ColorIds=ColorIds.ToArray(),HandleIds=HandleIds.ToArray() };

    private static string Format(decimal value) => value.ToString("0.##", CultureInfo.InvariantCulture);
}

/// <summary>Represents one selectable furniture color in the transient options preview.</summary>
public sealed class FurnitureColorOption : ObservableObject
{
    private bool isActive;

    public FurnitureColorOption(Guid id, string name, string swatchHex, decimal priceAdjustment, bool isActive = true)
    {
        Id = id;
        Name = name;
        SwatchHex = swatchHex;
        PriceAdjustment = priceAdjustment;
        this.isActive = isActive;
    }

    public Guid Id { get; }

    public string Name { get; set; }

    public string SwatchHex { get; }

    public decimal PriceAdjustment { get; }

    public bool IsActive
    {
        get => isActive;
        set
        {
            if (isActive == value)
            {
                return;
            }

            isActive = value;
            Raise();
            Raise(nameof(StatusLabel));
            Raise(nameof(ToggleActionLabel));
        }
    }

    public System.Windows.Media.Brush SwatchBrush => new SolidColorBrush((MediaColor)MediaColorConverter.ConvertFromString(SwatchHex));

    public string PriceAdjustmentLabel => PriceAdjustment == 0m
        ? "مشمول"
        : $"+{PriceAdjustment.ToString("N0", CultureInfo.InvariantCulture)} ر.ي";

    public string StatusLabel => IsActive ? "نشط" : "غير نشط";

    public string ToggleActionLabel => IsActive ? "تعطيل" : "تفعيل";

}

/// <summary>Represents one selectable furniture handle in the transient options preview.</summary>
public sealed class FurnitureHandleOption : ObservableObject
{
    private bool isActive;

    public FurnitureHandleOption(Guid id, string name, string handleKind, decimal priceAdjustment, bool isActive = true)
    {
        Id = id;
        Name = name;
        HandleKind = handleKind;
        PriceAdjustment = priceAdjustment;
        this.isActive = isActive;
    }

    public Guid Id { get; }

    public string Name { get; set; }

    public string HandleKind { get; }

    public decimal PriceAdjustment { get; }

    public bool IsActive
    {
        get => isActive;
        set
        {
            if (isActive == value)
            {
                return;
            }

            isActive = value;
            Raise();
            Raise(nameof(StatusLabel));
            Raise(nameof(ToggleActionLabel));
        }
    }

    public System.Windows.Media.Brush HandleBrush => HandleKind switch
    {
        "BlackMetal" => new SolidColorBrush(MediaColor.FromRgb(44, 45, 45)),
        "Brass" => new SolidColorBrush(MediaColor.FromRgb(184, 131, 58)),
        _ => new SolidColorBrush(MediaColor.FromRgb(153, 100, 54)),
    };

    public System.Windows.Media.Brush HandleAccentBrush => HandleKind switch
    {
        "BlackMetal" => new SolidColorBrush(MediaColor.FromRgb(116, 119, 118)),
        "Brass" => new SolidColorBrush(MediaColor.FromRgb(239, 209, 145)),
        _ => new SolidColorBrush(MediaColor.FromRgb(221, 180, 134)),
    };

    public string PriceAdjustmentLabel => PriceAdjustment == 0m
        ? "مشمول"
        : $"+{PriceAdjustment.ToString("N0", CultureInfo.InvariantCulture)} ر.ي";

    public string StatusLabel => IsActive ? "نشط" : "غير نشط";

    public string ToggleActionLabel => IsActive ? "تعطيل" : "تفعيل";

    /// <summary>Copies unsaved presentation values without committing a record.</summary>
}
