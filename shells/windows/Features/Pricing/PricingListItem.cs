using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;

namespace Eitmad.WindowsShell.Features.Pricing;

/// <summary>Represents one product variant in the transient pricing preview.</summary>
public sealed class PricingListItem : INotifyPropertyChanged
{
    private decimal sellingPrice;

    public PricingListItem(
        Guid id,
        string product,
        string variant,
        string category,
        decimal cost,
        decimal sellingPrice,
        bool isActive = true)
    {
        Id = id;
        Product = product;
        Variant = variant;
        Category = category;
        Cost = cost;
        this.sellingPrice = sellingPrice;
        IsActive = isActive;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public Guid Id { get; }

    public string Product { get; }

    public string Variant { get; }

    public string Category { get; }

    public decimal Cost { get; }

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
            Raise();
            Raise(nameof(SellingPriceLabel));
            Raise(nameof(Margin));
            Raise(nameof(MarginLabel));
            Raise(nameof(HasNegativeMargin));
        }
    }

    public decimal Margin => SellingPrice - Cost;

    public bool HasNegativeMargin => Margin < 0m;

    public bool IsActive { get; }

    public string CostLabel => FormatMoney(Cost);

    public string SellingPriceLabel => FormatMoney(SellingPrice);

    public string MarginLabel => FormatMoney(Margin);

    public string StatusLabel => IsActive ? "نشط" : "غير نشط";

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} YER";

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}
