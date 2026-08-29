namespace Eitmad.WindowsShell.Features.RawMaterials;

/// <summary>Represents one raw-material row projected for the Windows preview surface.</summary>
public sealed class RawMaterialListItem
{
    public RawMaterialListItem(
        Guid id,
        string name,
        string category,
        string unit,
        decimal currentCost,
        bool isArchived = false)
    {
        Id = id;
        Name = name;
        Category = category;
        Unit = unit;
        CurrentCost = currentCost;
        IsArchived = isArchived;
    }

    public Guid Id { get; }

    public string Name { get; set; }

    public string Category { get; set; }

    public string Unit { get; set; }

    public decimal CurrentCost { get; set; }

    public bool IsArchived { get; set; }

    public bool CanArchive => !IsArchived;

    public string StatusLabel => IsArchived ? "مؤرشفة" : "نشطة";

    public string CostLabel => $"ر.س. {CurrentCost:N0}";
}
