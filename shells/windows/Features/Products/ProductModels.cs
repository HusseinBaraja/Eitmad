using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Features.Products;

/// <summary>Represents one ready-made product row in the transient manager preview.</summary>
public sealed class ProductListItem : INotifyPropertyChanged
{
    private bool isArchived;

    public ProductListItem(
        Guid id,
        string name,
        string category,
        decimal purchaseCost,
        decimal sellingPrice,
        string variantSummary,
        string thumbnailKind,
        ImageSource? image = null,
        bool isArchived = false)
    {
        Id = id;
        Name = name;
        Category = category;
        PurchaseCost = purchaseCost;
        SellingPrice = sellingPrice;
        VariantSummary = variantSummary;
        ThumbnailKind = thumbnailKind;
        Image = image;
        this.isArchived = isArchived;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public Guid Id { get; }

    public string Name { get; set; }

    public string Category { get; set; }

    public decimal PurchaseCost { get; set; }

    public decimal SellingPrice { get; set; }

    public string VariantSummary { get; set; }

    public string ThumbnailKind { get; }

    public ImageSource? Image { get; set; }

    public bool IsArchived
    {
        get => isArchived;
        set
        {
            if (isArchived == value)
            {
                return;
            }

            isArchived = value;
            Raise();
            Raise(nameof(CanArchive));
            Raise(nameof(StatusLabel));
        }
    }

    public bool CanArchive => !IsArchived;

    public string PurchaseCostLabel => PurchaseCost.ToString("N0", CultureInfo.InvariantCulture);

    public string SellingPriceLabel => SellingPrice.ToString("N0", CultureInfo.InvariantCulture);

    public string StatusLabel => IsArchived ? "مؤرشف" : "نشط";

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}

/// <summary>Represents one supplier-defined ready-made option and its direct pricing.</summary>
public sealed class ProductVariant : INotifyPropertyChanged
{
    private string name;
    private decimal purchaseCost;
    private decimal sellingPrice;

    public ProductVariant(Guid id, string name, decimal purchaseCost, decimal sellingPrice)
    {
        Id = id;
        this.name = name;
        this.purchaseCost = purchaseCost;
        this.sellingPrice = sellingPrice;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public Guid Id { get; }

    public string Name
    {
        get => name;
        set => Set(ref name, value ?? string.Empty);
    }

    public decimal PurchaseCost
    {
        get => purchaseCost;
        set
        {
            if (Set(ref purchaseCost, value))
            {
                Raise(nameof(Margin));
                Raise(nameof(MarginLabel));
            }
        }
    }

    public decimal SellingPrice
    {
        get => sellingPrice;
        set
        {
            if (Set(ref sellingPrice, value))
            {
                Raise(nameof(Margin));
                Raise(nameof(MarginLabel));
            }
        }
    }

    public decimal Margin => SellingPrice - PurchaseCost;

    public string MarginLabel => Margin.ToString("N0", CultureInfo.InvariantCulture);

    public ProductVariant Copy() => new(Guid.NewGuid(), Name, PurchaseCost, SellingPrice);

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

/// <summary>Represents one transient product category in the established inline category interaction.</summary>
public sealed class ProductCategoryOption : INotifyPropertyChanged
{
    private string name;
    private bool isArchived;

    public ProductCategoryOption(string name)
    {
        this.name = name;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public string Name
    {
        get => name;
        set => Set(ref name, value ?? string.Empty);
    }

    public bool IsArchived
    {
        get => isArchived;
        set
        {
            if (Set(ref isArchived, value))
            {
                Raise(nameof(CanArchive));
                Raise(nameof(StatusLabel));
            }
        }
    }

    public bool CanArchive => !IsArchived;

    public string StatusLabel => IsArchived ? "مؤرشفة" : string.Empty;

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

/// <summary>Keeps presentation-only details that are not projected in the manager list row.</summary>
public sealed record ProductDraftDetails(
    string Description,
    string Notes,
    ImageSource? Image,
    string ImageName,
    IReadOnlyList<ProductVariant> Variants);
