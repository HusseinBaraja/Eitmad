using System.Globalization;

namespace Eitmad.WindowsShell.Features.Products;

public sealed record ProductListItem(
    Guid Id,
    string Name,
    string Category,
    decimal PurchaseCost,
    string VariantSummary,
    string ThumbnailKind,
    bool IsArchived,
    bool HasPurchaseCost) : System.ComponentModel.INotifyPropertyChanged
{
    private System.Windows.Media.ImageSource? image;
    public System.Windows.Media.ImageSource? Image { get => image; set { image = value; PropertyChanged?.Invoke(this, new(nameof(Image))); } }
    public event System.ComponentModel.PropertyChangedEventHandler? PropertyChanged;

    public bool CanArchive => !IsArchived;
    public string PurchaseCostLabel => !HasPurchaseCost ? "—" : PurchaseCost.ToString("N0", CultureInfo.InvariantCulture);
    public string StatusLabel => IsArchived ? "مؤرشف" : "نشط";
}

/// <summary>Represents one supplier-defined ready-made option and its purchase cost.</summary>
public sealed class ProductVariant : ObservableObject
{
    private string name;
    private decimal purchaseCost;

    /// <summary>Stages a supplier option with its stable identity and whole-YER purchase cost.</summary>
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
}

public sealed record ProductCategoryOption(Guid Id, long Revision, string Name, bool IsArchived)
{
    public bool CanArchive => !IsArchived;
    public string StatusLabel => IsArchived ? "مؤرشفة" : string.Empty;
}
