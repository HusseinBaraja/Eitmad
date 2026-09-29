namespace Eitmad.WindowsShell.Features.RawMaterials;

/// <summary>Projects a stable category or unit reference.</summary>
public sealed class RawMaterialReferenceOption : ObservableObject
{
    private string name;
    private string shortName;
    private bool isArchived;

    public RawMaterialReferenceOption(string name, string shortName = "", Guid? id = null,
        long? revision = null, Eitmad.Contracts.UnitDimension dimension = Eitmad.Contracts.UnitDimension.Count,
        long numerator = 1, long denominator = 1)
    {
        this.name = name;
        this.shortName = shortName;
        Id = id;
        Revision = revision;
        Dimension = dimension;
        Numerator = numerator;
        Denominator = denominator;
    }

    public Guid? Id { get; }
    public long? Revision { get; }
    public Eitmad.Contracts.UnitDimension Dimension { get; }
    public long Numerator { get; }
    public long Denominator { get; }

    public string Name
    {
        get => name;
        internal set
        {
            if (Set(ref name, value))
            {
                Raise(nameof(DisplayLabel));
            }
        }
    }

    public string ShortName
    {
        get => shortName;
        internal set
        {
            if (Set(ref shortName, value))
            {
                Raise(nameof(DisplayLabel));
            }
        }
    }

    public bool IsArchived
    {
        get => isArchived;
        internal set
        {
            if (Set(ref isArchived, value))
            {
                Raise(nameof(CanArchive));
                Raise(nameof(StatusLabel));
                Raise(nameof(DisplayLabel));
            }
        }
    }

    public bool CanArchive => !IsArchived;

    public string DisplayLabel => (string.IsNullOrEmpty(ShortName)
        || string.Equals(Name, ShortName, StringComparison.CurrentCultureIgnoreCase)
            ? Name
            : $"{Name} — {ShortName}") + (IsArchived ? " — مؤرشف" : string.Empty);

    public string StatusLabel => IsArchived ? "مؤرشفة" : string.Empty;
}
