using System.Globalization;

namespace Eitmad.WindowsShell.Features.Parts;

public sealed record PartListItem(Guid Id, string Name, string Category, decimal Cost, bool IsArchived)
{
    public bool CanArchive => !IsArchived;

    public string StatusLabel => IsArchived ? "مؤرشف" : "نشط";

    public string CurrencyLabel => "ر.ي";

    public string CostAmountLabel => Cost.ToString("N0", CultureInfo.InvariantCulture);

    public string CostLabel => $"{CostAmountLabel} {CurrencyLabel}";
}
