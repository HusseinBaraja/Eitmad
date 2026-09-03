using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;

namespace Eitmad.WindowsShell.Features.WorkOrders;

public enum WorkOrderStatus
{
    New,
    InProgress,
    Completed,
    Cancelled,
}

public enum FurnitureIllustration
{
    Wardrobe,
    Table,
    Bed,
}

/// <summary>Represents one required Part in the transient work-order preview.</summary>
public sealed record WorkOrderPart(string Name, int Quantity)
{
    public string QuantityLabel => Quantity.ToString(CultureInfo.InvariantCulture);
}

/// <summary>Represents one furniture specification in the transient work-order preview.</summary>
public sealed record WorkOrderFurnitureItem(
    string Name,
    string Variant,
    string Dimensions,
    string Color,
    string Handle,
    int Quantity,
    FurnitureIllustration Illustration)
{
    public string QuantityLabel => Quantity.ToString(CultureInfo.InvariantCulture);

    public bool IsWardrobe => Illustration == FurnitureIllustration.Wardrobe;

    public bool IsTable => Illustration == FurnitureIllustration.Table;

    public bool IsBed => Illustration == FurnitureIllustration.Bed;
}

/// <summary>Represents one synthetic manager work order and its observable preview status.</summary>
public sealed class WorkOrderListItem : INotifyPropertyChanged
{
    private static readonly string[] ArabicMonths =
    [
        "يناير", "فبراير", "مارس", "أبريل", "مايو", "يونيو",
        "يوليو", "أغسطس", "سبتمبر", "أكتوبر", "نوفمبر", "ديسمبر",
    ];

    private WorkOrderStatus status;

    public WorkOrderListItem(
        Guid id,
        string number,
        string orderNumber,
        string customer,
        string assignedTo,
        DateOnly dueDate,
        WorkOrderStatus status,
        IReadOnlyList<WorkOrderFurnitureItem> furniture,
        IReadOnlyList<WorkOrderPart> parts,
        string notes)
    {
        Id = id;
        Number = number;
        OrderNumber = orderNumber;
        Customer = customer;
        AssignedTo = assignedTo;
        DueDate = dueDate;
        this.status = status;
        Furniture = furniture;
        Parts = parts;
        Notes = notes;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public Guid Id { get; }

    public string Number { get; }

    public string OrderNumber { get; }

    public string Customer { get; }

    public string AssignedTo { get; }

    public DateOnly DueDate { get; }

    public IReadOnlyList<WorkOrderFurnitureItem> Furniture { get; }

    public IReadOnlyList<WorkOrderPart> Parts { get; }

    public string Notes { get; }

    public WorkOrderStatus Status
    {
        get => status;
        private set
        {
            if (status == value)
            {
                return;
            }

            status = value;
            Raise();
            Raise(nameof(StatusLabel));
            Raise(nameof(IsNew));
            Raise(nameof(IsInProgress));
            Raise(nameof(IsCompleted));
            Raise(nameof(IsCancelled));
            Raise(nameof(CanAdvance));
            Raise(nameof(NextStatusActionLabel));
        }
    }

    public string CustomerOrderLabel => $"الطلب {OrderNumber} — {Customer}";

    public string DetailNumberLabel => $"#{Number[3..]}";

    public string FurnitureSummary => Furniture.Count == 1
        ? Furniture[0].Name
        : $"{Furniture[0].Name} +{(Furniture.Count - 1).ToString(CultureInfo.InvariantCulture)}";

    public string QuantityLabel => Furniture.Sum(item => item.Quantity).ToString(CultureInfo.InvariantCulture);

    public string DueDateLabel => $"{DueDate.Day.ToString(CultureInfo.InvariantCulture)} {ArabicMonths[DueDate.Month - 1]} {DueDate.Year.ToString(CultureInfo.InvariantCulture)}";

    public string StatusLabel => Status switch
    {
        WorkOrderStatus.New => "جديد",
        WorkOrderStatus.InProgress => "قيد التنفيذ",
        WorkOrderStatus.Completed => "مكتمل",
        WorkOrderStatus.Cancelled => "ملغي",
        _ => throw new InvalidOperationException("Unsupported work-order status."),
    };

    public bool IsNew => Status == WorkOrderStatus.New;

    public bool IsInProgress => Status == WorkOrderStatus.InProgress;

    public bool IsCompleted => Status == WorkOrderStatus.Completed;

    public bool IsCancelled => Status == WorkOrderStatus.Cancelled;

    public bool CanAdvance => Status is WorkOrderStatus.New or WorkOrderStatus.InProgress;

    public string NextStatusActionLabel => Status switch
    {
        WorkOrderStatus.New => "بدء التنفيذ",
        WorkOrderStatus.InProgress => "تحديد كمكتمل",
        WorkOrderStatus.Completed => "اكتمل أمر العمل",
        WorkOrderStatus.Cancelled => "أمر العمل ملغي",
        _ => throw new InvalidOperationException("Unsupported work-order status."),
    };

    public bool AdvanceStatus()
    {
        var next = Status switch
        {
            WorkOrderStatus.New => WorkOrderStatus.InProgress,
            WorkOrderStatus.InProgress => WorkOrderStatus.Completed,
            _ => Status,
        };

        if (next == Status)
        {
            return false;
        }

        Status = next;
        return true;
    }

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}
