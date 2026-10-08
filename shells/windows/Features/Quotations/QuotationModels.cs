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
    public Eitmad.Contracts.QuotationRecord? Lifecycle { get; init; }
    public bool Permits(Eitmad.Contracts.QuotationPermittedAction action) => (Lifecycle?.PermittedActions ?? Draft?.PermittedActions ?? []).Contains(action);
    public bool CanIssue => Permits(Eitmad.Contracts.QuotationPermittedAction.Issue);
    public bool CanRevise => Permits(Eitmad.Contracts.QuotationPermittedAction.Revise);
    public bool CanManageValidity => Permits(Eitmad.Contracts.QuotationPermittedAction.ManageValidity);
    public bool CanCancel => Permits(Eitmad.Contracts.QuotationPermittedAction.Cancel);
    public bool IsPreview => Draft is null && Lifecycle is null;
    public bool ShowValidityInput => CanManageValidity || CanRevise;
    public bool ShowCancellationReason => CanCancel && Lifecycle?.Number is not null;
    public bool CanPrintPreview => IsPreview && CanPrint;
    public bool CanPrintConfirmed => Lifecycle is not null && CanPrint;
    public bool CanAccept => Permits(Eitmad.Contracts.QuotationPermittedAction.Accept);
    public bool CanConvert => Permits(Eitmad.Contracts.QuotationPermittedAction.Convert) || Draft is null && Lifecycle is null && CanPrint;
    public string ValidityLabel => Lifecycle is { } q ? $"الإصدار {q.DocumentRevision} · الصلاحية {q.ValidityDays} يوماً" + (q.ValidUntil is { } until ? " · ينتهي " + DateTimeOffset.FromUnixTimeMilliseconds(until).ToOffset(TimeSpan.FromHours(3)).ToString("yyyy/MM/dd", CultureInfo.InvariantCulture) : "") : "";
    public Eitmad.Contracts.QuotationDraft? Draft { get; init; }
    public Eitmad.Contracts.DiscountApproval? Approval { get; init; }
    public bool HasDraft => Draft is not null;
    public string DraftNotice => Draft is { } draft ? Lifecycle is null ? QuotationDraftClient.SyncLabel(draft) : ValidityLabel : "";

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

    public bool CanEdit => Draft is not null || Lifecycle is not null ? Permits(Eitmad.Contracts.QuotationPermittedAction.Edit) : !HasPendingDiscountApproval && (Status is QuotationStatus.Draft or QuotationStatus.Active or QuotationStatus.WaitingApproval);
    public bool IsWaitingApproval => HasPendingDiscountApproval;
    public bool NeedsApprovalToComplete { get; init; }
    public bool CanPrint => Lifecycle is not null ? Lifecycle.Number is not null && Permits(Eitmad.Contracts.QuotationPermittedAction.Print) : Draft is null && CanEdit && (!NeedsApprovalToComplete && !RequiresDiscountApproval || ApprovalDecision == DiscountApprovalDecision.Approved);
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

    public bool HasPendingDiscountApproval => Approval?.State == Eitmad.Contracts.DiscountApprovalState.Pending && (Lifecycle is null || Lifecycle.State is Eitmad.Contracts.QuotationState.Draft or Eitmad.Contracts.QuotationState.PendingApproval);

    public bool HasApprovalDecision => Approval is not null && Approval.State != Eitmad.Contracts.DiscountApprovalState.Pending;

    public bool IsDraft => Status == QuotationStatus.Draft;

    public bool IsActive => Status == QuotationStatus.Active;

    public bool IsConverted => Status == QuotationStatus.Converted;

    public string DateLabel => Date.ToString("yyyy/MM/dd", CultureInfo.InvariantCulture);

    public string SubtotalLabel => FormatMoney(Subtotal);

    public string DiscountLabel => Discount == 0m ? "—" : FormatMoney(Discount);

    public string DiscountDetailLabel => $"{FormatMoney(Discount)} · {DiscountPercent.ToString("0.#", CultureInfo.InvariantCulture)}%";

    public string FinalTotalLabel => FormatMoney(FinalTotal);

    public string StatusLabel => Lifecycle is not null ? Lifecycle.State switch { Eitmad.Contracts.QuotationState.Accepted => "مقبول", Eitmad.Contracts.QuotationState.Converted => "محوّل", Eitmad.Contracts.QuotationState.Issued => "صادر", Eitmad.Contracts.QuotationState.Expired => "منتهي", Eitmad.Contracts.QuotationState.Cancelled => "ملغي", Eitmad.Contracts.QuotationState.PendingApproval => "بانتظار الموافقة", _ => "مسودة" } : Approval is not null ? QuotationDraftClient.ApprovalLabel(Approval) : HasApprovalDecision ? (ApprovalDecision == DiscountApprovalDecision.Approved ? "الخصم مقبول" : "الخصم مرفوض") : HasPendingDiscountApproval ? "بانتظار الموافقة" : Status switch
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
