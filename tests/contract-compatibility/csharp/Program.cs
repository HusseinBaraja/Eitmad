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

static string? QueryKind(QueryEnvelope envelope) =>
    envelope.Query.TryGetValue("kind", out var kind)
        ? kind switch
        {
            string text => text,
            JsonElement element => element.GetString(),
            _ => null,
        }
        : null;

if (QueryKind(query) != Query.ConfigGetKind
    || query.ProtocolVersion.Minor != 5
    || queryProtocol10.ProtocolVersion.Minor != 0
    || QueryKind(queryProtocol10) != Query.ConfigGetKind)
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
var structuredError = JsonSerializer.Deserialize<ContractError>(errorJson, Converter.Settings)
    ?? throw new InvalidOperationException("C# binding did not decode the Rust structured error fixture.");
var encodedError = JsonSerializer.Serialize(structuredError, Converter.Settings);
var decodedError = JsonSerializer.Deserialize<ContractError>(encodedError, Converter.Settings)
    ?? throw new InvalidOperationException("C# binding did not round-trip the structured error fixture.");
if (decodedError.Parameters[0].Name != "expected-revision"
    || decodedError.Parameters[0].Value.Kind != ErrorParameterValueKind.Integer
    || decodedError.Parameters[0].Value.Value.Integer != 6)
{
    throw new InvalidOperationException(
        "C# binding changed the safe structured error parameter during round-trip."
    );
}

var observationSeverity = JsonSerializer.Deserialize<ObservationSeverity>(
    fixture.RootElement.GetProperty("observationSeverity").GetRawText(),
    Converter.Settings
);
var observationClassification = JsonSerializer.Deserialize<DataClassification>(
    fixture.RootElement.GetProperty("observationClassification").GetRawText(),
    Converter.Settings
);
var observationValueKind = JsonSerializer.Deserialize<ObservationValueKind>(
    fixture.RootElement.GetProperty("observationValueKind").GetRawText(),
    Converter.Settings
);
if (fixture.RootElement.GetProperty("observationEventId").GetString()
        != "eitmad.observation.engine-failure.v1"
    || fixture.RootElement.GetProperty("observationFieldName").GetString() != "operation"
    || fixture.RootElement.GetProperty("observationComponentId").GetString() != "engine-runtime"
    || observationSeverity != ObservationSeverity.Error
    || observationClassification != DataClassification.Sensitive
    || observationValueKind != ObservationValueKind.Identifier)
{
    throw new InvalidOperationException("C# binding changed an observability contract value.");
}

var samplesJson = fixture.RootElement.GetProperty("mixedDirectionSamples").GetRawText();
var mixedDirectionSamples = JsonSerializer.Deserialize<string[]>(samplesJson, Converter.Settings)
    ?? throw new InvalidOperationException("C# binding did not decode mixed-direction samples.");
var encodedSamples = JsonSerializer.Serialize(mixedDirectionSamples, Converter.Settings);
var decodedSamples = JsonSerializer.Deserialize<string[]>(encodedSamples, Converter.Settings)
    ?? throw new InvalidOperationException("C# binding did not round-trip mixed-direction samples.");
if (!decodedSamples.SequenceEqual(mixedDirectionSamples)
    || !decodedSamples.Contains("ملف عرض السعر Quote-١٢.pdf"))
{
    throw new InvalidOperationException(
        "C# binding lost Arabic or mixed-direction text during sample round-trip."
    );
}

if (!OpenProtocolId.TryParse(ProtocolIds.Capabilities.EitmadCapabilitySyncV1, out _)
    || OpenProtocolId.TryParse("Eitmad Sync", out _))
{
    throw new InvalidOperationException("C# open protocol identifier validation drifted from Rust.");
}
