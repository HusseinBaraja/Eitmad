using System.Globalization;

namespace Eitmad.WindowsShell.Features.RawMaterials;

public sealed record RawMaterialListItem(Guid Id, string Name, string Category, string Unit,
    decimal CurrentCost, bool IsArchived, Guid CategoryId, Guid UnitId, long Revision)
{
    public bool CanArchive => !IsArchived;
    public string StatusLabel => IsArchived ? "مؤرشفة" : "نشطة";
    public string CurrencyLabel => "ر.ي";
    public string CostAmountLabel => CurrentCost.ToString("N0", CultureInfo.InvariantCulture);
    public string CostLabel => $"{CurrencyLabel} {CostAmountLabel}";
}
