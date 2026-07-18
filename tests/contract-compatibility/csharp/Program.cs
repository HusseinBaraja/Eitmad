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
var queryProtocol10Json = fixture.RootElement.GetProperty("queryProtocol10").GetRawText();
var queryProtocol10 = JsonSerializer.Deserialize<QueryEnvelope>(queryProtocol10Json, Converter.Settings)
    ?? throw new InvalidOperationException("C# binding did not decode the Rust protocol 1.0 fixture.");

if (query.Query.Kind != QueryKind.EitmadConfigGetV1
    || query.ProtocolVersion.Minor != 2
    || queryProtocol10.ProtocolVersion.Minor != 0
    || queryProtocol10.Query.Kind != QueryKind.EitmadConfigGetV1)
{
    throw new InvalidOperationException("C# binding decoded the wrong query version or identifier.");
}

var encoded = JsonSerializer.Serialize(query, Converter.Settings);
using var roundTrip = JsonDocument.Parse(encoded);
if (roundTrip.RootElement.GetProperty("query").GetProperty("kind").GetString()
    != ProtocolIds.Queries.EitmadConfigGetV1)
{
    throw new InvalidOperationException("C# binding changed the Rust protocol identifier.");
}

var errorJson = fixture.RootElement.GetProperty("structuredError").GetRawText();
var structuredError = JsonSerializer.Deserialize<QueryResult>(errorJson, Converter.Settings)
    ?? throw new InvalidOperationException("C# binding did not decode the Rust structured error fixture.");
var encodedError = JsonSerializer.Serialize(structuredError, Converter.Settings);
var decodedError = JsonSerializer.Deserialize<QueryResult>(encodedError, Converter.Settings)
    ?? throw new InvalidOperationException("C# binding did not round-trip the structured error fixture.");
if (decodedError.Parameters[0].Value.Value.String != "ملف عرض السعر Quote-١٢.pdf")
{
    throw new InvalidOperationException(
        "C# binding lost Arabic or mixed-direction text during structured error round-trip."
    );
}

if (!OpenProtocolId.TryParse(ProtocolIds.Capabilities.EitmadCapabilitySyncV1, out _)
    || OpenProtocolId.TryParse("Eitmad Sync", out _))
{
    throw new InvalidOperationException("C# open protocol identifier validation drifted from Rust.");
}
