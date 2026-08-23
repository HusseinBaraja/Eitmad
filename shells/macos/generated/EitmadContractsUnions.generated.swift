// Generated from Rust contracts. Do not edit.
import Foundation

public enum Command: Codable, Sendable {
    case configUpdate(UpdateConfiguration)
    case authorizationRelationshipGrant(GrantScopeRelationship)
    case authorizationRelationshipRevoke(RevokeScopeRelationship)
    case operationCancel(CancelOperation)
    case updateReportInstallerOutcome(ReportInstallerOutcome)

    private enum Kind: String, Codable, Sendable {
        case configUpdate = "eitmad.config.update.v1"
        case authorizationRelationshipGrant = "eitmad.authorization.relationship.grant.v1"
        case authorizationRelationshipRevoke = "eitmad.authorization.relationship.revoke.v1"
        case operationCancel = "eitmad.operation.cancel.v1"
        case updateReportInstallerOutcome = "eitmad.update.report-installer-outcome.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .configUpdate: self = .configUpdate(try container.decode(UpdateConfiguration.self, forKey: .payload))
        case .authorizationRelationshipGrant: self = .authorizationRelationshipGrant(try container.decode(GrantScopeRelationship.self, forKey: .payload))
        case .authorizationRelationshipRevoke: self = .authorizationRelationshipRevoke(try container.decode(RevokeScopeRelationship.self, forKey: .payload))
        case .operationCancel: self = .operationCancel(try container.decode(CancelOperation.self, forKey: .payload))
        case .updateReportInstallerOutcome: self = .updateReportInstallerOutcome(try container.decode(ReportInstallerOutcome.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .configUpdate(let payload):
            try container.encode(Kind.configUpdate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationRelationshipGrant(let payload):
            try container.encode(Kind.authorizationRelationshipGrant, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationRelationshipRevoke(let payload):
            try container.encode(Kind.authorizationRelationshipRevoke, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .operationCancel(let payload):
            try container.encode(Kind.operationCancel, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .updateReportInstallerOutcome(let payload):
            try container.encode(Kind.updateReportInstallerOutcome, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum Event: Codable, Sendable {
    case configChangedEvent(ConfigSnapshot)
    case permissionsChangedEvent(EffectivePermissions)
    case authorizationPolicyChangedEvent(AuthorizationPolicyChangeNotice)
    case updateStateEvent(UpdateState)
    case syncStatusEvent(SyncStatus)
    case recordChangedEvent(RecordChangeNotice)
    case backgroundJobStatusEvent(BackgroundJobStatus)
    case notificationEvent(Notification)
    case errorEvent(ScopedError)

    private enum Kind: String, Codable, Sendable {
        case configChangedEvent = "eitmad.config.changed.event.v1"
        case permissionsChangedEvent = "eitmad.permissions.changed.event.v1"
        case authorizationPolicyChangedEvent = "eitmad.authorization.policy.changed.event.v1"
        case updateStateEvent = "eitmad.update.state.event.v1"
        case syncStatusEvent = "eitmad.sync.status.event.v1"
        case recordChangedEvent = "eitmad.record.changed.event.v1"
        case backgroundJobStatusEvent = "eitmad.background-job.status.event.v1"
        case notificationEvent = "eitmad.notification.event.v1"
        case errorEvent = "eitmad.error.event.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .configChangedEvent: self = .configChangedEvent(try container.decode(ConfigSnapshot.self, forKey: .payload))
        case .permissionsChangedEvent: self = .permissionsChangedEvent(try container.decode(EffectivePermissions.self, forKey: .payload))
        case .authorizationPolicyChangedEvent: self = .authorizationPolicyChangedEvent(try container.decode(AuthorizationPolicyChangeNotice.self, forKey: .payload))
        case .updateStateEvent: self = .updateStateEvent(try container.decode(UpdateState.self, forKey: .payload))
        case .syncStatusEvent: self = .syncStatusEvent(try container.decode(SyncStatus.self, forKey: .payload))
        case .recordChangedEvent: self = .recordChangedEvent(try container.decode(RecordChangeNotice.self, forKey: .payload))
        case .backgroundJobStatusEvent: self = .backgroundJobStatusEvent(try container.decode(BackgroundJobStatus.self, forKey: .payload))
        case .notificationEvent: self = .notificationEvent(try container.decode(Notification.self, forKey: .payload))
        case .errorEvent: self = .errorEvent(try container.decode(ScopedError.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .configChangedEvent(let payload):
            try container.encode(Kind.configChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .permissionsChangedEvent(let payload):
            try container.encode(Kind.permissionsChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationPolicyChangedEvent(let payload):
            try container.encode(Kind.authorizationPolicyChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .updateStateEvent(let payload):
            try container.encode(Kind.updateStateEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncStatusEvent(let payload):
            try container.encode(Kind.syncStatusEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .recordChangedEvent(let payload):
            try container.encode(Kind.recordChangedEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .backgroundJobStatusEvent(let payload):
            try container.encode(Kind.backgroundJobStatusEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .notificationEvent(let payload):
            try container.encode(Kind.notificationEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .errorEvent(let payload):
            try container.encode(Kind.errorEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum IpcClientMessage: Codable, Sendable {
    case ipcHandshake(HandshakeRequest)
    case ipcCommand(CommandEnvelope)
    case ipcQuery(QueryEnvelope)
    case ipcSubscribe(SubscriptionEnvelope)
    case ipcUnsubscribe(UnsubscribeRequest)
    case ipcShutdown(ShutdownRequest)

    private enum Kind: String, Codable, Sendable {
        case ipcHandshake = "eitmad.ipc.handshake.v1"
        case ipcCommand = "eitmad.ipc.command.v1"
        case ipcQuery = "eitmad.ipc.query.v1"
        case ipcSubscribe = "eitmad.ipc.subscribe.v1"
        case ipcUnsubscribe = "eitmad.ipc.unsubscribe.v1"
        case ipcShutdown = "eitmad.ipc.shutdown.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .ipcHandshake: self = .ipcHandshake(try container.decode(HandshakeRequest.self, forKey: .payload))
        case .ipcCommand: self = .ipcCommand(try container.decode(CommandEnvelope.self, forKey: .payload))
        case .ipcQuery: self = .ipcQuery(try container.decode(QueryEnvelope.self, forKey: .payload))
        case .ipcSubscribe: self = .ipcSubscribe(try container.decode(SubscriptionEnvelope.self, forKey: .payload))
        case .ipcUnsubscribe: self = .ipcUnsubscribe(try container.decode(UnsubscribeRequest.self, forKey: .payload))
        case .ipcShutdown: self = .ipcShutdown(try container.decode(ShutdownRequest.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .ipcHandshake(let payload):
            try container.encode(Kind.ipcHandshake, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcCommand(let payload):
            try container.encode(Kind.ipcCommand, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcQuery(let payload):
            try container.encode(Kind.ipcQuery, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcSubscribe(let payload):
            try container.encode(Kind.ipcSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcUnsubscribe(let payload):
            try container.encode(Kind.ipcUnsubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcShutdown(let payload):
            try container.encode(Kind.ipcShutdown, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum IpcServerMessage: Codable, Sendable {
    case ipcHandshakeResponse(HandshakeResponse)
    case ipcCommandResponse(CommandResponseEnvelope)
    case ipcQueryResponse(QueryResponseEnvelope)
    case ipcSubscribeResponse(SubscriptionResponseEnvelope)
    case ipcUnsubscribeResponse(UnsubscribeResponse)
    case ipcEvent(EventEnvelope)
    case ipcSubscriptionClosed(SubscriptionClosedEnvelope)
    case ipcShutdownResponse(ShutdownResponse)
    case ipcFailure(IpcFailureResponse)

    private enum Kind: String, Codable, Sendable {
        case ipcHandshakeResponse = "eitmad.ipc.handshake-response.v1"
        case ipcCommandResponse = "eitmad.ipc.command-response.v1"
        case ipcQueryResponse = "eitmad.ipc.query-response.v1"
        case ipcSubscribeResponse = "eitmad.ipc.subscribe-response.v1"
        case ipcUnsubscribeResponse = "eitmad.ipc.unsubscribe-response.v1"
        case ipcEvent = "eitmad.ipc.event.v1"
        case ipcSubscriptionClosed = "eitmad.ipc.subscription-closed.v1"
        case ipcShutdownResponse = "eitmad.ipc.shutdown-response.v1"
        case ipcFailure = "eitmad.ipc.failure.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .ipcHandshakeResponse: self = .ipcHandshakeResponse(try container.decode(HandshakeResponse.self, forKey: .payload))
        case .ipcCommandResponse: self = .ipcCommandResponse(try container.decode(CommandResponseEnvelope.self, forKey: .payload))
        case .ipcQueryResponse: self = .ipcQueryResponse(try container.decode(QueryResponseEnvelope.self, forKey: .payload))
        case .ipcSubscribeResponse: self = .ipcSubscribeResponse(try container.decode(SubscriptionResponseEnvelope.self, forKey: .payload))
        case .ipcUnsubscribeResponse: self = .ipcUnsubscribeResponse(try container.decode(UnsubscribeResponse.self, forKey: .payload))
        case .ipcEvent: self = .ipcEvent(try container.decode(EventEnvelope.self, forKey: .payload))
        case .ipcSubscriptionClosed: self = .ipcSubscriptionClosed(try container.decode(SubscriptionClosedEnvelope.self, forKey: .payload))
        case .ipcShutdownResponse: self = .ipcShutdownResponse(try container.decode(ShutdownResponse.self, forKey: .payload))
        case .ipcFailure: self = .ipcFailure(try container.decode(IpcFailureResponse.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .ipcHandshakeResponse(let payload):
            try container.encode(Kind.ipcHandshakeResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcCommandResponse(let payload):
            try container.encode(Kind.ipcCommandResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcQueryResponse(let payload):
            try container.encode(Kind.ipcQueryResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcSubscribeResponse(let payload):
            try container.encode(Kind.ipcSubscribeResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcUnsubscribeResponse(let payload):
            try container.encode(Kind.ipcUnsubscribeResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcEvent(let payload):
            try container.encode(Kind.ipcEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcSubscriptionClosed(let payload):
            try container.encode(Kind.ipcSubscriptionClosed, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcShutdownResponse(let payload):
            try container.encode(Kind.ipcShutdownResponse, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .ipcFailure(let payload):
            try container.encode(Kind.ipcFailure, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum Query: Codable, Sendable {
    case configGet(GetConfiguration)
    case permissionsGetEffective(GetEffectivePermissions)
    case authorizationRelationshipsList(ListScopeRelationships)
    case updateGetState(GetUpdateState)
    case syncGetStatus(GetSyncStatus)

    private enum Kind: String, Codable, Sendable {
        case configGet = "eitmad.config.get.v1"
        case permissionsGetEffective = "eitmad.permissions.get-effective.v1"
        case authorizationRelationshipsList = "eitmad.authorization.relationships.list.v1"
        case updateGetState = "eitmad.update.get-state.v1"
        case syncGetStatus = "eitmad.sync.get-status.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .configGet: self = .configGet(try container.decode(GetConfiguration.self, forKey: .payload))
        case .permissionsGetEffective: self = .permissionsGetEffective(try container.decode(GetEffectivePermissions.self, forKey: .payload))
        case .authorizationRelationshipsList: self = .authorizationRelationshipsList(try container.decode(ListScopeRelationships.self, forKey: .payload))
        case .updateGetState: self = .updateGetState(try container.decode(GetUpdateState.self, forKey: .payload))
        case .syncGetStatus: self = .syncGetStatus(try container.decode(GetSyncStatus.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .configGet(let payload):
            try container.encode(Kind.configGet, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .permissionsGetEffective(let payload):
            try container.encode(Kind.permissionsGetEffective, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationRelationshipsList(let payload):
            try container.encode(Kind.authorizationRelationshipsList, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .updateGetState(let payload):
            try container.encode(Kind.updateGetState, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncGetStatus(let payload):
            try container.encode(Kind.syncGetStatus, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum QueryResult: Codable, Sendable {
    case configuration(ConfigSnapshot)
    case effectivePermissions(EffectivePermissions)
    case scopeRelationships(RelationshipPage)
    case updateState(UpdateState)
    case syncStatus(SyncStatus)

    private enum Kind: String, Codable, Sendable {
        case configuration = "configuration"
        case effectivePermissions = "effectivePermissions"
        case scopeRelationships = "scopeRelationships"
        case updateState = "updateState"
        case syncStatus = "syncStatus"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .configuration: self = .configuration(try container.decode(ConfigSnapshot.self, forKey: .payload))
        case .effectivePermissions: self = .effectivePermissions(try container.decode(EffectivePermissions.self, forKey: .payload))
        case .scopeRelationships: self = .scopeRelationships(try container.decode(RelationshipPage.self, forKey: .payload))
        case .updateState: self = .updateState(try container.decode(UpdateState.self, forKey: .payload))
        case .syncStatus: self = .syncStatus(try container.decode(SyncStatus.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .configuration(let payload):
            try container.encode(Kind.configuration, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .effectivePermissions(let payload):
            try container.encode(Kind.effectivePermissions, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .scopeRelationships(let payload):
            try container.encode(Kind.scopeRelationships, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .updateState(let payload):
            try container.encode(Kind.updateState, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncStatus(let payload):
            try container.encode(Kind.syncStatus, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum ServerClientMessage: Codable, Sendable {
    case serverHello(ServerConnectionHello)
    case serverSync(SyncTransportFrame)
    case serverSubscribe(ServerSubscriptionRequest)
    case serverAcknowledge(ServerSubscriptionAcknowledgement)

    private enum Kind: String, Codable, Sendable {
        case serverHello = "eitmad.server.hello.v1"
        case serverSync = "eitmad.server.sync.v1"
        case serverSubscribe = "eitmad.server.subscribe.v1"
        case serverAcknowledge = "eitmad.server.acknowledge.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .serverHello: self = .serverHello(try container.decode(ServerConnectionHello.self, forKey: .payload))
        case .serverSync: self = .serverSync(try container.decode(SyncTransportFrame.self, forKey: .payload))
        case .serverSubscribe: self = .serverSubscribe(try container.decode(ServerSubscriptionRequest.self, forKey: .payload))
        case .serverAcknowledge: self = .serverAcknowledge(try container.decode(ServerSubscriptionAcknowledgement.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .serverHello(let payload):
            try container.encode(Kind.serverHello, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .serverSync(let payload):
            try container.encode(Kind.serverSync, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .serverSubscribe(let payload):
            try container.encode(Kind.serverSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .serverAcknowledge(let payload):
            try container.encode(Kind.serverAcknowledge, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum ServerMessage: Codable, Sendable {
    case serverHelloAccepted(PeerHello)
    case serverSyncMessage(SyncMessage)
    case serverEvent(ServerSubscriptionEvent)
    case serverFailure(ServerFailure)

    private enum Kind: String, Codable, Sendable {
        case serverHelloAccepted = "eitmad.server.hello-accepted.v1"
        case serverSyncMessage = "eitmad.server.sync-message.v1"
        case serverEvent = "eitmad.server.event.v1"
        case serverFailure = "eitmad.server.failure.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .serverHelloAccepted: self = .serverHelloAccepted(try container.decode(PeerHello.self, forKey: .payload))
        case .serverSyncMessage: self = .serverSyncMessage(try container.decode(SyncMessage.self, forKey: .payload))
        case .serverEvent: self = .serverEvent(try container.decode(ServerSubscriptionEvent.self, forKey: .payload))
        case .serverFailure: self = .serverFailure(try container.decode(ServerFailure.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .serverHelloAccepted(let payload):
            try container.encode(Kind.serverHelloAccepted, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .serverSyncMessage(let payload):
            try container.encode(Kind.serverSyncMessage, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .serverEvent(let payload):
            try container.encode(Kind.serverEvent, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .serverFailure(let payload):
            try container.encode(Kind.serverFailure, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum Subscription: Codable, Sendable {
    case configChangedSubscribe(ConfigurationChanges)
    case permissionsChangedSubscribe(PermissionChanges)
    case authorizationPolicyChangedSubscribe(AuthorizationPolicyChanges)
    case updateStateSubscribe(UpdateStateChanges)
    case syncStatusSubscribe(SyncStatusChanges)
    case recordChangedSubscribe(RecordChanges)
    case backgroundJobStatusSubscribe(BackgroundJobChanges)
    case notificationSubscribe(Notifications)
    case errorSubscribe(Errors)

    private enum Kind: String, Codable, Sendable {
        case configChangedSubscribe = "eitmad.config.changed.subscribe.v1"
        case permissionsChangedSubscribe = "eitmad.permissions.changed.subscribe.v1"
        case authorizationPolicyChangedSubscribe = "eitmad.authorization.policy.changed.subscribe.v1"
        case updateStateSubscribe = "eitmad.update.state.subscribe.v1"
        case syncStatusSubscribe = "eitmad.sync.status.subscribe.v1"
        case recordChangedSubscribe = "eitmad.record.changed.subscribe.v1"
        case backgroundJobStatusSubscribe = "eitmad.background-job.status.subscribe.v1"
        case notificationSubscribe = "eitmad.notification.subscribe.v1"
        case errorSubscribe = "eitmad.error.subscribe.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .configChangedSubscribe: self = .configChangedSubscribe(try container.decode(ConfigurationChanges.self, forKey: .payload))
        case .permissionsChangedSubscribe: self = .permissionsChangedSubscribe(try container.decode(PermissionChanges.self, forKey: .payload))
        case .authorizationPolicyChangedSubscribe: self = .authorizationPolicyChangedSubscribe(try container.decode(AuthorizationPolicyChanges.self, forKey: .payload))
        case .updateStateSubscribe: self = .updateStateSubscribe(try container.decode(UpdateStateChanges.self, forKey: .payload))
        case .syncStatusSubscribe: self = .syncStatusSubscribe(try container.decode(SyncStatusChanges.self, forKey: .payload))
        case .recordChangedSubscribe: self = .recordChangedSubscribe(try container.decode(RecordChanges.self, forKey: .payload))
        case .backgroundJobStatusSubscribe: self = .backgroundJobStatusSubscribe(try container.decode(BackgroundJobChanges.self, forKey: .payload))
        case .notificationSubscribe: self = .notificationSubscribe(try container.decode(Notifications.self, forKey: .payload))
        case .errorSubscribe: self = .errorSubscribe(try container.decode(Errors.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .configChangedSubscribe(let payload):
            try container.encode(Kind.configChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .permissionsChangedSubscribe(let payload):
            try container.encode(Kind.permissionsChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .authorizationPolicyChangedSubscribe(let payload):
            try container.encode(Kind.authorizationPolicyChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .updateStateSubscribe(let payload):
            try container.encode(Kind.updateStateSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncStatusSubscribe(let payload):
            try container.encode(Kind.syncStatusSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .recordChangedSubscribe(let payload):
            try container.encode(Kind.recordChangedSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .backgroundJobStatusSubscribe(let payload):
            try container.encode(Kind.backgroundJobStatusSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .notificationSubscribe(let payload):
            try container.encode(Kind.notificationSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .errorSubscribe(let payload):
            try container.encode(Kind.errorSubscribe, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
public enum SyncMessage: Codable, Sendable {
    case syncNegotiate(SyncNegotiation)
    case syncPull(PullRequest)
    case syncChanges(ChangeBatch)
    case syncReconcile(ReconciliationDelivery)
    case syncAcknowledge(BatchAcknowledgement)
    case syncConflict(ConflictNotice)
    case syncBackpressure(RetryAfter)
    case syncSnapshotManifest(SnapshotManifest)
    case syncSnapshotChunk(SnapshotChunk)
    case syncSnapshotComplete(SnapshotCompletion)
    case syncSnapshotRequired(SnapshotRequired)

    private enum Kind: String, Codable, Sendable {
        case syncNegotiate = "eitmad.sync.negotiate.v1"
        case syncPull = "eitmad.sync.pull.v1"
        case syncChanges = "eitmad.sync.changes.v1"
        case syncReconcile = "eitmad.sync.reconcile.v1"
        case syncAcknowledge = "eitmad.sync.acknowledge.v1"
        case syncConflict = "eitmad.sync.conflict.v1"
        case syncBackpressure = "eitmad.sync.backpressure.v1"
        case syncSnapshotManifest = "eitmad.sync.snapshot-manifest.v1"
        case syncSnapshotChunk = "eitmad.sync.snapshot-chunk.v1"
        case syncSnapshotComplete = "eitmad.sync.snapshot-complete.v1"
        case syncSnapshotRequired = "eitmad.sync.snapshot-required.v1"
    }

    private enum CodingKeys: String, CodingKey {
        case kind
        case payload
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        switch try container.decode(Kind.self, forKey: .kind) {
        case .syncNegotiate: self = .syncNegotiate(try container.decode(SyncNegotiation.self, forKey: .payload))
        case .syncPull: self = .syncPull(try container.decode(PullRequest.self, forKey: .payload))
        case .syncChanges: self = .syncChanges(try container.decode(ChangeBatch.self, forKey: .payload))
        case .syncReconcile: self = .syncReconcile(try container.decode(ReconciliationDelivery.self, forKey: .payload))
        case .syncAcknowledge: self = .syncAcknowledge(try container.decode(BatchAcknowledgement.self, forKey: .payload))
        case .syncConflict: self = .syncConflict(try container.decode(ConflictNotice.self, forKey: .payload))
        case .syncBackpressure: self = .syncBackpressure(try container.decode(RetryAfter.self, forKey: .payload))
        case .syncSnapshotManifest: self = .syncSnapshotManifest(try container.decode(SnapshotManifest.self, forKey: .payload))
        case .syncSnapshotChunk: self = .syncSnapshotChunk(try container.decode(SnapshotChunk.self, forKey: .payload))
        case .syncSnapshotComplete: self = .syncSnapshotComplete(try container.decode(SnapshotCompletion.self, forKey: .payload))
        case .syncSnapshotRequired: self = .syncSnapshotRequired(try container.decode(SnapshotRequired.self, forKey: .payload))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .syncNegotiate(let payload):
            try container.encode(Kind.syncNegotiate, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncPull(let payload):
            try container.encode(Kind.syncPull, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncChanges(let payload):
            try container.encode(Kind.syncChanges, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncReconcile(let payload):
            try container.encode(Kind.syncReconcile, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncAcknowledge(let payload):
            try container.encode(Kind.syncAcknowledge, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncConflict(let payload):
            try container.encode(Kind.syncConflict, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncBackpressure(let payload):
            try container.encode(Kind.syncBackpressure, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncSnapshotManifest(let payload):
            try container.encode(Kind.syncSnapshotManifest, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncSnapshotChunk(let payload):
            try container.encode(Kind.syncSnapshotChunk, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncSnapshotComplete(let payload):
            try container.encode(Kind.syncSnapshotComplete, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        case .syncSnapshotRequired(let payload):
            try container.encode(Kind.syncSnapshotRequired, forKey: .kind)
            try container.encode(payload, forKey: .payload)
        }
    }
}
