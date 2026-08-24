// Generated from Rust contracts. Do not edit.
#nullable enable
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Eitmad.Contracts;

public partial class Command
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string ConfigUpdateKind = "eitmad.config.update.v1";

    public static Command ForConfigUpdate(UpdateConfiguration payload) =>
        new() { Kind = ConfigUpdateKind, Payload = payload };

    public UpdateConfiguration? AsConfigUpdate() =>
        Kind == ConfigUpdateKind ? PayloadAs<UpdateConfiguration>() : null;

    public const string AuthorizationRelationshipGrantKind = "eitmad.authorization.relationship.grant.v1";

    public static Command ForAuthorizationRelationshipGrant(GrantScopeRelationship payload) =>
        new() { Kind = AuthorizationRelationshipGrantKind, Payload = payload };

    public GrantScopeRelationship? AsAuthorizationRelationshipGrant() =>
        Kind == AuthorizationRelationshipGrantKind ? PayloadAs<GrantScopeRelationship>() : null;

    public const string AuthorizationRelationshipRevokeKind = "eitmad.authorization.relationship.revoke.v1";

    public static Command ForAuthorizationRelationshipRevoke(RevokeScopeRelationship payload) =>
        new() { Kind = AuthorizationRelationshipRevokeKind, Payload = payload };

    public RevokeScopeRelationship? AsAuthorizationRelationshipRevoke() =>
        Kind == AuthorizationRelationshipRevokeKind ? PayloadAs<RevokeScopeRelationship>() : null;

    public const string OperationCancelKind = "eitmad.operation.cancel.v1";

    public static Command ForOperationCancel(CancelOperation payload) =>
        new() { Kind = OperationCancelKind, Payload = payload };

    public CancelOperation? AsOperationCancel() =>
        Kind == OperationCancelKind ? PayloadAs<CancelOperation>() : null;

    public const string UpdateReportInstallerOutcomeKind = "eitmad.update.report-installer-outcome.v1";

    public static Command ForUpdateReportInstallerOutcome(ReportInstallerOutcome payload) =>
        new() { Kind = UpdateReportInstallerOutcomeKind, Payload = payload };

    public ReportInstallerOutcome? AsUpdateReportInstallerOutcome() =>
        Kind == UpdateReportInstallerOutcomeKind ? PayloadAs<ReportInstallerOutcome>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class Event
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string ConfigChangedEventKind = "eitmad.config.changed.event.v1";

    public static Event ForConfigChangedEvent(ConfigSnapshot payload) =>
        new() { Kind = ConfigChangedEventKind, Payload = payload };

    public ConfigSnapshot? AsConfigChangedEvent() =>
        Kind == ConfigChangedEventKind ? PayloadAs<ConfigSnapshot>() : null;

    public const string PermissionsChangedEventKind = "eitmad.permissions.changed.event.v1";

    public static Event ForPermissionsChangedEvent(EffectivePermissions payload) =>
        new() { Kind = PermissionsChangedEventKind, Payload = payload };

    public EffectivePermissions? AsPermissionsChangedEvent() =>
        Kind == PermissionsChangedEventKind ? PayloadAs<EffectivePermissions>() : null;

    public const string AuthorizationPolicyChangedEventKind = "eitmad.authorization.policy.changed.event.v1";

    public static Event ForAuthorizationPolicyChangedEvent(AuthorizationPolicyChangeNotice payload) =>
        new() { Kind = AuthorizationPolicyChangedEventKind, Payload = payload };

    public AuthorizationPolicyChangeNotice? AsAuthorizationPolicyChangedEvent() =>
        Kind == AuthorizationPolicyChangedEventKind ? PayloadAs<AuthorizationPolicyChangeNotice>() : null;

    public const string UpdateStateEventKind = "eitmad.update.state.event.v1";

    public static Event ForUpdateStateEvent(UpdateState payload) =>
        new() { Kind = UpdateStateEventKind, Payload = payload };

    public UpdateState? AsUpdateStateEvent() =>
        Kind == UpdateStateEventKind ? PayloadAs<UpdateState>() : null;

    public const string SyncStatusEventKind = "eitmad.sync.status.event.v1";

    public static Event ForSyncStatusEvent(SyncStatus payload) =>
        new() { Kind = SyncStatusEventKind, Payload = payload };

    public SyncStatus? AsSyncStatusEvent() =>
        Kind == SyncStatusEventKind ? PayloadAs<SyncStatus>() : null;

    public const string RecordChangedEventKind = "eitmad.record.changed.event.v1";

    public static Event ForRecordChangedEvent(RecordChangeNotice payload) =>
        new() { Kind = RecordChangedEventKind, Payload = payload };

    public RecordChangeNotice? AsRecordChangedEvent() =>
        Kind == RecordChangedEventKind ? PayloadAs<RecordChangeNotice>() : null;

    public const string BackgroundJobStatusEventKind = "eitmad.background-job.status.event.v1";

    public static Event ForBackgroundJobStatusEvent(BackgroundJobStatus payload) =>
        new() { Kind = BackgroundJobStatusEventKind, Payload = payload };

    public BackgroundJobStatus? AsBackgroundJobStatusEvent() =>
        Kind == BackgroundJobStatusEventKind ? PayloadAs<BackgroundJobStatus>() : null;

    public const string NotificationEventKind = "eitmad.notification.event.v1";

    public static Event ForNotificationEvent(Notification payload) =>
        new() { Kind = NotificationEventKind, Payload = payload };

    public Notification? AsNotificationEvent() =>
        Kind == NotificationEventKind ? PayloadAs<Notification>() : null;

    public const string ErrorEventKind = "eitmad.error.event.v1";

    public static Event ForErrorEvent(ScopedError payload) =>
        new() { Kind = ErrorEventKind, Payload = payload };

    public ScopedError? AsErrorEvent() =>
        Kind == ErrorEventKind ? PayloadAs<ScopedError>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class IpcClientMessage
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string IpcHandshakeKind = "eitmad.ipc.handshake.v1";

    public static IpcClientMessage ForIpcHandshake(HandshakeRequest payload) =>
        new() { Kind = IpcHandshakeKind, Payload = payload };

    public HandshakeRequest? AsIpcHandshake() =>
        Kind == IpcHandshakeKind ? PayloadAs<HandshakeRequest>() : null;

    public const string IpcCommandKind = "eitmad.ipc.command.v1";

    public static IpcClientMessage ForIpcCommand(CommandEnvelope payload) =>
        new() { Kind = IpcCommandKind, Payload = payload };

    public CommandEnvelope? AsIpcCommand() =>
        Kind == IpcCommandKind ? PayloadAs<CommandEnvelope>() : null;

    public const string IpcQueryKind = "eitmad.ipc.query.v1";

    public static IpcClientMessage ForIpcQuery(QueryEnvelope payload) =>
        new() { Kind = IpcQueryKind, Payload = payload };

    public QueryEnvelope? AsIpcQuery() =>
        Kind == IpcQueryKind ? PayloadAs<QueryEnvelope>() : null;

    public const string IpcSubscribeKind = "eitmad.ipc.subscribe.v1";

    public static IpcClientMessage ForIpcSubscribe(SubscriptionEnvelope payload) =>
        new() { Kind = IpcSubscribeKind, Payload = payload };

    public SubscriptionEnvelope? AsIpcSubscribe() =>
        Kind == IpcSubscribeKind ? PayloadAs<SubscriptionEnvelope>() : null;

    public const string IpcUnsubscribeKind = "eitmad.ipc.unsubscribe.v1";

    public static IpcClientMessage ForIpcUnsubscribe(UnsubscribeRequest payload) =>
        new() { Kind = IpcUnsubscribeKind, Payload = payload };

    public UnsubscribeRequest? AsIpcUnsubscribe() =>
        Kind == IpcUnsubscribeKind ? PayloadAs<UnsubscribeRequest>() : null;

    public const string IpcShutdownKind = "eitmad.ipc.shutdown.v1";

    public static IpcClientMessage ForIpcShutdown(ShutdownRequest payload) =>
        new() { Kind = IpcShutdownKind, Payload = payload };

    public ShutdownRequest? AsIpcShutdown() =>
        Kind == IpcShutdownKind ? PayloadAs<ShutdownRequest>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class IpcServerMessage
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string IpcHandshakeResponseKind = "eitmad.ipc.handshake-response.v1";

    public static IpcServerMessage ForIpcHandshakeResponse(HandshakeResponse payload) =>
        new() { Kind = IpcHandshakeResponseKind, Payload = payload };

    public HandshakeResponse? AsIpcHandshakeResponse() =>
        Kind == IpcHandshakeResponseKind ? PayloadAs<HandshakeResponse>() : null;

    public const string IpcCommandResponseKind = "eitmad.ipc.command-response.v1";

    public static IpcServerMessage ForIpcCommandResponse(CommandResponseEnvelope payload) =>
        new() { Kind = IpcCommandResponseKind, Payload = payload };

    public CommandResponseEnvelope? AsIpcCommandResponse() =>
        Kind == IpcCommandResponseKind ? PayloadAs<CommandResponseEnvelope>() : null;

    public const string IpcQueryResponseKind = "eitmad.ipc.query-response.v1";

    public static IpcServerMessage ForIpcQueryResponse(QueryResponseEnvelope payload) =>
        new() { Kind = IpcQueryResponseKind, Payload = payload };

    public QueryResponseEnvelope? AsIpcQueryResponse() =>
        Kind == IpcQueryResponseKind ? PayloadAs<QueryResponseEnvelope>() : null;

    public const string IpcSubscribeResponseKind = "eitmad.ipc.subscribe-response.v1";

    public static IpcServerMessage ForIpcSubscribeResponse(SubscriptionResponseEnvelope payload) =>
        new() { Kind = IpcSubscribeResponseKind, Payload = payload };

    public SubscriptionResponseEnvelope? AsIpcSubscribeResponse() =>
        Kind == IpcSubscribeResponseKind ? PayloadAs<SubscriptionResponseEnvelope>() : null;

    public const string IpcUnsubscribeResponseKind = "eitmad.ipc.unsubscribe-response.v1";

    public static IpcServerMessage ForIpcUnsubscribeResponse(UnsubscribeResponse payload) =>
        new() { Kind = IpcUnsubscribeResponseKind, Payload = payload };

    public UnsubscribeResponse? AsIpcUnsubscribeResponse() =>
        Kind == IpcUnsubscribeResponseKind ? PayloadAs<UnsubscribeResponse>() : null;

    public const string IpcEventKind = "eitmad.ipc.event.v1";

    public static IpcServerMessage ForIpcEvent(EventEnvelope payload) =>
        new() { Kind = IpcEventKind, Payload = payload };

    public EventEnvelope? AsIpcEvent() =>
        Kind == IpcEventKind ? PayloadAs<EventEnvelope>() : null;

    public const string IpcSubscriptionClosedKind = "eitmad.ipc.subscription-closed.v1";

    public static IpcServerMessage ForIpcSubscriptionClosed(SubscriptionClosedEnvelope payload) =>
        new() { Kind = IpcSubscriptionClosedKind, Payload = payload };

    public SubscriptionClosedEnvelope? AsIpcSubscriptionClosed() =>
        Kind == IpcSubscriptionClosedKind ? PayloadAs<SubscriptionClosedEnvelope>() : null;

    public const string IpcShutdownResponseKind = "eitmad.ipc.shutdown-response.v1";

    public static IpcServerMessage ForIpcShutdownResponse(ShutdownResponse payload) =>
        new() { Kind = IpcShutdownResponseKind, Payload = payload };

    public ShutdownResponse? AsIpcShutdownResponse() =>
        Kind == IpcShutdownResponseKind ? PayloadAs<ShutdownResponse>() : null;

    public const string IpcFailureKind = "eitmad.ipc.failure.v1";

    public static IpcServerMessage ForIpcFailure(IpcFailureResponse payload) =>
        new() { Kind = IpcFailureKind, Payload = payload };

    public IpcFailureResponse? AsIpcFailure() =>
        Kind == IpcFailureKind ? PayloadAs<IpcFailureResponse>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class Query
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string ConfigGetKind = "eitmad.config.get.v1";

    public static Query ForConfigGet(GetConfiguration payload) =>
        new() { Kind = ConfigGetKind, Payload = payload };

    public GetConfiguration? AsConfigGet() =>
        Kind == ConfigGetKind ? PayloadAs<GetConfiguration>() : null;

    public const string PermissionsGetEffectiveKind = "eitmad.permissions.get-effective.v1";

    public static Query ForPermissionsGetEffective(GetEffectivePermissions payload) =>
        new() { Kind = PermissionsGetEffectiveKind, Payload = payload };

    public GetEffectivePermissions? AsPermissionsGetEffective() =>
        Kind == PermissionsGetEffectiveKind ? PayloadAs<GetEffectivePermissions>() : null;

    public const string AuthorizationRelationshipsListKind = "eitmad.authorization.relationships.list.v1";

    public static Query ForAuthorizationRelationshipsList(ListScopeRelationships payload) =>
        new() { Kind = AuthorizationRelationshipsListKind, Payload = payload };

    public ListScopeRelationships? AsAuthorizationRelationshipsList() =>
        Kind == AuthorizationRelationshipsListKind ? PayloadAs<ListScopeRelationships>() : null;

    public const string UpdateGetStateKind = "eitmad.update.get-state.v1";

    public static Query ForUpdateGetState(GetUpdateState payload) =>
        new() { Kind = UpdateGetStateKind, Payload = payload };

    public GetUpdateState? AsUpdateGetState() =>
        Kind == UpdateGetStateKind ? PayloadAs<GetUpdateState>() : null;

    public const string SyncGetStatusKind = "eitmad.sync.get-status.v1";

    public static Query ForSyncGetStatus(GetSyncStatus payload) =>
        new() { Kind = SyncGetStatusKind, Payload = payload };

    public GetSyncStatus? AsSyncGetStatus() =>
        Kind == SyncGetStatusKind ? PayloadAs<GetSyncStatus>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class QueryResult
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string ConfigurationKind = "configuration";

    public static QueryResult ForConfiguration(ConfigSnapshot payload) =>
        new() { Kind = ConfigurationKind, Payload = payload };

    public ConfigSnapshot? AsConfiguration() =>
        Kind == ConfigurationKind ? PayloadAs<ConfigSnapshot>() : null;

    public const string EffectivePermissionsKind = "effectivePermissions";

    public static QueryResult ForEffectivePermissions(EffectivePermissions payload) =>
        new() { Kind = EffectivePermissionsKind, Payload = payload };

    public EffectivePermissions? AsEffectivePermissions() =>
        Kind == EffectivePermissionsKind ? PayloadAs<EffectivePermissions>() : null;

    public const string ScopeRelationshipsKind = "scopeRelationships";

    public static QueryResult ForScopeRelationships(RelationshipPage payload) =>
        new() { Kind = ScopeRelationshipsKind, Payload = payload };

    public RelationshipPage? AsScopeRelationships() =>
        Kind == ScopeRelationshipsKind ? PayloadAs<RelationshipPage>() : null;

    public const string UpdateStateKind = "updateState";

    public static QueryResult ForUpdateState(UpdateState payload) =>
        new() { Kind = UpdateStateKind, Payload = payload };

    public UpdateState? AsUpdateState() =>
        Kind == UpdateStateKind ? PayloadAs<UpdateState>() : null;

    public const string SyncStatusKind = "syncStatus";

    public static QueryResult ForSyncStatus(SyncStatus payload) =>
        new() { Kind = SyncStatusKind, Payload = payload };

    public SyncStatus? AsSyncStatus() =>
        Kind == SyncStatusKind ? PayloadAs<SyncStatus>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class ServerClientMessage
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string ServerHelloKind = "eitmad.server.hello.v1";

    public static ServerClientMessage ForServerHello(ServerConnectionHello payload) =>
        new() { Kind = ServerHelloKind, Payload = payload };

    public ServerConnectionHello? AsServerHello() =>
        Kind == ServerHelloKind ? PayloadAs<ServerConnectionHello>() : null;

    public const string ServerSyncKind = "eitmad.server.sync.v1";

    public static ServerClientMessage ForServerSync(SyncTransportFrame payload) =>
        new() { Kind = ServerSyncKind, Payload = payload };

    public SyncTransportFrame? AsServerSync() =>
        Kind == ServerSyncKind ? PayloadAs<SyncTransportFrame>() : null;

    public const string ServerSubscribeKind = "eitmad.server.subscribe.v1";

    public static ServerClientMessage ForServerSubscribe(ServerSubscriptionRequest payload) =>
        new() { Kind = ServerSubscribeKind, Payload = payload };

    public ServerSubscriptionRequest? AsServerSubscribe() =>
        Kind == ServerSubscribeKind ? PayloadAs<ServerSubscriptionRequest>() : null;

    public const string ServerAcknowledgeKind = "eitmad.server.acknowledge.v1";

    public static ServerClientMessage ForServerAcknowledge(ServerSubscriptionAcknowledgement payload) =>
        new() { Kind = ServerAcknowledgeKind, Payload = payload };

    public ServerSubscriptionAcknowledgement? AsServerAcknowledge() =>
        Kind == ServerAcknowledgeKind ? PayloadAs<ServerSubscriptionAcknowledgement>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class ServerMessage
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string ServerHelloAcceptedKind = "eitmad.server.hello-accepted.v1";

    public static ServerMessage ForServerHelloAccepted(PeerHello payload) =>
        new() { Kind = ServerHelloAcceptedKind, Payload = payload };

    public PeerHello? AsServerHelloAccepted() =>
        Kind == ServerHelloAcceptedKind ? PayloadAs<PeerHello>() : null;

    public const string ServerSyncMessageKind = "eitmad.server.sync-message.v1";

    public static ServerMessage ForServerSyncMessage(SyncMessage payload) =>
        new() { Kind = ServerSyncMessageKind, Payload = payload };

    public SyncMessage? AsServerSyncMessage() =>
        Kind == ServerSyncMessageKind ? PayloadAs<SyncMessage>() : null;

    public const string ServerEventKind = "eitmad.server.event.v1";

    public static ServerMessage ForServerEvent(ServerSubscriptionEvent payload) =>
        new() { Kind = ServerEventKind, Payload = payload };

    public ServerSubscriptionEvent? AsServerEvent() =>
        Kind == ServerEventKind ? PayloadAs<ServerSubscriptionEvent>() : null;

    public const string ServerFailureKind = "eitmad.server.failure.v1";

    public static ServerMessage ForServerFailure(ServerFailure payload) =>
        new() { Kind = ServerFailureKind, Payload = payload };

    public ServerFailure? AsServerFailure() =>
        Kind == ServerFailureKind ? PayloadAs<ServerFailure>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class Subscription
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string ConfigChangedSubscribeKind = "eitmad.config.changed.subscribe.v1";

    public static Subscription ForConfigChangedSubscribe(ConfigurationChanges payload) =>
        new() { Kind = ConfigChangedSubscribeKind, Payload = payload };

    public ConfigurationChanges? AsConfigChangedSubscribe() =>
        Kind == ConfigChangedSubscribeKind ? PayloadAs<ConfigurationChanges>() : null;

    public const string PermissionsChangedSubscribeKind = "eitmad.permissions.changed.subscribe.v1";

    public static Subscription ForPermissionsChangedSubscribe(PermissionChanges payload) =>
        new() { Kind = PermissionsChangedSubscribeKind, Payload = payload };

    public PermissionChanges? AsPermissionsChangedSubscribe() =>
        Kind == PermissionsChangedSubscribeKind ? PayloadAs<PermissionChanges>() : null;

    public const string AuthorizationPolicyChangedSubscribeKind = "eitmad.authorization.policy.changed.subscribe.v1";

    public static Subscription ForAuthorizationPolicyChangedSubscribe(AuthorizationPolicyChanges payload) =>
        new() { Kind = AuthorizationPolicyChangedSubscribeKind, Payload = payload };

    public AuthorizationPolicyChanges? AsAuthorizationPolicyChangedSubscribe() =>
        Kind == AuthorizationPolicyChangedSubscribeKind ? PayloadAs<AuthorizationPolicyChanges>() : null;

    public const string UpdateStateSubscribeKind = "eitmad.update.state.subscribe.v1";

    public static Subscription ForUpdateStateSubscribe(UpdateStateChanges payload) =>
        new() { Kind = UpdateStateSubscribeKind, Payload = payload };

    public UpdateStateChanges? AsUpdateStateSubscribe() =>
        Kind == UpdateStateSubscribeKind ? PayloadAs<UpdateStateChanges>() : null;

    public const string SyncStatusSubscribeKind = "eitmad.sync.status.subscribe.v1";

    public static Subscription ForSyncStatusSubscribe(SyncStatusChanges payload) =>
        new() { Kind = SyncStatusSubscribeKind, Payload = payload };

    public SyncStatusChanges? AsSyncStatusSubscribe() =>
        Kind == SyncStatusSubscribeKind ? PayloadAs<SyncStatusChanges>() : null;

    public const string RecordChangedSubscribeKind = "eitmad.record.changed.subscribe.v1";

    public static Subscription ForRecordChangedSubscribe(RecordChanges payload) =>
        new() { Kind = RecordChangedSubscribeKind, Payload = payload };

    public RecordChanges? AsRecordChangedSubscribe() =>
        Kind == RecordChangedSubscribeKind ? PayloadAs<RecordChanges>() : null;

    public const string BackgroundJobStatusSubscribeKind = "eitmad.background-job.status.subscribe.v1";

    public static Subscription ForBackgroundJobStatusSubscribe(BackgroundJobChanges payload) =>
        new() { Kind = BackgroundJobStatusSubscribeKind, Payload = payload };

    public BackgroundJobChanges? AsBackgroundJobStatusSubscribe() =>
        Kind == BackgroundJobStatusSubscribeKind ? PayloadAs<BackgroundJobChanges>() : null;

    public const string NotificationSubscribeKind = "eitmad.notification.subscribe.v1";

    public static Subscription ForNotificationSubscribe(Notifications payload) =>
        new() { Kind = NotificationSubscribeKind, Payload = payload };

    public Notifications? AsNotificationSubscribe() =>
        Kind == NotificationSubscribeKind ? PayloadAs<Notifications>() : null;

    public const string ErrorSubscribeKind = "eitmad.error.subscribe.v1";

    public static Subscription ForErrorSubscribe(Errors payload) =>
        new() { Kind = ErrorSubscribeKind, Payload = payload };

    public Errors? AsErrorSubscribe() =>
        Kind == ErrorSubscribeKind ? PayloadAs<Errors>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class SyncMessage
{
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = string.Empty;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    public const string SyncNegotiateKind = "eitmad.sync.negotiate.v1";

    public static SyncMessage ForSyncNegotiate(SyncNegotiation payload) =>
        new() { Kind = SyncNegotiateKind, Payload = payload };

    public SyncNegotiation? AsSyncNegotiate() =>
        Kind == SyncNegotiateKind ? PayloadAs<SyncNegotiation>() : null;

    public const string SyncPullKind = "eitmad.sync.pull.v1";

    public static SyncMessage ForSyncPull(PullRequest payload) =>
        new() { Kind = SyncPullKind, Payload = payload };

    public PullRequest? AsSyncPull() =>
        Kind == SyncPullKind ? PayloadAs<PullRequest>() : null;

    public const string SyncChangesKind = "eitmad.sync.changes.v1";

    public static SyncMessage ForSyncChanges(ChangeBatch payload) =>
        new() { Kind = SyncChangesKind, Payload = payload };

    public ChangeBatch? AsSyncChanges() =>
        Kind == SyncChangesKind ? PayloadAs<ChangeBatch>() : null;

    public const string SyncReconcileKind = "eitmad.sync.reconcile.v1";

    public static SyncMessage ForSyncReconcile(ReconciliationDelivery payload) =>
        new() { Kind = SyncReconcileKind, Payload = payload };

    public ReconciliationDelivery? AsSyncReconcile() =>
        Kind == SyncReconcileKind ? PayloadAs<ReconciliationDelivery>() : null;

    public const string SyncAcknowledgeKind = "eitmad.sync.acknowledge.v1";

    public static SyncMessage ForSyncAcknowledge(BatchAcknowledgement payload) =>
        new() { Kind = SyncAcknowledgeKind, Payload = payload };

    public BatchAcknowledgement? AsSyncAcknowledge() =>
        Kind == SyncAcknowledgeKind ? PayloadAs<BatchAcknowledgement>() : null;

    public const string SyncConflictKind = "eitmad.sync.conflict.v1";

    public static SyncMessage ForSyncConflict(ConflictNotice payload) =>
        new() { Kind = SyncConflictKind, Payload = payload };

    public ConflictNotice? AsSyncConflict() =>
        Kind == SyncConflictKind ? PayloadAs<ConflictNotice>() : null;

    public const string SyncBackpressureKind = "eitmad.sync.backpressure.v1";

    public static SyncMessage ForSyncBackpressure(RetryAfter payload) =>
        new() { Kind = SyncBackpressureKind, Payload = payload };

    public RetryAfter? AsSyncBackpressure() =>
        Kind == SyncBackpressureKind ? PayloadAs<RetryAfter>() : null;

    public const string SyncSnapshotManifestKind = "eitmad.sync.snapshot-manifest.v1";

    public static SyncMessage ForSyncSnapshotManifest(SnapshotManifest payload) =>
        new() { Kind = SyncSnapshotManifestKind, Payload = payload };

    public SnapshotManifest? AsSyncSnapshotManifest() =>
        Kind == SyncSnapshotManifestKind ? PayloadAs<SnapshotManifest>() : null;

    public const string SyncSnapshotChunkKind = "eitmad.sync.snapshot-chunk.v1";

    public static SyncMessage ForSyncSnapshotChunk(SnapshotChunk payload) =>
        new() { Kind = SyncSnapshotChunkKind, Payload = payload };

    public SnapshotChunk? AsSyncSnapshotChunk() =>
        Kind == SyncSnapshotChunkKind ? PayloadAs<SnapshotChunk>() : null;

    public const string SyncSnapshotCompleteKind = "eitmad.sync.snapshot-complete.v1";

    public static SyncMessage ForSyncSnapshotComplete(SnapshotCompletion payload) =>
        new() { Kind = SyncSnapshotCompleteKind, Payload = payload };

    public SnapshotCompletion? AsSyncSnapshotComplete() =>
        Kind == SyncSnapshotCompleteKind ? PayloadAs<SnapshotCompletion>() : null;

    public const string SyncSnapshotRequiredKind = "eitmad.sync.snapshot-required.v1";

    public static SyncMessage ForSyncSnapshotRequired(SnapshotRequired payload) =>
        new() { Kind = SyncSnapshotRequiredKind, Payload = payload };

    public SnapshotRequired? AsSyncSnapshotRequired() =>
        Kind == SyncSnapshotRequiredKind ? PayloadAs<SnapshotRequired>() : null;

    internal T? PayloadAs<T>() => Payload switch
    {
        T typed => typed,
        JsonElement element => element.Deserialize<T>(Converter.Settings),
        _ => default,
    };
}

public partial class AuthorizationPolicyChanges
{
}

public partial class BackgroundJobChanges
{
}

public partial class ConfigurationChanges
{
}

public partial class Errors
{
}

public partial class GetConfiguration
{
}

public partial class GetEffectivePermissions
{
}

public partial class GetSyncStatus
{
}

public partial class GetUpdateState
{
}

public partial class Notifications
{
}

public partial class PermissionChanges
{
}

public partial class RecordChanges
{
}

public partial class SyncStatusChanges
{
}

public partial class UpdateStateChanges
{
}
