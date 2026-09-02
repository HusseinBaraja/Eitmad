using System.Globalization;

namespace Eitmad.WindowsShell.Features.Orders;

public enum OrderStatus
{
    New,
    InProduction,
    Ready,
    Delivered,
    Cancelled,
}

/// <summary>Represents one furniture line in a synthetic manager order.</summary>
public sealed record OrderLineItem(
    string Product,
    string Variant,
    string Dimensions,
    string Color,
    string Handle,
    int Quantity,
    decimal SellingPrice)
{
    public decimal Total => checked(Quantity * SellingPrice);

    public string QuantityLabel => Quantity.ToString(CultureInfo.InvariantCulture);

    public string SellingPriceLabel => FormatMoney(SellingPrice);

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} YER";
}

/// <summary>Represents one read-only synthetic order for the manager preview.</summary>
public sealed record OrderListItem(
    Guid Id,
    string Number,
    string Customer,
    DateOnly Date,
    OrderStatus Status,
    decimal Discount,
    IReadOnlyList<OrderLineItem> Items)
{
    public decimal Subtotal => Items.Sum(item => item.Total);

    public decimal FinalTotal => Subtotal - Discount;

    public bool IsNew => Status == OrderStatus.New;

    public bool IsInProduction => Status == OrderStatus.InProduction;

    public bool IsReady => Status == OrderStatus.Ready;

    public bool IsDelivered => Status == OrderStatus.Delivered;

    public bool IsCancelled => Status == OrderStatus.Cancelled;

    public string DateLabel => Date.ToString("yyyy/MM/dd", CultureInfo.InvariantCulture);

    public string SubtotalLabel => FormatMoney(Subtotal);

    public string DiscountLabel => Discount == 0m ? "—" : FormatMoney(Discount);

    public string FinalTotalLabel => FormatMoney(FinalTotal);

    public string StatusLabel => Status switch
    {
        OrderStatus.New => "جديد",
        OrderStatus.InProduction => "قيد الإنتاج",
        OrderStatus.Ready => "جاهز",
        OrderStatus.Delivered => "تم التسليم",
        OrderStatus.Cancelled => "ملغي",
        _ => throw new InvalidOperationException("Unsupported order status."),
    };

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} YER";
}
