using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Windows.Media;
using MediaColor = System.Windows.Media.Color;
using MediaColorConverter = System.Windows.Media.ColorConverter;

namespace Eitmad.WindowsShell.Features.Furniture;

/// <summary>Represents one furniture row on the transient manager preview.</summary>
public sealed class FurnitureListItem
{
    public FurnitureListItem(
        Guid id,
        string name,
        string category,
        int variantCount,
        decimal sellingPrice,
        string thumbnailKind,
        bool isArchived = false)
    {
        Id = id;
        Name = name;
        Category = category;
        VariantCount = variantCount;
        SellingPrice = sellingPrice;
        ThumbnailKind = thumbnailKind;
        IsArchived = isArchived;
    }

    public Guid Id { get; }

    public string Name { get; set; }

    public string Category { get; set; }

    public int VariantCount { get; set; }

    public decimal SellingPrice { get; set; }

    public string ThumbnailKind { get; }

    public bool IsArchived { get; set; }

    public bool CanArchive => !IsArchived;

    public string VariantCountLabel => VariantCount switch
    {
        1 => "مقاس واحد",
        2 => "مقاسان",
        _ => $"{VariantCount} مقاسات",
    };

    public string SellingPriceAmountLabel => SellingPrice.ToString("N0", CultureInfo.InvariantCulture);

    public string StatusLabel => IsArchived ? "مؤرشف" : "نشط";
}

/// <summary>Describes one selectable furniture part in the transient picker.</summary>
public sealed record FurniturePartOption(Guid Id, string Name, string Category, decimal UnitCost)
{
    public string UnitCostLabel => UnitCost.ToString("N0", CultureInfo.InvariantCulture);
}

/// <summary>Owns the local quantity and calculated row total for a selected part.</summary>
public sealed class FurniturePartUsage : INotifyPropertyChanged
{
    private decimal quantity;

    public FurniturePartUsage(FurniturePartOption part, decimal quantity = 1m)
    {
        Part = part;
        this.quantity = quantity;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

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

    public decimal TotalCost => TryCalculateTotalCost(out var total) ? total : 0m;

    public string UnitCostLabel => Part.UnitCost.ToString("N0", CultureInfo.InvariantCulture);

    public string TotalCostLabel => TryCalculateTotalCost(out var total)
        ? total.ToString("N0", CultureInfo.InvariantCulture)
        : "—";

    public bool TryCalculateTotalCost(out decimal total)
    {
        try
        {
            total = decimal.Round(checked(Quantity * Part.UnitCost), 0, MidpointRounding.AwayFromZero);
            return true;
        }
        catch (OverflowException)
        {
            total = 0m;
            return false;
        }
    }

    public FurniturePartUsage Copy() => new(Part, Quantity);

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}

/// <summary>Represents one fixed manager-defined furniture size in the preview.</summary>
public sealed class FurnitureVariant
{
    public FurnitureVariant(Guid id, string name, decimal width, decimal height, decimal depth, decimal calculatedCost)
    {
        Id = id;
        Name = name;
        Width = width;
        Height = height;
        Depth = depth;
        CalculatedCost = calculatedCost;
    }

    public Guid Id { get; }

    public string Name { get; set; }

    public decimal Width { get; set; }

    public decimal Height { get; set; }

    public decimal Depth { get; set; }

    public decimal CalculatedCost { get; set; }

    public string DimensionsLabel => $"{Format(Width)} × {Format(Height)} × {Format(Depth)} cm";

    public string CalculatedCostLabel => CalculatedCost.ToString("N0", CultureInfo.InvariantCulture);

    public FurnitureVariant Copy(string name) =>
        new(Guid.NewGuid(), name, Width, Height, Depth, CalculatedCost);

    private static string Format(decimal value) => value.ToString("0.##", CultureInfo.InvariantCulture);
}

/// <summary>Represents one selectable furniture color in the transient options preview.</summary>
public sealed class FurnitureColorOption : INotifyPropertyChanged
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

    public event PropertyChangedEventHandler? PropertyChanged;

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
        ? "Included"
        : $"+{PriceAdjustment.ToString("N0", CultureInfo.InvariantCulture)} YER";

    public string StatusLabel => IsActive ? "نشط" : "غير نشط";

    public string ToggleActionLabel => IsActive ? "تعطيل" : "تفعيل";

    public FurnitureColorOption Copy() => new(Guid.NewGuid(), Name, SwatchHex, PriceAdjustment, IsActive);

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}

/// <summary>Represents one selectable furniture handle in the transient options preview.</summary>
public sealed class FurnitureHandleOption : INotifyPropertyChanged
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

    public event PropertyChangedEventHandler? PropertyChanged;

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
        ? "Included"
        : $"+{PriceAdjustment.ToString("N0", CultureInfo.InvariantCulture)} YER";

    public string StatusLabel => IsActive ? "نشط" : "غير نشط";

    public string ToggleActionLabel => IsActive ? "تعطيل" : "تفعيل";

    public FurnitureHandleOption Copy() => new(Guid.NewGuid(), Name, HandleKind, PriceAdjustment, IsActive);

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}
