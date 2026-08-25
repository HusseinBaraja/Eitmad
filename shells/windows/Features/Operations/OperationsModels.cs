using Eitmad.Contracts;

namespace Eitmad.WindowsShell.Features.Operations;

public sealed record StatusCard(string Title, string Value, string Detail, string Tone, double Progress = 0);

public sealed record ConfigItem(
    string Key,
    string Label,
    string Value,
    ConfigReadValueKind Kind,
    ConfigSensitivity Sensitivity,
    RestartRequirement RestartRequirement);

public sealed record JobItem(Guid Id, string Title, string State, string Detail, double Progress, string Tone);

public sealed record ActivityItem(Guid Id, string Title, string Detail, string Time, string Tone, string Kind);

public sealed class EventOrderGate
{
    private readonly Dictionary<string, (Guid SubscriptionId, long Sequence)> cursors = [];

    public bool TryAccept(string stream, EventEnvelope delivered)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(stream);
        ArgumentNullException.ThrowIfNull(delivered);
        if (cursors.TryGetValue(stream, out var current)
            && current.SubscriptionId == delivered.SubscriptionId
            && delivered.Sequence <= current.Sequence)
        {
            return false;
        }

        cursors[stream] = (delivered.SubscriptionId, delivered.Sequence);
        return true;
    }

    public void Reset(string stream) => cursors.Remove(stream);

    public void ResetAll() => cursors.Clear();
}
