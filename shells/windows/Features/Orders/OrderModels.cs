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

public sealed record OrderLineItem(
    string Product,
    string Variant,
    string Dimensions,
    string Color,
    string Handle,
    int Quantity,
    decimal SellingPrice,
    bool IsFurniture = true,
    string ThumbnailKind = "Wardrobe",
    System.Windows.Media.ImageSource? Image = null)
{
    public decimal? EvaluatedTotal { get; init; }
    public decimal Total => EvaluatedTotal ?? checked(Quantity * SellingPrice);

    public string QuantityLabel => Quantity.ToString(CultureInfo.InvariantCulture);

    public string SellingPriceLabel => FormatMoney(SellingPrice);

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} ر.ي";
}

public sealed record OrderListItem(
    Guid Id,
    string Number,
    string Customer,
    DateOnly Date,
    OrderStatus Status,
    decimal Discount,
    IReadOnlyList<OrderLineItem> Items,
    string Phone = "",
    Features.Quotations.QuotationListItem? OriginalQuotation = null)
{
    public Eitmad.Contracts.OrderRecord? Record { get; init; }
    public bool CanCancel => Record?.PermittedActions.Contains(Eitmad.Contracts.OrderPermittedAction.Cancel) == true;
    public bool CanEditFulfillment => Record?.PermittedActions.Contains(Eitmad.Contracts.OrderPermittedAction.EditFulfillment) == true;
    public bool CanDeliver => Record?.PermittedActions.Contains(Eitmad.Contracts.OrderPermittedAction.Deliver) == true;
    public bool CanStartWork => Record?.PermittedActions.Contains(Eitmad.Contracts.OrderPermittedAction.StartWork) == true;
    public bool CanCompleteWork => Record?.PermittedActions.Contains(Eitmad.Contracts.OrderPermittedAction.CompleteWork) == true;
    public string FulfillmentLabel => Record?.FulfillmentNote ?? "";
    public string WorkSummary => Record is null ? "" : string.Join(" · ", Record.Work.Select(w => w.Number + " / " + (w.State switch { Eitmad.Contracts.WorkState.Planned => "مخطط", Eitmad.Contracts.WorkState.InProgress => "قيد التنفيذ", Eitmad.Contracts.WorkState.Completed => "مكتمل", _ => "ملغي" })));
    public Guid? CustomerId { get; init; }
    public string ReadyFromWorkOrder { get; init; } = "";
    public bool IsNewlyReady => ReadyFromWorkOrder.Length > 0;
    public string ReadyNotice => "جاهز حديثاً — اكتمل التصنيع. راجع الطلب للتواصل مع العميل. معاينة فقط.";
    public bool CanShowProduction => Record is null && Items.Any(item => item.IsFurniture) && Status is not OrderStatus.Cancelled and not OrderStatus.Delivered;
    public bool HasOriginalQuotation => OriginalQuotation is not null;

    public decimal Subtotal => Record?.Source.Quotation.Evaluation.Totals.SubtotalYer ?? Items.Sum(item => item.Total);

    public decimal FinalTotal => Record?.Source.Quotation.Evaluation.Totals.TotalYer ?? Subtotal - Discount;

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
        OrderStatus.New => "مؤكد",
        OrderStatus.InProduction => "قيد الإنتاج",
        OrderStatus.Ready => "جاهز",
        OrderStatus.Delivered => "تم التسليم",
        OrderStatus.Cancelled => "ملغي",
        _ => throw new InvalidOperationException("Unsupported order status."),
    };

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} ر.ي";
}
