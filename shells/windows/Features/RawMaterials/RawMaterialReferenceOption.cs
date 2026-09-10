namespace Eitmad.WindowsShell.Features.RawMaterials;

/// <summary>Represents an ephemeral category or unit shown by the raw-material preview.</summary>
public sealed class RawMaterialReferenceOption : ObservableObject
{
    private string name;
    private string shortName;
    private bool isArchived;

    public RawMaterialReferenceOption(string name, string shortName = "")
    {
        this.name = name;
        this.shortName = shortName;
    }

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
            }
        }
    }

    public bool CanArchive => !IsArchived;

    public string DisplayLabel => string.IsNullOrEmpty(ShortName)
        || string.Equals(Name, ShortName, StringComparison.CurrentCultureIgnoreCase)
            ? Name
            : $"{Name} — {ShortName}";

    public string StatusLabel => IsArchived ? "مؤرشفة" : string.Empty;
}
