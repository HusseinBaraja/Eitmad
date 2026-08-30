using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace Eitmad.WindowsShell.Features.RawMaterials;

/// <summary>Represents an ephemeral category or unit shown by the raw-material preview.</summary>
public sealed class RawMaterialReferenceOption : INotifyPropertyChanged
{
    private string name;
    private string shortName;
    private bool isArchived;

    public RawMaterialReferenceOption(string name, string shortName = "")
    {
        this.name = name;
        this.shortName = shortName;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

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

    private bool Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
        {
            return false;
        }

        field = value;
        Raise(propertyName);
        return true;
    }

    private void Raise([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}
