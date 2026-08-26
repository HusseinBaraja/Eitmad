using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.LocalIpc;

public static class EngineContractCodec
{
    public static Event DecodeEvent(EventEnvelope delivered)
    {
        ArgumentNullException.ThrowIfNull(delivered);
        if (!delivered.Event.TryGetValue("kind", out var kindValue))
        {
            throw new InvalidDataException("The engine emitted an event without a kind.");
        }
        var kind = kindValue switch
        {
            string text => text,
            JsonElement { ValueKind: JsonValueKind.String } element => element.GetString(),
            _ => null,
        };
        if (string.IsNullOrWhiteSpace(kind))
        {
            throw new InvalidDataException("The engine emitted an invalid event kind.");
        }
        delivered.Event.TryGetValue("payload", out var payload);
        return new Event { Kind = kind, Payload = payload };
    }
}
