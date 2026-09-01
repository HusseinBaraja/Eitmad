using System.ComponentModel;
using System.Globalization;
using System.Runtime.CompilerServices;

namespace Eitmad.WindowsShell.Features.Parts;

/// <summary>Describes one selectable raw-material fixture in the transient Parts preview.</summary>
public sealed record PartMaterialOption(Guid Id, string Name, string Unit, decimal UnitCost)
{
    public string UnitCostLabel => UnitCost.ToString("N0", CultureInfo.InvariantCulture);
}

/// <summary>Owns the transient usage amount and calculated cost for one selected material.</summary>
public sealed class PartMaterialUsage : INotifyPropertyChanged
{
    private decimal quantity;

    public PartMaterialUsage(PartMaterialOption material, decimal quantity = 1m)
    {
        Material = material;
        this.quantity = quantity;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public PartMaterialOption Material { get; }

    public decimal Quantity
    {
        get => quantity;
        set
        {
            if (quantity == value)
            {
                return;
            }

            quantity = value;
            Raise();
            Raise(nameof(TotalCost));
            Raise(nameof(TotalCostLabel));
        }
    }

    public decimal TotalCost => decimal.Round(Quantity * Material.UnitCost, 0, MidpointRounding.AwayFromZero);

    public string UnitCostLabel => Material.UnitCost.ToString("N0", CultureInfo.InvariantCulture);

    public string TotalCostLabel => TotalCost.ToString("N0", CultureInfo.InvariantCulture);

    public PartMaterialUsage Copy() => new(Material, Quantity);

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}
