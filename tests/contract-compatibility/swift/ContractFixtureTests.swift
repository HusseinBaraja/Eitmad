import Foundation

private struct Fixture: Decodable {
    let query: QueryEnvelope
    let queryResponse: QueryResponseEnvelope
    let structuredError: QueryResult
    let mixedDirectionSamples: [String]
}

@main
private struct ContractFixtureTests {
    static func main() throws {
        guard CommandLine.arguments.count == 2 else {
            throw ContractTestError.invalidArguments
        }
        let data = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
        let fixture = try JSONDecoder().decode(Fixture.self, from: data)
        let encoded = try JSONEncoder().encode(fixture.query)
        let decoded = try JSONDecoder().decode(QueryEnvelope.self, from: encoded)
        let encodedResponse = try JSONEncoder().encode(fixture.queryResponse)
        let decodedResponse = try JSONDecoder().decode(QueryResponseEnvelope.self, from: encodedResponse)
        let encodedError = try JSONEncoder().encode(fixture.structuredError)
        let decodedError = try JSONDecoder().decode(QueryResult.self, from: encodedError)
        let encodedSamples = try JSONEncoder().encode(fixture.mixedDirectionSamples)
        let decodedSamples = try JSONDecoder().decode([String].self, from: encodedSamples)

        guard try hasSameJSON(decoded, fixture.query) else {
            throw ContractTestError.queryCorrupted
        }
        guard try hasSameJSON(decodedResponse, fixture.queryResponse) else {
            throw ContractTestError.responseCorrupted
        }
        guard try hasSameJSON(decodedError, fixture.structuredError) else {
            throw ContractTestError.structuredErrorCorrupted
        }
        guard decodedSamples == fixture.mixedDirectionSamples else {
            throw ContractTestError.textCorrupted
        }
        guard OpenProtocolId(rawValue: ProtocolIds.Capabilities.eitmadCapabilitySyncV1) != nil,
              OpenProtocolId(rawValue: "Eitmad Sync") == nil else {
            throw ContractTestError.identifierValidationDrift
        }
    }
}

private func hasSameJSON<Value: Encodable>(_ lhs: Value, _ rhs: Value) throws -> Bool {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return try encoder.encode(lhs) == encoder.encode(rhs)
}

private enum ContractTestError: Error {
    case invalidArguments
    case queryCorrupted
    case responseCorrupted
    case structuredErrorCorrupted
    case textCorrupted
    case identifierValidationDrift
}
