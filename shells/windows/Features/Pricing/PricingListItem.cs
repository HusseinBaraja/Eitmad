using System.Globalization;
using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.Pricing;

public sealed class PricingListItem(PriceItem record)
{
    public PriceItem Record { get; } = record;
    public Guid Id { get; } = TargetId(record);
    public string Product => Record.Name;
    public string Variant => Record.VariantName;
    public string Category => Record.CategoryName;
    public long? Cost => Record.CostYer;
    public long? SellingPrice => Record.Published?.SellingPriceYer;
    public long? Margin => Record.MarginYer;
    public bool HasNegativeMargin => Margin < 0;
    public bool IsActive => Record.Published is not null;
    public string CostLabel => FormatMoney(Cost);
    public string SellingPriceLabel => FormatMoney(SellingPrice);
    public string MarginLabel => FormatMoney(Margin);
    public string StatusLabel => Record.PublicationRequired ? "بانتظار النشر" : "منشور";
    public static string FormatMoney(long? value) => value.HasValue ? $"{value.Value.ToString("N0", CultureInfo.InvariantCulture)} ر.ي" : "—";
    private static Guid TargetId(PriceItem item)
    {
        var target = JsonSerializer.Deserialize<PriceTarget>(JsonSerializer.Serialize(item.Target));
        return target?.AsProduct()?.VariantId ?? target?.AsFurniture()?.VariantId ?? Guid.Empty;
    }
}
