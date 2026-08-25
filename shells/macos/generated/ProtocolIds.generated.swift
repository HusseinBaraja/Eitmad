// Generated from Rust contracts. Do not edit.
import Foundation

public enum ProtocolIds {
    public enum IpcMessages {
        public static let eitmadIpcCommandResponseV1 = "eitmad.ipc.command-response.v1"
        public static let eitmadIpcCommandV1 = "eitmad.ipc.command.v1"
        public static let eitmadIpcFailureV1 = "eitmad.ipc.failure.v1"
        public static let eitmadIpcEventV1 = "eitmad.ipc.event.v1"
        public static let eitmadIpcHandshakeResponseV1 = "eitmad.ipc.handshake-response.v1"
        public static let eitmadIpcHandshakeV1 = "eitmad.ipc.handshake.v1"
        public static let eitmadIpcQueryResponseV1 = "eitmad.ipc.query-response.v1"
        public static let eitmadIpcQueryV1 = "eitmad.ipc.query.v1"
        public static let eitmadIpcSubscribeResponseV1 = "eitmad.ipc.subscribe-response.v1"
        public static let eitmadIpcSubscribeV1 = "eitmad.ipc.subscribe.v1"
        public static let eitmadIpcSubscriptionClosedV1 = "eitmad.ipc.subscription-closed.v1"
        public static let eitmadIpcShutdownResponseV1 = "eitmad.ipc.shutdown-response.v1"
        public static let eitmadIpcShutdownV1 = "eitmad.ipc.shutdown.v1"
        public static let eitmadIpcUnsubscribeResponseV1 = "eitmad.ipc.unsubscribe-response.v1"
        public static let eitmadIpcUnsubscribeV1 = "eitmad.ipc.unsubscribe.v1"
    }

    public enum Commands {
        public static let eitmadConfigUpdateV1 = "eitmad.config.update.v1"
        public static let eitmadAuthorizationRelationshipGrantV1 = "eitmad.authorization.relationship.grant.v1"
        public static let eitmadAuthorizationRelationshipRevokeV1 = "eitmad.authorization.relationship.revoke.v1"
        public static let eitmadOperationCancelV1 = "eitmad.operation.cancel.v1"
        public static let eitmadUpdateReportInstallerOutcomeV1 = "eitmad.update.report-installer-outcome.v1"
    }

    public enum Queries {
        public static let eitmadConfigGetV1 = "eitmad.config.get.v1"
        public static let eitmadPermissionsGetEffectiveV1 = "eitmad.permissions.get-effective.v1"
        public static let eitmadAuthorizationRelationshipsListV1 = "eitmad.authorization.relationships.list.v1"
        public static let eitmadUpdateGetStateV1 = "eitmad.update.get-state.v1"
        public static let eitmadSyncGetStatusV1 = "eitmad.sync.get-status.v1"
    }

    public enum Subscriptions {
        public static let eitmadConfigChangedSubscribeV1 = "eitmad.config.changed.subscribe.v1"
        public static let eitmadPermissionsChangedSubscribeV1 = "eitmad.permissions.changed.subscribe.v1"
        public static let eitmadAuthorizationPolicyChangedSubscribeV1 = "eitmad.authorization.policy.changed.subscribe.v1"
        public static let eitmadUpdateStateSubscribeV1 = "eitmad.update.state.subscribe.v1"
        public static let eitmadSyncStatusSubscribeV1 = "eitmad.sync.status.subscribe.v1"
        public static let eitmadRecordChangedSubscribeV1 = "eitmad.record.changed.subscribe.v1"
        public static let eitmadBackgroundJobStatusSubscribeV1 = "eitmad.background-job.status.subscribe.v1"
        public static let eitmadNotificationSubscribeV1 = "eitmad.notification.subscribe.v1"
        public static let eitmadErrorSubscribeV1 = "eitmad.error.subscribe.v1"
    }

    public enum Events {
        public static let eitmadConfigChangedEventV1 = "eitmad.config.changed.event.v1"
        public static let eitmadPermissionsChangedEventV1 = "eitmad.permissions.changed.event.v1"
        public static let eitmadAuthorizationPolicyChangedEventV1 = "eitmad.authorization.policy.changed.event.v1"
        public static let eitmadUpdateStateEventV1 = "eitmad.update.state.event.v1"
        public static let eitmadSyncStatusEventV1 = "eitmad.sync.status.event.v1"
        public static let eitmadRecordChangedEventV1 = "eitmad.record.changed.event.v1"
        public static let eitmadBackgroundJobStatusEventV1 = "eitmad.background-job.status.event.v1"
        public static let eitmadNotificationEventV1 = "eitmad.notification.event.v1"
        public static let eitmadErrorEventV1 = "eitmad.error.event.v1"
    }

    public enum SyncMessages {
        public static let eitmadSyncNegotiateV1 = "eitmad.sync.negotiate.v1"
        public static let eitmadSyncPullV1 = "eitmad.sync.pull.v1"
        public static let eitmadSyncChangesV1 = "eitmad.sync.changes.v1"
        public static let eitmadSyncReconcileV1 = "eitmad.sync.reconcile.v1"
        public static let eitmadSyncAcknowledgeV1 = "eitmad.sync.acknowledge.v1"
        public static let eitmadSyncConflictV1 = "eitmad.sync.conflict.v1"
        public static let eitmadSyncBackpressureV1 = "eitmad.sync.backpressure.v1"
        public static let eitmadSyncSnapshotManifestV1 = "eitmad.sync.snapshot-manifest.v1"
        public static let eitmadSyncSnapshotChunkV1 = "eitmad.sync.snapshot-chunk.v1"
        public static let eitmadSyncSnapshotCompleteV1 = "eitmad.sync.snapshot-complete.v1"
        public static let eitmadSyncSnapshotRequiredV1 = "eitmad.sync.snapshot-required.v1"
    }

    public enum ServerMessages {
        public static let eitmadServerHelloV1 = "eitmad.server.hello.v1"
        public static let eitmadServerSyncV1 = "eitmad.server.sync.v1"
        public static let eitmadServerSubscribeV1 = "eitmad.server.subscribe.v1"
        public static let eitmadServerAcknowledgeV1 = "eitmad.server.acknowledge.v1"
        public static let eitmadServerHelloAcceptedV1 = "eitmad.server.hello-accepted.v1"
        public static let eitmadServerSyncMessageV1 = "eitmad.server.sync-message.v1"
        public static let eitmadServerEventV1 = "eitmad.server.event.v1"
        public static let eitmadServerFailureV1 = "eitmad.server.failure.v1"
    }

    public enum Capabilities {
        public static let eitmadCapabilityEngineLifecycleV1 = "eitmad.capability.engine-lifecycle.v1"
        public static let eitmadCapabilityLocalIpcV1 = "eitmad.capability.local-ipc.v1"
        public static let eitmadCapabilityLocalIpcSubscriptionsV1 = "eitmad.capability.local-ipc-subscriptions.v1"
        public static let eitmadCapabilityAuthorizationPolicyEventsV1 = "eitmad.capability.authorization-policy-events.v1"
        public static let eitmadCapabilityAuthorizationScopesV1 = "eitmad.capability.authorization-scopes.v1"
        public static let eitmadCapabilityConfigV1 = "eitmad.capability.config.v1"
        public static let eitmadCapabilityPermissionsV1 = "eitmad.capability.permissions.v1"
        public static let eitmadCapabilitySyncV1 = "eitmad.capability.sync.v1"
        public static let eitmadCapabilityServerConnectionV1 = "eitmad.capability.server-connection.v1"
        public static let eitmadCapabilityServerDeviceProofV1 = "eitmad.capability.server-device-proof.v1"
        public static let eitmadCapabilityServerSnapshotChunksV1 = "eitmad.capability.server-snapshot-chunks.v1"
        public static let eitmadCapabilityServerSubscriptionResumeV1 = "eitmad.capability.server-subscription-resume.v1"
        public static let eitmadCapabilityServerRelayV1 = "eitmad.capability.server-relay.v1"
        public static let eitmadCapabilityServerUpdateDistributionV1 = "eitmad.capability.server-update-distribution.v1"
        public static let eitmadCapabilityServerAdministrationV1 = "eitmad.capability.server-administration.v1"
        public static let eitmadCapabilityUpdateV1 = "eitmad.capability.update.v1"
    }

    public enum Permissions {
        public static let eitmadPermissionConfigReadV1 = "eitmad.permission.config.read.v1"
        public static let eitmadPermissionConfigWriteV1 = "eitmad.permission.config.write.v1"
        public static let eitmadPermissionConfigImportV1 = "eitmad.permission.config.import.v1"
        public static let eitmadPermissionConfigExportV1 = "eitmad.permission.config.export.v1"
        public static let eitmadPermissionAuthorizationManageV1 = "eitmad.permission.authorization.manage.v1"
        public static let eitmadPermissionPermissionsReadV1 = "eitmad.permission.permissions.read.v1"
        public static let eitmadPermissionObservabilitySensitiveDebugV1 = "eitmad.permission.observability.sensitive-debug.v1"
        public static let eitmadPermissionSyncReadV1 = "eitmad.permission.sync.read.v1"
        public static let eitmadPermissionServerAccountsManageV1 = "eitmad.permission.server.accounts.manage.v1"
        public static let eitmadPermissionServerDevicesManageV1 = "eitmad.permission.server.devices.manage.v1"
        public static let eitmadPermissionServerLicenseReadV1 = "eitmad.permission.server.license.read.v1"
        public static let eitmadPermissionServerUpdateChannelManageV1 = "eitmad.permission.server.update-channel.manage.v1"
        public static let eitmadPermissionServerRelayConnectV1 = "eitmad.permission.server.relay.connect.v1"
        public static let eitmadPermissionServerRelayHealthReadV1 = "eitmad.permission.server.relay.health.read.v1"
        public static let eitmadPermissionServerRelayFailureReportV1 = "eitmad.permission.server.relay.failure.report.v1"
        public static let eitmadPermissionServerRelayAdminCloseV1 = "eitmad.permission.server.relay.admin-close.v1"
        public static let eitmadPermissionServerUpdateManifestPublishV1 = "eitmad.permission.server.update-manifest.publish.v1"
        public static let eitmadPermissionServerAdminDiagnosticsReadV1 = "eitmad.permission.server.admin.diagnostics.read.v1"
        public static let eitmadPermissionServerAdminHealthReadV1 = "eitmad.permission.server.admin.health.read.v1"
        public static let eitmadPermissionServerAdminBackupReadV1 = "eitmad.permission.server.admin.backup.read.v1"
        public static let eitmadPermissionServerAdminMigrationReadV1 = "eitmad.permission.server.admin.migration.read.v1"
        public static let eitmadPermissionServerAdminAuditReadV1 = "eitmad.permission.server.admin.audit.read.v1"
        public static let eitmadPermissionServerAdminTenantReadV1 = "eitmad.permission.server.admin.tenant.read.v1"
        public static let eitmadPermissionServerAdminDeviceReadV1 = "eitmad.permission.server.admin.device.read.v1"
        public static let eitmadPermissionServerAdminSupportExecuteV1 = "eitmad.permission.server.admin.support.execute.v1"
        public static let eitmadPermissionUpdateReadV1 = "eitmad.permission.update.read.v1"
        public static let eitmadPermissionUpdateReportInstallerV1 = "eitmad.permission.update.report-installer.v1"
    }

    public enum ConfigKeys {
        public static let eitmadConfigLocalePrimaryV1 = "eitmad.config.locale.primary.v1"
    }

    public enum Relations {
        public static let eitmadRelationOrganizationConfigManagerV1 = "eitmad.relation.organization.config-manager.v1"
        public static let eitmadRelationOrganizationMemberV1 = "eitmad.relation.organization.member.v1"
        public static let eitmadRelationOrganizationOwnerV1 = "eitmad.relation.organization.owner.v1"
    }

    public enum SchemaIds {
        public static let eitmadSchemaProtocolV1 = "eitmad.schema.protocol.v1"
    }

    public enum ErrorCodes {
        public static let eitmadErrorAuthorizationDeniedV1 = "eitmad.error.authorization-denied.v1"
        public static let eitmadErrorAuthorizationLastOwnerV1 = "eitmad.error.authorization-last-owner.v1"
        public static let eitmadErrorAuthorizationPolicyConflictV1 = "eitmad.error.authorization-policy-conflict.v1"
        public static let eitmadErrorAuthorizationRelationInvalidV1 = "eitmad.error.authorization-relation-invalid.v1"
        public static let eitmadErrorAuthorizationUnavailableV1 = "eitmad.error.authorization-unavailable.v1"
        public static let eitmadErrorConfigInvalidV1 = "eitmad.error.config-invalid.v1"
        public static let eitmadErrorConfigUnavailableV1 = "eitmad.error.config-unavailable.v1"
        public static let eitmadErrorConfigRevisionConflictV1 = "eitmad.error.config-revision-conflict.v1"
        public static let eitmadErrorContractInvalidV1 = "eitmad.error.contract-invalid.v1"
        public static let eitmadErrorEngineAlreadyRunningV1 = "eitmad.error.engine-already-running.v1"
        public static let eitmadErrorEngineHealthCheckFailedV1 = "eitmad.error.engine-health-check-failed.v1"
        public static let eitmadErrorEngineShutdownFailedV1 = "eitmad.error.engine-shutdown-failed.v1"
        public static let eitmadErrorEngineStartupFailedV1 = "eitmad.error.engine-startup-failed.v1"
        public static let eitmadErrorEngineSupervisorInvalidV1 = "eitmad.error.engine-supervisor-invalid.v1"
        public static let eitmadErrorIpcEngineStoppingV1 = "eitmad.error.ipc-engine-stopping.v1"
        public static let eitmadErrorIpcPayloadTooLargeV1 = "eitmad.error.ipc-payload-too-large.v1"
        public static let eitmadErrorIpcSessionInvalidV1 = "eitmad.error.ipc-session-invalid.v1"
        public static let eitmadErrorIpcSubscriptionResyncRequiredV1 = "eitmad.error.ipc-subscription-resync-required.v1"
        public static let eitmadErrorIpcSubscriptionUnsupportedV1 = "eitmad.error.ipc-subscription-unsupported.v1"
        public static let eitmadErrorIpcDeadlineExceededV1 = "eitmad.error.ipc-deadline-exceeded.v1"
        public static let eitmadErrorProtocolIncompatibleV1 = "eitmad.error.protocol-incompatible.v1"
        public static let eitmadErrorSyncBackpressureV1 = "eitmad.error.sync-backpressure.v1"
        public static let eitmadErrorServerAuthenticationFailedV1 = "eitmad.error.server-authentication-failed.v1"
        public static let eitmadErrorServerBootstrapFailedV1 = "eitmad.error.server-bootstrap-failed.v1"
        public static let eitmadErrorServerClientIncompatibleV1 = "eitmad.error.server-client-incompatible.v1"
        public static let eitmadErrorServerConfigInvalidV1 = "eitmad.error.server-config-invalid.v1"
        public static let eitmadErrorServerDatabaseUnavailableV1 = "eitmad.error.server-database-unavailable.v1"
        public static let eitmadErrorServerDeviceProofInvalidV1 = "eitmad.error.server-device-proof-invalid.v1"
        public static let eitmadErrorServerIdempotencyMismatchV1 = "eitmad.error.server-idempotency-mismatch.v1"
        public static let eitmadErrorServerLicenseRequiredV1 = "eitmad.error.server-license-required.v1"
        public static let eitmadErrorServerMigrationFailedV1 = "eitmad.error.server-migration-failed.v1"
        public static let eitmadErrorServerRuntimeFailedV1 = "eitmad.error.server-runtime-failed.v1"
        public static let eitmadErrorServerSnapshotRequiredV1 = "eitmad.error.server-snapshot-required.v1"
        public static let eitmadErrorServerTokenExpiredV1 = "eitmad.error.server-token-expired.v1"
        public static let eitmadErrorServerTokenReuseV1 = "eitmad.error.server-token-reuse.v1"
        public static let eitmadErrorRelaySessionNotFoundV1 = "eitmad.error.relay-session-not-found.v1"
        public static let eitmadErrorRelayUnavailableV1 = "eitmad.error.relay-unavailable.v1"
        public static let eitmadErrorUpdateManifestInvalidV1 = "eitmad.error.update-manifest-invalid.v1"
        public static let eitmadErrorUpdateManifestNotFoundV1 = "eitmad.error.update-manifest-not-found.v1"
        public static let eitmadErrorUpdateDistributionUnavailableV1 = "eitmad.error.update-distribution-unavailable.v1"
        public static let eitmadErrorAdminUnavailableV1 = "eitmad.error.admin-unavailable.v1"
        public static let eitmadErrorUpdateInstallerFailedV1 = "eitmad.error.update-installer-failed.v1"
    }

    public enum MessageIds {
        public static let eitmadMessageAuthorizationDeniedV1 = "eitmad.message.authorization-denied.v1"
        public static let eitmadMessageAuthorizationLastOwnerV1 = "eitmad.message.authorization-last-owner.v1"
        public static let eitmadMessageAuthorizationPolicyConflictV1 = "eitmad.message.authorization-policy-conflict.v1"
        public static let eitmadMessageAuthorizationRelationInvalidV1 = "eitmad.message.authorization-relation-invalid.v1"
        public static let eitmadMessageAuthorizationUnavailableV1 = "eitmad.message.authorization-unavailable.v1"
        public static let eitmadMessageConfigInvalidV1 = "eitmad.message.config-invalid.v1"
        public static let eitmadMessageConfigUnavailableV1 = "eitmad.message.config-unavailable.v1"
        public static let eitmadMessageConfigRevisionConflictV1 = "eitmad.message.config-revision-conflict.v1"
        public static let eitmadMessageContractInvalidV1 = "eitmad.message.contract-invalid.v1"
        public static let eitmadMessageEngineAlreadyRunningV1 = "eitmad.message.engine-already-running.v1"
        public static let eitmadMessageEngineHealthCheckFailedV1 = "eitmad.message.engine-health-check-failed.v1"
        public static let eitmadMessageEngineShutdownFailedV1 = "eitmad.message.engine-shutdown-failed.v1"
        public static let eitmadMessageEngineStartupFailedV1 = "eitmad.message.engine-startup-failed.v1"
        public static let eitmadMessageEngineSupervisorInvalidV1 = "eitmad.message.engine-supervisor-invalid.v1"
        public static let eitmadMessageIpcEngineStoppingV1 = "eitmad.message.ipc-engine-stopping.v1"
        public static let eitmadMessageIpcPayloadTooLargeV1 = "eitmad.message.ipc-payload-too-large.v1"
        public static let eitmadMessageIpcSessionInvalidV1 = "eitmad.message.ipc-session-invalid.v1"
        public static let eitmadMessageIpcSubscriptionResyncRequiredV1 = "eitmad.message.ipc-subscription-resync-required.v1"
        public static let eitmadMessageIpcSubscriptionUnsupportedV1 = "eitmad.message.ipc-subscription-unsupported.v1"
        public static let eitmadMessageIpcDeadlineExceededV1 = "eitmad.message.ipc-deadline-exceeded.v1"
        public static let eitmadMessageObservabilitySensitiveDebugWarningV1 = "eitmad.message.observability-sensitive-debug-warning.v1"
        public static let eitmadMessageProtocolIncompatibleV1 = "eitmad.message.protocol-incompatible.v1"
        public static let eitmadMessageSyncBackpressureV1 = "eitmad.message.sync-backpressure.v1"
        public static let eitmadMessageServerAuthenticationFailedV1 = "eitmad.message.server-authentication-failed.v1"
        public static let eitmadMessageServerBootstrapFailedV1 = "eitmad.message.server-bootstrap-failed.v1"
        public static let eitmadMessageServerClientIncompatibleV1 = "eitmad.message.server-client-incompatible.v1"
        public static let eitmadMessageServerConfigInvalidV1 = "eitmad.message.server-config-invalid.v1"
        public static let eitmadMessageServerDatabaseUnavailableV1 = "eitmad.message.server-database-unavailable.v1"
        public static let eitmadMessageServerDeviceProofInvalidV1 = "eitmad.message.server-device-proof-invalid.v1"
        public static let eitmadMessageServerIdempotencyMismatchV1 = "eitmad.message.server-idempotency-mismatch.v1"
        public static let eitmadMessageServerLicenseRequiredV1 = "eitmad.message.server-license-required.v1"
        public static let eitmadMessageServerMigrationFailedV1 = "eitmad.message.server-migration-failed.v1"
        public static let eitmadMessageServerRuntimeFailedV1 = "eitmad.message.server-runtime-failed.v1"
        public static let eitmadMessageServerSnapshotRequiredV1 = "eitmad.message.server-snapshot-required.v1"
        public static let eitmadMessageServerTokenExpiredV1 = "eitmad.message.server-token-expired.v1"
        public static let eitmadMessageServerTokenReuseV1 = "eitmad.message.server-token-reuse.v1"
        public static let eitmadMessageRelaySessionNotFoundV1 = "eitmad.message.relay-session-not-found.v1"
        public static let eitmadMessageRelayUnavailableV1 = "eitmad.message.relay-unavailable.v1"
        public static let eitmadMessageUpdateManifestInvalidV1 = "eitmad.message.update-manifest-invalid.v1"
        public static let eitmadMessageUpdateManifestNotFoundV1 = "eitmad.message.update-manifest-not-found.v1"
        public static let eitmadMessageUpdateDistributionUnavailableV1 = "eitmad.message.update-distribution-unavailable.v1"
        public static let eitmadMessageAdminUnavailableV1 = "eitmad.message.admin-unavailable.v1"
        public static let eitmadMessageUpdateInstallerFailedV1 = "eitmad.message.update-installer-failed.v1"
    }

    public enum ErrorParameterNames {
        public static let actualRevision = "actual-revision"
        public static let configurationKey = "configuration-key"
        public static let expectedRevision = "expected-revision"
        public static let relation = "relation"
        public static let requiredCapability = "required-capability"
        public static let retryAfterMs = "retry-after-ms"
        public static let maximumPayloadBytes = "maximum-payload-bytes"
    }

}

public struct OpenProtocolId: RawRepresentable, Codable, Hashable, Sendable {
    public let rawValue: String

    public init?(rawValue: String) {
        let allowed = CharacterSet(charactersIn: "abcdefghijklmnopqrstuvwxyz0123456789.-_")
        guard (3...128).contains(rawValue.utf8.count),
              rawValue.unicodeScalars.allSatisfy(allowed.contains),
              rawValue.first?.isLowercase == true,
              rawValue.last?.isLetter == true || rawValue.last?.isNumber == true,
              !rawValue.contains(".."), !rawValue.contains("--"), !rawValue.contains("__")
        else { return nil }
        self.rawValue = rawValue
    }
}
