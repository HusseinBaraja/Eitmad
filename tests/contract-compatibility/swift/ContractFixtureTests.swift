import Foundation

private struct Fixture: Decodable {
    let query: QueryEnvelope
    let queryResponse: QueryResponseEnvelope
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

        guard decoded.query.kind == .eitmadConfigGetV1 else {
            throw ContractTestError.wrongQuery
        }
        guard fixture.mixedDirectionSamples[0].contains("خزانة"),
              fixture.mixedDirectionSamples[1].contains("Quote-١٢.pdf") else {
            throw ContractTestError.textCorrupted
        }
        guard OpenProtocolId(rawValue: ProtocolIds.Capabilities.eitmadCapabilitySyncV1) != nil,
              OpenProtocolId(rawValue: "Eitmad Sync") == nil else {
            throw ContractTestError.identifierValidationDrift
        }
        _ = fixture.queryResponse
    }
}

private enum ContractTestError: Error {
    case invalidArguments
    case wrongQuery
    case textCorrupted
    case identifierValidationDrift
}
