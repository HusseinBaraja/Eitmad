// Generated from Rust contracts. Do not edit.
// This file was generated from JSON Schema using quicktype, do not modify it directly.
// To parse the JSON, add this file to your project and do:
//
//   let eitmadContractSchema = try EitmadContractSchema(json)

import Foundation

// MARK: - EitmadContractSchema
public struct EitmadContractSchema: Codable, Sendable {
    public let catalog: ProtocolCatalog
    public let commandRequest: CommandEnvelope
    public let commandResponse: CommandResponseEnvelope
    public let diagnosticReport: DiagnosticReport
    public let effectivePermissions: EffectivePermissions
    public let event: EventEnvelope
    public let ipcClientMessage: IPCClientMessage
    public let ipcServerMessage: IPCServerMessage
    public let lifecycleSnapshot: LifecycleSnapshot
    public let negotiation: NegotiationOutcome
    public let peerHello: PeerHello
    public let queryRequest: QueryEnvelope
    public let queryResponse: QueryResponseEnvelope
    public let subscriptionRequest: SubscriptionEnvelope
    public let syncMessage: SyncMessage
    public let syncStatus: SyncStatus
    public let updateState: UpdateState

    public enum CodingKeys: String, CodingKey {
        case catalog
        case commandRequest = "command_request"
        case commandResponse = "command_response"
        case diagnosticReport = "diagnostic_report"
        case effectivePermissions = "effective_permissions"
        case event
        case ipcClientMessage = "ipc_client_message"
        case ipcServerMessage = "ipc_server_message"
        case lifecycleSnapshot = "lifecycle_snapshot"
        case negotiation
        case peerHello = "peer_hello"
        case queryRequest = "query_request"
        case queryResponse = "query_response"
        case subscriptionRequest = "subscription_request"
        case syncMessage = "sync_message"
        case syncStatus = "sync_status"
        case updateState = "update_state"
    }

    public init(catalog: ProtocolCatalog, commandRequest: CommandEnvelope, commandResponse: CommandResponseEnvelope, diagnosticReport: DiagnosticReport, effectivePermissions: EffectivePermissions, event: EventEnvelope, ipcClientMessage: IPCClientMessage, ipcServerMessage: IPCServerMessage, lifecycleSnapshot: LifecycleSnapshot, negotiation: NegotiationOutcome, peerHello: PeerHello, queryRequest: QueryEnvelope, queryResponse: QueryResponseEnvelope, subscriptionRequest: SubscriptionEnvelope, syncMessage: SyncMessage, syncStatus: SyncStatus, updateState: UpdateState) {
        self.catalog = catalog
        self.commandRequest = commandRequest
        self.commandResponse = commandResponse
        self.diagnosticReport = diagnosticReport
        self.effectivePermissions = effectivePermissions
        self.event = event
        self.ipcClientMessage = ipcClientMessage
        self.ipcServerMessage = ipcServerMessage
        self.lifecycleSnapshot = lifecycleSnapshot
        self.negotiation = negotiation
        self.peerHello = peerHello
        self.queryRequest = queryRequest
        self.queryResponse = queryResponse
        self.subscriptionRequest = subscriptionRequest
        self.syncMessage = syncMessage
        self.syncStatus = syncStatus
        self.updateState = updateState
    }
}

// MARK: EitmadContractSchema convenience initializers and mutators

public extension EitmadContractSchema {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(EitmadContractSchema.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        catalog: ProtocolCatalog? = nil,
        commandRequest: CommandEnvelope? = nil,
        commandResponse: CommandResponseEnvelope? = nil,
        diagnosticReport: DiagnosticReport? = nil,
        effectivePermissions: EffectivePermissions? = nil,
        event: EventEnvelope? = nil,
        ipcClientMessage: IPCClientMessage? = nil,
        ipcServerMessage: IPCServerMessage? = nil,
        lifecycleSnapshot: LifecycleSnapshot? = nil,
        negotiation: NegotiationOutcome? = nil,
        peerHello: PeerHello? = nil,
        queryRequest: QueryEnvelope? = nil,
        queryResponse: QueryResponseEnvelope? = nil,
        subscriptionRequest: SubscriptionEnvelope? = nil,
        syncMessage: SyncMessage? = nil,
        syncStatus: SyncStatus? = nil,
        updateState: UpdateState? = nil
    ) -> EitmadContractSchema {
        return EitmadContractSchema(
            catalog: catalog ?? self.catalog,
            commandRequest: commandRequest ?? self.commandRequest,
            commandResponse: commandResponse ?? self.commandResponse,
            diagnosticReport: diagnosticReport ?? self.diagnosticReport,
            effectivePermissions: effectivePermissions ?? self.effectivePermissions,
            event: event ?? self.event,
            ipcClientMessage: ipcClientMessage ?? self.ipcClientMessage,
            ipcServerMessage: ipcServerMessage ?? self.ipcServerMessage,
            lifecycleSnapshot: lifecycleSnapshot ?? self.lifecycleSnapshot,
            negotiation: negotiation ?? self.negotiation,
            peerHello: peerHello ?? self.peerHello,
            queryRequest: queryRequest ?? self.queryRequest,
            queryResponse: queryResponse ?? self.queryResponse,
            subscriptionRequest: subscriptionRequest ?? self.subscriptionRequest,
            syncMessage: syncMessage ?? self.syncMessage,
            syncStatus: syncStatus ?? self.syncStatus,
            updateState: updateState ?? self.updateState
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ProtocolCatalog
public struct ProtocolCatalog: Codable, Sendable {
    public let capabilities, commands, configKeys, errorCodes: [String]
    public let errorParameterNames, events, ipcMessages, messageIDS: [String]
    public let permissions, queries, schemaIDS, subscriptions: [String]
    public let syncMessages: [String]

    public enum CodingKeys: String, CodingKey {
        case capabilities, commands, configKeys, errorCodes, errorParameterNames, events, ipcMessages
        case messageIDS = "messageIds"
        case permissions, queries
        case schemaIDS = "schemaIds"
        case subscriptions, syncMessages
    }

    public init(capabilities: [String], commands: [String], configKeys: [String], errorCodes: [String], errorParameterNames: [String], events: [String], ipcMessages: [String], messageIDS: [String], permissions: [String], queries: [String], schemaIDS: [String], subscriptions: [String], syncMessages: [String]) {
        self.capabilities = capabilities
        self.commands = commands
        self.configKeys = configKeys
        self.errorCodes = errorCodes
        self.errorParameterNames = errorParameterNames
        self.events = events
        self.ipcMessages = ipcMessages
        self.messageIDS = messageIDS
        self.permissions = permissions
        self.queries = queries
        self.schemaIDS = schemaIDS
        self.subscriptions = subscriptions
        self.syncMessages = syncMessages
    }
}

// MARK: ProtocolCatalog convenience initializers and mutators

public extension ProtocolCatalog {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ProtocolCatalog.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        capabilities: [String]? = nil,
        commands: [String]? = nil,
        configKeys: [String]? = nil,
        errorCodes: [String]? = nil,
        errorParameterNames: [String]? = nil,
        events: [String]? = nil,
        ipcMessages: [String]? = nil,
        messageIDS: [String]? = nil,
        permissions: [String]? = nil,
        queries: [String]? = nil,
        schemaIDS: [String]? = nil,
        subscriptions: [String]? = nil,
        syncMessages: [String]? = nil
    ) -> ProtocolCatalog {
        return ProtocolCatalog(
            capabilities: capabilities ?? self.capabilities,
            commands: commands ?? self.commands,
            configKeys: configKeys ?? self.configKeys,
            errorCodes: errorCodes ?? self.errorCodes,
            errorParameterNames: errorParameterNames ?? self.errorParameterNames,
            events: events ?? self.events,
            ipcMessages: ipcMessages ?? self.ipcMessages,
            messageIDS: messageIDS ?? self.messageIDS,
            permissions: permissions ?? self.permissions,
            queries: queries ?? self.queries,
            schemaIDS: schemaIDS ?? self.schemaIDS,
            subscriptions: subscriptions ?? self.subscriptions,
            syncMessages: syncMessages ?? self.syncMessages
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - CommandEnvelope
public struct CommandEnvelope: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let causationID: String?
    public let command: Command
    public let correlationID: String
    public let deadline: Int
    public let idempotencyKey: String
    public let protocolVersion: ProtocolVersion
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case authorization
        case causationID = "causationId"
        case command
        case correlationID = "correlationId"
        case deadline, idempotencyKey, protocolVersion
        case requestID = "requestId"
    }

    public init(authorization: AuthorizationContext, causationID: String?, command: Command, correlationID: String, deadline: Int, idempotencyKey: String, protocolVersion: ProtocolVersion, requestID: String) {
        self.authorization = authorization
        self.causationID = causationID
        self.command = command
        self.correlationID = correlationID
        self.deadline = deadline
        self.idempotencyKey = idempotencyKey
        self.protocolVersion = protocolVersion
        self.requestID = requestID
    }
}

// MARK: CommandEnvelope convenience initializers and mutators

public extension CommandEnvelope {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(CommandEnvelope.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        authorization: AuthorizationContext? = nil,
        causationID: String?? = nil,
        command: Command? = nil,
        correlationID: String? = nil,
        deadline: Int? = nil,
        idempotencyKey: String? = nil,
        protocolVersion: ProtocolVersion? = nil,
        requestID: String? = nil
    ) -> CommandEnvelope {
        return CommandEnvelope(
            authorization: authorization ?? self.authorization,
            causationID: causationID ?? self.causationID,
            command: command ?? self.command,
            correlationID: correlationID ?? self.correlationID,
            deadline: deadline ?? self.deadline,
            idempotencyKey: idempotencyKey ?? self.idempotencyKey,
            protocolVersion: protocolVersion ?? self.protocolVersion,
            requestID: requestID ?? self.requestID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - AuthorizationContext
public struct AuthorizationContext: Codable, Sendable {
    public let identity: AuthenticatedIdentity
    public let scope: ScopeRef
    public let sessionID: String

    public enum CodingKeys: String, CodingKey {
        case identity, scope
        case sessionID = "sessionId"
    }

    public init(identity: AuthenticatedIdentity, scope: ScopeRef, sessionID: String) {
        self.identity = identity
        self.scope = scope
        self.sessionID = sessionID
    }
}

// MARK: AuthorizationContext convenience initializers and mutators

public extension AuthorizationContext {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(AuthorizationContext.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        identity: AuthenticatedIdentity? = nil,
        scope: ScopeRef? = nil,
        sessionID: String? = nil
    ) -> AuthorizationContext {
        return AuthorizationContext(
            identity: identity ?? self.identity,
            scope: scope ?? self.scope,
            sessionID: sessionID ?? self.sessionID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - AuthenticatedIdentity
public struct AuthenticatedIdentity: Codable, Sendable {
    public let deviceID: String?
    public let principalID: String
    public let principalKind: PrincipalKind
    public let serviceID: String?

    public enum CodingKeys: String, CodingKey {
        case deviceID = "deviceId"
        case principalID = "principalId"
        case principalKind
        case serviceID = "serviceId"
    }

    public init(deviceID: String?, principalID: String, principalKind: PrincipalKind, serviceID: String?) {
        self.deviceID = deviceID
        self.principalID = principalID
        self.principalKind = principalKind
        self.serviceID = serviceID
    }
}

// MARK: AuthenticatedIdentity convenience initializers and mutators

public extension AuthenticatedIdentity {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(AuthenticatedIdentity.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        deviceID: String?? = nil,
        principalID: String? = nil,
        principalKind: PrincipalKind? = nil,
        serviceID: String?? = nil
    ) -> AuthenticatedIdentity {
        return AuthenticatedIdentity(
            deviceID: deviceID ?? self.deviceID,
            principalID: principalID ?? self.principalID,
            principalKind: principalKind ?? self.principalKind,
            serviceID: serviceID ?? self.serviceID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum PrincipalKind: String, Codable, Sendable {
    case device = "device"
    case service = "service"
    case user = "user"
}

// MARK: - ScopeRef
public struct ScopeRef: Codable, Sendable {
    public let id, kind: String

    public init(id: String, kind: String) {
        self.id = id
        self.kind = kind
    }
}

// MARK: ScopeRef convenience initializers and mutators

public extension ScopeRef {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ScopeRef.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        id: String? = nil,
        kind: String? = nil
    ) -> ScopeRef {
        return ScopeRef(
            id: id ?? self.id,
            kind: kind ?? self.kind
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

/// Authoritative state-changing requests.
// MARK: - Command
public struct Command: Codable, Sendable {
    public let kind: CommandKind
    public let payload: UpdateConfiguration

    public init(kind: CommandKind, payload: UpdateConfiguration) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: Command convenience initializers and mutators

public extension Command {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Command.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: CommandKind? = nil,
        payload: UpdateConfiguration? = nil
    ) -> Command {
        return Command(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum CommandKind: String, Codable, Sendable {
    case eitmadConfigUpdateV1 = "eitmad.config.update.v1"
    case eitmadOperationCancelV1 = "eitmad.operation.cancel.v1"
    case eitmadUpdateReportInstallerOutcomeV1 = "eitmad.update.report-installer-outcome.v1"
}

// MARK: - UpdateConfiguration
public struct UpdateConfiguration: Codable, Sendable {
    public let changes: [ConfigChange]?
    public let expectedRevision: Int?
    public let operationID, handoffID: String?
    public let outcome: InstallerOutcome?

    public enum CodingKeys: String, CodingKey {
        case changes, expectedRevision
        case operationID = "operationId"
        case handoffID = "handoffId"
        case outcome
    }

    public init(changes: [ConfigChange]?, expectedRevision: Int?, operationID: String?, handoffID: String?, outcome: InstallerOutcome?) {
        self.changes = changes
        self.expectedRevision = expectedRevision
        self.operationID = operationID
        self.handoffID = handoffID
        self.outcome = outcome
    }
}

// MARK: UpdateConfiguration convenience initializers and mutators

public extension UpdateConfiguration {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(UpdateConfiguration.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        changes: [ConfigChange]?? = nil,
        expectedRevision: Int?? = nil,
        operationID: String?? = nil,
        handoffID: String?? = nil,
        outcome: InstallerOutcome?? = nil
    ) -> UpdateConfiguration {
        return UpdateConfiguration(
            changes: changes ?? self.changes,
            expectedRevision: expectedRevision ?? self.expectedRevision,
            operationID: operationID ?? self.operationID,
            handoffID: handoffID ?? self.handoffID,
            outcome: outcome ?? self.outcome
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ConfigChange
public struct ConfigChange: Codable, Sendable {
    public let key: String
    public let value: ConfigWriteValue

    public init(key: String, value: ConfigWriteValue) {
        self.key = key
        self.value = value
    }
}

// MARK: ConfigChange convenience initializers and mutators

public extension ConfigChange {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ConfigChange.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        key: String? = nil,
        value: ConfigWriteValue? = nil
    ) -> ConfigChange {
        return ConfigChange(
            key: key ?? self.key,
            value: value ?? self.value
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ConfigWriteValue
public struct ConfigWriteValue: Codable, Sendable {
    public let kind: ConfigWriteValueKind
    public let value: ConfigWriteValueValue

    public init(kind: ConfigWriteValueKind, value: ConfigWriteValueValue) {
        self.kind = kind
        self.value = value
    }
}

// MARK: ConfigWriteValue convenience initializers and mutators

public extension ConfigWriteValue {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ConfigWriteValue.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: ConfigWriteValueKind? = nil,
        value: ConfigWriteValueValue? = nil
    ) -> ConfigWriteValue {
        return ConfigWriteValue(
            kind: kind ?? self.kind,
            value: value ?? self.value
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum ConfigWriteValueKind: String, Codable, Sendable {
    case boolean = "boolean"
    case decimal = "decimal"
    case integer = "integer"
    case secretReference = "secretReference"
    case text = "text"
    case textList = "textList"
}

public enum ConfigWriteValueValue: Codable, Sendable {
    case bool(Bool)
    case integer(Int)
    case string(String)
    case stringArray([String])

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let x = try? container.decode(Bool.self) {
            self = .bool(x)
            return
        }
        if let x = try? container.decode(Int.self) {
            self = .integer(x)
            return
        }
        if let x = try? container.decode([String].self) {
            self = .stringArray(x)
            return
        }
        if let x = try? container.decode(String.self) {
            self = .string(x)
            return
        }
        throw DecodingError.typeMismatch(ConfigWriteValueValue.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Wrong type for ConfigWriteValueValue"))
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .bool(let x):
            try container.encode(x)
        case .integer(let x):
            try container.encode(x)
        case .string(let x):
            try container.encode(x)
        case .stringArray(let x):
            try container.encode(x)
        }
    }
}

// MARK: - InstallerOutcome
public struct InstallerOutcome: Codable, Sendable {
    public let kind: InstallerOutcomeKind
    public let payload: InstallerOutcomePayload?

    public init(kind: InstallerOutcomeKind, payload: InstallerOutcomePayload?) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: InstallerOutcome convenience initializers and mutators

public extension InstallerOutcome {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(InstallerOutcome.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: InstallerOutcomeKind? = nil,
        payload: InstallerOutcomePayload?? = nil
    ) -> InstallerOutcome {
        return InstallerOutcome(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum InstallerOutcomeKind: String, Codable, Sendable {
    case cancelled = "cancelled"
    case failed = "failed"
    case succeeded = "succeeded"
}

// MARK: - InstallerOutcomePayload
public struct InstallerOutcomePayload: Codable, Sendable {
    public let installedVersion: String?
    public let errorCode: String?

    public enum CodingKeys: String, CodingKey {
        case installedVersion = "installed_version"
        case errorCode = "error_code"
    }

    public init(installedVersion: String?, errorCode: String?) {
        self.installedVersion = installedVersion
        self.errorCode = errorCode
    }
}

// MARK: InstallerOutcomePayload convenience initializers and mutators

public extension InstallerOutcomePayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(InstallerOutcomePayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        installedVersion: String?? = nil,
        errorCode: String?? = nil
    ) -> InstallerOutcomePayload {
        return InstallerOutcomePayload(
            installedVersion: installedVersion ?? self.installedVersion,
            errorCode: errorCode ?? self.errorCode
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ProtocolVersion
public struct ProtocolVersion: Codable, Sendable {
    public let major, minor: Int

    public init(major: Int, minor: Int) {
        self.major = major
        self.minor = minor
    }
}

// MARK: ProtocolVersion convenience initializers and mutators

public extension ProtocolVersion {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ProtocolVersion.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        major: Int? = nil,
        minor: Int? = nil
    ) -> ProtocolVersion {
        return ProtocolVersion(
            major: major ?? self.major,
            minor: minor ?? self.minor
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - CommandResponseEnvelope
public struct CommandResponseEnvelope: Codable, Sendable {
    public let correlationID: String
    public let outcome: CommandOutcome
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
    }

    public init(correlationID: String, outcome: CommandOutcome, requestID: String) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
    }
}

// MARK: CommandResponseEnvelope convenience initializers and mutators

public extension CommandResponseEnvelope {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(CommandResponseEnvelope.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        correlationID: String? = nil,
        outcome: CommandOutcome? = nil,
        requestID: String? = nil
    ) -> CommandResponseEnvelope {
        return CommandResponseEnvelope(
            correlationID: correlationID ?? self.correlationID,
            outcome: outcome ?? self.outcome,
            requestID: requestID ?? self.requestID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - CommandOutcome
public struct CommandOutcome: Codable, Sendable {
    public let payload: CommandResult
    public let status: CommandOutcomeStatus

    public init(payload: CommandResult, status: CommandOutcomeStatus) {
        self.payload = payload
        self.status = status
    }
}

// MARK: CommandOutcome convenience initializers and mutators

public extension CommandOutcome {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(CommandOutcome.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        payload: CommandResult? = nil,
        status: CommandOutcomeStatus? = nil
    ) -> CommandOutcome {
        return CommandOutcome(
            payload: payload ?? self.payload,
            status: status ?? self.status
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - CommandResult
public struct CommandResult: Codable, Sendable {
    public let kind: PurpleKind?
    public let payload: PurpleUpdateState?
    public let code, correlationID: String?
    public let detail: ErrorDetail?
    public let messageID: String?
    public let parameters: [ErrorParameter]?
    public let retry: RetryDisposition?

    public enum CodingKeys: String, CodingKey {
        case kind, payload, code
        case correlationID = "correlationId"
        case detail
        case messageID = "messageId"
        case parameters, retry
    }

    public init(kind: PurpleKind?, payload: PurpleUpdateState?, code: String?, correlationID: String?, detail: ErrorDetail?, messageID: String?, parameters: [ErrorParameter]?, retry: RetryDisposition?) {
        self.kind = kind
        self.payload = payload
        self.code = code
        self.correlationID = correlationID
        self.detail = detail
        self.messageID = messageID
        self.parameters = parameters
        self.retry = retry
    }
}

// MARK: CommandResult convenience initializers and mutators

public extension CommandResult {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(CommandResult.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: PurpleKind?? = nil,
        payload: PurpleUpdateState?? = nil,
        code: String?? = nil,
        correlationID: String?? = nil,
        detail: ErrorDetail?? = nil,
        messageID: String?? = nil,
        parameters: [ErrorParameter]?? = nil,
        retry: RetryDisposition?? = nil
    ) -> CommandResult {
        return CommandResult(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload,
            code: code ?? self.code,
            correlationID: correlationID ?? self.correlationID,
            detail: detail ?? self.detail,
            messageID: messageID ?? self.messageID,
            parameters: parameters ?? self.parameters,
            retry: retry ?? self.retry
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ErrorDetail
public struct ErrorDetail: Codable, Sendable {
    public let kind: DetailKind
    public let payload: DetailPayload

    public init(kind: DetailKind, payload: DetailPayload) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: ErrorDetail convenience initializers and mutators

public extension ErrorDetail {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ErrorDetail.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: DetailKind? = nil,
        payload: DetailPayload? = nil
    ) -> ErrorDetail {
        return ErrorDetail(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum DetailKind: String, Codable, Sendable {
    case compatibility = "compatibility"
    case deadline = "deadline"
    case lifecycle = "lifecycle"
    case payloadLimit = "payloadLimit"
    case revisionConflict = "revisionConflict"
    case validation = "validation"
}

// MARK: - DetailPayload
public struct DetailPayload: Codable, Sendable {
    public let fields: [String]?
    public let actual, expected: Int?
    public let reason: String?
    public let stage: LifecycleStage?
    public let deadline: Int?
    public let maximumBytes: Int?

    public enum CodingKeys: String, CodingKey {
        case fields, actual, expected, reason, stage, deadline
        case maximumBytes = "maximum_bytes"
    }

    public init(fields: [String]?, actual: Int?, expected: Int?, reason: String?, stage: LifecycleStage?, deadline: Int?, maximumBytes: Int?) {
        self.fields = fields
        self.actual = actual
        self.expected = expected
        self.reason = reason
        self.stage = stage
        self.deadline = deadline
        self.maximumBytes = maximumBytes
    }
}

// MARK: DetailPayload convenience initializers and mutators

public extension DetailPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(DetailPayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        fields: [String]?? = nil,
        actual: Int?? = nil,
        expected: Int?? = nil,
        reason: String?? = nil,
        stage: LifecycleStage?? = nil,
        deadline: Int?? = nil,
        maximumBytes: Int?? = nil
    ) -> DetailPayload {
        return DetailPayload(
            fields: fields ?? self.fields,
            actual: actual ?? self.actual,
            expected: expected ?? self.expected,
            reason: reason ?? self.reason,
            stage: stage ?? self.stage,
            deadline: deadline ?? self.deadline,
            maximumBytes: maximumBytes ?? self.maximumBytes
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum LifecycleStage: String, Codable, Sendable {
    case authorityLock = "authorityLock"
    case componentShutdown = "componentShutdown"
    case componentStartup = "componentStartup"
    case processIdentity = "processIdentity"
    case readinessCheck = "readinessCheck"
}

public enum PurpleKind: String, Codable, Sendable {
    case configurationUpdated = "configurationUpdated"
    case installerOutcomeRecorded = "installerOutcomeRecorded"
    case operationCancelled = "operationCancelled"
}

// MARK: - ErrorParameter
public struct ErrorParameter: Codable, Sendable {
    public let name: String
    public let value: ErrorParameterValue

    public init(name: String, value: ErrorParameterValue) {
        self.name = name
        self.value = value
    }
}

// MARK: ErrorParameter convenience initializers and mutators

public extension ErrorParameter {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ErrorParameter.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        name: String? = nil,
        value: ErrorParameterValue? = nil
    ) -> ErrorParameter {
        return ErrorParameter(
            name: name ?? self.name,
            value: value ?? self.value
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ErrorParameterValue
public struct ErrorParameterValue: Codable, Sendable {
    public let kind: ErrorParameterValueKind
    public let value: ErrorParameterValueValue

    public init(kind: ErrorParameterValueKind, value: ErrorParameterValueValue) {
        self.kind = kind
        self.value = value
    }
}

// MARK: ErrorParameterValue convenience initializers and mutators

public extension ErrorParameterValue {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ErrorParameterValue.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: ErrorParameterValueKind? = nil,
        value: ErrorParameterValueValue? = nil
    ) -> ErrorParameterValue {
        return ErrorParameterValue(
            kind: kind ?? self.kind,
            value: value ?? self.value
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum ErrorParameterValueKind: String, Codable, Sendable {
    case identifier = "identifier"
    case integer = "integer"
    case text = "text"
}

public enum ErrorParameterValueValue: Codable, Sendable {
    case integer(Int)
    case string(String)

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let x = try? container.decode(Int.self) {
            self = .integer(x)
            return
        }
        if let x = try? container.decode(String.self) {
            self = .string(x)
            return
        }
        throw DecodingError.typeMismatch(ErrorParameterValueValue.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Wrong type for ErrorParameterValueValue"))
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .integer(let x):
            try container.encode(x)
        case .string(let x):
            try container.encode(x)
        }
    }
}

// MARK: - PurpleUpdateState
public struct PurpleUpdateState: Codable, Sendable {
    public let entries: [ConfigEntry]?
    public let revision, schemaVersion: Int?
    public let scope: ScopeRef?
    public let operationID: String?
    public let kind: UpdateStateKind?
    public let payload: UpdateStatePayload?

    public enum CodingKeys: String, CodingKey {
        case entries, revision, schemaVersion, scope
        case operationID = "operation_id"
        case kind, payload
    }

    public init(entries: [ConfigEntry]?, revision: Int?, schemaVersion: Int?, scope: ScopeRef?, operationID: String?, kind: UpdateStateKind?, payload: UpdateStatePayload?) {
        self.entries = entries
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
        self.operationID = operationID
        self.kind = kind
        self.payload = payload
    }
}

// MARK: PurpleUpdateState convenience initializers and mutators

public extension PurpleUpdateState {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(PurpleUpdateState.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        entries: [ConfigEntry]?? = nil,
        revision: Int?? = nil,
        schemaVersion: Int?? = nil,
        scope: ScopeRef?? = nil,
        operationID: String?? = nil,
        kind: UpdateStateKind?? = nil,
        payload: UpdateStatePayload?? = nil
    ) -> PurpleUpdateState {
        return PurpleUpdateState(
            entries: entries ?? self.entries,
            revision: revision ?? self.revision,
            schemaVersion: schemaVersion ?? self.schemaVersion,
            scope: scope ?? self.scope,
            operationID: operationID ?? self.operationID,
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ConfigEntry
public struct ConfigEntry: Codable, Sendable {
    public let key: String
    public let restartRequirement: RestartRequirement
    public let sensitivity: ConfigSensitivity
    public let value: ConfigReadValue

    public init(key: String, restartRequirement: RestartRequirement, sensitivity: ConfigSensitivity, value: ConfigReadValue) {
        self.key = key
        self.restartRequirement = restartRequirement
        self.sensitivity = sensitivity
        self.value = value
    }
}

// MARK: ConfigEntry convenience initializers and mutators

public extension ConfigEntry {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ConfigEntry.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        key: String? = nil,
        restartRequirement: RestartRequirement? = nil,
        sensitivity: ConfigSensitivity? = nil,
        value: ConfigReadValue? = nil
    ) -> ConfigEntry {
        return ConfigEntry(
            key: key ?? self.key,
            restartRequirement: restartRequirement ?? self.restartRequirement,
            sensitivity: sensitivity ?? self.sensitivity,
            value: value ?? self.value
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum RestartRequirement: String, Codable, Sendable {
    case application = "application"
    case engine = "engine"
    case none = "none"
}

public enum ConfigSensitivity: String, Codable, Sendable {
    case configSensitivityPublic = "public"
    case secret = "secret"
    case sensitive = "sensitive"
}

// MARK: - ConfigReadValue
public struct ConfigReadValue: Codable, Sendable {
    public let kind: ConfigReadValueKind
    public let value: ConfigWriteValueValue?

    public init(kind: ConfigReadValueKind, value: ConfigWriteValueValue?) {
        self.kind = kind
        self.value = value
    }
}

// MARK: ConfigReadValue convenience initializers and mutators

public extension ConfigReadValue {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ConfigReadValue.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: ConfigReadValueKind? = nil,
        value: ConfigWriteValueValue?? = nil
    ) -> ConfigReadValue {
        return ConfigReadValue(
            kind: kind ?? self.kind,
            value: value ?? self.value
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum ConfigReadValueKind: String, Codable, Sendable {
    case boolean = "boolean"
    case decimal = "decimal"
    case integer = "integer"
    case redacted = "redacted"
    case secretReference = "secretReference"
    case text = "text"
    case textList = "textList"
}

public enum UpdateStateKind: String, Codable, Sendable {
    case available = "available"
    case checking = "checking"
    case downloading = "downloading"
    case failed = "failed"
    case idle = "idle"
    case installationHandoff = "installationHandoff"
    case installing = "installing"
    case paused = "paused"
    case preflight = "preflight"
    case ready = "ready"
    case recoveryRequired = "recoveryRequired"
    case revoked = "revoked"
    case succeeded = "succeeded"
    case verifying = "verifying"
}

// MARK: - UpdateStatePayload
public struct UpdateStatePayload: Codable, Sendable {
    public let version: String?
    public let progressBps: Int?
    public let handoffID, errorCode: String?

    public enum CodingKeys: String, CodingKey {
        case version
        case progressBps = "progress_bps"
        case handoffID = "handoff_id"
        case errorCode = "error_code"
    }

    public init(version: String?, progressBps: Int?, handoffID: String?, errorCode: String?) {
        self.version = version
        self.progressBps = progressBps
        self.handoffID = handoffID
        self.errorCode = errorCode
    }
}

// MARK: UpdateStatePayload convenience initializers and mutators

public extension UpdateStatePayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(UpdateStatePayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        version: String?? = nil,
        progressBps: Int?? = nil,
        handoffID: String?? = nil,
        errorCode: String?? = nil
    ) -> UpdateStatePayload {
        return UpdateStatePayload(
            version: version ?? self.version,
            progressBps: progressBps ?? self.progressBps,
            handoffID: handoffID ?? self.handoffID,
            errorCode: errorCode ?? self.errorCode
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - RetryDisposition
public struct RetryDisposition: Codable, Sendable {
    public let kind: RetryDispositionKind
    public let retryAfterMS: Int?

    public enum CodingKeys: String, CodingKey {
        case kind
        case retryAfterMS = "retryAfterMs"
    }

    public init(kind: RetryDispositionKind, retryAfterMS: Int?) {
        self.kind = kind
        self.retryAfterMS = retryAfterMS
    }
}

// MARK: RetryDisposition convenience initializers and mutators

public extension RetryDisposition {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(RetryDisposition.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: RetryDispositionKind? = nil,
        retryAfterMS: Int?? = nil
    ) -> RetryDisposition {
        return RetryDisposition(
            kind: kind ?? self.kind,
            retryAfterMS: retryAfterMS ?? self.retryAfterMS
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum RetryDispositionKind: String, Codable, Sendable {
    case never = "never"
    case safeAfterDelay = "safeAfterDelay"
    case safeImmediately = "safeImmediately"
}

public enum CommandOutcomeStatus: String, Codable, Sendable {
    case failed = "failed"
    case succeeded = "succeeded"
}

// MARK: - DiagnosticReport
public struct DiagnosticReport: Codable, Sendable {
    public let checks: [HealthCheckResult]
    public let identity: EngineProcessIdentity
    public let observedAt: Int
    public let readyToStart: Bool
    public let status: HealthStatus

    public init(checks: [HealthCheckResult], identity: EngineProcessIdentity, observedAt: Int, readyToStart: Bool, status: HealthStatus) {
        self.checks = checks
        self.identity = identity
        self.observedAt = observedAt
        self.readyToStart = readyToStart
        self.status = status
    }
}

// MARK: DiagnosticReport convenience initializers and mutators

public extension DiagnosticReport {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(DiagnosticReport.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        checks: [HealthCheckResult]? = nil,
        identity: EngineProcessIdentity? = nil,
        observedAt: Int? = nil,
        readyToStart: Bool? = nil,
        status: HealthStatus? = nil
    ) -> DiagnosticReport {
        return DiagnosticReport(
            checks: checks ?? self.checks,
            identity: identity ?? self.identity,
            observedAt: observedAt ?? self.observedAt,
            readyToStart: readyToStart ?? self.readyToStart,
            status: status ?? self.status
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - HealthCheckResult
public struct HealthCheckResult: Codable, Sendable {
    public let error: ContractError?
    public let id: String
    public let impact: HealthCheckImpact
    public let observedAt: Int
    public let status: HealthStatus

    public init(error: ContractError?, id: String, impact: HealthCheckImpact, observedAt: Int, status: HealthStatus) {
        self.error = error
        self.id = id
        self.impact = impact
        self.observedAt = observedAt
        self.status = status
    }
}

// MARK: HealthCheckResult convenience initializers and mutators

public extension HealthCheckResult {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(HealthCheckResult.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        error: ContractError?? = nil,
        id: String? = nil,
        impact: HealthCheckImpact? = nil,
        observedAt: Int? = nil,
        status: HealthStatus? = nil
    ) -> HealthCheckResult {
        return HealthCheckResult(
            error: error ?? self.error,
            id: id ?? self.id,
            impact: impact ?? self.impact,
            observedAt: observedAt ?? self.observedAt,
            status: status ?? self.status
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ContractError
public struct ContractError: Codable, Sendable {
    public let code, correlationID: String
    public let detail: ErrorDetail?
    public let messageID: String
    public let parameters: [ErrorParameter]
    public let retry: RetryDisposition

    public enum CodingKeys: String, CodingKey {
        case code
        case correlationID = "correlationId"
        case detail
        case messageID = "messageId"
        case parameters, retry
    }

    public init(code: String, correlationID: String, detail: ErrorDetail?, messageID: String, parameters: [ErrorParameter], retry: RetryDisposition) {
        self.code = code
        self.correlationID = correlationID
        self.detail = detail
        self.messageID = messageID
        self.parameters = parameters
        self.retry = retry
    }
}

// MARK: ContractError convenience initializers and mutators

public extension ContractError {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ContractError.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        code: String? = nil,
        correlationID: String? = nil,
        detail: ErrorDetail?? = nil,
        messageID: String? = nil,
        parameters: [ErrorParameter]? = nil,
        retry: RetryDisposition? = nil
    ) -> ContractError {
        return ContractError(
            code: code ?? self.code,
            correlationID: correlationID ?? self.correlationID,
            detail: detail ?? self.detail,
            messageID: messageID ?? self.messageID,
            parameters: parameters ?? self.parameters,
            retry: retry ?? self.retry
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum HealthCheckImpact: String, Codable, Sendable {
    case advisory = "advisory"
    case requiredForReadiness = "requiredForReadiness"
}

public enum HealthStatus: String, Codable, Sendable {
    case degraded = "degraded"
    case healthy = "healthy"
    case unhealthy = "unhealthy"
}

// MARK: - EngineProcessIdentity
public struct EngineProcessIdentity: Codable, Sendable {
    public let instanceID: String
    public let mode: EngineMode
    public let processID: Int
    public let productVersion: String
    public let protocolVersion: ProtocolVersion
    public let startedAt: Int

    public enum CodingKeys: String, CodingKey {
        case instanceID = "instanceId"
        case mode
        case processID = "processId"
        case productVersion, protocolVersion, startedAt
    }

    public init(instanceID: String, mode: EngineMode, processID: Int, productVersion: String, protocolVersion: ProtocolVersion, startedAt: Int) {
        self.instanceID = instanceID
        self.mode = mode
        self.processID = processID
        self.productVersion = productVersion
        self.protocolVersion = protocolVersion
        self.startedAt = startedAt
    }
}

// MARK: EngineProcessIdentity convenience initializers and mutators

public extension EngineProcessIdentity {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(EngineProcessIdentity.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        instanceID: String? = nil,
        mode: EngineMode? = nil,
        processID: Int? = nil,
        productVersion: String? = nil,
        protocolVersion: ProtocolVersion? = nil,
        startedAt: Int? = nil
    ) -> EngineProcessIdentity {
        return EngineProcessIdentity(
            instanceID: instanceID ?? self.instanceID,
            mode: mode ?? self.mode,
            processID: processID ?? self.processID,
            productVersion: productVersion ?? self.productVersion,
            protocolVersion: protocolVersion ?? self.protocolVersion,
            startedAt: startedAt ?? self.startedAt
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum EngineMode: String, Codable, Sendable {
    case diagnostic = "diagnostic"
    case headless = "headless"
    case supervisedDesktop = "supervisedDesktop"
}

// MARK: - EffectivePermissions
public struct EffectivePermissions: Codable, Sendable {
    public let permissions: [EffectivePermission]
    public let policyVersion: Int

    public init(permissions: [EffectivePermission], policyVersion: Int) {
        self.permissions = permissions
        self.policyVersion = policyVersion
    }
}

// MARK: EffectivePermissions convenience initializers and mutators

public extension EffectivePermissions {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(EffectivePermissions.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        permissions: [EffectivePermission]? = nil,
        policyVersion: Int? = nil
    ) -> EffectivePermissions {
        return EffectivePermissions(
            permissions: permissions ?? self.permissions,
            policyVersion: policyVersion ?? self.policyVersion
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - EffectivePermission
public struct EffectivePermission: Codable, Sendable {
    public let decision: PermissionDecision
    public let permission: String

    public init(decision: PermissionDecision, permission: String) {
        self.decision = decision
        self.permission = permission
    }
}

// MARK: EffectivePermission convenience initializers and mutators

public extension EffectivePermission {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(EffectivePermission.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        decision: PermissionDecision? = nil,
        permission: String? = nil
    ) -> EffectivePermission {
        return EffectivePermission(
            decision: decision ?? self.decision,
            permission: permission ?? self.permission
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum PermissionDecision: String, Codable, Sendable {
    case denied = "denied"
    case granted = "granted"
}

// MARK: - EventEnvelope
public struct EventEnvelope: Codable, Sendable {
    public let correlationID, cursor: String
    public let event: Event
    public let occurredAt: Int
    public let sequence: Int
    public let subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case cursor, event, occurredAt, sequence
        case subscriptionID = "subscriptionId"
    }

    public init(correlationID: String, cursor: String, event: Event, occurredAt: Int, sequence: Int, subscriptionID: String) {
        self.correlationID = correlationID
        self.cursor = cursor
        self.event = event
        self.occurredAt = occurredAt
        self.sequence = sequence
        self.subscriptionID = subscriptionID
    }
}

// MARK: EventEnvelope convenience initializers and mutators

public extension EventEnvelope {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(EventEnvelope.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        correlationID: String? = nil,
        cursor: String? = nil,
        event: Event? = nil,
        occurredAt: Int? = nil,
        sequence: Int? = nil,
        subscriptionID: String? = nil
    ) -> EventEnvelope {
        return EventEnvelope(
            correlationID: correlationID ?? self.correlationID,
            cursor: cursor ?? self.cursor,
            event: event ?? self.event,
            occurredAt: occurredAt ?? self.occurredAt,
            sequence: sequence ?? self.sequence,
            subscriptionID: subscriptionID ?? self.subscriptionID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

/// Ordered values emitted by subscriptions.
// MARK: - Event
public struct Event: Codable, Sendable {
    public let kind: EventKind
    public let payload: EventPayload

    public init(kind: EventKind, payload: EventPayload) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: Event convenience initializers and mutators

public extension Event {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Event.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: EventKind? = nil,
        payload: EventPayload? = nil
    ) -> Event {
        return Event(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum EventKind: String, Codable, Sendable {
    case eitmadBackgroundJobStatusEventV1 = "eitmad.background-job.status.event.v1"
    case eitmadConfigChangedEventV1 = "eitmad.config.changed.event.v1"
    case eitmadErrorEventV1 = "eitmad.error.event.v1"
    case eitmadNotificationEventV1 = "eitmad.notification.event.v1"
    case eitmadPermissionsChangedEventV1 = "eitmad.permissions.changed.event.v1"
    case eitmadRecordChangedEventV1 = "eitmad.record.changed.event.v1"
    case eitmadSyncStatusEventV1 = "eitmad.sync.status.event.v1"
    case eitmadUpdateStateEventV1 = "eitmad.update.state.event.v1"
}

// MARK: - EventPayload
public struct EventPayload: Codable, Sendable {
    public let entries: [ConfigEntry]?
    public let revision, schemaVersion: Int?
    public let scope: ScopeRef?
    public let permissions: [EffectivePermission]?
    public let policyVersion: Int?
    public let kind: FluffyKind?
    public let payload: PurplePayload?
    public let changedAt: Int?
    public let operation: ChangeOperation?
    public let recordID, schemaID: String?
    public let completedUnits: Int?
    public let error: ContractError?
    public let jobID, jobKind: String?
    public let state: BackgroundJobState?
    public let totalUnits: Int?
    public let correlationID: String?
    public let messageID, notificationID: String?
    public let parameters: [ErrorParameter]?
    public let severity: NotificationSeverity?

    public enum CodingKeys: String, CodingKey {
        case entries, revision, schemaVersion, scope, permissions, policyVersion, kind, payload, changedAt, operation
        case recordID = "recordId"
        case schemaID = "schemaId"
        case completedUnits, error
        case jobID = "jobId"
        case jobKind, state, totalUnits
        case correlationID = "correlationId"
        case messageID = "messageId"
        case notificationID = "notificationId"
        case parameters, severity
    }

    public init(entries: [ConfigEntry]?, revision: Int?, schemaVersion: Int?, scope: ScopeRef?, permissions: [EffectivePermission]?, policyVersion: Int?, kind: FluffyKind?, payload: PurplePayload?, changedAt: Int?, operation: ChangeOperation?, recordID: String?, schemaID: String?, completedUnits: Int?, error: ContractError?, jobID: String?, jobKind: String?, state: BackgroundJobState?, totalUnits: Int?, correlationID: String?, messageID: String?, notificationID: String?, parameters: [ErrorParameter]?, severity: NotificationSeverity?) {
        self.entries = entries
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
        self.permissions = permissions
        self.policyVersion = policyVersion
        self.kind = kind
        self.payload = payload
        self.changedAt = changedAt
        self.operation = operation
        self.recordID = recordID
        self.schemaID = schemaID
        self.completedUnits = completedUnits
        self.error = error
        self.jobID = jobID
        self.jobKind = jobKind
        self.state = state
        self.totalUnits = totalUnits
        self.correlationID = correlationID
        self.messageID = messageID
        self.notificationID = notificationID
        self.parameters = parameters
        self.severity = severity
    }
}

// MARK: EventPayload convenience initializers and mutators

public extension EventPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(EventPayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        entries: [ConfigEntry]?? = nil,
        revision: Int?? = nil,
        schemaVersion: Int?? = nil,
        scope: ScopeRef?? = nil,
        permissions: [EffectivePermission]?? = nil,
        policyVersion: Int?? = nil,
        kind: FluffyKind?? = nil,
        payload: PurplePayload?? = nil,
        changedAt: Int?? = nil,
        operation: ChangeOperation?? = nil,
        recordID: String?? = nil,
        schemaID: String?? = nil,
        completedUnits: Int?? = nil,
        error: ContractError?? = nil,
        jobID: String?? = nil,
        jobKind: String?? = nil,
        state: BackgroundJobState?? = nil,
        totalUnits: Int?? = nil,
        correlationID: String?? = nil,
        messageID: String?? = nil,
        notificationID: String?? = nil,
        parameters: [ErrorParameter]?? = nil,
        severity: NotificationSeverity?? = nil
    ) -> EventPayload {
        return EventPayload(
            entries: entries ?? self.entries,
            revision: revision ?? self.revision,
            schemaVersion: schemaVersion ?? self.schemaVersion,
            scope: scope ?? self.scope,
            permissions: permissions ?? self.permissions,
            policyVersion: policyVersion ?? self.policyVersion,
            kind: kind ?? self.kind,
            payload: payload ?? self.payload,
            changedAt: changedAt ?? self.changedAt,
            operation: operation ?? self.operation,
            recordID: recordID ?? self.recordID,
            schemaID: schemaID ?? self.schemaID,
            completedUnits: completedUnits ?? self.completedUnits,
            error: error ?? self.error,
            jobID: jobID ?? self.jobID,
            jobKind: jobKind ?? self.jobKind,
            state: state ?? self.state,
            totalUnits: totalUnits ?? self.totalUnits,
            correlationID: correlationID ?? self.correlationID,
            messageID: messageID ?? self.messageID,
            notificationID: notificationID ?? self.notificationID,
            parameters: parameters ?? self.parameters,
            severity: severity ?? self.severity
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum FluffyKind: String, Codable, Sendable {
    case available = "available"
    case checking = "checking"
    case conflicted = "conflicted"
    case current = "current"
    case downloading = "downloading"
    case failed = "failed"
    case idle = "idle"
    case installationHandoff = "installationHandoff"
    case installing = "installing"
    case offline = "offline"
    case paused = "paused"
    case preflight = "preflight"
    case queued = "queued"
    case ready = "ready"
    case recoveryRequired = "recoveryRequired"
    case revoked = "revoked"
    case succeeded = "succeeded"
    case syncing = "syncing"
    case verifying = "verifying"
}

public enum ChangeOperation: String, Codable, Sendable {
    case tombstone = "tombstone"
    case upsert = "upsert"
}

// MARK: - PurplePayload
public struct PurplePayload: Codable, Sendable {
    public let version: String?
    public let progressBps: Int?
    public let handoffID, errorCode, checkpoint: String?
    public let records, completed: Int?
    public let total: Int?
    public let reason: String?

    public enum CodingKeys: String, CodingKey {
        case version
        case progressBps = "progress_bps"
        case handoffID = "handoff_id"
        case errorCode = "error_code"
        case checkpoint, records, completed, total, reason
    }

    public init(version: String?, progressBps: Int?, handoffID: String?, errorCode: String?, checkpoint: String?, records: Int?, completed: Int?, total: Int?, reason: String?) {
        self.version = version
        self.progressBps = progressBps
        self.handoffID = handoffID
        self.errorCode = errorCode
        self.checkpoint = checkpoint
        self.records = records
        self.completed = completed
        self.total = total
        self.reason = reason
    }
}

// MARK: PurplePayload convenience initializers and mutators

public extension PurplePayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(PurplePayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        version: String?? = nil,
        progressBps: Int?? = nil,
        handoffID: String?? = nil,
        errorCode: String?? = nil,
        checkpoint: String?? = nil,
        records: Int?? = nil,
        completed: Int?? = nil,
        total: Int?? = nil,
        reason: String?? = nil
    ) -> PurplePayload {
        return PurplePayload(
            version: version ?? self.version,
            progressBps: progressBps ?? self.progressBps,
            handoffID: handoffID ?? self.handoffID,
            errorCode: errorCode ?? self.errorCode,
            checkpoint: checkpoint ?? self.checkpoint,
            records: records ?? self.records,
            completed: completed ?? self.completed,
            total: total ?? self.total,
            reason: reason ?? self.reason
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum NotificationSeverity: String, Codable, Sendable {
    case error = "error"
    case information = "information"
    case success = "success"
    case warning = "warning"
}

public enum BackgroundJobState: String, Codable, Sendable {
    case cancelled = "cancelled"
    case failed = "failed"
    case queued = "queued"
    case running = "running"
    case succeeded = "succeeded"
}

// MARK: - IPCClientMessage
public struct IPCClientMessage: Codable, Sendable {
    public let kind: IPCClientMessageKind
    public let payload: HandshakeRequest

    public init(kind: IPCClientMessageKind, payload: HandshakeRequest) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: IPCClientMessage convenience initializers and mutators

public extension IPCClientMessage {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(IPCClientMessage.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: IPCClientMessageKind? = nil,
        payload: HandshakeRequest? = nil
    ) -> IPCClientMessage {
        return IPCClientMessage(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum IPCClientMessageKind: String, Codable, Sendable {
    case eitmadIPCCommandV1 = "eitmad.ipc.command.v1"
    case eitmadIPCHandshakeV1 = "eitmad.ipc.handshake.v1"
    case eitmadIPCQueryV1 = "eitmad.ipc.query.v1"
    case eitmadIPCShutdownV1 = "eitmad.ipc.shutdown.v1"
    case eitmadIPCSubscribeV1 = "eitmad.ipc.subscribe.v1"
    case eitmadIPCUnsubscribeV1 = "eitmad.ipc.unsubscribe.v1"
}

// MARK: - HandshakeRequest
public struct HandshakeRequest: Codable, Sendable {
    public let assertedAuthorization: DevelopmentIdentityAssertion?
    public let correlationID: String
    public let developmentBearerToken: String?
    public let peer: PeerHello?
    public let requestID: String
    public let authorization: AuthorizationContext?
    public let causationID: String?
    public let command: Command?
    public let deadline: Int?
    public let idempotencyKey: String?
    public let protocolVersion: ProtocolVersion?
    public let query: Query?
    public let resumeAfter: String?
    public let subscription: Subscription?
    public let subscriptionID: String?

    public enum CodingKeys: String, CodingKey {
        case assertedAuthorization
        case correlationID = "correlationId"
        case developmentBearerToken, peer
        case requestID = "requestId"
        case authorization
        case causationID = "causationId"
        case command, deadline, idempotencyKey, protocolVersion, query, resumeAfter, subscription
        case subscriptionID = "subscriptionId"
    }

    public init(assertedAuthorization: DevelopmentIdentityAssertion?, correlationID: String, developmentBearerToken: String?, peer: PeerHello?, requestID: String, authorization: AuthorizationContext?, causationID: String?, command: Command?, deadline: Int?, idempotencyKey: String?, protocolVersion: ProtocolVersion?, query: Query?, resumeAfter: String?, subscription: Subscription?, subscriptionID: String?) {
        self.assertedAuthorization = assertedAuthorization
        self.correlationID = correlationID
        self.developmentBearerToken = developmentBearerToken
        self.peer = peer
        self.requestID = requestID
        self.authorization = authorization
        self.causationID = causationID
        self.command = command
        self.deadline = deadline
        self.idempotencyKey = idempotencyKey
        self.protocolVersion = protocolVersion
        self.query = query
        self.resumeAfter = resumeAfter
        self.subscription = subscription
        self.subscriptionID = subscriptionID
    }
}

// MARK: HandshakeRequest convenience initializers and mutators

public extension HandshakeRequest {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(HandshakeRequest.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        assertedAuthorization: DevelopmentIdentityAssertion?? = nil,
        correlationID: String? = nil,
        developmentBearerToken: String?? = nil,
        peer: PeerHello?? = nil,
        requestID: String? = nil,
        authorization: AuthorizationContext?? = nil,
        causationID: String?? = nil,
        command: Command?? = nil,
        deadline: Int?? = nil,
        idempotencyKey: String?? = nil,
        protocolVersion: ProtocolVersion?? = nil,
        query: Query?? = nil,
        resumeAfter: String?? = nil,
        subscription: Subscription?? = nil,
        subscriptionID: String?? = nil
    ) -> HandshakeRequest {
        return HandshakeRequest(
            assertedAuthorization: assertedAuthorization ?? self.assertedAuthorization,
            correlationID: correlationID ?? self.correlationID,
            developmentBearerToken: developmentBearerToken ?? self.developmentBearerToken,
            peer: peer ?? self.peer,
            requestID: requestID ?? self.requestID,
            authorization: authorization ?? self.authorization,
            causationID: causationID ?? self.causationID,
            command: command ?? self.command,
            deadline: deadline ?? self.deadline,
            idempotencyKey: idempotencyKey ?? self.idempotencyKey,
            protocolVersion: protocolVersion ?? self.protocolVersion,
            query: query ?? self.query,
            resumeAfter: resumeAfter ?? self.resumeAfter,
            subscription: subscription ?? self.subscription,
            subscriptionID: subscriptionID ?? self.subscriptionID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - DevelopmentIdentityAssertion
public struct DevelopmentIdentityAssertion: Codable, Sendable {
    public let identity: AuthenticatedIdentity
    public let scope: ScopeRef

    public init(identity: AuthenticatedIdentity, scope: ScopeRef) {
        self.identity = identity
        self.scope = scope
    }
}

// MARK: DevelopmentIdentityAssertion convenience initializers and mutators

public extension DevelopmentIdentityAssertion {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(DevelopmentIdentityAssertion.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        identity: AuthenticatedIdentity? = nil,
        scope: ScopeRef? = nil
    ) -> DevelopmentIdentityAssertion {
        return DevelopmentIdentityAssertion(
            identity: identity ?? self.identity,
            scope: scope ?? self.scope
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - PeerHello
public struct PeerHello: Codable, Sendable {
    public let capabilities: [String]
    public let peerKind: PeerKind
    public let productVersion: String
    public let protocols: [SupportedProtocol]
    public let requiredCapabilities: [String]
    public let schemas: [SchemaSupport]

    public init(capabilities: [String], peerKind: PeerKind, productVersion: String, protocols: [SupportedProtocol], requiredCapabilities: [String], schemas: [SchemaSupport]) {
        self.capabilities = capabilities
        self.peerKind = peerKind
        self.productVersion = productVersion
        self.protocols = protocols
        self.requiredCapabilities = requiredCapabilities
        self.schemas = schemas
    }
}

// MARK: PeerHello convenience initializers and mutators

public extension PeerHello {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(PeerHello.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        capabilities: [String]? = nil,
        peerKind: PeerKind? = nil,
        productVersion: String? = nil,
        protocols: [SupportedProtocol]? = nil,
        requiredCapabilities: [String]? = nil,
        schemas: [SchemaSupport]? = nil
    ) -> PeerHello {
        return PeerHello(
            capabilities: capabilities ?? self.capabilities,
            peerKind: peerKind ?? self.peerKind,
            productVersion: productVersion ?? self.productVersion,
            protocols: protocols ?? self.protocols,
            requiredCapabilities: requiredCapabilities ?? self.requiredCapabilities,
            schemas: schemas ?? self.schemas
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum PeerKind: String, Codable, Sendable {
    case diagnosticClient = "diagnosticClient"
    case engine = "engine"
    case server = "server"
    case shell = "shell"
}

// MARK: - SupportedProtocol
public struct SupportedProtocol: Codable, Sendable {
    public let major, maximumMinor, minimumMinor: Int

    public init(major: Int, maximumMinor: Int, minimumMinor: Int) {
        self.major = major
        self.maximumMinor = maximumMinor
        self.minimumMinor = minimumMinor
    }
}

// MARK: SupportedProtocol convenience initializers and mutators

public extension SupportedProtocol {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SupportedProtocol.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        major: Int? = nil,
        maximumMinor: Int? = nil,
        minimumMinor: Int? = nil
    ) -> SupportedProtocol {
        return SupportedProtocol(
            major: major ?? self.major,
            maximumMinor: maximumMinor ?? self.maximumMinor,
            minimumMinor: minimumMinor ?? self.minimumMinor
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SchemaSupport
public struct SchemaSupport: Codable, Sendable {
    public let maximumVersion, minimumVersion: Int
    public let schemaSupportRequired: Bool
    public let schemaID: String

    public enum CodingKeys: String, CodingKey {
        case maximumVersion, minimumVersion
        case schemaSupportRequired = "required"
        case schemaID = "schemaId"
    }

    public init(maximumVersion: Int, minimumVersion: Int, schemaSupportRequired: Bool, schemaID: String) {
        self.maximumVersion = maximumVersion
        self.minimumVersion = minimumVersion
        self.schemaSupportRequired = schemaSupportRequired
        self.schemaID = schemaID
    }
}

// MARK: SchemaSupport convenience initializers and mutators

public extension SchemaSupport {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SchemaSupport.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        maximumVersion: Int? = nil,
        minimumVersion: Int? = nil,
        schemaSupportRequired: Bool? = nil,
        schemaID: String? = nil
    ) -> SchemaSupport {
        return SchemaSupport(
            maximumVersion: maximumVersion ?? self.maximumVersion,
            minimumVersion: minimumVersion ?? self.minimumVersion,
            schemaSupportRequired: schemaSupportRequired ?? self.schemaSupportRequired,
            schemaID: schemaID ?? self.schemaID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

/// Authorized read-only requests.
// MARK: - Query
public struct Query: Codable, Sendable {
    public let kind: QueryKind
    public let payload: [String: JSONAny]

    public init(kind: QueryKind, payload: [String: JSONAny]) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: Query convenience initializers and mutators

public extension Query {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Query.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: QueryKind? = nil,
        payload: [String: JSONAny]? = nil
    ) -> Query {
        return Query(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum QueryKind: String, Codable, Sendable {
    case eitmadConfigGetV1 = "eitmad.config.get.v1"
    case eitmadPermissionsGetEffectiveV1 = "eitmad.permissions.get-effective.v1"
    case eitmadSyncGetStatusV1 = "eitmad.sync.get-status.v1"
    case eitmadUpdateGetStateV1 = "eitmad.update.get-state.v1"
}

/// Resumable streams requested by clients.
// MARK: - Subscription
public struct Subscription: Codable, Sendable {
    public let kind: SubscriptionKind
    public let payload: [String: JSONAny]

    public init(kind: SubscriptionKind, payload: [String: JSONAny]) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: Subscription convenience initializers and mutators

public extension Subscription {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Subscription.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: SubscriptionKind? = nil,
        payload: [String: JSONAny]? = nil
    ) -> Subscription {
        return Subscription(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum SubscriptionKind: String, Codable, Sendable {
    case eitmadBackgroundJobStatusSubscribeV1 = "eitmad.background-job.status.subscribe.v1"
    case eitmadConfigChangedSubscribeV1 = "eitmad.config.changed.subscribe.v1"
    case eitmadErrorSubscribeV1 = "eitmad.error.subscribe.v1"
    case eitmadNotificationSubscribeV1 = "eitmad.notification.subscribe.v1"
    case eitmadPermissionsChangedSubscribeV1 = "eitmad.permissions.changed.subscribe.v1"
    case eitmadRecordChangedSubscribeV1 = "eitmad.record.changed.subscribe.v1"
    case eitmadSyncStatusSubscribeV1 = "eitmad.sync.status.subscribe.v1"
    case eitmadUpdateStateSubscribeV1 = "eitmad.update.state.subscribe.v1"
}

// MARK: - IPCServerMessage
public struct IPCServerMessage: Codable, Sendable {
    public let kind: IPCServerMessageKind
    public let payload: HandshakeResponse

    public init(kind: IPCServerMessageKind, payload: HandshakeResponse) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: IPCServerMessage convenience initializers and mutators

public extension IPCServerMessage {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(IPCServerMessage.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: IPCServerMessageKind? = nil,
        payload: HandshakeResponse? = nil
    ) -> IPCServerMessage {
        return IPCServerMessage(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum IPCServerMessageKind: String, Codable, Sendable {
    case eitmadIPCCommandResponseV1 = "eitmad.ipc.command-response.v1"
    case eitmadIPCEventV1 = "eitmad.ipc.event.v1"
    case eitmadIPCFailureV1 = "eitmad.ipc.failure.v1"
    case eitmadIPCHandshakeResponseV1 = "eitmad.ipc.handshake-response.v1"
    case eitmadIPCQueryResponseV1 = "eitmad.ipc.query-response.v1"
    case eitmadIPCShutdownResponseV1 = "eitmad.ipc.shutdown-response.v1"
    case eitmadIPCSubscribeResponseV1 = "eitmad.ipc.subscribe-response.v1"
    case eitmadIPCSubscriptionClosedV1 = "eitmad.ipc.subscription-closed.v1"
    case eitmadIPCUnsubscribeResponseV1 = "eitmad.ipc.unsubscribe-response.v1"
}

// MARK: - HandshakeResponse
public struct HandshakeResponse: Codable, Sendable {
    public let correlationID: String?
    public let outcome: Outcome?
    public let requestID: String?
    public let accepted: Bool?
    public let subscriptionID, cursor: String?
    public let event: Event?
    public let occurredAt: Int?
    public let sequence: Int?
    public let lastDeliveredCursor: String?
    public let reason: SubscriptionCloseReason?
    public let error: ContractError?

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
        case accepted
        case subscriptionID = "subscriptionId"
        case cursor, event, occurredAt, sequence, lastDeliveredCursor, reason, error
    }

    public init(correlationID: String?, outcome: Outcome?, requestID: String?, accepted: Bool?, subscriptionID: String?, cursor: String?, event: Event?, occurredAt: Int?, sequence: Int?, lastDeliveredCursor: String?, reason: SubscriptionCloseReason?, error: ContractError?) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
        self.accepted = accepted
        self.subscriptionID = subscriptionID
        self.cursor = cursor
        self.event = event
        self.occurredAt = occurredAt
        self.sequence = sequence
        self.lastDeliveredCursor = lastDeliveredCursor
        self.reason = reason
        self.error = error
    }
}

// MARK: HandshakeResponse convenience initializers and mutators

public extension HandshakeResponse {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(HandshakeResponse.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        correlationID: String?? = nil,
        outcome: Outcome?? = nil,
        requestID: String?? = nil,
        accepted: Bool?? = nil,
        subscriptionID: String?? = nil,
        cursor: String?? = nil,
        event: Event?? = nil,
        occurredAt: Int?? = nil,
        sequence: Int?? = nil,
        lastDeliveredCursor: String?? = nil,
        reason: SubscriptionCloseReason?? = nil,
        error: ContractError?? = nil
    ) -> HandshakeResponse {
        return HandshakeResponse(
            correlationID: correlationID ?? self.correlationID,
            outcome: outcome ?? self.outcome,
            requestID: requestID ?? self.requestID,
            accepted: accepted ?? self.accepted,
            subscriptionID: subscriptionID ?? self.subscriptionID,
            cursor: cursor ?? self.cursor,
            event: event ?? self.event,
            occurredAt: occurredAt ?? self.occurredAt,
            sequence: sequence ?? self.sequence,
            lastDeliveredCursor: lastDeliveredCursor ?? self.lastDeliveredCursor,
            reason: reason ?? self.reason,
            error: error ?? self.error
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - Outcome
public struct Outcome: Codable, Sendable {
    public let payload: HandshakeRejection
    public let status: OutcomeStatus

    public init(payload: HandshakeRejection, status: OutcomeStatus) {
        self.payload = payload
        self.status = status
    }
}

// MARK: Outcome convenience initializers and mutators

public extension Outcome {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Outcome.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        payload: HandshakeRejection? = nil,
        status: OutcomeStatus? = nil
    ) -> Outcome {
        return Outcome(
            payload: payload ?? self.payload,
            status: status ?? self.status
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - HandshakeRejection
public struct HandshakeRejection: Codable, Sendable {
    public let authorization: AuthorizationContext?
    public let engine: PeerHello?
    public let negotiated: NegotiatedSession?
    public let kind: TentacledKind?
    public let payload: NegotiationRejection?
    public let code, correlationID: String?
    public let detail: ErrorDetail?
    public let messageID: String?
    public let parameters: [ErrorParameter]?
    public let retry: RetryDisposition?
    public let resumed: Bool?
    public let streamCursor, subscriptionID: String?

    public enum CodingKeys: String, CodingKey {
        case authorization, engine, negotiated, kind, payload, code
        case correlationID = "correlationId"
        case detail
        case messageID = "messageId"
        case parameters, retry, resumed, streamCursor
        case subscriptionID = "subscriptionId"
    }

    public init(authorization: AuthorizationContext?, engine: PeerHello?, negotiated: NegotiatedSession?, kind: TentacledKind?, payload: NegotiationRejection?, code: String?, correlationID: String?, detail: ErrorDetail?, messageID: String?, parameters: [ErrorParameter]?, retry: RetryDisposition?, resumed: Bool?, streamCursor: String?, subscriptionID: String?) {
        self.authorization = authorization
        self.engine = engine
        self.negotiated = negotiated
        self.kind = kind
        self.payload = payload
        self.code = code
        self.correlationID = correlationID
        self.detail = detail
        self.messageID = messageID
        self.parameters = parameters
        self.retry = retry
        self.resumed = resumed
        self.streamCursor = streamCursor
        self.subscriptionID = subscriptionID
    }
}

// MARK: HandshakeRejection convenience initializers and mutators

public extension HandshakeRejection {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(HandshakeRejection.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        authorization: AuthorizationContext?? = nil,
        engine: PeerHello?? = nil,
        negotiated: NegotiatedSession?? = nil,
        kind: TentacledKind?? = nil,
        payload: NegotiationRejection?? = nil,
        code: String?? = nil,
        correlationID: String?? = nil,
        detail: ErrorDetail?? = nil,
        messageID: String?? = nil,
        parameters: [ErrorParameter]?? = nil,
        retry: RetryDisposition?? = nil,
        resumed: Bool?? = nil,
        streamCursor: String?? = nil,
        subscriptionID: String?? = nil
    ) -> HandshakeRejection {
        return HandshakeRejection(
            authorization: authorization ?? self.authorization,
            engine: engine ?? self.engine,
            negotiated: negotiated ?? self.negotiated,
            kind: kind ?? self.kind,
            payload: payload ?? self.payload,
            code: code ?? self.code,
            correlationID: correlationID ?? self.correlationID,
            detail: detail ?? self.detail,
            messageID: messageID ?? self.messageID,
            parameters: parameters ?? self.parameters,
            retry: retry ?? self.retry,
            resumed: resumed ?? self.resumed,
            streamCursor: streamCursor ?? self.streamCursor,
            subscriptionID: subscriptionID ?? self.subscriptionID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum TentacledKind: String, Codable, Sendable {
    case authenticationFailed = "authenticationFailed"
    case authenticationRequired = "authenticationRequired"
    case configuration = "configuration"
    case configurationUpdated = "configurationUpdated"
    case effectivePermissions = "effectivePermissions"
    case installerOutcomeRecorded = "installerOutcomeRecorded"
    case negotiation = "negotiation"
    case operationCancelled = "operationCancelled"
    case syncStatus = "syncStatus"
    case updateState = "updateState"
}

// MARK: - NegotiatedSession
public struct NegotiatedSession: Codable, Sendable {
    public let capabilities: [String]
    public let negotiatedSessionProtocol: ProtocolVersion
    public let schemas: [NegotiatedSchema]

    public enum CodingKeys: String, CodingKey {
        case capabilities
        case negotiatedSessionProtocol = "protocol"
        case schemas
    }

    public init(capabilities: [String], negotiatedSessionProtocol: ProtocolVersion, schemas: [NegotiatedSchema]) {
        self.capabilities = capabilities
        self.negotiatedSessionProtocol = negotiatedSessionProtocol
        self.schemas = schemas
    }
}

// MARK: NegotiatedSession convenience initializers and mutators

public extension NegotiatedSession {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(NegotiatedSession.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        capabilities: [String]? = nil,
        negotiatedSessionProtocol: ProtocolVersion? = nil,
        schemas: [NegotiatedSchema]? = nil
    ) -> NegotiatedSession {
        return NegotiatedSession(
            capabilities: capabilities ?? self.capabilities,
            negotiatedSessionProtocol: negotiatedSessionProtocol ?? self.negotiatedSessionProtocol,
            schemas: schemas ?? self.schemas
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - NegotiatedSchema
public struct NegotiatedSchema: Codable, Sendable {
    public let schemaID: String
    public let version: Int

    public enum CodingKeys: String, CodingKey {
        case schemaID = "schemaId"
        case version
    }

    public init(schemaID: String, version: Int) {
        self.schemaID = schemaID
        self.version = version
    }
}

// MARK: NegotiatedSchema convenience initializers and mutators

public extension NegotiatedSchema {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(NegotiatedSchema.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        schemaID: String? = nil,
        version: Int? = nil
    ) -> NegotiatedSchema {
        return NegotiatedSchema(
            schemaID: schemaID ?? self.schemaID,
            version: version ?? self.version
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - NegotiationRejection
public struct NegotiationRejection: Codable, Sendable {
    public let kind: StickyKind?
    public let payload: FluffyPayload?
    public let entries: [ConfigEntry]?
    public let revision, schemaVersion: Int?
    public let scope: ScopeRef?
    public let operationID: String?
    public let permissions: [EffectivePermission]?
    public let policyVersion: Int?

    public enum CodingKeys: String, CodingKey {
        case kind, payload, entries, revision, schemaVersion, scope
        case operationID = "operation_id"
        case permissions, policyVersion
    }

    public init(kind: StickyKind?, payload: FluffyPayload?, entries: [ConfigEntry]?, revision: Int?, schemaVersion: Int?, scope: ScopeRef?, operationID: String?, permissions: [EffectivePermission]?, policyVersion: Int?) {
        self.kind = kind
        self.payload = payload
        self.entries = entries
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
        self.operationID = operationID
        self.permissions = permissions
        self.policyVersion = policyVersion
    }
}

// MARK: NegotiationRejection convenience initializers and mutators

public extension NegotiationRejection {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(NegotiationRejection.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: StickyKind?? = nil,
        payload: FluffyPayload?? = nil,
        entries: [ConfigEntry]?? = nil,
        revision: Int?? = nil,
        schemaVersion: Int?? = nil,
        scope: ScopeRef?? = nil,
        operationID: String?? = nil,
        permissions: [EffectivePermission]?? = nil,
        policyVersion: Int?? = nil
    ) -> NegotiationRejection {
        return NegotiationRejection(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload,
            entries: entries ?? self.entries,
            revision: revision ?? self.revision,
            schemaVersion: schemaVersion ?? self.schemaVersion,
            scope: scope ?? self.scope,
            operationID: operationID ?? self.operationID,
            permissions: permissions ?? self.permissions,
            policyVersion: policyVersion ?? self.policyVersion
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum StickyKind: String, Codable, Sendable {
    case available = "available"
    case checking = "checking"
    case conflicted = "conflicted"
    case current = "current"
    case downloading = "downloading"
    case failed = "failed"
    case idle = "idle"
    case incompatibleSchema = "incompatibleSchema"
    case installationHandoff = "installationHandoff"
    case installing = "installing"
    case missingCapability = "missingCapability"
    case noCommonProtocol = "noCommonProtocol"
    case offline = "offline"
    case paused = "paused"
    case preflight = "preflight"
    case queued = "queued"
    case ready = "ready"
    case recoveryRequired = "recoveryRequired"
    case revoked = "revoked"
    case succeeded = "succeeded"
    case syncing = "syncing"
    case verifying = "verifying"
}

// MARK: - FluffyPayload
public struct FluffyPayload: Codable, Sendable {
    public let capability: String?
    public let requiredBy: RequiredBy?
    public let schemaID: String?
    public let version: String?
    public let progressBps: Int?
    public let handoffID, errorCode, checkpoint: String?
    public let records, completed: Int?
    public let total: Int?
    public let reason: String?

    public enum CodingKeys: String, CodingKey {
        case capability
        case requiredBy = "required_by"
        case schemaID = "schema_id"
        case version
        case progressBps = "progress_bps"
        case handoffID = "handoff_id"
        case errorCode = "error_code"
        case checkpoint, records, completed, total, reason
    }

    public init(capability: String?, requiredBy: RequiredBy?, schemaID: String?, version: String?, progressBps: Int?, handoffID: String?, errorCode: String?, checkpoint: String?, records: Int?, completed: Int?, total: Int?, reason: String?) {
        self.capability = capability
        self.requiredBy = requiredBy
        self.schemaID = schemaID
        self.version = version
        self.progressBps = progressBps
        self.handoffID = handoffID
        self.errorCode = errorCode
        self.checkpoint = checkpoint
        self.records = records
        self.completed = completed
        self.total = total
        self.reason = reason
    }
}

// MARK: FluffyPayload convenience initializers and mutators

public extension FluffyPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(FluffyPayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        capability: String?? = nil,
        requiredBy: RequiredBy?? = nil,
        schemaID: String?? = nil,
        version: String?? = nil,
        progressBps: Int?? = nil,
        handoffID: String?? = nil,
        errorCode: String?? = nil,
        checkpoint: String?? = nil,
        records: Int?? = nil,
        completed: Int?? = nil,
        total: Int?? = nil,
        reason: String?? = nil
    ) -> FluffyPayload {
        return FluffyPayload(
            capability: capability ?? self.capability,
            requiredBy: requiredBy ?? self.requiredBy,
            schemaID: schemaID ?? self.schemaID,
            version: version ?? self.version,
            progressBps: progressBps ?? self.progressBps,
            handoffID: handoffID ?? self.handoffID,
            errorCode: errorCode ?? self.errorCode,
            checkpoint: checkpoint ?? self.checkpoint,
            records: records ?? self.records,
            completed: completed ?? self.completed,
            total: total ?? self.total,
            reason: reason ?? self.reason
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum RequiredBy: String, Codable, Sendable {
    case local = "local"
    case remote = "remote"
}

public enum OutcomeStatus: String, Codable, Sendable {
    case accepted = "accepted"
    case failed = "failed"
    case rejected = "rejected"
    case succeeded = "succeeded"
}

public enum SubscriptionCloseReason: String, Codable, Sendable {
    case backpressure = "backpressure"
    case clientRequested = "clientRequested"
    case engineStopping = "engineStopping"
}

// MARK: - LifecycleSnapshot
public struct LifecycleSnapshot: Codable, Sendable {
    public let checks: [HealthCheckResult]
    public let error: ContractError?
    public let health: HealthStatus
    public let identity: EngineProcessIdentity
    public let live: Bool
    public let observedAt: Int
    public let ready: Bool
    public let state: LifecycleState

    public init(checks: [HealthCheckResult], error: ContractError?, health: HealthStatus, identity: EngineProcessIdentity, live: Bool, observedAt: Int, ready: Bool, state: LifecycleState) {
        self.checks = checks
        self.error = error
        self.health = health
        self.identity = identity
        self.live = live
        self.observedAt = observedAt
        self.ready = ready
        self.state = state
    }
}

// MARK: LifecycleSnapshot convenience initializers and mutators

public extension LifecycleSnapshot {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(LifecycleSnapshot.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        checks: [HealthCheckResult]? = nil,
        error: ContractError?? = nil,
        health: HealthStatus? = nil,
        identity: EngineProcessIdentity? = nil,
        live: Bool? = nil,
        observedAt: Int? = nil,
        ready: Bool? = nil,
        state: LifecycleState? = nil
    ) -> LifecycleSnapshot {
        return LifecycleSnapshot(
            checks: checks ?? self.checks,
            error: error ?? self.error,
            health: health ?? self.health,
            identity: identity ?? self.identity,
            live: live ?? self.live,
            observedAt: observedAt ?? self.observedAt,
            ready: ready ?? self.ready,
            state: state ?? self.state
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum LifecycleState: String, Codable, Sendable {
    case failed = "failed"
    case ready = "ready"
    case starting = "starting"
    case stopped = "stopped"
    case stopping = "stopping"
}

// MARK: - NegotiationOutcome
public struct NegotiationOutcome: Codable, Sendable {
    public let payload: Negotiation
    public let status: NegotiationOutcomeStatus

    public init(payload: Negotiation, status: NegotiationOutcomeStatus) {
        self.payload = payload
        self.status = status
    }
}

// MARK: NegotiationOutcome convenience initializers and mutators

public extension NegotiationOutcome {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(NegotiationOutcome.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        payload: Negotiation? = nil,
        status: NegotiationOutcomeStatus? = nil
    ) -> NegotiationOutcome {
        return NegotiationOutcome(
            payload: payload ?? self.payload,
            status: status ?? self.status
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - Negotiation
public struct Negotiation: Codable, Sendable {
    public let capabilities: [String]?
    public let negotiationProtocol: ProtocolVersion?
    public let schemas: [NegotiatedSchema]?
    public let kind: IndigoKind?
    public let payload: TentacledPayload?

    public enum CodingKeys: String, CodingKey {
        case capabilities
        case negotiationProtocol = "protocol"
        case schemas, kind, payload
    }

    public init(capabilities: [String]?, negotiationProtocol: ProtocolVersion?, schemas: [NegotiatedSchema]?, kind: IndigoKind?, payload: TentacledPayload?) {
        self.capabilities = capabilities
        self.negotiationProtocol = negotiationProtocol
        self.schemas = schemas
        self.kind = kind
        self.payload = payload
    }
}

// MARK: Negotiation convenience initializers and mutators

public extension Negotiation {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Negotiation.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        capabilities: [String]?? = nil,
        negotiationProtocol: ProtocolVersion?? = nil,
        schemas: [NegotiatedSchema]?? = nil,
        kind: IndigoKind?? = nil,
        payload: TentacledPayload?? = nil
    ) -> Negotiation {
        return Negotiation(
            capabilities: capabilities ?? self.capabilities,
            negotiationProtocol: negotiationProtocol ?? self.negotiationProtocol,
            schemas: schemas ?? self.schemas,
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum IndigoKind: String, Codable, Sendable {
    case incompatibleSchema = "incompatibleSchema"
    case missingCapability = "missingCapability"
    case noCommonProtocol = "noCommonProtocol"
}

// MARK: - TentacledPayload
public struct TentacledPayload: Codable, Sendable {
    public let capability: String?
    public let requiredBy: RequiredBy?
    public let schemaID: String?

    public enum CodingKeys: String, CodingKey {
        case capability
        case requiredBy = "required_by"
        case schemaID = "schema_id"
    }

    public init(capability: String?, requiredBy: RequiredBy?, schemaID: String?) {
        self.capability = capability
        self.requiredBy = requiredBy
        self.schemaID = schemaID
    }
}

// MARK: TentacledPayload convenience initializers and mutators

public extension TentacledPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(TentacledPayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        capability: String?? = nil,
        requiredBy: RequiredBy?? = nil,
        schemaID: String?? = nil
    ) -> TentacledPayload {
        return TentacledPayload(
            capability: capability ?? self.capability,
            requiredBy: requiredBy ?? self.requiredBy,
            schemaID: schemaID ?? self.schemaID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum NegotiationOutcomeStatus: String, Codable, Sendable {
    case accepted = "accepted"
    case rejected = "rejected"
}

// MARK: - QueryEnvelope
public struct QueryEnvelope: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let causationID: String?
    public let correlationID: String
    public let deadline: Int
    public let protocolVersion: ProtocolVersion
    public let query: Query
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case authorization
        case causationID = "causationId"
        case correlationID = "correlationId"
        case deadline, protocolVersion, query
        case requestID = "requestId"
    }

    public init(authorization: AuthorizationContext, causationID: String?, correlationID: String, deadline: Int, protocolVersion: ProtocolVersion, query: Query, requestID: String) {
        self.authorization = authorization
        self.causationID = causationID
        self.correlationID = correlationID
        self.deadline = deadline
        self.protocolVersion = protocolVersion
        self.query = query
        self.requestID = requestID
    }
}

// MARK: QueryEnvelope convenience initializers and mutators

public extension QueryEnvelope {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(QueryEnvelope.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        authorization: AuthorizationContext? = nil,
        causationID: String?? = nil,
        correlationID: String? = nil,
        deadline: Int? = nil,
        protocolVersion: ProtocolVersion? = nil,
        query: Query? = nil,
        requestID: String? = nil
    ) -> QueryEnvelope {
        return QueryEnvelope(
            authorization: authorization ?? self.authorization,
            causationID: causationID ?? self.causationID,
            correlationID: correlationID ?? self.correlationID,
            deadline: deadline ?? self.deadline,
            protocolVersion: protocolVersion ?? self.protocolVersion,
            query: query ?? self.query,
            requestID: requestID ?? self.requestID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - QueryResponseEnvelope
public struct QueryResponseEnvelope: Codable, Sendable {
    public let correlationID: String
    public let outcome: QueryOutcome
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
    }

    public init(correlationID: String, outcome: QueryOutcome, requestID: String) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
    }
}

// MARK: QueryResponseEnvelope convenience initializers and mutators

public extension QueryResponseEnvelope {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(QueryResponseEnvelope.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        correlationID: String? = nil,
        outcome: QueryOutcome? = nil,
        requestID: String? = nil
    ) -> QueryResponseEnvelope {
        return QueryResponseEnvelope(
            correlationID: correlationID ?? self.correlationID,
            outcome: outcome ?? self.outcome,
            requestID: requestID ?? self.requestID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - QueryOutcome
public struct QueryOutcome: Codable, Sendable {
    public let payload: QueryResult
    public let status: CommandOutcomeStatus

    public init(payload: QueryResult, status: CommandOutcomeStatus) {
        self.payload = payload
        self.status = status
    }
}

// MARK: QueryOutcome convenience initializers and mutators

public extension QueryOutcome {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(QueryOutcome.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        payload: QueryResult? = nil,
        status: CommandOutcomeStatus? = nil
    ) -> QueryOutcome {
        return QueryOutcome(
            payload: payload ?? self.payload,
            status: status ?? self.status
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - QueryResult
public struct QueryResult: Codable, Sendable {
    public let kind: IndecentKind?
    public let payload: FluffyUpdateState?
    public let code, correlationID: String?
    public let detail: ErrorDetail?
    public let messageID: String?
    public let parameters: [ErrorParameter]?
    public let retry: RetryDisposition?

    public enum CodingKeys: String, CodingKey {
        case kind, payload, code
        case correlationID = "correlationId"
        case detail
        case messageID = "messageId"
        case parameters, retry
    }

    public init(kind: IndecentKind?, payload: FluffyUpdateState?, code: String?, correlationID: String?, detail: ErrorDetail?, messageID: String?, parameters: [ErrorParameter]?, retry: RetryDisposition?) {
        self.kind = kind
        self.payload = payload
        self.code = code
        self.correlationID = correlationID
        self.detail = detail
        self.messageID = messageID
        self.parameters = parameters
        self.retry = retry
    }
}

// MARK: QueryResult convenience initializers and mutators

public extension QueryResult {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(QueryResult.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: IndecentKind?? = nil,
        payload: FluffyUpdateState?? = nil,
        code: String?? = nil,
        correlationID: String?? = nil,
        detail: ErrorDetail?? = nil,
        messageID: String?? = nil,
        parameters: [ErrorParameter]?? = nil,
        retry: RetryDisposition?? = nil
    ) -> QueryResult {
        return QueryResult(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload,
            code: code ?? self.code,
            correlationID: correlationID ?? self.correlationID,
            detail: detail ?? self.detail,
            messageID: messageID ?? self.messageID,
            parameters: parameters ?? self.parameters,
            retry: retry ?? self.retry
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum IndecentKind: String, Codable, Sendable {
    case configuration = "configuration"
    case effectivePermissions = "effectivePermissions"
    case syncStatus = "syncStatus"
    case updateState = "updateState"
}

// MARK: - FluffyUpdateState
public struct FluffyUpdateState: Codable, Sendable {
    public let entries: [ConfigEntry]?
    public let revision, schemaVersion: Int?
    public let scope: ScopeRef?
    public let permissions: [EffectivePermission]?
    public let policyVersion: Int?
    public let kind: FluffyKind?
    public let payload: PurplePayload?

    public init(entries: [ConfigEntry]?, revision: Int?, schemaVersion: Int?, scope: ScopeRef?, permissions: [EffectivePermission]?, policyVersion: Int?, kind: FluffyKind?, payload: PurplePayload?) {
        self.entries = entries
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
        self.permissions = permissions
        self.policyVersion = policyVersion
        self.kind = kind
        self.payload = payload
    }
}

// MARK: FluffyUpdateState convenience initializers and mutators

public extension FluffyUpdateState {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(FluffyUpdateState.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        entries: [ConfigEntry]?? = nil,
        revision: Int?? = nil,
        schemaVersion: Int?? = nil,
        scope: ScopeRef?? = nil,
        permissions: [EffectivePermission]?? = nil,
        policyVersion: Int?? = nil,
        kind: FluffyKind?? = nil,
        payload: PurplePayload?? = nil
    ) -> FluffyUpdateState {
        return FluffyUpdateState(
            entries: entries ?? self.entries,
            revision: revision ?? self.revision,
            schemaVersion: schemaVersion ?? self.schemaVersion,
            scope: scope ?? self.scope,
            permissions: permissions ?? self.permissions,
            policyVersion: policyVersion ?? self.policyVersion,
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SubscriptionEnvelope
public struct SubscriptionEnvelope: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let correlationID: String
    public let protocolVersion: ProtocolVersion
    public let requestID: String
    public let resumeAfter: String?
    public let subscription: Subscription

    public enum CodingKeys: String, CodingKey {
        case authorization
        case correlationID = "correlationId"
        case protocolVersion
        case requestID = "requestId"
        case resumeAfter, subscription
    }

    public init(authorization: AuthorizationContext, correlationID: String, protocolVersion: ProtocolVersion, requestID: String, resumeAfter: String?, subscription: Subscription) {
        self.authorization = authorization
        self.correlationID = correlationID
        self.protocolVersion = protocolVersion
        self.requestID = requestID
        self.resumeAfter = resumeAfter
        self.subscription = subscription
    }
}

// MARK: SubscriptionEnvelope convenience initializers and mutators

public extension SubscriptionEnvelope {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SubscriptionEnvelope.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        authorization: AuthorizationContext? = nil,
        correlationID: String? = nil,
        protocolVersion: ProtocolVersion? = nil,
        requestID: String? = nil,
        resumeAfter: String?? = nil,
        subscription: Subscription? = nil
    ) -> SubscriptionEnvelope {
        return SubscriptionEnvelope(
            authorization: authorization ?? self.authorization,
            correlationID: correlationID ?? self.correlationID,
            protocolVersion: protocolVersion ?? self.protocolVersion,
            requestID: requestID ?? self.requestID,
            resumeAfter: resumeAfter ?? self.resumeAfter,
            subscription: subscription ?? self.subscription
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SyncMessage
public struct SyncMessage: Codable, Sendable {
    public let kind: SyncMessageKind
    public let payload: SyncNegotiation

    public init(kind: SyncMessageKind, payload: SyncNegotiation) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: SyncMessage convenience initializers and mutators

public extension SyncMessage {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SyncMessage.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: SyncMessageKind? = nil,
        payload: SyncNegotiation? = nil
    ) -> SyncMessage {
        return SyncMessage(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum SyncMessageKind: String, Codable, Sendable {
    case eitmadSyncAcknowledgeV1 = "eitmad.sync.acknowledge.v1"
    case eitmadSyncBackpressureV1 = "eitmad.sync.backpressure.v1"
    case eitmadSyncChangesV1 = "eitmad.sync.changes.v1"
    case eitmadSyncConflictV1 = "eitmad.sync.conflict.v1"
    case eitmadSyncNegotiateV1 = "eitmad.sync.negotiate.v1"
    case eitmadSyncPullV1 = "eitmad.sync.pull.v1"
}

// MARK: - SyncNegotiation
public struct SyncNegotiation: Codable, Sendable {
    public let checkpoint: String?
    public let mode: SyncMode?
    public let schemas: [String]?
    public let after: String?
    public let maximumRecords: Int?
    public let fromCheckpoint: String?
    public let hasMore: Bool?
    public let records: [ChangeRecord]?
    public let acceptedRecords, localRevision: Int?
    public let recordID: String?
    public let remoteRevision, delayMS: Int?
    public let reason: String?

    public enum CodingKeys: String, CodingKey {
        case checkpoint, mode, schemas, after, maximumRecords, fromCheckpoint, hasMore, records, acceptedRecords, localRevision
        case recordID = "recordId"
        case remoteRevision
        case delayMS = "delayMs"
        case reason
    }

    public init(checkpoint: String?, mode: SyncMode?, schemas: [String]?, after: String?, maximumRecords: Int?, fromCheckpoint: String?, hasMore: Bool?, records: [ChangeRecord]?, acceptedRecords: Int?, localRevision: Int?, recordID: String?, remoteRevision: Int?, delayMS: Int?, reason: String?) {
        self.checkpoint = checkpoint
        self.mode = mode
        self.schemas = schemas
        self.after = after
        self.maximumRecords = maximumRecords
        self.fromCheckpoint = fromCheckpoint
        self.hasMore = hasMore
        self.records = records
        self.acceptedRecords = acceptedRecords
        self.localRevision = localRevision
        self.recordID = recordID
        self.remoteRevision = remoteRevision
        self.delayMS = delayMS
        self.reason = reason
    }
}

// MARK: SyncNegotiation convenience initializers and mutators

public extension SyncNegotiation {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SyncNegotiation.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        checkpoint: String?? = nil,
        mode: SyncMode?? = nil,
        schemas: [String]?? = nil,
        after: String?? = nil,
        maximumRecords: Int?? = nil,
        fromCheckpoint: String?? = nil,
        hasMore: Bool?? = nil,
        records: [ChangeRecord]?? = nil,
        acceptedRecords: Int?? = nil,
        localRevision: Int?? = nil,
        recordID: String?? = nil,
        remoteRevision: Int?? = nil,
        delayMS: Int?? = nil,
        reason: String?? = nil
    ) -> SyncNegotiation {
        return SyncNegotiation(
            checkpoint: checkpoint ?? self.checkpoint,
            mode: mode ?? self.mode,
            schemas: schemas ?? self.schemas,
            after: after ?? self.after,
            maximumRecords: maximumRecords ?? self.maximumRecords,
            fromCheckpoint: fromCheckpoint ?? self.fromCheckpoint,
            hasMore: hasMore ?? self.hasMore,
            records: records ?? self.records,
            acceptedRecords: acceptedRecords ?? self.acceptedRecords,
            localRevision: localRevision ?? self.localRevision,
            recordID: recordID ?? self.recordID,
            remoteRevision: remoteRevision ?? self.remoteRevision,
            delayMS: delayMS ?? self.delayMS,
            reason: reason ?? self.reason
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum SyncMode: String, Codable, Sendable {
    case localFirst = "localFirst"
    case serverAuthoritative = "serverAuthoritative"
}

// MARK: - ChangeRecord
public struct ChangeRecord: Codable, Sendable {
    public let changedAt: Int
    public let operation: ChangeOperation
    public let payload: EncodedDomainPayload?
    public let recordID: String
    public let revision: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case changedAt, operation, payload
        case recordID = "recordId"
        case revision, scope
    }

    public init(changedAt: Int, operation: ChangeOperation, payload: EncodedDomainPayload?, recordID: String, revision: Int, scope: ScopeRef) {
        self.changedAt = changedAt
        self.operation = operation
        self.payload = payload
        self.recordID = recordID
        self.revision = revision
        self.scope = scope
    }
}

// MARK: ChangeRecord convenience initializers and mutators

public extension ChangeRecord {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ChangeRecord.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        changedAt: Int? = nil,
        operation: ChangeOperation? = nil,
        payload: EncodedDomainPayload?? = nil,
        recordID: String? = nil,
        revision: Int? = nil,
        scope: ScopeRef? = nil
    ) -> ChangeRecord {
        return ChangeRecord(
            changedAt: changedAt ?? self.changedAt,
            operation: operation ?? self.operation,
            payload: payload ?? self.payload,
            recordID: recordID ?? self.recordID,
            revision: revision ?? self.revision,
            scope: scope ?? self.scope
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - EncodedDomainPayload
public struct EncodedDomainPayload: Codable, Sendable {
    public let base64, schemaID: String
    public let schemaVersion: Int

    public enum CodingKeys: String, CodingKey {
        case base64
        case schemaID = "schemaId"
        case schemaVersion
    }

    public init(base64: String, schemaID: String, schemaVersion: Int) {
        self.base64 = base64
        self.schemaID = schemaID
        self.schemaVersion = schemaVersion
    }
}

// MARK: EncodedDomainPayload convenience initializers and mutators

public extension EncodedDomainPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(EncodedDomainPayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        base64: String? = nil,
        schemaID: String? = nil,
        schemaVersion: Int? = nil
    ) -> EncodedDomainPayload {
        return EncodedDomainPayload(
            base64: base64 ?? self.base64,
            schemaID: schemaID ?? self.schemaID,
            schemaVersion: schemaVersion ?? self.schemaVersion
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SyncStatus
public struct SyncStatus: Codable, Sendable {
    public let kind: SyncStatusKind
    public let payload: SyncStatusPayload?

    public init(kind: SyncStatusKind, payload: SyncStatusPayload?) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: SyncStatus convenience initializers and mutators

public extension SyncStatus {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SyncStatus.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: SyncStatusKind? = nil,
        payload: SyncStatusPayload?? = nil
    ) -> SyncStatus {
        return SyncStatus(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum SyncStatusKind: String, Codable, Sendable {
    case conflicted = "conflicted"
    case current = "current"
    case failed = "failed"
    case offline = "offline"
    case queued = "queued"
    case syncing = "syncing"
}

// MARK: - SyncStatusPayload
public struct SyncStatusPayload: Codable, Sendable {
    public let checkpoint: String?
    public let records, completed: Int?
    public let total: Int?
    public let reason: String?

    public init(checkpoint: String?, records: Int?, completed: Int?, total: Int?, reason: String?) {
        self.checkpoint = checkpoint
        self.records = records
        self.completed = completed
        self.total = total
        self.reason = reason
    }
}

// MARK: SyncStatusPayload convenience initializers and mutators

public extension SyncStatusPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SyncStatusPayload.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        checkpoint: String?? = nil,
        records: Int?? = nil,
        completed: Int?? = nil,
        total: Int?? = nil,
        reason: String?? = nil
    ) -> SyncStatusPayload {
        return SyncStatusPayload(
            checkpoint: checkpoint ?? self.checkpoint,
            records: records ?? self.records,
            completed: completed ?? self.completed,
            total: total ?? self.total,
            reason: reason ?? self.reason
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - UpdateState
public struct UpdateState: Codable, Sendable {
    public let kind: UpdateStateKind
    public let payload: UpdateStatePayload?

    public init(kind: UpdateStateKind, payload: UpdateStatePayload?) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: UpdateState convenience initializers and mutators

public extension UpdateState {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(UpdateState.self, from: data)
    }

    init(_ json: String, using encoding: String.Encoding = .utf8) throws {
        guard let data = json.data(using: encoding) else {
            throw NSError(domain: "JSONDecoding", code: 0, userInfo: nil)
        }
        try self.init(data: data)
    }

    init(fromURL url: URL) throws {
        try self.init(data: try Data(contentsOf: url))
    }

    func with(
        kind: UpdateStateKind? = nil,
        payload: UpdateStatePayload?? = nil
    ) -> UpdateState {
        return UpdateState(
            kind: kind ?? self.kind,
            payload: payload ?? self.payload
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - Helper functions for creating encoders and decoders

func newJSONDecoder() -> JSONDecoder {
    let decoder = JSONDecoder()
    if #available(iOS 10.0, OSX 10.12, tvOS 10.0, watchOS 3.0, *) {
        decoder.dateDecodingStrategy = .iso8601
    }
    return decoder
}

func newJSONEncoder() -> JSONEncoder {
    let encoder = JSONEncoder()
    if #available(iOS 10.0, OSX 10.12, tvOS 10.0, watchOS 3.0, *) {
        encoder.dateEncodingStrategy = .iso8601
    }
    return encoder
}

// MARK: - Encode/decode helpers

public class JSONNull: Codable, Hashable {

    public static func == (lhs: JSONNull, rhs: JSONNull) -> Bool {
            return true
    }

    public var hashValue: Int {
            return 0
    }

    public func hash(into hasher: inout Hasher) {
            // No-op
    }

    public init() {}

    public required init(from decoder: Decoder) throws {
            let container = try decoder.singleValueContainer()
            if !container.decodeNil() {
                    throw DecodingError.typeMismatch(JSONNull.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Wrong type for JSONNull"))
            }
    }

    public func encode(to encoder: Encoder) throws {
            var container = encoder.singleValueContainer()
            try container.encodeNil()
    }
}

class JSONCodingKey: CodingKey {
    let key: String

    required init?(intValue: Int) {
            return nil
    }

    required init?(stringValue: String) {
            key = stringValue
    }

    var intValue: Int? {
            return nil
    }

    var stringValue: String {
            return key
    }
}

public class JSONAny: Codable {

    public let value: Any

    static func decodingError(forCodingPath codingPath: [CodingKey]) -> DecodingError {
            let context = DecodingError.Context(codingPath: codingPath, debugDescription: "Cannot decode JSONAny")
            return DecodingError.typeMismatch(JSONAny.self, context)
    }

    static func encodingError(forValue value: Any, codingPath: [CodingKey]) -> EncodingError {
            let context = EncodingError.Context(codingPath: codingPath, debugDescription: "Cannot encode JSONAny")
            return EncodingError.invalidValue(value, context)
    }

    static func decode(from container: SingleValueDecodingContainer) throws -> Any {
            if let value = try? container.decode(Bool.self) {
                    return value
            }
            if let value = try? container.decode(Int64.self) {
                    return value
            }
            if let value = try? container.decode(Double.self) {
                    return value
            }
            if let value = try? container.decode(String.self) {
                    return value
            }
            if container.decodeNil() {
                    return JSONNull()
            }
            throw decodingError(forCodingPath: container.codingPath)
    }

    static func decode(from container: inout UnkeyedDecodingContainer) throws -> Any {
            if let value = try? container.decode(Bool.self) {
                    return value
            }
            if let value = try? container.decode(Int64.self) {
                    return value
            }
            if let value = try? container.decode(Double.self) {
                    return value
            }
            if let value = try? container.decode(String.self) {
                    return value
            }
            if let value = try? container.decodeNil() {
                    if value {
                            return JSONNull()
                    }
            }
            if var container = try? container.nestedUnkeyedContainer() {
                    return try decodeArray(from: &container)
            }
            if var container = try? container.nestedContainer(keyedBy: JSONCodingKey.self) {
                    return try decodeDictionary(from: &container)
            }
            throw decodingError(forCodingPath: container.codingPath)
    }

    static func decode(from container: inout KeyedDecodingContainer<JSONCodingKey>, forKey key: JSONCodingKey) throws -> Any {
            if let value = try? container.decode(Bool.self, forKey: key) {
                    return value
            }
            if let value = try? container.decode(Int64.self, forKey: key) {
                    return value
            }
            if let value = try? container.decode(Double.self, forKey: key) {
                    return value
            }
            if let value = try? container.decode(String.self, forKey: key) {
                    return value
            }
            if let value = try? container.decodeNil(forKey: key) {
                    if value {
                            return JSONNull()
                    }
            }
            if var container = try? container.nestedUnkeyedContainer(forKey: key) {
                    return try decodeArray(from: &container)
            }
            if var container = try? container.nestedContainer(keyedBy: JSONCodingKey.self, forKey: key) {
                    return try decodeDictionary(from: &container)
            }
            throw decodingError(forCodingPath: container.codingPath)
    }

    static func decodeArray(from container: inout UnkeyedDecodingContainer) throws -> [Any] {
            var arr: [Any] = []
            while !container.isAtEnd {
                    let value = try decode(from: &container)
                    arr.append(value)
            }
            return arr
    }

    static func decodeDictionary(from container: inout KeyedDecodingContainer<JSONCodingKey>) throws -> [String: Any] {
            var dict = [String: Any]()
            for key in container.allKeys {
                    let value = try decode(from: &container, forKey: key)
                    dict[key.stringValue] = value
            }
            return dict
    }

    static func encode(to container: inout UnkeyedEncodingContainer, array: [Any]) throws {
            for value in array {
                    if let value = value as? Bool {
                            try container.encode(value)
                    } else if let value = value as? Int64 {
                            try container.encode(value)
                    } else if let value = value as? Double {
                            try container.encode(value)
                    } else if let value = value as? String {
                            try container.encode(value)
                    } else if value is JSONNull {
                            try container.encodeNil()
                    } else if let value = value as? [Any] {
                            var container = container.nestedUnkeyedContainer()
                            try encode(to: &container, array: value)
                    } else if let value = value as? [String: Any] {
                            var container = container.nestedContainer(keyedBy: JSONCodingKey.self)
                            try encode(to: &container, dictionary: value)
                    } else {
                            throw encodingError(forValue: value, codingPath: container.codingPath)
                    }
            }
    }

    static func encode(to container: inout KeyedEncodingContainer<JSONCodingKey>, dictionary: [String: Any]) throws {
            for (key, value) in dictionary {
                    let key = JSONCodingKey(stringValue: key)!
                    if let value = value as? Bool {
                            try container.encode(value, forKey: key)
                    } else if let value = value as? Int64 {
                            try container.encode(value, forKey: key)
                    } else if let value = value as? Double {
                            try container.encode(value, forKey: key)
                    } else if let value = value as? String {
                            try container.encode(value, forKey: key)
                    } else if value is JSONNull {
                            try container.encodeNil(forKey: key)
                    } else if let value = value as? [Any] {
                            var container = container.nestedUnkeyedContainer(forKey: key)
                            try encode(to: &container, array: value)
                    } else if let value = value as? [String: Any] {
                            var container = container.nestedContainer(keyedBy: JSONCodingKey.self, forKey: key)
                            try encode(to: &container, dictionary: value)
                    } else {
                            throw encodingError(forValue: value, codingPath: container.codingPath)
                    }
            }
    }

    static func encode(to container: inout SingleValueEncodingContainer, value: Any) throws {
            if let value = value as? Bool {
                    try container.encode(value)
            } else if let value = value as? Int64 {
                    try container.encode(value)
            } else if let value = value as? Double {
                    try container.encode(value)
            } else if let value = value as? String {
                    try container.encode(value)
            } else if value is JSONNull {
                    try container.encodeNil()
            } else {
                    throw encodingError(forValue: value, codingPath: container.codingPath)
            }
    }

    public required init(from decoder: Decoder) throws {
            if var arrayContainer = try? decoder.unkeyedContainer() {
                    self.value = try JSONAny.decodeArray(from: &arrayContainer)
            } else if var container = try? decoder.container(keyedBy: JSONCodingKey.self) {
                    self.value = try JSONAny.decodeDictionary(from: &container)
            } else {
                    let container = try decoder.singleValueContainer()
                    self.value = try JSONAny.decode(from: container)
            }
    }

    public func encode(to encoder: Encoder) throws {
            if let arr = self.value as? [Any] {
                    var container = encoder.unkeyedContainer()
                    try JSONAny.encode(to: &container, array: arr)
            } else if let dict = self.value as? [String: Any] {
                    var container = encoder.container(keyedBy: JSONCodingKey.self)
                    try JSONAny.encode(to: &container, dictionary: dict)
            } else {
                    var container = encoder.singleValueContainer()
                    try JSONAny.encode(to: &container, value: self.value)
            }
    }
}
