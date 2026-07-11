using System.Text.Json;
using Eitmad.Contracts;

if (args.Length != 1)
{
    throw new ArgumentException("Expected the path to protocol-v1.json fixture.");
}

using var fixture = JsonDocument.Parse(File.ReadAllText(args[0]));
var queryJson = fixture.RootElement.GetProperty("query").GetRawText();
var query = JsonSerializer.Deserialize<QueryEnvelope>(queryJson, Converter.Settings)
    ?? throw new InvalidOperationException("C# binding did not decode the Rust query fixture.");

if (query.Query.Kind != QueryKind.EitmadConfigGetV1)
{
    throw new InvalidOperationException("C# binding decoded the wrong query identifier.");
}

var encoded = JsonSerializer.Serialize(query, Converter.Settings);
using var roundTrip = JsonDocument.Parse(encoded);
if (roundTrip.RootElement.GetProperty("query").GetProperty("kind").GetString()
    != ProtocolIds.Queries.EitmadConfigGetV1)
{
    throw new InvalidOperationException("C# binding changed the Rust protocol identifier.");
}

var samples = fixture.RootElement.GetProperty("mixedDirectionSamples");
if (!samples[0].GetString()!.Contains("خزانة", StringComparison.Ordinal)
    || !samples[1].GetString()!.Contains("Quote-١٢.pdf", StringComparison.Ordinal))
{
    throw new InvalidOperationException("C# binding fixture lost Arabic or mixed-direction text.");
}

if (!OpenProtocolId.TryParse(ProtocolIds.Capabilities.EitmadCapabilitySyncV1, out _)
    || OpenProtocolId.TryParse("Eitmad Sync", out _))
{
    throw new InvalidOperationException("C# open protocol identifier validation drifted from Rust.");
}
