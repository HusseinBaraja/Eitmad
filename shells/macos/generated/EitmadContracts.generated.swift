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
    public let ipcClientMessage, ipcServerMessage: [String: JSONAny]
    public let lifecycleSnapshot: LifecycleSnapshot
    public let negotiation: NegotiationOutcome
    public let observationClassification: DataClassification
    public let observationComponentID, observationEventID, observationFieldName: String
    public let observationSeverity: ObservationSeverity
    public let observationValueKind: ObservationValueKind
    public let peerHello: PeerHello
    public let queryRequest: QueryEnvelope
    public let queryResponse: QueryResponseEnvelope
    public let serverClientMessage, serverMessage: [String: JSONAny]
    public let subscriptionRequest: SubscriptionEnvelope
    public let syncMessage: [String: JSONAny]
    public let syncStatus: SyncStatus
    public let syncTransportFrame: SyncTransportFrame
    public let unionPayloadKeepAlive: UnionPayloadKeepAlive
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
        case observationClassification = "observation_classification"
        case observationComponentID = "observation_component_id"
        case observationEventID = "observation_event_id"
        case observationFieldName = "observation_field_name"
        case observationSeverity = "observation_severity"
        case observationValueKind = "observation_value_kind"
        case peerHello = "peer_hello"
        case queryRequest = "query_request"
        case queryResponse = "query_response"
        case serverClientMessage = "server_client_message"
        case serverMessage = "server_message"
        case subscriptionRequest = "subscription_request"
        case syncMessage = "sync_message"
        case syncStatus = "sync_status"
        case syncTransportFrame = "sync_transport_frame"
        case unionPayloadKeepAlive
        case updateState = "update_state"
    }

    public init(catalog: ProtocolCatalog, commandRequest: CommandEnvelope, commandResponse: CommandResponseEnvelope, diagnosticReport: DiagnosticReport, effectivePermissions: EffectivePermissions, event: EventEnvelope, ipcClientMessage: [String: JSONAny], ipcServerMessage: [String: JSONAny], lifecycleSnapshot: LifecycleSnapshot, negotiation: NegotiationOutcome, observationClassification: DataClassification, observationComponentID: String, observationEventID: String, observationFieldName: String, observationSeverity: ObservationSeverity, observationValueKind: ObservationValueKind, peerHello: PeerHello, queryRequest: QueryEnvelope, queryResponse: QueryResponseEnvelope, serverClientMessage: [String: JSONAny], serverMessage: [String: JSONAny], subscriptionRequest: SubscriptionEnvelope, syncMessage: [String: JSONAny], syncStatus: SyncStatus, syncTransportFrame: SyncTransportFrame, unionPayloadKeepAlive: UnionPayloadKeepAlive, updateState: UpdateState) {
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
        self.observationClassification = observationClassification
        self.observationComponentID = observationComponentID
        self.observationEventID = observationEventID
        self.observationFieldName = observationFieldName
        self.observationSeverity = observationSeverity
        self.observationValueKind = observationValueKind
        self.peerHello = peerHello
        self.queryRequest = queryRequest
        self.queryResponse = queryResponse
        self.serverClientMessage = serverClientMessage
        self.serverMessage = serverMessage
        self.subscriptionRequest = subscriptionRequest
        self.syncMessage = syncMessage
        self.syncStatus = syncStatus
        self.syncTransportFrame = syncTransportFrame
        self.unionPayloadKeepAlive = unionPayloadKeepAlive
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
        ipcClientMessage: [String: JSONAny]? = nil,
        ipcServerMessage: [String: JSONAny]? = nil,
        lifecycleSnapshot: LifecycleSnapshot? = nil,
        negotiation: NegotiationOutcome? = nil,
        observationClassification: DataClassification? = nil,
        observationComponentID: String? = nil,
        observationEventID: String? = nil,
        observationFieldName: String? = nil,
        observationSeverity: ObservationSeverity? = nil,
        observationValueKind: ObservationValueKind? = nil,
        peerHello: PeerHello? = nil,
        queryRequest: QueryEnvelope? = nil,
        queryResponse: QueryResponseEnvelope? = nil,
        serverClientMessage: [String: JSONAny]? = nil,
        serverMessage: [String: JSONAny]? = nil,
        subscriptionRequest: SubscriptionEnvelope? = nil,
        syncMessage: [String: JSONAny]? = nil,
        syncStatus: SyncStatus? = nil,
        syncTransportFrame: SyncTransportFrame? = nil,
        unionPayloadKeepAlive: UnionPayloadKeepAlive? = nil,
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
            observationClassification: observationClassification ?? self.observationClassification,
            observationComponentID: observationComponentID ?? self.observationComponentID,
            observationEventID: observationEventID ?? self.observationEventID,
            observationFieldName: observationFieldName ?? self.observationFieldName,
            observationSeverity: observationSeverity ?? self.observationSeverity,
            observationValueKind: observationValueKind ?? self.observationValueKind,
            peerHello: peerHello ?? self.peerHello,
            queryRequest: queryRequest ?? self.queryRequest,
            queryResponse: queryResponse ?? self.queryResponse,
            serverClientMessage: serverClientMessage ?? self.serverClientMessage,
            serverMessage: serverMessage ?? self.serverMessage,
            subscriptionRequest: subscriptionRequest ?? self.subscriptionRequest,
            syncMessage: syncMessage ?? self.syncMessage,
            syncStatus: syncStatus ?? self.syncStatus,
            syncTransportFrame: syncTransportFrame ?? self.syncTransportFrame,
            unionPayloadKeepAlive: unionPayloadKeepAlive ?? self.unionPayloadKeepAlive,
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
    public let permissions, queries, relations, schemaIDS: [String]
    public let serverMessages, subscriptions, syncMessages: [String]

    public enum CodingKeys: String, CodingKey {
        case capabilities, commands, configKeys, errorCodes, errorParameterNames, events, ipcMessages
        case messageIDS = "messageIds"
        case permissions, queries, relations
        case schemaIDS = "schemaIds"
        case serverMessages, subscriptions, syncMessages
    }

    public init(capabilities: [String], commands: [String], configKeys: [String], errorCodes: [String], errorParameterNames: [String], events: [String], ipcMessages: [String], messageIDS: [String], permissions: [String], queries: [String], relations: [String], schemaIDS: [String], serverMessages: [String], subscriptions: [String], syncMessages: [String]) {
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
        self.relations = relations
        self.schemaIDS = schemaIDS
        self.serverMessages = serverMessages
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
        relations: [String]? = nil,
        schemaIDS: [String]? = nil,
        serverMessages: [String]? = nil,
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
            relations: relations ?? self.relations,
            schemaIDS: schemaIDS ?? self.schemaIDS,
            serverMessages: serverMessages ?? self.serverMessages,
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
    public let command: [String: JSONAny]
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

    public init(authorization: AuthorizationContext, causationID: String?, command: [String: JSONAny], correlationID: String, deadline: Int, idempotencyKey: String, protocolVersion: ProtocolVersion, requestID: String) {
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
        command: [String: JSONAny]? = nil,
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
    public let sessionID, tenantID: String
    public let workspaceID: String?

    public enum CodingKeys: String, CodingKey {
        case identity, scope
        case sessionID = "sessionId"
        case tenantID = "tenantId"
        case workspaceID = "workspaceId"
    }

    public init(identity: AuthenticatedIdentity, scope: ScopeRef, sessionID: String, tenantID: String, workspaceID: String?) {
        self.identity = identity
        self.scope = scope
        self.sessionID = sessionID
        self.tenantID = tenantID
        self.workspaceID = workspaceID
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
        sessionID: String? = nil,
        tenantID: String? = nil,
        workspaceID: String?? = nil
    ) -> AuthorizationContext {
        return AuthorizationContext(
            identity: identity ?? self.identity,
            scope: scope ?? self.scope,
            sessionID: sessionID ?? self.sessionID,
            tenantID: tenantID ?? self.tenantID,
            workspaceID: workspaceID ?? self.workspaceID
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
    public let payload: PayloadClass?
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

    public init(kind: PurpleKind?, payload: PayloadClass?, code: String?, correlationID: String?, detail: ErrorDetail?, messageID: String?, parameters: [ErrorParameter]?, retry: RetryDisposition?) {
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
        payload: PayloadClass?? = nil,
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
    case relationshipGranted = "relationshipGranted"
    case relationshipRevoked = "relationshipRevoked"
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

// MARK: - PayloadClass
public struct PayloadClass: Codable, Sendable {
    public let entries: [ConfigEntry]?
    public let revision, schemaVersion: Int?
    public let scope: ScopeRef?
    public let changed: Bool?
    public let policyVersion: Int?
    public let relationship: ScopeRelationship?
    public let operationID: String?
    public let kind: UpdateStateKind?
    public let payload: UpdateStatePayload?

    public enum CodingKeys: String, CodingKey {
        case entries, revision, schemaVersion, scope, changed, policyVersion, relationship
        case operationID = "operation_id"
        case kind, payload
    }

    public init(entries: [ConfigEntry]?, revision: Int?, schemaVersion: Int?, scope: ScopeRef?, changed: Bool?, policyVersion: Int?, relationship: ScopeRelationship?, operationID: String?, kind: UpdateStateKind?, payload: UpdateStatePayload?) {
        self.entries = entries
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
        self.changed = changed
        self.policyVersion = policyVersion
        self.relationship = relationship
        self.operationID = operationID
        self.kind = kind
        self.payload = payload
    }
}

// MARK: PayloadClass convenience initializers and mutators

public extension PayloadClass {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(PayloadClass.self, from: data)
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
        changed: Bool?? = nil,
        policyVersion: Int?? = nil,
        relationship: ScopeRelationship?? = nil,
        operationID: String?? = nil,
        kind: UpdateStateKind?? = nil,
        payload: UpdateStatePayload?? = nil
    ) -> PayloadClass {
        return PayloadClass(
            entries: entries ?? self.entries,
            revision: revision ?? self.revision,
            schemaVersion: schemaVersion ?? self.schemaVersion,
            scope: scope ?? self.scope,
            changed: changed ?? self.changed,
            policyVersion: policyVersion ?? self.policyVersion,
            relationship: relationship ?? self.relationship,
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
    public let value: ConfigReadValueValue?

    public init(kind: ConfigReadValueKind, value: ConfigReadValueValue?) {
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
        value: ConfigReadValueValue?? = nil
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

public enum ConfigReadValueValue: Codable, Sendable {
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
        throw DecodingError.typeMismatch(ConfigReadValueValue.self, DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Wrong type for ConfigReadValueValue"))
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

// MARK: - ScopeRelationship
public struct ScopeRelationship: Codable, Sendable {
    public let relation, relationshipID: String
    public let scope: ScopeRef
    public let subject: RelationshipSubject

    public enum CodingKeys: String, CodingKey {
        case relation
        case relationshipID = "relationshipId"
        case scope, subject
    }

    public init(relation: String, relationshipID: String, scope: ScopeRef, subject: RelationshipSubject) {
        self.relation = relation
        self.relationshipID = relationshipID
        self.scope = scope
        self.subject = subject
    }
}

// MARK: ScopeRelationship convenience initializers and mutators

public extension ScopeRelationship {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ScopeRelationship.self, from: data)
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
        relation: String? = nil,
        relationshipID: String? = nil,
        scope: ScopeRef? = nil,
        subject: RelationshipSubject? = nil
    ) -> ScopeRelationship {
        return ScopeRelationship(
            relation: relation ?? self.relation,
            relationshipID: relationshipID ?? self.relationshipID,
            scope: scope ?? self.scope,
            subject: subject ?? self.subject
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - RelationshipSubject
public struct RelationshipSubject: Codable, Sendable {
    public let principalID: String
    public let principalKind: PrincipalKind

    public enum CodingKeys: String, CodingKey {
        case principalID = "principalId"
        case principalKind
    }

    public init(principalID: String, principalKind: PrincipalKind) {
        self.principalID = principalID
        self.principalKind = principalKind
    }
}

// MARK: RelationshipSubject convenience initializers and mutators

public extension RelationshipSubject {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(RelationshipSubject.self, from: data)
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
        principalID: String? = nil,
        principalKind: PrincipalKind? = nil
    ) -> RelationshipSubject {
        return RelationshipSubject(
            principalID: principalID ?? self.principalID,
            principalKind: principalKind ?? self.principalKind
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
    public let event: [String: JSONAny]
    public let occurredAt: Int
    public let sequence: Int
    public let subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case cursor, event, occurredAt, sequence
        case subscriptionID = "subscriptionId"
    }

    public init(correlationID: String, cursor: String, event: [String: JSONAny], occurredAt: Int, sequence: Int, subscriptionID: String) {
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
        event: [String: JSONAny]? = nil,
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
    public let kind: NegotiationRejectionKind?
    public let payload: NegotiationRejectionPayload?

    public enum CodingKeys: String, CodingKey {
        case capabilities
        case negotiationProtocol = "protocol"
        case schemas, kind, payload
    }

    public init(capabilities: [String]?, negotiationProtocol: ProtocolVersion?, schemas: [NegotiatedSchema]?, kind: NegotiationRejectionKind?, payload: NegotiationRejectionPayload?) {
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
        kind: NegotiationRejectionKind?? = nil,
        payload: NegotiationRejectionPayload?? = nil
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

public enum NegotiationRejectionKind: String, Codable, Sendable {
    case incompatibleSchema = "incompatibleSchema"
    case missingCapability = "missingCapability"
    case noCommonProtocol = "noCommonProtocol"
}

// MARK: - NegotiationRejectionPayload
public struct NegotiationRejectionPayload: Codable, Sendable {
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

// MARK: NegotiationRejectionPayload convenience initializers and mutators

public extension NegotiationRejectionPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(NegotiationRejectionPayload.self, from: data)
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
    ) -> NegotiationRejectionPayload {
        return NegotiationRejectionPayload(
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

public enum RequiredBy: String, Codable, Sendable {
    case local = "local"
    case remote = "remote"
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

public enum NegotiationOutcomeStatus: String, Codable, Sendable {
    case accepted = "accepted"
    case rejected = "rejected"
}

public enum DataClassification: String, Codable, Sendable {
    case metadata = "metadata"
    case secret = "secret"
    case sensitive = "sensitive"
}

public enum ObservationSeverity: String, Codable, Sendable {
    case critical = "critical"
    case debug = "debug"
    case error = "error"
    case info = "info"
    case warning = "warning"
}

public enum ObservationValueKind: String, Codable, Sendable {
    case boolean = "boolean"
    case identifier = "identifier"
    case integer = "integer"
    case text = "text"
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

// MARK: - QueryEnvelope
public struct QueryEnvelope: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let causationID: String?
    public let correlationID: String
    public let deadline: Int
    public let protocolVersion: ProtocolVersion
    public let query: [String: JSONAny]
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case authorization
        case causationID = "causationId"
        case correlationID = "correlationId"
        case deadline, protocolVersion, query
        case requestID = "requestId"
    }

    public init(authorization: AuthorizationContext, causationID: String?, correlationID: String, deadline: Int, protocolVersion: ProtocolVersion, query: [String: JSONAny], requestID: String) {
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
        query: [String: JSONAny]? = nil,
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

/// Discriminated union rendered in platform-specific bindings; every kind maps to a distinct
/// typed payload.
// MARK: - QueryResult
public struct QueryResult: Codable, Sendable {
    public let code, correlationID: String?
    public let detail: ErrorDetail?
    public let messageID: String?
    public let parameters: [ErrorParameter]?
    public let retry: RetryDisposition?

    public enum CodingKeys: String, CodingKey {
        case code
        case correlationID = "correlationId"
        case detail
        case messageID = "messageId"
        case parameters, retry
    }

    public init(code: String?, correlationID: String?, detail: ErrorDetail?, messageID: String?, parameters: [ErrorParameter]?, retry: RetryDisposition?) {
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
        code: String?? = nil,
        correlationID: String?? = nil,
        detail: ErrorDetail?? = nil,
        messageID: String?? = nil,
        parameters: [ErrorParameter]?? = nil,
        retry: RetryDisposition?? = nil
    ) -> QueryResult {
        return QueryResult(
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

// MARK: - SubscriptionEnvelope
public struct SubscriptionEnvelope: Codable, Sendable {
    public let authorization: AuthorizationContext
    public let correlationID: String
    public let protocolVersion: ProtocolVersion
    public let requestID: String
    public let resumeAfter: String?
    public let subscription: [String: JSONAny]

    public enum CodingKeys: String, CodingKey {
        case authorization
        case correlationID = "correlationId"
        case protocolVersion
        case requestID = "requestId"
        case resumeAfter, subscription
    }

    public init(authorization: AuthorizationContext, correlationID: String, protocolVersion: ProtocolVersion, requestID: String, resumeAfter: String?, subscription: [String: JSONAny]) {
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
        subscription: [String: JSONAny]? = nil
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

/// One transport-independent sync frame used by simulation, LAN, and WAN links.
// MARK: - SyncTransportFrame
public struct SyncTransportFrame: Codable, Sendable {
    public let correlationID: String
    public let endOfStream: Bool
    public let frameID, idempotencyKey: String
    public let payload: SyncTransportPayload
    public let protocolVersion: ProtocolVersion
    public let sequence: Int
    public let streamID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case endOfStream
        case frameID = "frameId"
        case idempotencyKey, payload, protocolVersion, sequence
        case streamID = "streamId"
    }

    public init(correlationID: String, endOfStream: Bool, frameID: String, idempotencyKey: String, payload: SyncTransportPayload, protocolVersion: ProtocolVersion, sequence: Int, streamID: String) {
        self.correlationID = correlationID
        self.endOfStream = endOfStream
        self.frameID = frameID
        self.idempotencyKey = idempotencyKey
        self.payload = payload
        self.protocolVersion = protocolVersion
        self.sequence = sequence
        self.streamID = streamID
    }
}

// MARK: SyncTransportFrame convenience initializers and mutators

public extension SyncTransportFrame {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SyncTransportFrame.self, from: data)
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
        endOfStream: Bool? = nil,
        frameID: String? = nil,
        idempotencyKey: String? = nil,
        payload: SyncTransportPayload? = nil,
        protocolVersion: ProtocolVersion? = nil,
        sequence: Int? = nil,
        streamID: String? = nil
    ) -> SyncTransportFrame {
        return SyncTransportFrame(
            correlationID: correlationID ?? self.correlationID,
            endOfStream: endOfStream ?? self.endOfStream,
            frameID: frameID ?? self.frameID,
            idempotencyKey: idempotencyKey ?? self.idempotencyKey,
            payload: payload ?? self.payload,
            protocolVersion: protocolVersion ?? self.protocolVersion,
            sequence: sequence ?? self.sequence,
            streamID: streamID ?? self.streamID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SyncTransportPayload
public struct SyncTransportPayload: Codable, Sendable {
    public let kind: SyncTransportPayloadKind
    public let payload: Sync

    public init(kind: SyncTransportPayloadKind, payload: Sync) {
        self.kind = kind
        self.payload = payload
    }
}

// MARK: SyncTransportPayload convenience initializers and mutators

public extension SyncTransportPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SyncTransportPayload.self, from: data)
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
        kind: SyncTransportPayloadKind? = nil,
        payload: Sync? = nil
    ) -> SyncTransportPayload {
        return SyncTransportPayload(
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

public enum SyncTransportPayloadKind: String, Codable, Sendable {
    case cancel = "cancel"
    case heartbeat = "heartbeat"
    case heartbeatAcknowledged = "heartbeatAcknowledged"
    case message = "message"
}

/// Discriminated union rendered in platform-specific bindings; every kind maps to a distinct
/// typed payload.
// MARK: - Sync
public struct Sync: Codable, Sendable {
    public let lastAcceptedSequence: Int?
    public let reason: SyncCancellationReason?
    public let streamID: String?
    public let sentAt: Int?

    public enum CodingKeys: String, CodingKey {
        case lastAcceptedSequence, reason
        case streamID = "streamId"
        case sentAt = "sent_at"
    }

    public init(lastAcceptedSequence: Int?, reason: SyncCancellationReason?, streamID: String?, sentAt: Int?) {
        self.lastAcceptedSequence = lastAcceptedSequence
        self.reason = reason
        self.streamID = streamID
        self.sentAt = sentAt
    }
}

// MARK: Sync convenience initializers and mutators

public extension Sync {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Sync.self, from: data)
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
        lastAcceptedSequence: Int?? = nil,
        reason: SyncCancellationReason?? = nil,
        streamID: String?? = nil,
        sentAt: Int?? = nil
    ) -> Sync {
        return Sync(
            lastAcceptedSequence: lastAcceptedSequence ?? self.lastAcceptedSequence,
            reason: reason ?? self.reason,
            streamID: streamID ?? self.streamID,
            sentAt: sentAt ?? self.sentAt
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum SyncCancellationReason: String, Codable, Sendable {
    case clientRequested = "clientRequested"
    case deadlineExceeded = "deadlineExceeded"
    case shuttingDown = "shuttingDown"
    case superseded = "superseded"
}

// MARK: - UnionPayloadKeepAlive
public struct UnionPayloadKeepAlive: Codable, Sendable {
    public let commandAuthorizationRelationshipGrant: GrantScopeRelationship?
    public let commandAuthorizationRelationshipRevoke: RevokeScopeRelationship?
    public let commandConfigUpdate: UpdateConfiguration?
    public let commandOperationCancel: CancelOperation?
    public let commandUpdateReportInstallerOutcome: ReportInstallerOutcome?
    public let eventAuthorizationPolicyChangedEvent: AuthorizationPolicyChangeNotice?
    public let eventBackgroundJobStatusEvent: BackgroundJobStatus?
    public let eventConfigChangedEvent: ConfigSnapshot?
    public let eventErrorEvent: ScopedError?
    public let eventNotificationEvent: Notification?
    public let eventPermissionsChangedEvent: EffectivePermissions?
    public let eventRecordChangedEvent: RecordChangeNotice?
    public let eventSyncStatusEvent: SyncStatus?
    public let eventUpdateStateEvent: UpdateState?
    public let ipcClientMessageIPCCommand: CommandEnvelope?
    public let ipcClientMessageIPCHandshake: HandshakeRequest?
    public let ipcClientMessageIPCQuery: QueryEnvelope?
    public let ipcClientMessageIPCShutdown: ShutdownRequest?
    public let ipcClientMessageIPCSubscribe: SubscriptionEnvelope?
    public let ipcClientMessageIPCUnsubscribe: UnsubscribeRequest?
    public let ipcServerMessageIPCCommandResponse: CommandResponseEnvelope?
    public let ipcServerMessageIPCEvent: EventEnvelope?
    public let ipcServerMessageIPCFailure: IPCFailureResponse?
    public let ipcServerMessageIPCHandshakeResponse: HandshakeResponse?
    public let ipcServerMessageIPCQueryResponse: QueryResponseEnvelope?
    public let ipcServerMessageIPCShutdownResponse: ShutdownResponse?
    public let ipcServerMessageIPCSubscribeResponse: SubscriptionResponseEnvelope?
    public let ipcServerMessageIPCSubscriptionClosed: SubscriptionClosedEnvelope?
    public let ipcServerMessageIPCUnsubscribeResponse: UnsubscribeResponse?
    public let queryAuthorizationRelationshipsList: ListScopeRelationships?
    public let queryConfigGet, queryPermissionsGetEffective, querySyncGetStatus, queryUpdateGetState: [String: JSONAny]?
    public let queryResultConfiguration: ConfigSnapshot?
    public let queryResultEffectivePermissions: EffectivePermissions?
    public let queryResultScopeRelationships: RelationshipPage?
    public let queryResultSyncStatus: SyncStatus?
    public let queryResultUpdateState: UpdateState?
    public let serverClientMessageServerAcknowledge: ServerSubscriptionAcknowledgement?
    public let serverClientMessageServerHello: ServerConnectionHello?
    public let serverClientMessageServerSubscribe: ServerSubscriptionRequest?
    public let serverClientMessageServerSync: SyncTransportFrame?
    public let serverMessageServerEvent: ServerSubscriptionEvent?
    public let serverMessageServerFailure: ServerFailure?
    public let serverMessageServerHelloAccepted: PeerHello?
    public let serverMessageServerSyncMessage, subscriptionAuthorizationPolicyChangedSubscribe, subscriptionBackgroundJobStatusSubscribe, subscriptionConfigChangedSubscribe: [String: JSONAny]?
    public let subscriptionErrorSubscribe, subscriptionNotificationSubscribe, subscriptionPermissionsChangedSubscribe, subscriptionRecordChangedSubscribe: [String: JSONAny]?
    public let subscriptionSyncStatusSubscribe, subscriptionUpdateStateSubscribe: [String: JSONAny]?
    public let syncMessageSyncAcknowledge: BatchAcknowledgement?
    public let syncMessageSyncBackpressure: RetryAfter?
    public let syncMessageSyncChanges: ChangeBatch?
    public let syncMessageSyncConflict: ConflictNotice?
    public let syncMessageSyncNegotiate: SyncNegotiation?
    public let syncMessageSyncPull: PullRequest?
    public let syncMessageSyncReconcile: ReconciliationDelivery?
    public let syncMessageSyncSnapshotChunk: SnapshotChunk?
    public let syncMessageSyncSnapshotComplete: SnapshotCompletion?
    public let syncMessageSyncSnapshotManifest: SnapshotManifest?
    public let syncMessageSyncSnapshotRequired: SnapshotRequired?

    public enum CodingKeys: String, CodingKey {
        case commandAuthorizationRelationshipGrant = "Command_AuthorizationRelationshipGrant"
        case commandAuthorizationRelationshipRevoke = "Command_AuthorizationRelationshipRevoke"
        case commandConfigUpdate = "Command_ConfigUpdate"
        case commandOperationCancel = "Command_OperationCancel"
        case commandUpdateReportInstallerOutcome = "Command_UpdateReportInstallerOutcome"
        case eventAuthorizationPolicyChangedEvent = "Event_AuthorizationPolicyChangedEvent"
        case eventBackgroundJobStatusEvent = "Event_BackgroundJobStatusEvent"
        case eventConfigChangedEvent = "Event_ConfigChangedEvent"
        case eventErrorEvent = "Event_ErrorEvent"
        case eventNotificationEvent = "Event_NotificationEvent"
        case eventPermissionsChangedEvent = "Event_PermissionsChangedEvent"
        case eventRecordChangedEvent = "Event_RecordChangedEvent"
        case eventSyncStatusEvent = "Event_SyncStatusEvent"
        case eventUpdateStateEvent = "Event_UpdateStateEvent"
        case ipcClientMessageIPCCommand = "IpcClientMessage_IpcCommand"
        case ipcClientMessageIPCHandshake = "IpcClientMessage_IpcHandshake"
        case ipcClientMessageIPCQuery = "IpcClientMessage_IpcQuery"
        case ipcClientMessageIPCShutdown = "IpcClientMessage_IpcShutdown"
        case ipcClientMessageIPCSubscribe = "IpcClientMessage_IpcSubscribe"
        case ipcClientMessageIPCUnsubscribe = "IpcClientMessage_IpcUnsubscribe"
        case ipcServerMessageIPCCommandResponse = "IpcServerMessage_IpcCommandResponse"
        case ipcServerMessageIPCEvent = "IpcServerMessage_IpcEvent"
        case ipcServerMessageIPCFailure = "IpcServerMessage_IpcFailure"
        case ipcServerMessageIPCHandshakeResponse = "IpcServerMessage_IpcHandshakeResponse"
        case ipcServerMessageIPCQueryResponse = "IpcServerMessage_IpcQueryResponse"
        case ipcServerMessageIPCShutdownResponse = "IpcServerMessage_IpcShutdownResponse"
        case ipcServerMessageIPCSubscribeResponse = "IpcServerMessage_IpcSubscribeResponse"
        case ipcServerMessageIPCSubscriptionClosed = "IpcServerMessage_IpcSubscriptionClosed"
        case ipcServerMessageIPCUnsubscribeResponse = "IpcServerMessage_IpcUnsubscribeResponse"
        case queryAuthorizationRelationshipsList = "Query_AuthorizationRelationshipsList"
        case queryConfigGet = "Query_ConfigGet"
        case queryPermissionsGetEffective = "Query_PermissionsGetEffective"
        case querySyncGetStatus = "Query_SyncGetStatus"
        case queryUpdateGetState = "Query_UpdateGetState"
        case queryResultConfiguration = "QueryResult_Configuration"
        case queryResultEffectivePermissions = "QueryResult_EffectivePermissions"
        case queryResultScopeRelationships = "QueryResult_ScopeRelationships"
        case queryResultSyncStatus = "QueryResult_SyncStatus"
        case queryResultUpdateState = "QueryResult_UpdateState"
        case serverClientMessageServerAcknowledge = "ServerClientMessage_ServerAcknowledge"
        case serverClientMessageServerHello = "ServerClientMessage_ServerHello"
        case serverClientMessageServerSubscribe = "ServerClientMessage_ServerSubscribe"
        case serverClientMessageServerSync = "ServerClientMessage_ServerSync"
        case serverMessageServerEvent = "ServerMessage_ServerEvent"
        case serverMessageServerFailure = "ServerMessage_ServerFailure"
        case serverMessageServerHelloAccepted = "ServerMessage_ServerHelloAccepted"
        case serverMessageServerSyncMessage = "ServerMessage_ServerSyncMessage"
        case subscriptionAuthorizationPolicyChangedSubscribe = "Subscription_AuthorizationPolicyChangedSubscribe"
        case subscriptionBackgroundJobStatusSubscribe = "Subscription_BackgroundJobStatusSubscribe"
        case subscriptionConfigChangedSubscribe = "Subscription_ConfigChangedSubscribe"
        case subscriptionErrorSubscribe = "Subscription_ErrorSubscribe"
        case subscriptionNotificationSubscribe = "Subscription_NotificationSubscribe"
        case subscriptionPermissionsChangedSubscribe = "Subscription_PermissionsChangedSubscribe"
        case subscriptionRecordChangedSubscribe = "Subscription_RecordChangedSubscribe"
        case subscriptionSyncStatusSubscribe = "Subscription_SyncStatusSubscribe"
        case subscriptionUpdateStateSubscribe = "Subscription_UpdateStateSubscribe"
        case syncMessageSyncAcknowledge = "SyncMessage_SyncAcknowledge"
        case syncMessageSyncBackpressure = "SyncMessage_SyncBackpressure"
        case syncMessageSyncChanges = "SyncMessage_SyncChanges"
        case syncMessageSyncConflict = "SyncMessage_SyncConflict"
        case syncMessageSyncNegotiate = "SyncMessage_SyncNegotiate"
        case syncMessageSyncPull = "SyncMessage_SyncPull"
        case syncMessageSyncReconcile = "SyncMessage_SyncReconcile"
        case syncMessageSyncSnapshotChunk = "SyncMessage_SyncSnapshotChunk"
        case syncMessageSyncSnapshotComplete = "SyncMessage_SyncSnapshotComplete"
        case syncMessageSyncSnapshotManifest = "SyncMessage_SyncSnapshotManifest"
        case syncMessageSyncSnapshotRequired = "SyncMessage_SyncSnapshotRequired"
    }

    public init(commandAuthorizationRelationshipGrant: GrantScopeRelationship?, commandAuthorizationRelationshipRevoke: RevokeScopeRelationship?, commandConfigUpdate: UpdateConfiguration?, commandOperationCancel: CancelOperation?, commandUpdateReportInstallerOutcome: ReportInstallerOutcome?, eventAuthorizationPolicyChangedEvent: AuthorizationPolicyChangeNotice?, eventBackgroundJobStatusEvent: BackgroundJobStatus?, eventConfigChangedEvent: ConfigSnapshot?, eventErrorEvent: ScopedError?, eventNotificationEvent: Notification?, eventPermissionsChangedEvent: EffectivePermissions?, eventRecordChangedEvent: RecordChangeNotice?, eventSyncStatusEvent: SyncStatus?, eventUpdateStateEvent: UpdateState?, ipcClientMessageIPCCommand: CommandEnvelope?, ipcClientMessageIPCHandshake: HandshakeRequest?, ipcClientMessageIPCQuery: QueryEnvelope?, ipcClientMessageIPCShutdown: ShutdownRequest?, ipcClientMessageIPCSubscribe: SubscriptionEnvelope?, ipcClientMessageIPCUnsubscribe: UnsubscribeRequest?, ipcServerMessageIPCCommandResponse: CommandResponseEnvelope?, ipcServerMessageIPCEvent: EventEnvelope?, ipcServerMessageIPCFailure: IPCFailureResponse?, ipcServerMessageIPCHandshakeResponse: HandshakeResponse?, ipcServerMessageIPCQueryResponse: QueryResponseEnvelope?, ipcServerMessageIPCShutdownResponse: ShutdownResponse?, ipcServerMessageIPCSubscribeResponse: SubscriptionResponseEnvelope?, ipcServerMessageIPCSubscriptionClosed: SubscriptionClosedEnvelope?, ipcServerMessageIPCUnsubscribeResponse: UnsubscribeResponse?, queryAuthorizationRelationshipsList: ListScopeRelationships?, queryConfigGet: [String: JSONAny]?, queryPermissionsGetEffective: [String: JSONAny]?, querySyncGetStatus: [String: JSONAny]?, queryUpdateGetState: [String: JSONAny]?, queryResultConfiguration: ConfigSnapshot?, queryResultEffectivePermissions: EffectivePermissions?, queryResultScopeRelationships: RelationshipPage?, queryResultSyncStatus: SyncStatus?, queryResultUpdateState: UpdateState?, serverClientMessageServerAcknowledge: ServerSubscriptionAcknowledgement?, serverClientMessageServerHello: ServerConnectionHello?, serverClientMessageServerSubscribe: ServerSubscriptionRequest?, serverClientMessageServerSync: SyncTransportFrame?, serverMessageServerEvent: ServerSubscriptionEvent?, serverMessageServerFailure: ServerFailure?, serverMessageServerHelloAccepted: PeerHello?, serverMessageServerSyncMessage: [String: JSONAny]?, subscriptionAuthorizationPolicyChangedSubscribe: [String: JSONAny]?, subscriptionBackgroundJobStatusSubscribe: [String: JSONAny]?, subscriptionConfigChangedSubscribe: [String: JSONAny]?, subscriptionErrorSubscribe: [String: JSONAny]?, subscriptionNotificationSubscribe: [String: JSONAny]?, subscriptionPermissionsChangedSubscribe: [String: JSONAny]?, subscriptionRecordChangedSubscribe: [String: JSONAny]?, subscriptionSyncStatusSubscribe: [String: JSONAny]?, subscriptionUpdateStateSubscribe: [String: JSONAny]?, syncMessageSyncAcknowledge: BatchAcknowledgement?, syncMessageSyncBackpressure: RetryAfter?, syncMessageSyncChanges: ChangeBatch?, syncMessageSyncConflict: ConflictNotice?, syncMessageSyncNegotiate: SyncNegotiation?, syncMessageSyncPull: PullRequest?, syncMessageSyncReconcile: ReconciliationDelivery?, syncMessageSyncSnapshotChunk: SnapshotChunk?, syncMessageSyncSnapshotComplete: SnapshotCompletion?, syncMessageSyncSnapshotManifest: SnapshotManifest?, syncMessageSyncSnapshotRequired: SnapshotRequired?) {
        self.commandAuthorizationRelationshipGrant = commandAuthorizationRelationshipGrant
        self.commandAuthorizationRelationshipRevoke = commandAuthorizationRelationshipRevoke
        self.commandConfigUpdate = commandConfigUpdate
        self.commandOperationCancel = commandOperationCancel
        self.commandUpdateReportInstallerOutcome = commandUpdateReportInstallerOutcome
        self.eventAuthorizationPolicyChangedEvent = eventAuthorizationPolicyChangedEvent
        self.eventBackgroundJobStatusEvent = eventBackgroundJobStatusEvent
        self.eventConfigChangedEvent = eventConfigChangedEvent
        self.eventErrorEvent = eventErrorEvent
        self.eventNotificationEvent = eventNotificationEvent
        self.eventPermissionsChangedEvent = eventPermissionsChangedEvent
        self.eventRecordChangedEvent = eventRecordChangedEvent
        self.eventSyncStatusEvent = eventSyncStatusEvent
        self.eventUpdateStateEvent = eventUpdateStateEvent
        self.ipcClientMessageIPCCommand = ipcClientMessageIPCCommand
        self.ipcClientMessageIPCHandshake = ipcClientMessageIPCHandshake
        self.ipcClientMessageIPCQuery = ipcClientMessageIPCQuery
        self.ipcClientMessageIPCShutdown = ipcClientMessageIPCShutdown
        self.ipcClientMessageIPCSubscribe = ipcClientMessageIPCSubscribe
        self.ipcClientMessageIPCUnsubscribe = ipcClientMessageIPCUnsubscribe
        self.ipcServerMessageIPCCommandResponse = ipcServerMessageIPCCommandResponse
        self.ipcServerMessageIPCEvent = ipcServerMessageIPCEvent
        self.ipcServerMessageIPCFailure = ipcServerMessageIPCFailure
        self.ipcServerMessageIPCHandshakeResponse = ipcServerMessageIPCHandshakeResponse
        self.ipcServerMessageIPCQueryResponse = ipcServerMessageIPCQueryResponse
        self.ipcServerMessageIPCShutdownResponse = ipcServerMessageIPCShutdownResponse
        self.ipcServerMessageIPCSubscribeResponse = ipcServerMessageIPCSubscribeResponse
        self.ipcServerMessageIPCSubscriptionClosed = ipcServerMessageIPCSubscriptionClosed
        self.ipcServerMessageIPCUnsubscribeResponse = ipcServerMessageIPCUnsubscribeResponse
        self.queryAuthorizationRelationshipsList = queryAuthorizationRelationshipsList
        self.queryConfigGet = queryConfigGet
        self.queryPermissionsGetEffective = queryPermissionsGetEffective
        self.querySyncGetStatus = querySyncGetStatus
        self.queryUpdateGetState = queryUpdateGetState
        self.queryResultConfiguration = queryResultConfiguration
        self.queryResultEffectivePermissions = queryResultEffectivePermissions
        self.queryResultScopeRelationships = queryResultScopeRelationships
        self.queryResultSyncStatus = queryResultSyncStatus
        self.queryResultUpdateState = queryResultUpdateState
        self.serverClientMessageServerAcknowledge = serverClientMessageServerAcknowledge
        self.serverClientMessageServerHello = serverClientMessageServerHello
        self.serverClientMessageServerSubscribe = serverClientMessageServerSubscribe
        self.serverClientMessageServerSync = serverClientMessageServerSync
        self.serverMessageServerEvent = serverMessageServerEvent
        self.serverMessageServerFailure = serverMessageServerFailure
        self.serverMessageServerHelloAccepted = serverMessageServerHelloAccepted
        self.serverMessageServerSyncMessage = serverMessageServerSyncMessage
        self.subscriptionAuthorizationPolicyChangedSubscribe = subscriptionAuthorizationPolicyChangedSubscribe
        self.subscriptionBackgroundJobStatusSubscribe = subscriptionBackgroundJobStatusSubscribe
        self.subscriptionConfigChangedSubscribe = subscriptionConfigChangedSubscribe
        self.subscriptionErrorSubscribe = subscriptionErrorSubscribe
        self.subscriptionNotificationSubscribe = subscriptionNotificationSubscribe
        self.subscriptionPermissionsChangedSubscribe = subscriptionPermissionsChangedSubscribe
        self.subscriptionRecordChangedSubscribe = subscriptionRecordChangedSubscribe
        self.subscriptionSyncStatusSubscribe = subscriptionSyncStatusSubscribe
        self.subscriptionUpdateStateSubscribe = subscriptionUpdateStateSubscribe
        self.syncMessageSyncAcknowledge = syncMessageSyncAcknowledge
        self.syncMessageSyncBackpressure = syncMessageSyncBackpressure
        self.syncMessageSyncChanges = syncMessageSyncChanges
        self.syncMessageSyncConflict = syncMessageSyncConflict
        self.syncMessageSyncNegotiate = syncMessageSyncNegotiate
        self.syncMessageSyncPull = syncMessageSyncPull
        self.syncMessageSyncReconcile = syncMessageSyncReconcile
        self.syncMessageSyncSnapshotChunk = syncMessageSyncSnapshotChunk
        self.syncMessageSyncSnapshotComplete = syncMessageSyncSnapshotComplete
        self.syncMessageSyncSnapshotManifest = syncMessageSyncSnapshotManifest
        self.syncMessageSyncSnapshotRequired = syncMessageSyncSnapshotRequired
    }
}

// MARK: UnionPayloadKeepAlive convenience initializers and mutators

public extension UnionPayloadKeepAlive {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(UnionPayloadKeepAlive.self, from: data)
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
        commandAuthorizationRelationshipGrant: GrantScopeRelationship?? = nil,
        commandAuthorizationRelationshipRevoke: RevokeScopeRelationship?? = nil,
        commandConfigUpdate: UpdateConfiguration?? = nil,
        commandOperationCancel: CancelOperation?? = nil,
        commandUpdateReportInstallerOutcome: ReportInstallerOutcome?? = nil,
        eventAuthorizationPolicyChangedEvent: AuthorizationPolicyChangeNotice?? = nil,
        eventBackgroundJobStatusEvent: BackgroundJobStatus?? = nil,
        eventConfigChangedEvent: ConfigSnapshot?? = nil,
        eventErrorEvent: ScopedError?? = nil,
        eventNotificationEvent: Notification?? = nil,
        eventPermissionsChangedEvent: EffectivePermissions?? = nil,
        eventRecordChangedEvent: RecordChangeNotice?? = nil,
        eventSyncStatusEvent: SyncStatus?? = nil,
        eventUpdateStateEvent: UpdateState?? = nil,
        ipcClientMessageIPCCommand: CommandEnvelope?? = nil,
        ipcClientMessageIPCHandshake: HandshakeRequest?? = nil,
        ipcClientMessageIPCQuery: QueryEnvelope?? = nil,
        ipcClientMessageIPCShutdown: ShutdownRequest?? = nil,
        ipcClientMessageIPCSubscribe: SubscriptionEnvelope?? = nil,
        ipcClientMessageIPCUnsubscribe: UnsubscribeRequest?? = nil,
        ipcServerMessageIPCCommandResponse: CommandResponseEnvelope?? = nil,
        ipcServerMessageIPCEvent: EventEnvelope?? = nil,
        ipcServerMessageIPCFailure: IPCFailureResponse?? = nil,
        ipcServerMessageIPCHandshakeResponse: HandshakeResponse?? = nil,
        ipcServerMessageIPCQueryResponse: QueryResponseEnvelope?? = nil,
        ipcServerMessageIPCShutdownResponse: ShutdownResponse?? = nil,
        ipcServerMessageIPCSubscribeResponse: SubscriptionResponseEnvelope?? = nil,
        ipcServerMessageIPCSubscriptionClosed: SubscriptionClosedEnvelope?? = nil,
        ipcServerMessageIPCUnsubscribeResponse: UnsubscribeResponse?? = nil,
        queryAuthorizationRelationshipsList: ListScopeRelationships?? = nil,
        queryConfigGet: [String: JSONAny]?? = nil,
        queryPermissionsGetEffective: [String: JSONAny]?? = nil,
        querySyncGetStatus: [String: JSONAny]?? = nil,
        queryUpdateGetState: [String: JSONAny]?? = nil,
        queryResultConfiguration: ConfigSnapshot?? = nil,
        queryResultEffectivePermissions: EffectivePermissions?? = nil,
        queryResultScopeRelationships: RelationshipPage?? = nil,
        queryResultSyncStatus: SyncStatus?? = nil,
        queryResultUpdateState: UpdateState?? = nil,
        serverClientMessageServerAcknowledge: ServerSubscriptionAcknowledgement?? = nil,
        serverClientMessageServerHello: ServerConnectionHello?? = nil,
        serverClientMessageServerSubscribe: ServerSubscriptionRequest?? = nil,
        serverClientMessageServerSync: SyncTransportFrame?? = nil,
        serverMessageServerEvent: ServerSubscriptionEvent?? = nil,
        serverMessageServerFailure: ServerFailure?? = nil,
        serverMessageServerHelloAccepted: PeerHello?? = nil,
        serverMessageServerSyncMessage: [String: JSONAny]?? = nil,
        subscriptionAuthorizationPolicyChangedSubscribe: [String: JSONAny]?? = nil,
        subscriptionBackgroundJobStatusSubscribe: [String: JSONAny]?? = nil,
        subscriptionConfigChangedSubscribe: [String: JSONAny]?? = nil,
        subscriptionErrorSubscribe: [String: JSONAny]?? = nil,
        subscriptionNotificationSubscribe: [String: JSONAny]?? = nil,
        subscriptionPermissionsChangedSubscribe: [String: JSONAny]?? = nil,
        subscriptionRecordChangedSubscribe: [String: JSONAny]?? = nil,
        subscriptionSyncStatusSubscribe: [String: JSONAny]?? = nil,
        subscriptionUpdateStateSubscribe: [String: JSONAny]?? = nil,
        syncMessageSyncAcknowledge: BatchAcknowledgement?? = nil,
        syncMessageSyncBackpressure: RetryAfter?? = nil,
        syncMessageSyncChanges: ChangeBatch?? = nil,
        syncMessageSyncConflict: ConflictNotice?? = nil,
        syncMessageSyncNegotiate: SyncNegotiation?? = nil,
        syncMessageSyncPull: PullRequest?? = nil,
        syncMessageSyncReconcile: ReconciliationDelivery?? = nil,
        syncMessageSyncSnapshotChunk: SnapshotChunk?? = nil,
        syncMessageSyncSnapshotComplete: SnapshotCompletion?? = nil,
        syncMessageSyncSnapshotManifest: SnapshotManifest?? = nil,
        syncMessageSyncSnapshotRequired: SnapshotRequired?? = nil
    ) -> UnionPayloadKeepAlive {
        return UnionPayloadKeepAlive(
            commandAuthorizationRelationshipGrant: commandAuthorizationRelationshipGrant ?? self.commandAuthorizationRelationshipGrant,
            commandAuthorizationRelationshipRevoke: commandAuthorizationRelationshipRevoke ?? self.commandAuthorizationRelationshipRevoke,
            commandConfigUpdate: commandConfigUpdate ?? self.commandConfigUpdate,
            commandOperationCancel: commandOperationCancel ?? self.commandOperationCancel,
            commandUpdateReportInstallerOutcome: commandUpdateReportInstallerOutcome ?? self.commandUpdateReportInstallerOutcome,
            eventAuthorizationPolicyChangedEvent: eventAuthorizationPolicyChangedEvent ?? self.eventAuthorizationPolicyChangedEvent,
            eventBackgroundJobStatusEvent: eventBackgroundJobStatusEvent ?? self.eventBackgroundJobStatusEvent,
            eventConfigChangedEvent: eventConfigChangedEvent ?? self.eventConfigChangedEvent,
            eventErrorEvent: eventErrorEvent ?? self.eventErrorEvent,
            eventNotificationEvent: eventNotificationEvent ?? self.eventNotificationEvent,
            eventPermissionsChangedEvent: eventPermissionsChangedEvent ?? self.eventPermissionsChangedEvent,
            eventRecordChangedEvent: eventRecordChangedEvent ?? self.eventRecordChangedEvent,
            eventSyncStatusEvent: eventSyncStatusEvent ?? self.eventSyncStatusEvent,
            eventUpdateStateEvent: eventUpdateStateEvent ?? self.eventUpdateStateEvent,
            ipcClientMessageIPCCommand: ipcClientMessageIPCCommand ?? self.ipcClientMessageIPCCommand,
            ipcClientMessageIPCHandshake: ipcClientMessageIPCHandshake ?? self.ipcClientMessageIPCHandshake,
            ipcClientMessageIPCQuery: ipcClientMessageIPCQuery ?? self.ipcClientMessageIPCQuery,
            ipcClientMessageIPCShutdown: ipcClientMessageIPCShutdown ?? self.ipcClientMessageIPCShutdown,
            ipcClientMessageIPCSubscribe: ipcClientMessageIPCSubscribe ?? self.ipcClientMessageIPCSubscribe,
            ipcClientMessageIPCUnsubscribe: ipcClientMessageIPCUnsubscribe ?? self.ipcClientMessageIPCUnsubscribe,
            ipcServerMessageIPCCommandResponse: ipcServerMessageIPCCommandResponse ?? self.ipcServerMessageIPCCommandResponse,
            ipcServerMessageIPCEvent: ipcServerMessageIPCEvent ?? self.ipcServerMessageIPCEvent,
            ipcServerMessageIPCFailure: ipcServerMessageIPCFailure ?? self.ipcServerMessageIPCFailure,
            ipcServerMessageIPCHandshakeResponse: ipcServerMessageIPCHandshakeResponse ?? self.ipcServerMessageIPCHandshakeResponse,
            ipcServerMessageIPCQueryResponse: ipcServerMessageIPCQueryResponse ?? self.ipcServerMessageIPCQueryResponse,
            ipcServerMessageIPCShutdownResponse: ipcServerMessageIPCShutdownResponse ?? self.ipcServerMessageIPCShutdownResponse,
            ipcServerMessageIPCSubscribeResponse: ipcServerMessageIPCSubscribeResponse ?? self.ipcServerMessageIPCSubscribeResponse,
            ipcServerMessageIPCSubscriptionClosed: ipcServerMessageIPCSubscriptionClosed ?? self.ipcServerMessageIPCSubscriptionClosed,
            ipcServerMessageIPCUnsubscribeResponse: ipcServerMessageIPCUnsubscribeResponse ?? self.ipcServerMessageIPCUnsubscribeResponse,
            queryAuthorizationRelationshipsList: queryAuthorizationRelationshipsList ?? self.queryAuthorizationRelationshipsList,
            queryConfigGet: queryConfigGet ?? self.queryConfigGet,
            queryPermissionsGetEffective: queryPermissionsGetEffective ?? self.queryPermissionsGetEffective,
            querySyncGetStatus: querySyncGetStatus ?? self.querySyncGetStatus,
            queryUpdateGetState: queryUpdateGetState ?? self.queryUpdateGetState,
            queryResultConfiguration: queryResultConfiguration ?? self.queryResultConfiguration,
            queryResultEffectivePermissions: queryResultEffectivePermissions ?? self.queryResultEffectivePermissions,
            queryResultScopeRelationships: queryResultScopeRelationships ?? self.queryResultScopeRelationships,
            queryResultSyncStatus: queryResultSyncStatus ?? self.queryResultSyncStatus,
            queryResultUpdateState: queryResultUpdateState ?? self.queryResultUpdateState,
            serverClientMessageServerAcknowledge: serverClientMessageServerAcknowledge ?? self.serverClientMessageServerAcknowledge,
            serverClientMessageServerHello: serverClientMessageServerHello ?? self.serverClientMessageServerHello,
            serverClientMessageServerSubscribe: serverClientMessageServerSubscribe ?? self.serverClientMessageServerSubscribe,
            serverClientMessageServerSync: serverClientMessageServerSync ?? self.serverClientMessageServerSync,
            serverMessageServerEvent: serverMessageServerEvent ?? self.serverMessageServerEvent,
            serverMessageServerFailure: serverMessageServerFailure ?? self.serverMessageServerFailure,
            serverMessageServerHelloAccepted: serverMessageServerHelloAccepted ?? self.serverMessageServerHelloAccepted,
            serverMessageServerSyncMessage: serverMessageServerSyncMessage ?? self.serverMessageServerSyncMessage,
            subscriptionAuthorizationPolicyChangedSubscribe: subscriptionAuthorizationPolicyChangedSubscribe ?? self.subscriptionAuthorizationPolicyChangedSubscribe,
            subscriptionBackgroundJobStatusSubscribe: subscriptionBackgroundJobStatusSubscribe ?? self.subscriptionBackgroundJobStatusSubscribe,
            subscriptionConfigChangedSubscribe: subscriptionConfigChangedSubscribe ?? self.subscriptionConfigChangedSubscribe,
            subscriptionErrorSubscribe: subscriptionErrorSubscribe ?? self.subscriptionErrorSubscribe,
            subscriptionNotificationSubscribe: subscriptionNotificationSubscribe ?? self.subscriptionNotificationSubscribe,
            subscriptionPermissionsChangedSubscribe: subscriptionPermissionsChangedSubscribe ?? self.subscriptionPermissionsChangedSubscribe,
            subscriptionRecordChangedSubscribe: subscriptionRecordChangedSubscribe ?? self.subscriptionRecordChangedSubscribe,
            subscriptionSyncStatusSubscribe: subscriptionSyncStatusSubscribe ?? self.subscriptionSyncStatusSubscribe,
            subscriptionUpdateStateSubscribe: subscriptionUpdateStateSubscribe ?? self.subscriptionUpdateStateSubscribe,
            syncMessageSyncAcknowledge: syncMessageSyncAcknowledge ?? self.syncMessageSyncAcknowledge,
            syncMessageSyncBackpressure: syncMessageSyncBackpressure ?? self.syncMessageSyncBackpressure,
            syncMessageSyncChanges: syncMessageSyncChanges ?? self.syncMessageSyncChanges,
            syncMessageSyncConflict: syncMessageSyncConflict ?? self.syncMessageSyncConflict,
            syncMessageSyncNegotiate: syncMessageSyncNegotiate ?? self.syncMessageSyncNegotiate,
            syncMessageSyncPull: syncMessageSyncPull ?? self.syncMessageSyncPull,
            syncMessageSyncReconcile: syncMessageSyncReconcile ?? self.syncMessageSyncReconcile,
            syncMessageSyncSnapshotChunk: syncMessageSyncSnapshotChunk ?? self.syncMessageSyncSnapshotChunk,
            syncMessageSyncSnapshotComplete: syncMessageSyncSnapshotComplete ?? self.syncMessageSyncSnapshotComplete,
            syncMessageSyncSnapshotManifest: syncMessageSyncSnapshotManifest ?? self.syncMessageSyncSnapshotManifest,
            syncMessageSyncSnapshotRequired: syncMessageSyncSnapshotRequired ?? self.syncMessageSyncSnapshotRequired
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - GrantScopeRelationship
public struct GrantScopeRelationship: Codable, Sendable {
    public let expectedPolicyVersion: Int
    public let relation: String
    public let subject: RelationshipSubject

    public init(expectedPolicyVersion: Int, relation: String, subject: RelationshipSubject) {
        self.expectedPolicyVersion = expectedPolicyVersion
        self.relation = relation
        self.subject = subject
    }
}

// MARK: GrantScopeRelationship convenience initializers and mutators

public extension GrantScopeRelationship {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(GrantScopeRelationship.self, from: data)
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
        expectedPolicyVersion: Int? = nil,
        relation: String? = nil,
        subject: RelationshipSubject? = nil
    ) -> GrantScopeRelationship {
        return GrantScopeRelationship(
            expectedPolicyVersion: expectedPolicyVersion ?? self.expectedPolicyVersion,
            relation: relation ?? self.relation,
            subject: subject ?? self.subject
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - RevokeScopeRelationship
public struct RevokeScopeRelationship: Codable, Sendable {
    public let expectedPolicyVersion: Int
    public let relationshipID: String

    public enum CodingKeys: String, CodingKey {
        case expectedPolicyVersion
        case relationshipID = "relationshipId"
    }

    public init(expectedPolicyVersion: Int, relationshipID: String) {
        self.expectedPolicyVersion = expectedPolicyVersion
        self.relationshipID = relationshipID
    }
}

// MARK: RevokeScopeRelationship convenience initializers and mutators

public extension RevokeScopeRelationship {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(RevokeScopeRelationship.self, from: data)
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
        expectedPolicyVersion: Int? = nil,
        relationshipID: String? = nil
    ) -> RevokeScopeRelationship {
        return RevokeScopeRelationship(
            expectedPolicyVersion: expectedPolicyVersion ?? self.expectedPolicyVersion,
            relationshipID: relationshipID ?? self.relationshipID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - UpdateConfiguration
public struct UpdateConfiguration: Codable, Sendable {
    public let changes: [ConfigChange]
    public let expectedRevision: Int

    public init(changes: [ConfigChange], expectedRevision: Int) {
        self.changes = changes
        self.expectedRevision = expectedRevision
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
        changes: [ConfigChange]? = nil,
        expectedRevision: Int? = nil
    ) -> UpdateConfiguration {
        return UpdateConfiguration(
            changes: changes ?? self.changes,
            expectedRevision: expectedRevision ?? self.expectedRevision
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
    public let value: ConfigReadValueValue

    public init(kind: ConfigWriteValueKind, value: ConfigReadValueValue) {
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
        value: ConfigReadValueValue? = nil
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

// MARK: - CancelOperation
public struct CancelOperation: Codable, Sendable {
    public let operationID: String

    public enum CodingKeys: String, CodingKey {
        case operationID = "operationId"
    }

    public init(operationID: String) {
        self.operationID = operationID
    }
}

// MARK: CancelOperation convenience initializers and mutators

public extension CancelOperation {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(CancelOperation.self, from: data)
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
        operationID: String? = nil
    ) -> CancelOperation {
        return CancelOperation(
            operationID: operationID ?? self.operationID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ReportInstallerOutcome
public struct ReportInstallerOutcome: Codable, Sendable {
    public let handoffID: String
    public let outcome: InstallerOutcome

    public enum CodingKeys: String, CodingKey {
        case handoffID = "handoffId"
        case outcome
    }

    public init(handoffID: String, outcome: InstallerOutcome) {
        self.handoffID = handoffID
        self.outcome = outcome
    }
}

// MARK: ReportInstallerOutcome convenience initializers and mutators

public extension ReportInstallerOutcome {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ReportInstallerOutcome.self, from: data)
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
        handoffID: String? = nil,
        outcome: InstallerOutcome? = nil
    ) -> ReportInstallerOutcome {
        return ReportInstallerOutcome(
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

// MARK: - AuthorizationPolicyChangeNotice
public struct AuthorizationPolicyChangeNotice: Codable, Sendable {
    public let policyVersion: Int
    public let scope: ScopeRef

    public init(policyVersion: Int, scope: ScopeRef) {
        self.policyVersion = policyVersion
        self.scope = scope
    }
}

// MARK: AuthorizationPolicyChangeNotice convenience initializers and mutators

public extension AuthorizationPolicyChangeNotice {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(AuthorizationPolicyChangeNotice.self, from: data)
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
        policyVersion: Int? = nil,
        scope: ScopeRef? = nil
    ) -> AuthorizationPolicyChangeNotice {
        return AuthorizationPolicyChangeNotice(
            policyVersion: policyVersion ?? self.policyVersion,
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

// MARK: - BackgroundJobStatus
public struct BackgroundJobStatus: Codable, Sendable {
    public let completedUnits: Int
    public let error: ContractError?
    public let jobID, jobKind: String
    public let scope: ScopeRef
    public let state: BackgroundJobState
    public let totalUnits: Int?

    public enum CodingKeys: String, CodingKey {
        case completedUnits, error
        case jobID = "jobId"
        case jobKind, scope, state, totalUnits
    }

    public init(completedUnits: Int, error: ContractError?, jobID: String, jobKind: String, scope: ScopeRef, state: BackgroundJobState, totalUnits: Int?) {
        self.completedUnits = completedUnits
        self.error = error
        self.jobID = jobID
        self.jobKind = jobKind
        self.scope = scope
        self.state = state
        self.totalUnits = totalUnits
    }
}

// MARK: BackgroundJobStatus convenience initializers and mutators

public extension BackgroundJobStatus {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(BackgroundJobStatus.self, from: data)
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
        completedUnits: Int? = nil,
        error: ContractError?? = nil,
        jobID: String? = nil,
        jobKind: String? = nil,
        scope: ScopeRef? = nil,
        state: BackgroundJobState? = nil,
        totalUnits: Int?? = nil
    ) -> BackgroundJobStatus {
        return BackgroundJobStatus(
            completedUnits: completedUnits ?? self.completedUnits,
            error: error ?? self.error,
            jobID: jobID ?? self.jobID,
            jobKind: jobKind ?? self.jobKind,
            scope: scope ?? self.scope,
            state: state ?? self.state,
            totalUnits: totalUnits ?? self.totalUnits
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum BackgroundJobState: String, Codable, Sendable {
    case cancelled = "cancelled"
    case failed = "failed"
    case queued = "queued"
    case running = "running"
    case succeeded = "succeeded"
}

// MARK: - ConfigSnapshot
public struct ConfigSnapshot: Codable, Sendable {
    public let entries: [ConfigEntry]
    public let revision, schemaVersion: Int
    public let scope: ScopeRef

    public init(entries: [ConfigEntry], revision: Int, schemaVersion: Int, scope: ScopeRef) {
        self.entries = entries
        self.revision = revision
        self.schemaVersion = schemaVersion
        self.scope = scope
    }
}

// MARK: ConfigSnapshot convenience initializers and mutators

public extension ConfigSnapshot {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ConfigSnapshot.self, from: data)
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
        entries: [ConfigEntry]? = nil,
        revision: Int? = nil,
        schemaVersion: Int? = nil,
        scope: ScopeRef? = nil
    ) -> ConfigSnapshot {
        return ConfigSnapshot(
            entries: entries ?? self.entries,
            revision: revision ?? self.revision,
            schemaVersion: schemaVersion ?? self.schemaVersion,
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

// MARK: - ScopedError
public struct ScopedError: Codable, Sendable {
    public let error: ContractError
    public let scope: ScopeRef

    public init(error: ContractError, scope: ScopeRef) {
        self.error = error
        self.scope = scope
    }
}

// MARK: ScopedError convenience initializers and mutators

public extension ScopedError {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ScopedError.self, from: data)
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
        error: ContractError? = nil,
        scope: ScopeRef? = nil
    ) -> ScopedError {
        return ScopedError(
            error: error ?? self.error,
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

// MARK: - Notification
public struct Notification: Codable, Sendable {
    public let correlationID: String?
    public let messageID, notificationID: String
    public let parameters: [ErrorParameter]
    public let scope: ScopeRef
    public let severity: NotificationSeverity

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case messageID = "messageId"
        case notificationID = "notificationId"
        case parameters, scope, severity
    }

    public init(correlationID: String?, messageID: String, notificationID: String, parameters: [ErrorParameter], scope: ScopeRef, severity: NotificationSeverity) {
        self.correlationID = correlationID
        self.messageID = messageID
        self.notificationID = notificationID
        self.parameters = parameters
        self.scope = scope
        self.severity = severity
    }
}

// MARK: Notification convenience initializers and mutators

public extension Notification {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Notification.self, from: data)
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
        messageID: String? = nil,
        notificationID: String? = nil,
        parameters: [ErrorParameter]? = nil,
        scope: ScopeRef? = nil,
        severity: NotificationSeverity? = nil
    ) -> Notification {
        return Notification(
            correlationID: correlationID ?? self.correlationID,
            messageID: messageID ?? self.messageID,
            notificationID: notificationID ?? self.notificationID,
            parameters: parameters ?? self.parameters,
            scope: scope ?? self.scope,
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

public enum NotificationSeverity: String, Codable, Sendable {
    case error = "error"
    case information = "information"
    case success = "success"
    case warning = "warning"
}

// MARK: - RecordChangeNotice
public struct RecordChangeNotice: Codable, Sendable {
    public let changedAt: Int
    public let operation: ChangeOperation
    public let recordID: String
    public let revision: Int
    public let schemaID: String
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case changedAt, operation
        case recordID = "recordId"
        case revision
        case schemaID = "schemaId"
        case scope
    }

    public init(changedAt: Int, operation: ChangeOperation, recordID: String, revision: Int, schemaID: String, scope: ScopeRef) {
        self.changedAt = changedAt
        self.operation = operation
        self.recordID = recordID
        self.revision = revision
        self.schemaID = schemaID
        self.scope = scope
    }
}

// MARK: RecordChangeNotice convenience initializers and mutators

public extension RecordChangeNotice {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(RecordChangeNotice.self, from: data)
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
        recordID: String? = nil,
        revision: Int? = nil,
        schemaID: String? = nil,
        scope: ScopeRef? = nil
    ) -> RecordChangeNotice {
        return RecordChangeNotice(
            changedAt: changedAt ?? self.changedAt,
            operation: operation ?? self.operation,
            recordID: recordID ?? self.recordID,
            revision: revision ?? self.revision,
            schemaID: schemaID ?? self.schemaID,
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

public enum ChangeOperation: String, Codable, Sendable {
    case tombstone = "tombstone"
    case upsert = "upsert"
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

// MARK: - HandshakeRequest
public struct HandshakeRequest: Codable, Sendable {
    public let assertedAuthorization: DevelopmentIdentityAssertion
    public let correlationID, developmentBearerToken: String
    public let peer: PeerHello
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case assertedAuthorization
        case correlationID = "correlationId"
        case developmentBearerToken, peer
        case requestID = "requestId"
    }

    public init(assertedAuthorization: DevelopmentIdentityAssertion, correlationID: String, developmentBearerToken: String, peer: PeerHello, requestID: String) {
        self.assertedAuthorization = assertedAuthorization
        self.correlationID = correlationID
        self.developmentBearerToken = developmentBearerToken
        self.peer = peer
        self.requestID = requestID
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
        assertedAuthorization: DevelopmentIdentityAssertion? = nil,
        correlationID: String? = nil,
        developmentBearerToken: String? = nil,
        peer: PeerHello? = nil,
        requestID: String? = nil
    ) -> HandshakeRequest {
        return HandshakeRequest(
            assertedAuthorization: assertedAuthorization ?? self.assertedAuthorization,
            correlationID: correlationID ?? self.correlationID,
            developmentBearerToken: developmentBearerToken ?? self.developmentBearerToken,
            peer: peer ?? self.peer,
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

// MARK: - DevelopmentIdentityAssertion
public struct DevelopmentIdentityAssertion: Codable, Sendable {
    public let identity: AuthenticatedIdentity
    public let scope: ScopeRef
    public let tenantID: String
    public let workspaceID: String?

    public enum CodingKeys: String, CodingKey {
        case identity, scope
        case tenantID = "tenantId"
        case workspaceID = "workspaceId"
    }

    public init(identity: AuthenticatedIdentity, scope: ScopeRef, tenantID: String, workspaceID: String?) {
        self.identity = identity
        self.scope = scope
        self.tenantID = tenantID
        self.workspaceID = workspaceID
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
        scope: ScopeRef? = nil,
        tenantID: String? = nil,
        workspaceID: String?? = nil
    ) -> DevelopmentIdentityAssertion {
        return DevelopmentIdentityAssertion(
            identity: identity ?? self.identity,
            scope: scope ?? self.scope,
            tenantID: tenantID ?? self.tenantID,
            workspaceID: workspaceID ?? self.workspaceID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ShutdownRequest
public struct ShutdownRequest: Codable, Sendable {
    public let correlationID, requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case requestID = "requestId"
    }

    public init(correlationID: String, requestID: String) {
        self.correlationID = correlationID
        self.requestID = requestID
    }
}

// MARK: ShutdownRequest convenience initializers and mutators

public extension ShutdownRequest {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ShutdownRequest.self, from: data)
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
        requestID: String? = nil
    ) -> ShutdownRequest {
        return ShutdownRequest(
            correlationID: correlationID ?? self.correlationID,
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

// MARK: - UnsubscribeRequest
public struct UnsubscribeRequest: Codable, Sendable {
    public let correlationID, requestID, subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case requestID = "requestId"
        case subscriptionID = "subscriptionId"
    }

    public init(correlationID: String, requestID: String, subscriptionID: String) {
        self.correlationID = correlationID
        self.requestID = requestID
        self.subscriptionID = subscriptionID
    }
}

// MARK: UnsubscribeRequest convenience initializers and mutators

public extension UnsubscribeRequest {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(UnsubscribeRequest.self, from: data)
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
        requestID: String? = nil,
        subscriptionID: String? = nil
    ) -> UnsubscribeRequest {
        return UnsubscribeRequest(
            correlationID: correlationID ?? self.correlationID,
            requestID: requestID ?? self.requestID,
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

// MARK: - IPCFailureResponse
public struct IPCFailureResponse: Codable, Sendable {
    public let error: ContractError
    public let requestID: String?

    public enum CodingKeys: String, CodingKey {
        case error
        case requestID = "requestId"
    }

    public init(error: ContractError, requestID: String?) {
        self.error = error
        self.requestID = requestID
    }
}

// MARK: IPCFailureResponse convenience initializers and mutators

public extension IPCFailureResponse {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(IPCFailureResponse.self, from: data)
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
        error: ContractError? = nil,
        requestID: String?? = nil
    ) -> IPCFailureResponse {
        return IPCFailureResponse(
            error: error ?? self.error,
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

// MARK: - HandshakeResponse
public struct HandshakeResponse: Codable, Sendable {
    public let correlationID: String
    public let outcome: HandshakeOutcome
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
    }

    public init(correlationID: String, outcome: HandshakeOutcome, requestID: String) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
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
        correlationID: String? = nil,
        outcome: HandshakeOutcome? = nil,
        requestID: String? = nil
    ) -> HandshakeResponse {
        return HandshakeResponse(
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

// MARK: - HandshakeOutcome
public struct HandshakeOutcome: Codable, Sendable {
    public let payload: Handshake
    public let status: NegotiationOutcomeStatus

    public init(payload: Handshake, status: NegotiationOutcomeStatus) {
        self.payload = payload
        self.status = status
    }
}

// MARK: HandshakeOutcome convenience initializers and mutators

public extension HandshakeOutcome {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(HandshakeOutcome.self, from: data)
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
        payload: Handshake? = nil,
        status: NegotiationOutcomeStatus? = nil
    ) -> HandshakeOutcome {
        return HandshakeOutcome(
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

// MARK: - Handshake
public struct Handshake: Codable, Sendable {
    public let authorization: AuthorizationContext?
    public let engine: PeerHello?
    public let negotiated: NegotiatedSession?
    public let kind: FluffyKind?
    public let payload: NegotiationRejection?

    public init(authorization: AuthorizationContext?, engine: PeerHello?, negotiated: NegotiatedSession?, kind: FluffyKind?, payload: NegotiationRejection?) {
        self.authorization = authorization
        self.engine = engine
        self.negotiated = negotiated
        self.kind = kind
        self.payload = payload
    }
}

// MARK: Handshake convenience initializers and mutators

public extension Handshake {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(Handshake.self, from: data)
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
        kind: FluffyKind?? = nil,
        payload: NegotiationRejection?? = nil
    ) -> Handshake {
        return Handshake(
            authorization: authorization ?? self.authorization,
            engine: engine ?? self.engine,
            negotiated: negotiated ?? self.negotiated,
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

public enum FluffyKind: String, Codable, Sendable {
    case authenticationFailed = "authenticationFailed"
    case authenticationRequired = "authenticationRequired"
    case negotiation = "negotiation"
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

// MARK: - NegotiationRejection
public struct NegotiationRejection: Codable, Sendable {
    public let kind: NegotiationRejectionKind
    public let payload: NegotiationRejectionPayload?

    public init(kind: NegotiationRejectionKind, payload: NegotiationRejectionPayload?) {
        self.kind = kind
        self.payload = payload
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
        kind: NegotiationRejectionKind? = nil,
        payload: NegotiationRejectionPayload?? = nil
    ) -> NegotiationRejection {
        return NegotiationRejection(
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

// MARK: - ShutdownResponse
public struct ShutdownResponse: Codable, Sendable {
    public let accepted: Bool
    public let correlationID, requestID: String

    public enum CodingKeys: String, CodingKey {
        case accepted
        case correlationID = "correlationId"
        case requestID = "requestId"
    }

    public init(accepted: Bool, correlationID: String, requestID: String) {
        self.accepted = accepted
        self.correlationID = correlationID
        self.requestID = requestID
    }
}

// MARK: ShutdownResponse convenience initializers and mutators

public extension ShutdownResponse {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ShutdownResponse.self, from: data)
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
        accepted: Bool? = nil,
        correlationID: String? = nil,
        requestID: String? = nil
    ) -> ShutdownResponse {
        return ShutdownResponse(
            accepted: accepted ?? self.accepted,
            correlationID: correlationID ?? self.correlationID,
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

// MARK: - SubscriptionResponseEnvelope
public struct SubscriptionResponseEnvelope: Codable, Sendable {
    public let correlationID: String
    public let outcome: SubscriptionOutcome
    public let requestID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case outcome
        case requestID = "requestId"
    }

    public init(correlationID: String, outcome: SubscriptionOutcome, requestID: String) {
        self.correlationID = correlationID
        self.outcome = outcome
        self.requestID = requestID
    }
}

// MARK: SubscriptionResponseEnvelope convenience initializers and mutators

public extension SubscriptionResponseEnvelope {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SubscriptionResponseEnvelope.self, from: data)
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
        outcome: SubscriptionOutcome? = nil,
        requestID: String? = nil
    ) -> SubscriptionResponseEnvelope {
        return SubscriptionResponseEnvelope(
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

// MARK: - SubscriptionOutcome
public struct SubscriptionOutcome: Codable, Sendable {
    public let payload: SubscriptionAccepted
    public let status: CommandOutcomeStatus

    public init(payload: SubscriptionAccepted, status: CommandOutcomeStatus) {
        self.payload = payload
        self.status = status
    }
}

// MARK: SubscriptionOutcome convenience initializers and mutators

public extension SubscriptionOutcome {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SubscriptionOutcome.self, from: data)
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
        payload: SubscriptionAccepted? = nil,
        status: CommandOutcomeStatus? = nil
    ) -> SubscriptionOutcome {
        return SubscriptionOutcome(
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

// MARK: - SubscriptionAccepted
public struct SubscriptionAccepted: Codable, Sendable {
    public let resumed: Bool?
    public let streamCursor, subscriptionID, code, correlationID: String?
    public let detail: ErrorDetail?
    public let messageID: String?
    public let parameters: [ErrorParameter]?
    public let retry: RetryDisposition?

    public enum CodingKeys: String, CodingKey {
        case resumed, streamCursor
        case subscriptionID = "subscriptionId"
        case code
        case correlationID = "correlationId"
        case detail
        case messageID = "messageId"
        case parameters, retry
    }

    public init(resumed: Bool?, streamCursor: String?, subscriptionID: String?, code: String?, correlationID: String?, detail: ErrorDetail?, messageID: String?, parameters: [ErrorParameter]?, retry: RetryDisposition?) {
        self.resumed = resumed
        self.streamCursor = streamCursor
        self.subscriptionID = subscriptionID
        self.code = code
        self.correlationID = correlationID
        self.detail = detail
        self.messageID = messageID
        self.parameters = parameters
        self.retry = retry
    }
}

// MARK: SubscriptionAccepted convenience initializers and mutators

public extension SubscriptionAccepted {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SubscriptionAccepted.self, from: data)
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
        resumed: Bool?? = nil,
        streamCursor: String?? = nil,
        subscriptionID: String?? = nil,
        code: String?? = nil,
        correlationID: String?? = nil,
        detail: ErrorDetail?? = nil,
        messageID: String?? = nil,
        parameters: [ErrorParameter]?? = nil,
        retry: RetryDisposition?? = nil
    ) -> SubscriptionAccepted {
        return SubscriptionAccepted(
            resumed: resumed ?? self.resumed,
            streamCursor: streamCursor ?? self.streamCursor,
            subscriptionID: subscriptionID ?? self.subscriptionID,
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

// MARK: - SubscriptionClosedEnvelope
public struct SubscriptionClosedEnvelope: Codable, Sendable {
    public let correlationID: String
    public let lastDeliveredCursor: String?
    public let reason: SubscriptionCloseReason
    public let subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case correlationID = "correlationId"
        case lastDeliveredCursor, reason
        case subscriptionID = "subscriptionId"
    }

    public init(correlationID: String, lastDeliveredCursor: String?, reason: SubscriptionCloseReason, subscriptionID: String) {
        self.correlationID = correlationID
        self.lastDeliveredCursor = lastDeliveredCursor
        self.reason = reason
        self.subscriptionID = subscriptionID
    }
}

// MARK: SubscriptionClosedEnvelope convenience initializers and mutators

public extension SubscriptionClosedEnvelope {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SubscriptionClosedEnvelope.self, from: data)
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
        lastDeliveredCursor: String?? = nil,
        reason: SubscriptionCloseReason? = nil,
        subscriptionID: String? = nil
    ) -> SubscriptionClosedEnvelope {
        return SubscriptionClosedEnvelope(
            correlationID: correlationID ?? self.correlationID,
            lastDeliveredCursor: lastDeliveredCursor ?? self.lastDeliveredCursor,
            reason: reason ?? self.reason,
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

public enum SubscriptionCloseReason: String, Codable, Sendable {
    case authorizationRevoked = "authorizationRevoked"
    case backpressure = "backpressure"
    case clientRequested = "clientRequested"
    case engineStopping = "engineStopping"
}

// MARK: - UnsubscribeResponse
public struct UnsubscribeResponse: Codable, Sendable {
    public let accepted: Bool
    public let correlationID, requestID, subscriptionID: String

    public enum CodingKeys: String, CodingKey {
        case accepted
        case correlationID = "correlationId"
        case requestID = "requestId"
        case subscriptionID = "subscriptionId"
    }

    public init(accepted: Bool, correlationID: String, requestID: String, subscriptionID: String) {
        self.accepted = accepted
        self.correlationID = correlationID
        self.requestID = requestID
        self.subscriptionID = subscriptionID
    }
}

// MARK: UnsubscribeResponse convenience initializers and mutators

public extension UnsubscribeResponse {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(UnsubscribeResponse.self, from: data)
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
        accepted: Bool? = nil,
        correlationID: String? = nil,
        requestID: String? = nil,
        subscriptionID: String? = nil
    ) -> UnsubscribeResponse {
        return UnsubscribeResponse(
            accepted: accepted ?? self.accepted,
            correlationID: correlationID ?? self.correlationID,
            requestID: requestID ?? self.requestID,
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

// MARK: - ListScopeRelationships
public struct ListScopeRelationships: Codable, Sendable {
    public let after: String?
    public let limit: Int

    public init(after: String?, limit: Int) {
        self.after = after
        self.limit = limit
    }
}

// MARK: ListScopeRelationships convenience initializers and mutators

public extension ListScopeRelationships {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ListScopeRelationships.self, from: data)
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
        after: String?? = nil,
        limit: Int? = nil
    ) -> ListScopeRelationships {
        return ListScopeRelationships(
            after: after ?? self.after,
            limit: limit ?? self.limit
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - RelationshipPage
public struct RelationshipPage: Codable, Sendable {
    public let nextAfter: String?
    public let policyVersion: Int
    public let relationships: [ScopeRelationship]

    public init(nextAfter: String?, policyVersion: Int, relationships: [ScopeRelationship]) {
        self.nextAfter = nextAfter
        self.policyVersion = policyVersion
        self.relationships = relationships
    }
}

// MARK: RelationshipPage convenience initializers and mutators

public extension RelationshipPage {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(RelationshipPage.self, from: data)
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
        nextAfter: String?? = nil,
        policyVersion: Int? = nil,
        relationships: [ScopeRelationship]? = nil
    ) -> RelationshipPage {
        return RelationshipPage(
            nextAfter: nextAfter ?? self.nextAfter,
            policyVersion: policyVersion ?? self.policyVersion,
            relationships: relationships ?? self.relationships
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ServerSubscriptionAcknowledgement
public struct ServerSubscriptionAcknowledgement: Codable, Sendable {
    public let cursor: String

    public init(cursor: String) {
        self.cursor = cursor
    }
}

// MARK: ServerSubscriptionAcknowledgement convenience initializers and mutators

public extension ServerSubscriptionAcknowledgement {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ServerSubscriptionAcknowledgement.self, from: data)
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
        cursor: String? = nil
    ) -> ServerSubscriptionAcknowledgement {
        return ServerSubscriptionAcknowledgement(
            cursor: cursor ?? self.cursor
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ServerConnectionHello
public struct ServerConnectionHello: Codable, Sendable {
    public let apiVersion: Int
    public let peer: PeerHello
    public let resumeAfter: String?

    public init(apiVersion: Int, peer: PeerHello, resumeAfter: String?) {
        self.apiVersion = apiVersion
        self.peer = peer
        self.resumeAfter = resumeAfter
    }
}

// MARK: ServerConnectionHello convenience initializers and mutators

public extension ServerConnectionHello {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ServerConnectionHello.self, from: data)
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
        apiVersion: Int? = nil,
        peer: PeerHello? = nil,
        resumeAfter: String?? = nil
    ) -> ServerConnectionHello {
        return ServerConnectionHello(
            apiVersion: apiVersion ?? self.apiVersion,
            peer: peer ?? self.peer,
            resumeAfter: resumeAfter ?? self.resumeAfter
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ServerSubscriptionRequest
public struct ServerSubscriptionRequest: Codable, Sendable {
    public let resumeAfter: String?
    public let schemaID: String

    public enum CodingKeys: String, CodingKey {
        case resumeAfter
        case schemaID = "schemaId"
    }

    public init(resumeAfter: String?, schemaID: String) {
        self.resumeAfter = resumeAfter
        self.schemaID = schemaID
    }
}

// MARK: ServerSubscriptionRequest convenience initializers and mutators

public extension ServerSubscriptionRequest {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ServerSubscriptionRequest.self, from: data)
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
        resumeAfter: String?? = nil,
        schemaID: String? = nil
    ) -> ServerSubscriptionRequest {
        return ServerSubscriptionRequest(
            resumeAfter: resumeAfter ?? self.resumeAfter,
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

// MARK: - ServerSubscriptionEvent
public struct ServerSubscriptionEvent: Codable, Sendable {
    public let change: RecordChangeNotice
    public let cursor, eventID: String
    public let occurredAt: Int

    public enum CodingKeys: String, CodingKey {
        case change, cursor
        case eventID = "eventId"
        case occurredAt
    }

    public init(change: RecordChangeNotice, cursor: String, eventID: String, occurredAt: Int) {
        self.change = change
        self.cursor = cursor
        self.eventID = eventID
        self.occurredAt = occurredAt
    }
}

// MARK: ServerSubscriptionEvent convenience initializers and mutators

public extension ServerSubscriptionEvent {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ServerSubscriptionEvent.self, from: data)
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
        change: RecordChangeNotice? = nil,
        cursor: String? = nil,
        eventID: String? = nil,
        occurredAt: Int? = nil
    ) -> ServerSubscriptionEvent {
        return ServerSubscriptionEvent(
            change: change ?? self.change,
            cursor: cursor ?? self.cursor,
            eventID: eventID ?? self.eventID,
            occurredAt: occurredAt ?? self.occurredAt
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ServerFailure
public struct ServerFailure: Codable, Sendable {
    public let code, correlationID: String
    public let retryAfterMS: Int?

    public enum CodingKeys: String, CodingKey {
        case code
        case correlationID = "correlationId"
        case retryAfterMS = "retryAfterMs"
    }

    public init(code: String, correlationID: String, retryAfterMS: Int?) {
        self.code = code
        self.correlationID = correlationID
        self.retryAfterMS = retryAfterMS
    }
}

// MARK: ServerFailure convenience initializers and mutators

public extension ServerFailure {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ServerFailure.self, from: data)
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
        retryAfterMS: Int?? = nil
    ) -> ServerFailure {
        return ServerFailure(
            code: code ?? self.code,
            correlationID: correlationID ?? self.correlationID,
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

// MARK: - BatchAcknowledgement
public struct BatchAcknowledgement: Codable, Sendable {
    public let acceptedRecords: Int
    public let checkpoint, deliveryID: String

    public enum CodingKeys: String, CodingKey {
        case acceptedRecords, checkpoint
        case deliveryID = "deliveryId"
    }

    public init(acceptedRecords: Int, checkpoint: String, deliveryID: String) {
        self.acceptedRecords = acceptedRecords
        self.checkpoint = checkpoint
        self.deliveryID = deliveryID
    }
}

// MARK: BatchAcknowledgement convenience initializers and mutators

public extension BatchAcknowledgement {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(BatchAcknowledgement.self, from: data)
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
        acceptedRecords: Int? = nil,
        checkpoint: String? = nil,
        deliveryID: String? = nil
    ) -> BatchAcknowledgement {
        return BatchAcknowledgement(
            acceptedRecords: acceptedRecords ?? self.acceptedRecords,
            checkpoint: checkpoint ?? self.checkpoint,
            deliveryID: deliveryID ?? self.deliveryID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - RetryAfter
public struct RetryAfter: Codable, Sendable {
    public let delayMS: Int
    public let reason: String

    public enum CodingKeys: String, CodingKey {
        case delayMS = "delayMs"
        case reason
    }

    public init(delayMS: Int, reason: String) {
        self.delayMS = delayMS
        self.reason = reason
    }
}

// MARK: RetryAfter convenience initializers and mutators

public extension RetryAfter {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(RetryAfter.self, from: data)
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
        delayMS: Int? = nil,
        reason: String? = nil
    ) -> RetryAfter {
        return RetryAfter(
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

// MARK: - ChangeBatch
public struct ChangeBatch: Codable, Sendable {
    public let checkpoint, deliveryID: String
    public let fromCheckpoint: String?
    public let hasMore: Bool
    public let idempotencyKey: String
    public let records: [ChangeRecord]

    public enum CodingKeys: String, CodingKey {
        case checkpoint
        case deliveryID = "deliveryId"
        case fromCheckpoint, hasMore, idempotencyKey, records
    }

    public init(checkpoint: String, deliveryID: String, fromCheckpoint: String?, hasMore: Bool, idempotencyKey: String, records: [ChangeRecord]) {
        self.checkpoint = checkpoint
        self.deliveryID = deliveryID
        self.fromCheckpoint = fromCheckpoint
        self.hasMore = hasMore
        self.idempotencyKey = idempotencyKey
        self.records = records
    }
}

// MARK: ChangeBatch convenience initializers and mutators

public extension ChangeBatch {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ChangeBatch.self, from: data)
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
        checkpoint: String? = nil,
        deliveryID: String? = nil,
        fromCheckpoint: String?? = nil,
        hasMore: Bool? = nil,
        idempotencyKey: String? = nil,
        records: [ChangeRecord]? = nil
    ) -> ChangeBatch {
        return ChangeBatch(
            checkpoint: checkpoint ?? self.checkpoint,
            deliveryID: deliveryID ?? self.deliveryID,
            fromCheckpoint: fromCheckpoint ?? self.fromCheckpoint,
            hasMore: hasMore ?? self.hasMore,
            idempotencyKey: idempotencyKey ?? self.idempotencyKey,
            records: records ?? self.records
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ChangeRecord
public struct ChangeRecord: Codable, Sendable {
    public let baseRevision: Int?
    public let changedAt: Int
    public let changeID, idempotencyKey: String
    public let merge: MergeMetadata?
    public let operation: ChangeOperation
    public let payload: EncodedDomainPayload?
    public let recordID: String
    public let revision: Int
    public let scope: ScopeRef

    public enum CodingKeys: String, CodingKey {
        case baseRevision, changedAt
        case changeID = "changeId"
        case idempotencyKey, merge, operation, payload
        case recordID = "recordId"
        case revision, scope
    }

    public init(baseRevision: Int?, changedAt: Int, changeID: String, idempotencyKey: String, merge: MergeMetadata?, operation: ChangeOperation, payload: EncodedDomainPayload?, recordID: String, revision: Int, scope: ScopeRef) {
        self.baseRevision = baseRevision
        self.changedAt = changedAt
        self.changeID = changeID
        self.idempotencyKey = idempotencyKey
        self.merge = merge
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
        baseRevision: Int?? = nil,
        changedAt: Int? = nil,
        changeID: String? = nil,
        idempotencyKey: String? = nil,
        merge: MergeMetadata?? = nil,
        operation: ChangeOperation? = nil,
        payload: EncodedDomainPayload?? = nil,
        recordID: String? = nil,
        revision: Int? = nil,
        scope: ScopeRef? = nil
    ) -> ChangeRecord {
        return ChangeRecord(
            baseRevision: baseRevision ?? self.baseRevision,
            changedAt: changedAt ?? self.changedAt,
            changeID: changeID ?? self.changeID,
            idempotencyKey: idempotencyKey ?? self.idempotencyKey,
            merge: merge ?? self.merge,
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

// MARK: - MergeMetadata
public struct MergeMetadata: Codable, Sendable {
    public let commonAncestorRevision: Int?
    public let mergedAt: Int
    public let sourceChanges: [String]
    public let strategy: MergeStrategy

    public init(commonAncestorRevision: Int?, mergedAt: Int, sourceChanges: [String], strategy: MergeStrategy) {
        self.commonAncestorRevision = commonAncestorRevision
        self.mergedAt = mergedAt
        self.sourceChanges = sourceChanges
        self.strategy = strategy
    }
}

// MARK: MergeMetadata convenience initializers and mutators

public extension MergeMetadata {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(MergeMetadata.self, from: data)
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
        commonAncestorRevision: Int?? = nil,
        mergedAt: Int? = nil,
        sourceChanges: [String]? = nil,
        strategy: MergeStrategy? = nil
    ) -> MergeMetadata {
        return MergeMetadata(
            commonAncestorRevision: commonAncestorRevision ?? self.commonAncestorRevision,
            mergedAt: mergedAt ?? self.mergedAt,
            sourceChanges: sourceChanges ?? self.sourceChanges,
            strategy: strategy ?? self.strategy
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

public enum MergeStrategy: String, Codable, Sendable {
    case domainMerge = "domainMerge"
    case keepLocal = "keepLocal"
    case keepRemote = "keepRemote"
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

// MARK: - ConflictNotice
public struct ConflictNotice: Codable, Sendable {
    public let conflictID: String
    public let localRevision: Int
    public let recordID: String
    public let remoteRevision: Int

    public enum CodingKeys: String, CodingKey {
        case conflictID = "conflictId"
        case localRevision
        case recordID = "recordId"
        case remoteRevision
    }

    public init(conflictID: String, localRevision: Int, recordID: String, remoteRevision: Int) {
        self.conflictID = conflictID
        self.localRevision = localRevision
        self.recordID = recordID
        self.remoteRevision = remoteRevision
    }
}

// MARK: ConflictNotice convenience initializers and mutators

public extension ConflictNotice {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ConflictNotice.self, from: data)
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
        conflictID: String? = nil,
        localRevision: Int? = nil,
        recordID: String? = nil,
        remoteRevision: Int? = nil
    ) -> ConflictNotice {
        return ConflictNotice(
            conflictID: conflictID ?? self.conflictID,
            localRevision: localRevision ?? self.localRevision,
            recordID: recordID ?? self.recordID,
            remoteRevision: remoteRevision ?? self.remoteRevision
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SyncNegotiation
public struct SyncNegotiation: Codable, Sendable {
    public let checkpoint: String?
    public let mode: SyncMode
    public let peer: PeerHello

    public init(checkpoint: String?, mode: SyncMode, peer: PeerHello) {
        self.checkpoint = checkpoint
        self.mode = mode
        self.peer = peer
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
        mode: SyncMode? = nil,
        peer: PeerHello? = nil
    ) -> SyncNegotiation {
        return SyncNegotiation(
            checkpoint: checkpoint ?? self.checkpoint,
            mode: mode ?? self.mode,
            peer: peer ?? self.peer
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

// MARK: - PullRequest
public struct PullRequest: Codable, Sendable {
    public let after: String?
    public let maximumRecords: Int

    public init(after: String?, maximumRecords: Int) {
        self.after = after
        self.maximumRecords = maximumRecords
    }
}

// MARK: PullRequest convenience initializers and mutators

public extension PullRequest {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(PullRequest.self, from: data)
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
        after: String?? = nil,
        maximumRecords: Int? = nil
    ) -> PullRequest {
        return PullRequest(
            after: after ?? self.after,
            maximumRecords: maximumRecords ?? self.maximumRecords
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - ReconciliationDelivery
public struct ReconciliationDelivery: Codable, Sendable {
    public let changes: [ChangeRecord]
    public let checkpoint: String
    public let commandResults: [CommandResult2]
    public let deliveryID, idempotencyKey: String
    public let receivedAt: Int
    public let snapshot: SyncSnapshot?

    public enum CodingKeys: String, CodingKey {
        case changes, checkpoint, commandResults
        case deliveryID = "deliveryId"
        case idempotencyKey, receivedAt, snapshot
    }

    public init(changes: [ChangeRecord], checkpoint: String, commandResults: [CommandResult2], deliveryID: String, idempotencyKey: String, receivedAt: Int, snapshot: SyncSnapshot?) {
        self.changes = changes
        self.checkpoint = checkpoint
        self.commandResults = commandResults
        self.deliveryID = deliveryID
        self.idempotencyKey = idempotencyKey
        self.receivedAt = receivedAt
        self.snapshot = snapshot
    }
}

// MARK: ReconciliationDelivery convenience initializers and mutators

public extension ReconciliationDelivery {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(ReconciliationDelivery.self, from: data)
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
        changes: [ChangeRecord]? = nil,
        checkpoint: String? = nil,
        commandResults: [CommandResult2]? = nil,
        deliveryID: String? = nil,
        idempotencyKey: String? = nil,
        receivedAt: Int? = nil,
        snapshot: SyncSnapshot?? = nil
    ) -> ReconciliationDelivery {
        return ReconciliationDelivery(
            changes: changes ?? self.changes,
            checkpoint: checkpoint ?? self.checkpoint,
            commandResults: commandResults ?? self.commandResults,
            deliveryID: deliveryID ?? self.deliveryID,
            idempotencyKey: idempotencyKey ?? self.idempotencyKey,
            receivedAt: receivedAt ?? self.receivedAt,
            snapshot: snapshot ?? self.snapshot
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - CommandResult2
public struct CommandResult2: Codable, Sendable {
    public let commandID: String
    public let disposition: CommandDisposition

    public enum CodingKeys: String, CodingKey {
        case commandID = "commandId"
        case disposition
    }

    public init(commandID: String, disposition: CommandDisposition) {
        self.commandID = commandID
        self.disposition = disposition
    }
}

// MARK: CommandResult2 convenience initializers and mutators

public extension CommandResult2 {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(CommandResult2.self, from: data)
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
        commandID: String? = nil,
        disposition: CommandDisposition? = nil
    ) -> CommandResult2 {
        return CommandResult2(
            commandID: commandID ?? self.commandID,
            disposition: disposition ?? self.disposition
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - CommandDisposition
public struct CommandDisposition: Codable, Sendable {
    public let payload: CommandDispositionPayload
    public let status: CommandDispositionStatus

    public init(payload: CommandDispositionPayload, status: CommandDispositionStatus) {
        self.payload = payload
        self.status = status
    }
}

// MARK: CommandDisposition convenience initializers and mutators

public extension CommandDisposition {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(CommandDisposition.self, from: data)
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
        payload: CommandDispositionPayload? = nil,
        status: CommandDispositionStatus? = nil
    ) -> CommandDisposition {
        return CommandDisposition(
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

// MARK: - CommandDispositionPayload
public struct CommandDispositionPayload: Codable, Sendable {
    public let authoritativeChange: ChangeRecord?
    public let reason: String?

    public init(authoritativeChange: ChangeRecord?, reason: String?) {
        self.authoritativeChange = authoritativeChange
        self.reason = reason
    }
}

// MARK: CommandDispositionPayload convenience initializers and mutators

public extension CommandDispositionPayload {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(CommandDispositionPayload.self, from: data)
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
        authoritativeChange: ChangeRecord?? = nil,
        reason: String?? = nil
    ) -> CommandDispositionPayload {
        return CommandDispositionPayload(
            authoritativeChange: authoritativeChange ?? self.authoritativeChange,
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

public enum CommandDispositionStatus: String, Codable, Sendable {
    case accepted = "accepted"
    case denied = "denied"
}

// MARK: - SyncSnapshot
public struct SyncSnapshot: Codable, Sendable {
    public let checkpoint: String
    public let createdAt: Int
    public let records: [ChangeRecord]
    public let scope: ScopeRef
    public let serverGeneration: Int
    public let snapshotID: String
    public let validUntil: Int

    public enum CodingKeys: String, CodingKey {
        case checkpoint, createdAt, records, scope, serverGeneration
        case snapshotID = "snapshotId"
        case validUntil
    }

    public init(checkpoint: String, createdAt: Int, records: [ChangeRecord], scope: ScopeRef, serverGeneration: Int, snapshotID: String, validUntil: Int) {
        self.checkpoint = checkpoint
        self.createdAt = createdAt
        self.records = records
        self.scope = scope
        self.serverGeneration = serverGeneration
        self.snapshotID = snapshotID
        self.validUntil = validUntil
    }
}

// MARK: SyncSnapshot convenience initializers and mutators

public extension SyncSnapshot {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SyncSnapshot.self, from: data)
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
        checkpoint: String? = nil,
        createdAt: Int? = nil,
        records: [ChangeRecord]? = nil,
        scope: ScopeRef? = nil,
        serverGeneration: Int? = nil,
        snapshotID: String? = nil,
        validUntil: Int? = nil
    ) -> SyncSnapshot {
        return SyncSnapshot(
            checkpoint: checkpoint ?? self.checkpoint,
            createdAt: createdAt ?? self.createdAt,
            records: records ?? self.records,
            scope: scope ?? self.scope,
            serverGeneration: serverGeneration ?? self.serverGeneration,
            snapshotID: snapshotID ?? self.snapshotID,
            validUntil: validUntil ?? self.validUntil
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SnapshotChunk
public struct SnapshotChunk: Codable, Sendable {
    public let checksum: String
    public let chunkIndex: Int
    public let records: [ChangeRecord]
    public let snapshotID: String

    public enum CodingKeys: String, CodingKey {
        case checksum, chunkIndex, records
        case snapshotID = "snapshotId"
    }

    public init(checksum: String, chunkIndex: Int, records: [ChangeRecord], snapshotID: String) {
        self.checksum = checksum
        self.chunkIndex = chunkIndex
        self.records = records
        self.snapshotID = snapshotID
    }
}

// MARK: SnapshotChunk convenience initializers and mutators

public extension SnapshotChunk {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SnapshotChunk.self, from: data)
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
        checksum: String? = nil,
        chunkIndex: Int? = nil,
        records: [ChangeRecord]? = nil,
        snapshotID: String? = nil
    ) -> SnapshotChunk {
        return SnapshotChunk(
            checksum: checksum ?? self.checksum,
            chunkIndex: chunkIndex ?? self.chunkIndex,
            records: records ?? self.records,
            snapshotID: snapshotID ?? self.snapshotID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SnapshotCompletion
public struct SnapshotCompletion: Codable, Sendable {
    public let checksum, snapshotID: String

    public enum CodingKeys: String, CodingKey {
        case checksum
        case snapshotID = "snapshotId"
    }

    public init(checksum: String, snapshotID: String) {
        self.checksum = checksum
        self.snapshotID = snapshotID
    }
}

// MARK: SnapshotCompletion convenience initializers and mutators

public extension SnapshotCompletion {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SnapshotCompletion.self, from: data)
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
        checksum: String? = nil,
        snapshotID: String? = nil
    ) -> SnapshotCompletion {
        return SnapshotCompletion(
            checksum: checksum ?? self.checksum,
            snapshotID: snapshotID ?? self.snapshotID
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SnapshotManifest
public struct SnapshotManifest: Codable, Sendable {
    public let checkpoint, checksum: String
    public let createdAt: Int
    public let scope: ScopeRef
    public let serverGeneration: Int
    public let snapshotID: String
    public let totalChunks, totalRecords: Int
    public let validUntil: Int

    public enum CodingKeys: String, CodingKey {
        case checkpoint, checksum, createdAt, scope, serverGeneration
        case snapshotID = "snapshotId"
        case totalChunks, totalRecords, validUntil
    }

    public init(checkpoint: String, checksum: String, createdAt: Int, scope: ScopeRef, serverGeneration: Int, snapshotID: String, totalChunks: Int, totalRecords: Int, validUntil: Int) {
        self.checkpoint = checkpoint
        self.checksum = checksum
        self.createdAt = createdAt
        self.scope = scope
        self.serverGeneration = serverGeneration
        self.snapshotID = snapshotID
        self.totalChunks = totalChunks
        self.totalRecords = totalRecords
        self.validUntil = validUntil
    }
}

// MARK: SnapshotManifest convenience initializers and mutators

public extension SnapshotManifest {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SnapshotManifest.self, from: data)
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
        checkpoint: String? = nil,
        checksum: String? = nil,
        createdAt: Int? = nil,
        scope: ScopeRef? = nil,
        serverGeneration: Int? = nil,
        snapshotID: String? = nil,
        totalChunks: Int? = nil,
        totalRecords: Int? = nil,
        validUntil: Int? = nil
    ) -> SnapshotManifest {
        return SnapshotManifest(
            checkpoint: checkpoint ?? self.checkpoint,
            checksum: checksum ?? self.checksum,
            createdAt: createdAt ?? self.createdAt,
            scope: scope ?? self.scope,
            serverGeneration: serverGeneration ?? self.serverGeneration,
            snapshotID: snapshotID ?? self.snapshotID,
            totalChunks: totalChunks ?? self.totalChunks,
            totalRecords: totalRecords ?? self.totalRecords,
            validUntil: validUntil ?? self.validUntil
        )
    }

    func jsonData() throws -> Data {
        return try newJSONEncoder().encode(self)
    }

    func jsonString(encoding: String.Encoding = .utf8) throws -> String? {
        return String(data: try self.jsonData(), encoding: encoding)
    }
}

// MARK: - SnapshotRequired
public struct SnapshotRequired: Codable, Sendable {
    public let reason: String

    public init(reason: String) {
        self.reason = reason
    }
}

// MARK: SnapshotRequired convenience initializers and mutators

public extension SnapshotRequired {
    init(data: Data) throws {
        self = try newJSONDecoder().decode(SnapshotRequired.self, from: data)
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
        reason: String? = nil
    ) -> SnapshotRequired {
        return SnapshotRequired(
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
