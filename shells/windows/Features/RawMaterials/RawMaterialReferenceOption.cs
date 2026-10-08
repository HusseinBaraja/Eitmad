using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.RawMaterials;

public sealed record RawMaterialReferenceOption(string Name, Guid Id, long Revision,
    string ShortName = "", UnitDimension Dimension = UnitDimension.Count,
    long Numerator = 1, long Denominator = 1, bool IsArchived = false)
{
    public bool CanArchive => !IsArchived;
    public string StatusLabel => IsArchived ? "مؤرشفة" : string.Empty;
    public string DisplayLabel => (string.IsNullOrEmpty(ShortName)
        || string.Equals(Name, ShortName, StringComparison.CurrentCultureIgnoreCase)
            ? Name : $"{Name} — {ShortName}") + (IsArchived ? " — مؤرشف" : string.Empty);
}
