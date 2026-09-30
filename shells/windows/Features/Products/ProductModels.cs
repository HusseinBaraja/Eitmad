using System.Globalization;
using System.Windows.Media;

namespace Eitmad.WindowsShell.Features.Products;

/// <summary>Represents one ready-made product row in the Rust manager projection.</summary>
public sealed class ProductListItem : ObservableObject
{
    private bool isArchived;

    public ProductListItem(
        Guid id,
        string name,
        string category,
        decimal purchaseCost,
        string variantSummary,
        string thumbnailKind,
        ImageSource? image = null,
        bool isArchived = false)
    {
        Id = id;
        Name = name;
        Category = category;
        PurchaseCost = purchaseCost;
        VariantSummary = variantSummary;
        ThumbnailKind = thumbnailKind;
        Image = image;
        this.isArchived = isArchived;
    }

    public Guid Id { get; }

    public string Name { get; set; }

    public string Category { get; set; }

    public decimal PurchaseCost { get; set; }

    public string VariantSummary { get; set; }

    public string ThumbnailKind { get; private set; }

    public void UpdateThumbnailKind(string thumbnailKind)
    {
        if (ThumbnailKind == thumbnailKind)
        {
            return;
        }

        ThumbnailKind = thumbnailKind;
        Raise(nameof(ThumbnailKind));
    }

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

    public bool HasPurchaseCost { get; init; } = true;
    public string PurchaseCostLabel => !HasPurchaseCost ? "—" : PurchaseCost.ToString("N0", CultureInfo.InvariantCulture);

    public string StatusLabel => IsArchived ? "مؤرشف" : "نشط";
}

/// <summary>Represents one supplier-defined ready-made option and its purchase cost.</summary>
public sealed class ProductVariant : ObservableObject
{
    private string name;
    private decimal purchaseCost;

    public ProductVariant(Guid id, string name, decimal purchaseCost)
    {
        Id = id;
        this.name = name;
        this.purchaseCost = purchaseCost;
    }

    public Guid Id { get; }

    public string Name
    {
        get => name;
        set => Set(ref name, value ?? string.Empty);
    }

    public decimal PurchaseCost { get => purchaseCost; set => Set(ref purchaseCost, value); }

    private bool isArchived;
    public bool IsArchived { get => isArchived; set { Set(ref isArchived, value); Raise(nameof(StatusLabel)); Raise(nameof(IsActive)); } }
    public bool IsActive => !IsArchived;
    public string StatusLabel => IsArchived ? "مؤرشف" : "نشط";
    public ProductVariant Copy() => new(Id, Name, PurchaseCost) { IsArchived = IsArchived };
}

/// <summary>Represents one Rust product category in the established inline category interaction.</summary>
public sealed class ProductCategoryOption : ObservableObject
{
    private string name;
    private bool isArchived;

    public Guid Id { get; init; }
    public long Revision { get; init; }
    public ProductCategoryOption(string name)
    {
        this.name = name;
    }

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
}

/// <summary>Keeps presentation-only details that are not projected in the manager list row.</summary>
public sealed record ProductDraftDetails(
    string Description,
    string Notes,
    ImageSource? Image,
    string ImageName,
    IReadOnlyList<ProductVariant> Variants);
