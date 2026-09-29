using System.Globalization;

namespace Eitmad.WindowsShell.Features.RawMaterials;

/// <summary>Projects one raw-material row for the native list.</summary>
public sealed class RawMaterialListItem
{
    public RawMaterialListItem(
        Guid id,
        string name,
        string category,
        string unit,
        decimal currentCost,
        bool isArchived = false,
        Guid? categoryId = null,
        Guid? unitId = null,
        long? revision = null)
    {
        Id = id;
        Name = name;
        Category = category;
        Unit = unit;
        CurrentCost = currentCost;
        IsArchived = isArchived;
        CategoryId = categoryId;
        UnitId = unitId;
        Revision = revision;
    }

    public Guid Id { get; }
    public Guid? CategoryId { get; }
    public Guid? UnitId { get; }
    public long? Revision { get; }

    public string Name { get; set; }

    public string Category { get; set; }

    public string Unit { get; set; }

    public decimal CurrentCost { get; set; }

    public bool IsArchived { get; set; }

    public bool CanArchive => !IsArchived;

    public string StatusLabel => IsArchived ? "مؤرشفة" : "نشطة";

    public string CurrencyLabel => "ر.ي";

    public string CostAmountLabel => CurrentCost.ToString("N0", CultureInfo.InvariantCulture);

    public string CostLabel => $"{CurrencyLabel} {CostAmountLabel}";
}
