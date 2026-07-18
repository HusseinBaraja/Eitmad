// Generated from Rust contracts. Do not edit.
using System;
using System.Linq;

namespace Eitmad.Contracts;

public static class ProtocolIds
{
    public static class IpcMessages
    {
        public const string EitmadIpcCommandResponseV1 = "eitmad.ipc.command-response.v1";
        public const string EitmadIpcCommandV1 = "eitmad.ipc.command.v1";
        public const string EitmadIpcFailureV1 = "eitmad.ipc.failure.v1";
        public const string EitmadIpcEventV1 = "eitmad.ipc.event.v1";
        public const string EitmadIpcHandshakeResponseV1 = "eitmad.ipc.handshake-response.v1";
        public const string EitmadIpcHandshakeV1 = "eitmad.ipc.handshake.v1";
        public const string EitmadIpcQueryResponseV1 = "eitmad.ipc.query-response.v1";
        public const string EitmadIpcQueryV1 = "eitmad.ipc.query.v1";
        public const string EitmadIpcSubscribeResponseV1 = "eitmad.ipc.subscribe-response.v1";
        public const string EitmadIpcSubscribeV1 = "eitmad.ipc.subscribe.v1";
        public const string EitmadIpcSubscriptionClosedV1 = "eitmad.ipc.subscription-closed.v1";
        public const string EitmadIpcShutdownResponseV1 = "eitmad.ipc.shutdown-response.v1";
        public const string EitmadIpcShutdownV1 = "eitmad.ipc.shutdown.v1";
        public const string EitmadIpcUnsubscribeResponseV1 = "eitmad.ipc.unsubscribe-response.v1";
        public const string EitmadIpcUnsubscribeV1 = "eitmad.ipc.unsubscribe.v1";
    }

    public static class Commands
    {
        public const string EitmadConfigUpdateV1 = "eitmad.config.update.v1";
        public const string EitmadAuthorizationRelationshipGrantV1 = "eitmad.authorization.relationship.grant.v1";
        public const string EitmadAuthorizationRelationshipRevokeV1 = "eitmad.authorization.relationship.revoke.v1";
        public const string EitmadOperationCancelV1 = "eitmad.operation.cancel.v1";
        public const string EitmadUpdateReportInstallerOutcomeV1 = "eitmad.update.report-installer-outcome.v1";
    }

    public static class Queries
    {
        public const string EitmadConfigGetV1 = "eitmad.config.get.v1";
        public const string EitmadPermissionsGetEffectiveV1 = "eitmad.permissions.get-effective.v1";
        public const string EitmadAuthorizationRelationshipsListV1 = "eitmad.authorization.relationships.list.v1";
        public const string EitmadUpdateGetStateV1 = "eitmad.update.get-state.v1";
        public const string EitmadSyncGetStatusV1 = "eitmad.sync.get-status.v1";
    }

    public static class Subscriptions
    {
        public const string EitmadConfigChangedSubscribeV1 = "eitmad.config.changed.subscribe.v1";
        public const string EitmadPermissionsChangedSubscribeV1 = "eitmad.permissions.changed.subscribe.v1";
        public const string EitmadAuthorizationPolicyChangedSubscribeV1 = "eitmad.authorization.policy.changed.subscribe.v1";
        public const string EitmadUpdateStateSubscribeV1 = "eitmad.update.state.subscribe.v1";
        public const string EitmadSyncStatusSubscribeV1 = "eitmad.sync.status.subscribe.v1";
        public const string EitmadRecordChangedSubscribeV1 = "eitmad.record.changed.subscribe.v1";
        public const string EitmadBackgroundJobStatusSubscribeV1 = "eitmad.background-job.status.subscribe.v1";
        public const string EitmadNotificationSubscribeV1 = "eitmad.notification.subscribe.v1";
        public const string EitmadErrorSubscribeV1 = "eitmad.error.subscribe.v1";
    }

    public static class Events
    {
        public const string EitmadConfigChangedEventV1 = "eitmad.config.changed.event.v1";
        public const string EitmadPermissionsChangedEventV1 = "eitmad.permissions.changed.event.v1";
        public const string EitmadAuthorizationPolicyChangedEventV1 = "eitmad.authorization.policy.changed.event.v1";
        public const string EitmadUpdateStateEventV1 = "eitmad.update.state.event.v1";
        public const string EitmadSyncStatusEventV1 = "eitmad.sync.status.event.v1";
        public const string EitmadRecordChangedEventV1 = "eitmad.record.changed.event.v1";
        public const string EitmadBackgroundJobStatusEventV1 = "eitmad.background-job.status.event.v1";
        public const string EitmadNotificationEventV1 = "eitmad.notification.event.v1";
        public const string EitmadErrorEventV1 = "eitmad.error.event.v1";
    }

    public static class SyncMessages
    {
        public const string EitmadSyncNegotiateV1 = "eitmad.sync.negotiate.v1";
        public const string EitmadSyncPullV1 = "eitmad.sync.pull.v1";
        public const string EitmadSyncChangesV1 = "eitmad.sync.changes.v1";
        public const string EitmadSyncAcknowledgeV1 = "eitmad.sync.acknowledge.v1";
        public const string EitmadSyncConflictV1 = "eitmad.sync.conflict.v1";
        public const string EitmadSyncBackpressureV1 = "eitmad.sync.backpressure.v1";
    }

    public static class Capabilities
    {
        public const string EitmadCapabilityEngineLifecycleV1 = "eitmad.capability.engine-lifecycle.v1";
        public const string EitmadCapabilityLocalIpcV1 = "eitmad.capability.local-ipc.v1";
        public const string EitmadCapabilityLocalIpcSubscriptionsV1 = "eitmad.capability.local-ipc-subscriptions.v1";
        public const string EitmadCapabilityAuthorizationPolicyEventsV1 = "eitmad.capability.authorization-policy-events.v1";
        public const string EitmadCapabilityConfigV1 = "eitmad.capability.config.v1";
        public const string EitmadCapabilityPermissionsV1 = "eitmad.capability.permissions.v1";
        public const string EitmadCapabilitySyncV1 = "eitmad.capability.sync.v1";
        public const string EitmadCapabilityUpdateV1 = "eitmad.capability.update.v1";
    }

    public static class Permissions
    {
        public const string EitmadPermissionConfigReadV1 = "eitmad.permission.config.read.v1";
        public const string EitmadPermissionConfigWriteV1 = "eitmad.permission.config.write.v1";
        public const string EitmadPermissionConfigImportV1 = "eitmad.permission.config.import.v1";
        public const string EitmadPermissionConfigExportV1 = "eitmad.permission.config.export.v1";
        public const string EitmadPermissionAuthorizationManageV1 = "eitmad.permission.authorization.manage.v1";
        public const string EitmadPermissionPermissionsReadV1 = "eitmad.permission.permissions.read.v1";
        public const string EitmadPermissionSyncReadV1 = "eitmad.permission.sync.read.v1";
        public const string EitmadPermissionUpdateReadV1 = "eitmad.permission.update.read.v1";
        public const string EitmadPermissionUpdateReportInstallerV1 = "eitmad.permission.update.report-installer.v1";
    }

    public static class ConfigKeys
    {
        public const string EitmadConfigLocalePrimaryV1 = "eitmad.config.locale.primary.v1";
    }

    public static class Relations
    {
        public const string EitmadRelationOrganizationConfigManagerV1 = "eitmad.relation.organization.config-manager.v1";
        public const string EitmadRelationOrganizationMemberV1 = "eitmad.relation.organization.member.v1";
        public const string EitmadRelationOrganizationOwnerV1 = "eitmad.relation.organization.owner.v1";
    }

    public static class SchemaIds
    {
        public const string EitmadSchemaProtocolV1 = "eitmad.schema.protocol.v1";
    }

    public static class ErrorCodes
    {
        public const string EitmadErrorAuthorizationDeniedV1 = "eitmad.error.authorization-denied.v1";
        public const string EitmadErrorAuthorizationLastOwnerV1 = "eitmad.error.authorization-last-owner.v1";
        public const string EitmadErrorAuthorizationPolicyConflictV1 = "eitmad.error.authorization-policy-conflict.v1";
        public const string EitmadErrorAuthorizationRelationInvalidV1 = "eitmad.error.authorization-relation-invalid.v1";
        public const string EitmadErrorAuthorizationUnavailableV1 = "eitmad.error.authorization-unavailable.v1";
        public const string EitmadErrorConfigInvalidV1 = "eitmad.error.config-invalid.v1";
        public const string EitmadErrorConfigUnavailableV1 = "eitmad.error.config-unavailable.v1";
        public const string EitmadErrorConfigRevisionConflictV1 = "eitmad.error.config-revision-conflict.v1";
        public const string EitmadErrorContractInvalidV1 = "eitmad.error.contract-invalid.v1";
        public const string EitmadErrorEngineAlreadyRunningV1 = "eitmad.error.engine-already-running.v1";
        public const string EitmadErrorEngineHealthCheckFailedV1 = "eitmad.error.engine-health-check-failed.v1";
        public const string EitmadErrorEngineShutdownFailedV1 = "eitmad.error.engine-shutdown-failed.v1";
        public const string EitmadErrorEngineStartupFailedV1 = "eitmad.error.engine-startup-failed.v1";
        public const string EitmadErrorEngineSupervisorInvalidV1 = "eitmad.error.engine-supervisor-invalid.v1";
        public const string EitmadErrorIpcEngineStoppingV1 = "eitmad.error.ipc-engine-stopping.v1";
        public const string EitmadErrorIpcPayloadTooLargeV1 = "eitmad.error.ipc-payload-too-large.v1";
        public const string EitmadErrorIpcSessionInvalidV1 = "eitmad.error.ipc-session-invalid.v1";
        public const string EitmadErrorIpcSubscriptionResyncRequiredV1 = "eitmad.error.ipc-subscription-resync-required.v1";
        public const string EitmadErrorIpcSubscriptionUnsupportedV1 = "eitmad.error.ipc-subscription-unsupported.v1";
        public const string EitmadErrorIpcDeadlineExceededV1 = "eitmad.error.ipc-deadline-exceeded.v1";
        public const string EitmadErrorProtocolIncompatibleV1 = "eitmad.error.protocol-incompatible.v1";
        public const string EitmadErrorSyncBackpressureV1 = "eitmad.error.sync-backpressure.v1";
        public const string EitmadErrorUpdateInstallerFailedV1 = "eitmad.error.update-installer-failed.v1";
    }

    public static class MessageIds
    {
        public const string EitmadMessageAuthorizationDeniedV1 = "eitmad.message.authorization-denied.v1";
        public const string EitmadMessageAuthorizationLastOwnerV1 = "eitmad.message.authorization-last-owner.v1";
        public const string EitmadMessageAuthorizationPolicyConflictV1 = "eitmad.message.authorization-policy-conflict.v1";
        public const string EitmadMessageAuthorizationRelationInvalidV1 = "eitmad.message.authorization-relation-invalid.v1";
        public const string EitmadMessageAuthorizationUnavailableV1 = "eitmad.message.authorization-unavailable.v1";
        public const string EitmadMessageConfigInvalidV1 = "eitmad.message.config-invalid.v1";
        public const string EitmadMessageConfigUnavailableV1 = "eitmad.message.config-unavailable.v1";
        public const string EitmadMessageConfigRevisionConflictV1 = "eitmad.message.config-revision-conflict.v1";
        public const string EitmadMessageContractInvalidV1 = "eitmad.message.contract-invalid.v1";
        public const string EitmadMessageEngineAlreadyRunningV1 = "eitmad.message.engine-already-running.v1";
        public const string EitmadMessageEngineHealthCheckFailedV1 = "eitmad.message.engine-health-check-failed.v1";
        public const string EitmadMessageEngineShutdownFailedV1 = "eitmad.message.engine-shutdown-failed.v1";
        public const string EitmadMessageEngineStartupFailedV1 = "eitmad.message.engine-startup-failed.v1";
        public const string EitmadMessageEngineSupervisorInvalidV1 = "eitmad.message.engine-supervisor-invalid.v1";
        public const string EitmadMessageIpcEngineStoppingV1 = "eitmad.message.ipc-engine-stopping.v1";
        public const string EitmadMessageIpcPayloadTooLargeV1 = "eitmad.message.ipc-payload-too-large.v1";
        public const string EitmadMessageIpcSessionInvalidV1 = "eitmad.message.ipc-session-invalid.v1";
        public const string EitmadMessageIpcSubscriptionResyncRequiredV1 = "eitmad.message.ipc-subscription-resync-required.v1";
        public const string EitmadMessageIpcSubscriptionUnsupportedV1 = "eitmad.message.ipc-subscription-unsupported.v1";
        public const string EitmadMessageIpcDeadlineExceededV1 = "eitmad.message.ipc-deadline-exceeded.v1";
        public const string EitmadMessageProtocolIncompatibleV1 = "eitmad.message.protocol-incompatible.v1";
        public const string EitmadMessageSyncBackpressureV1 = "eitmad.message.sync-backpressure.v1";
        public const string EitmadMessageUpdateInstallerFailedV1 = "eitmad.message.update-installer-failed.v1";
    }

    public static class ErrorParameterNames
    {
        public const string ActualRevision = "actual-revision";
        public const string ConfigurationKey = "configuration-key";
        public const string ExpectedRevision = "expected-revision";
        public const string Relation = "relation";
        public const string RequiredCapability = "required-capability";
        public const string RetryAfterMs = "retry-after-ms";
        public const string MaximumPayloadBytes = "maximum-payload-bytes";
    }

}

public readonly record struct OpenProtocolId(string Value)
{
    public static bool TryParse(string value, out OpenProtocolId identifier)
    {
        var valid = value is { Length: >= 3 and <= 128 }
            && char.IsAsciiLetterLower(value[0])
            && (char.IsAsciiLetterLower(value[^1]) || char.IsAsciiDigit(value[^1]))
            && value.All(character => char.IsAsciiLetterLower(character)
                || char.IsAsciiDigit(character) || character is '.' or '-' or '_')
            && !value.Contains("..") && !value.Contains("--") && !value.Contains("__");
        identifier = valid ? new OpenProtocolId(value) : default;
        return valid;
    }
}
