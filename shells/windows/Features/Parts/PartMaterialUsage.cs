using System.Globalization;
using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.Parts;

public sealed record PartUnitOption(MaterialUnit Record)
{
    public string Name => Record.Symbol;
}

/// <summary>Projects one Rust material and its selectable units.</summary>
public sealed record PartMaterialOption(Material Record, MaterialUnit UnitRecord, IReadOnlyList<PartUnitOption> Units)
{
    public Guid Id => Record.Id;
    public string Name => Record.Name;
    public string Unit => UnitRecord.Symbol;
    public long UnitCost => Record.CurrentCostYer;
    public string UnitCostLabel => $"{Record.CurrentCostYer.ToString("N0", CultureInfo.InvariantCulture)} / {Unit}";
}

/// <summary>Keeps unsaved input and displays Rust-calculated costs.</summary>
public sealed class PartMaterialUsage : ObservableObject
{
    private string quantity;
    private PartUnitOption? selectedUnit;
    private long? cost;
    public PartMaterialUsage(PartMaterialOption material, string quantity = "1", Guid? unitId = null)
    {
        Material = material;
        this.quantity = quantity;
        selectedUnit = material.Units.FirstOrDefault(u => u.Record.Id == (unitId ?? material.UnitRecord.Id));
    }
    public PartMaterialOption Material { get; private set; }
    public string Quantity { get => quantity; set { if (Set(ref quantity, value)) SetCost(null); } }
    public IReadOnlyList<PartUnitOption> Units => Material.Units;
    public PartUnitOption? SelectedUnit { get => selectedUnit; set { if (Set(ref selectedUnit, value)) SetCost(null); } }
    public string UnitCostLabel => Material.UnitCostLabel;
    public long? TotalCost => cost;
    public string TotalCostLabel => cost?.ToString("N0", CultureInfo.InvariantCulture) ?? "—";
    public void SetCost(long? value) { cost = value; Raise(nameof(TotalCost)); Raise(nameof(TotalCostLabel)); }
    public void RefreshReference(PartMaterialOption material)
    {
        var id = SelectedUnit?.Record.Id;
        Material = material;
        selectedUnit = material.Units.FirstOrDefault(u => u.Record.Id == id);
        Raise(nameof(Material)); Raise(nameof(Units)); Raise(nameof(SelectedUnit)); Raise(nameof(UnitCostLabel));
    }
    public PartUsage ToInput() => new()
    {
        MaterialId = Material.Id, MaterialRevision = Material.Record.Revision,
        UnitId = SelectedUnit?.Record.Id ?? Guid.Empty, UnitRevision = SelectedUnit?.Record.Revision ?? 0,
        Quantity = Quantity,
    };
}
