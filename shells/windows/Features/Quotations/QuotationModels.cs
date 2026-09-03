using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;

namespace Eitmad.WindowsShell.Features.Quotations;

public enum QuotationStatus
{
    Draft,
    Active,
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
    public decimal Total => checked(Quantity * UnitPrice);

    public string QuantityLabel => Quantity.ToString(CultureInfo.InvariantCulture);

    public string UnitPriceLabel => FormatMoney(UnitPrice);

    public string TotalLabel => FormatMoney(Total);

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} YER";
}

/// <summary>Represents one quotation row and its transient approval preview state.</summary>
public sealed class QuotationListItem : INotifyPropertyChanged
{
    private DiscountApprovalDecision approvalDecision;

    public QuotationListItem(
        Guid id,
        string number,
        string customer,
        DateOnly date,
        QuotationStatus status,
        decimal discount,
        IReadOnlyList<QuotationLineItem> items,
        bool requiresDiscountApproval = false)
    {
        Id = id;
        Number = number;
        Customer = customer;
        Date = date;
        Status = status;
        Discount = discount;
        Items = items;
        RequiresDiscountApproval = requiresDiscountApproval;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public Guid Id { get; }

    public string Number { get; }

    public string Customer { get; }

    public DateOnly Date { get; }

    public QuotationStatus Status { get; }

    public decimal Discount { get; }

    public IReadOnlyList<QuotationLineItem> Items { get; }

    public bool RequiresDiscountApproval { get; }

    public DiscountApprovalDecision ApprovalDecision
    {
        get => approvalDecision;
        private set
        {
            if (approvalDecision == value)
            {
                return;
            }

            approvalDecision = value;
            Raise();
            Raise(nameof(HasPendingDiscountApproval));
            Raise(nameof(HasApprovalDecision));
            Raise(nameof(ApprovalDecisionLabel));
        }
    }

    public decimal Subtotal => Items.Sum(item => item.Total);

    public decimal FinalTotal => Subtotal - Discount;

    public decimal DiscountPercent => Subtotal == 0m ? 0m : decimal.Round(Discount / Subtotal * 100m, 1);

    public bool HasPendingDiscountApproval => RequiresDiscountApproval && ApprovalDecision == DiscountApprovalDecision.None;

    public bool HasApprovalDecision => ApprovalDecision != DiscountApprovalDecision.None;

    public bool IsDraft => Status == QuotationStatus.Draft;

    public bool IsActive => Status == QuotationStatus.Active;

    public bool IsConverted => Status == QuotationStatus.Converted;

    public string DateLabel => Date.ToString("yyyy/MM/dd", CultureInfo.InvariantCulture);

    public string SubtotalLabel => FormatMoney(Subtotal);

    public string DiscountLabel => Discount == 0m ? "—" : FormatMoney(Discount);

    public string DiscountDetailLabel => $"{FormatMoney(Discount)} · {DiscountPercent.ToString("0.#", CultureInfo.InvariantCulture)}%";

    public string FinalTotalLabel => FormatMoney(FinalTotal);

    public string StatusLabel => Status switch
    {
        QuotationStatus.Draft => "مسودة",
        QuotationStatus.Active => "نشط",
        QuotationStatus.Converted => "محوّل",
        QuotationStatus.Cancelled => "ملغي",
        QuotationStatus.Expired => "منتهي",
        _ => throw new InvalidOperationException("Unsupported quotation status."),
    };

    public string ApprovalDecisionLabel => ApprovalDecision switch
    {
        DiscountApprovalDecision.Approved => "تمت معاينة الموافقة على الخصم محلياً.",
        DiscountApprovalDecision.Rejected => "تمت معاينة رفض الخصم محلياً.",
        _ => string.Empty,
    };

    public void DecideDiscount(DiscountApprovalDecision decision)
    {
        if (!RequiresDiscountApproval || decision == DiscountApprovalDecision.None)
        {
            return;
        }

        ApprovalDecision = decision;
    }

    private static string FormatMoney(decimal value) => $"{value.ToString("N0", CultureInfo.InvariantCulture)} YER";

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}
