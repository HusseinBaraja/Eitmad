namespace Eitmad.WindowsShell.Features.Authentication;

/// <summary>Identifies the two transient preview accounts exposed by the shell.</summary>
public enum PreviewAccountRole
{
    Manager,
    Receptionist,
}

/// <summary>Carries the selected preview role without creating an authenticated session.</summary>
public sealed class PreviewSignedInEventArgs(PreviewAccountRole role) : EventArgs
{
    public PreviewAccountRole Role { get; } = role;
}
