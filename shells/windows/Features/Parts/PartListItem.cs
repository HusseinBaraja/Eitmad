using System.Globalization;

namespace Eitmad.WindowsShell.Features.Parts;

/// <summary>Represents one part row projected for the Windows surface.</summary>
public sealed class PartListItem
{
    public PartListItem(
        Guid id,
        string name,
        string category,
        decimal cost,
        int usedInCount,
        bool isArchived = false)
    {
        Id = id;
        Name = name;
        Category = category;
        Cost = cost;
        UsedInCount = usedInCount;
        IsArchived = isArchived;
    }

    public Guid Id { get; }

    public string Name { get; set; }

    public string Category { get; set; }

    public decimal Cost { get; set; }

    public int UsedInCount { get; set; }

    public bool IsArchived { get; set; }

    public bool CanArchive => !IsArchived;

    public string StatusLabel => IsArchived ? "مؤرشف" : "نشط";

    public string CurrencyLabel => "ر.ي";

    public string CostAmountLabel => Cost.ToString("N0", CultureInfo.InvariantCulture);

    public string CostLabel => $"{CostAmountLabel} {CurrencyLabel}";

    public string UsedInLabel => "—";
}
