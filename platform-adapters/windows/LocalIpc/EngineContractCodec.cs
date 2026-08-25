using System.Text.Json;
using Eitmad.Contracts;

namespace Eitmad.Platform.Windows.LocalIpc;

public static class EngineContractCodec
{
    public static Event DecodeEvent(EventEnvelope delivered)
    {
        ArgumentNullException.ThrowIfNull(delivered);
        var json = JsonSerializer.Serialize(delivered.Event, Converter.Settings);
        return JsonSerializer.Deserialize<Event>(json, Converter.Settings)
            ?? throw new InvalidDataException("The engine emitted an empty typed event.");
    }
}
