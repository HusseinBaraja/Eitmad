using System.Globalization;

namespace Eitmad.WindowsShell.Features.Quotations;

public enum QuotationStatus
{
    Draft,
    Active,
    WaitingApproval,
    Converted,
    Cancelled,
    Expired,
}

public enum DiscountApprovalDecision
{
    None,
    Approved,
    Rejected,
}

/// <summary>Represents one furniture line in a synthetic manager quotation.</summary>
public sealed record QuotationLineItem(
    string FurnitureName,
    string Variant,
    string Color,
    string Handle,
    int Quantity,
    decimal UnitPrice)
{
    public decimal? EvaluatedTotal { get; init; }
    public bool IsFurniture { get; init; } = true;
    public string Dimensions { get; init; } = "";
    public string ThumbnailKind { get; init; } = "wardrobe";
    public System.Windows.Media.ImageSource? Image { get; init; }
    public decimal Total => EvaluatedTotal ?? checked(Quantity * UnitPrice);

    public string QuantityLabel => Quantity.ToString(CultureInfo.InvariantCulture);

    public string UnitPriceLabel => FormatMoney(UnitPrice);

    public string TotalLabel => FormatMoney(Total);

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} ر.ي";
}

/// <summary>Represents one quotation row and its transient approval preview state.</summary>
public sealed class QuotationListItem : ObservableObject
{
    public Eitmad.Contracts.QuotationDraft? Draft { get; init; }
    public Eitmad.Contracts.DiscountApproval? Approval { get; init; }
    public bool HasDraft => Draft is not null;
    public string DraftNotice => Draft is { } draft ? QuotationDraftClient.SyncLabel(draft) + " — إصدار العرض والطباعة والتحويل غير متاحة بعد." : "";

    public QuotationListItem(
        Guid id,
        string number,
        string customer,
        DateOnly date,
        QuotationStatus status,
        decimal discount,
        IReadOnlyList<QuotationLineItem> items,
        bool requiresDiscountApproval = false,
        string phone = "000000000")
    {
        Phone = phone;
        Id = id;
        Number = number;
        Customer = customer;
        Date = date;
        Status = status;
        Discount = discount;
        Items = items;
        RequiresDiscountApproval = requiresDiscountApproval;
    }

    public string Phone { get; }
    public Guid? CustomerId { get; init; }
    public string Address { get; init; } = "";
    public string Notes { get; init; } = "";

    public bool CanEdit => Draft is { } draft ? draft.SyncState is Eitmad.Contracts.SyncState.Pending or Eitmad.Contracts.SyncState.Confirmed : !HasPendingDiscountApproval && (Status is QuotationStatus.Draft or QuotationStatus.Active or QuotationStatus.WaitingApproval);
    public bool IsWaitingApproval => HasPendingDiscountApproval;
    public bool NeedsApprovalToComplete { get; init; }
    public bool CanPrint => Draft is null && CanEdit && (!NeedsApprovalToComplete && !RequiresDiscountApproval || ApprovalDecision == DiscountApprovalDecision.Approved);
    public string ReceptionActivity { get; init; } = "عينة مستقلة";

    public Guid Id { get; }

    public string Number { get; }

    public string Customer { get; }

    public DateOnly Date { get; }

    public QuotationStatus Status { get; }

    public decimal Discount { get; }

    public IReadOnlyList<QuotationLineItem> Items { get; }

    public bool RequiresDiscountApproval { get; }

    public DiscountApprovalDecision ApprovalDecision => Approval?.State switch {
        Eitmad.Contracts.DiscountApprovalState.Approved => DiscountApprovalDecision.Approved,
        Eitmad.Contracts.DiscountApprovalState.Rejected => DiscountApprovalDecision.Rejected,
        _ => DiscountApprovalDecision.None,
    };

    public decimal Subtotal => Draft?.Snapshot.Evaluation.Totals.SubtotalYer ?? Items.Sum(item => item.Total);

    public decimal FinalTotal => Draft?.Snapshot.Evaluation.Totals.TotalYer ?? Subtotal - Discount;

    public string DiscountPercentLabel => DiscountPercent.ToString("0.##", CultureInfo.InvariantCulture) + "%";
    public string DiscountAmountLabel => Discount.ToString("N0", CultureInfo.InvariantCulture);

    public decimal DiscountPercent => Draft is { } draft ? draft.Snapshot.Intent.DiscountBasisPoints / 100m : Subtotal == 0m ? 0m : decimal.Round(Discount / Subtotal * 100m, 1);

    public bool HasPendingDiscountApproval => Approval?.State == Eitmad.Contracts.DiscountApprovalState.Pending;

    public bool HasApprovalDecision => Approval is not null && Approval.State != Eitmad.Contracts.DiscountApprovalState.Pending;

    public bool IsDraft => Status == QuotationStatus.Draft;

    public bool IsActive => Status == QuotationStatus.Active;

    public bool IsConverted => Status == QuotationStatus.Converted;

    public string DateLabel => Date.ToString("yyyy/MM/dd", CultureInfo.InvariantCulture);

    public string SubtotalLabel => FormatMoney(Subtotal);

    public string DiscountLabel => Discount == 0m ? "—" : FormatMoney(Discount);

    public string DiscountDetailLabel => $"{FormatMoney(Discount)} · {DiscountPercent.ToString("0.#", CultureInfo.InvariantCulture)}%";

    public string FinalTotalLabel => FormatMoney(FinalTotal);

    public string StatusLabel => Approval is not null ? QuotationDraftClient.ApprovalLabel(Approval) : HasApprovalDecision ? (ApprovalDecision == DiscountApprovalDecision.Approved ? "الخصم مقبول" : "الخصم مرفوض") : HasPendingDiscountApproval ? "بانتظار الموافقة" : Status switch
    {
        QuotationStatus.Draft => "مسودة",
        QuotationStatus.Active => "نشط",
        QuotationStatus.WaitingApproval => "بانتظار الموافقة",
        QuotationStatus.Converted => "محوّل",
        QuotationStatus.Cancelled => "ملغي",
        QuotationStatus.Expired => "منتهي",
        _ => throw new InvalidOperationException("Unsupported quotation status."),
    };

    public string ApprovalDecisionLabel => QuotationDraftClient.ApprovalLabel(Approval);

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} ر.ي";
}
