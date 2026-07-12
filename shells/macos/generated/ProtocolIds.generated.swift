// Generated from Rust contracts. Do not edit.
import Foundation

public enum ProtocolIds {
    public enum Commands {
        public static let eitmadConfigUpdateV1 = "eitmad.config.update.v1"
        public static let eitmadOperationCancelV1 = "eitmad.operation.cancel.v1"
        public static let eitmadUpdateReportInstallerOutcomeV1 = "eitmad.update.report-installer-outcome.v1"
    }

    public enum Queries {
        public static let eitmadConfigGetV1 = "eitmad.config.get.v1"
        public static let eitmadPermissionsGetEffectiveV1 = "eitmad.permissions.get-effective.v1"
        public static let eitmadUpdateGetStateV1 = "eitmad.update.get-state.v1"
        public static let eitmadSyncGetStatusV1 = "eitmad.sync.get-status.v1"
    }

    public enum Subscriptions {
        public static let eitmadConfigChangedSubscribeV1 = "eitmad.config.changed.subscribe.v1"
        public static let eitmadPermissionsChangedSubscribeV1 = "eitmad.permissions.changed.subscribe.v1"
        public static let eitmadUpdateStateSubscribeV1 = "eitmad.update.state.subscribe.v1"
        public static let eitmadSyncStatusSubscribeV1 = "eitmad.sync.status.subscribe.v1"
    }

    public enum Events {
        public static let eitmadConfigChangedEventV1 = "eitmad.config.changed.event.v1"
        public static let eitmadPermissionsChangedEventV1 = "eitmad.permissions.changed.event.v1"
        public static let eitmadUpdateStateEventV1 = "eitmad.update.state.event.v1"
        public static let eitmadSyncStatusEventV1 = "eitmad.sync.status.event.v1"
    }

    public enum SyncMessages {
        public static let eitmadSyncNegotiateV1 = "eitmad.sync.negotiate.v1"
        public static let eitmadSyncPullV1 = "eitmad.sync.pull.v1"
        public static let eitmadSyncChangesV1 = "eitmad.sync.changes.v1"
        public static let eitmadSyncAcknowledgeV1 = "eitmad.sync.acknowledge.v1"
        public static let eitmadSyncConflictV1 = "eitmad.sync.conflict.v1"
        public static let eitmadSyncBackpressureV1 = "eitmad.sync.backpressure.v1"
    }

    public enum Capabilities {
        public static let eitmadCapabilityConfigV1 = "eitmad.capability.config.v1"
        public static let eitmadCapabilityPermissionsV1 = "eitmad.capability.permissions.v1"
        public static let eitmadCapabilitySyncV1 = "eitmad.capability.sync.v1"
        public static let eitmadCapabilityUpdateV1 = "eitmad.capability.update.v1"
    }

    public enum Permissions {
        public static let eitmadPermissionConfigReadV1 = "eitmad.permission.config.read.v1"
        public static let eitmadPermissionConfigWriteV1 = "eitmad.permission.config.write.v1"
        public static let eitmadPermissionPermissionsReadV1 = "eitmad.permission.permissions.read.v1"
        public static let eitmadPermissionSyncReadV1 = "eitmad.permission.sync.read.v1"
        public static let eitmadPermissionUpdateReadV1 = "eitmad.permission.update.read.v1"
        public static let eitmadPermissionUpdateReportInstallerV1 = "eitmad.permission.update.report-installer.v1"
    }

    public enum ConfigKeys {
        public static let eitmadConfigLocalePrimaryV1 = "eitmad.config.locale.primary.v1"
    }

    public enum SchemaIds {
        public static let eitmadSchemaProtocolV1 = "eitmad.schema.protocol.v1"
    }

    public enum ErrorCodes {
        public static let eitmadErrorAuthorizationDeniedV1 = "eitmad.error.authorization-denied.v1"
        public static let eitmadErrorConfigRevisionConflictV1 = "eitmad.error.config-revision-conflict.v1"
        public static let eitmadErrorContractInvalidV1 = "eitmad.error.contract-invalid.v1"
        public static let eitmadErrorProtocolIncompatibleV1 = "eitmad.error.protocol-incompatible.v1"
        public static let eitmadErrorSyncBackpressureV1 = "eitmad.error.sync-backpressure.v1"
        public static let eitmadErrorUpdateInstallerFailedV1 = "eitmad.error.update-installer-failed.v1"
    }

    public enum MessageIds {
        public static let eitmadMessageAuthorizationDeniedV1 = "eitmad.message.authorization-denied.v1"
        public static let eitmadMessageConfigRevisionConflictV1 = "eitmad.message.config-revision-conflict.v1"
        public static let eitmadMessageContractInvalidV1 = "eitmad.message.contract-invalid.v1"
        public static let eitmadMessageProtocolIncompatibleV1 = "eitmad.message.protocol-incompatible.v1"
        public static let eitmadMessageSyncBackpressureV1 = "eitmad.message.sync-backpressure.v1"
        public static let eitmadMessageUpdateInstallerFailedV1 = "eitmad.message.update-installer-failed.v1"
    }

    public enum ErrorParameterNames {
        public static let actualRevision = "actual-revision"
        public static let expectedRevision = "expected-revision"
        public static let requiredCapability = "required-capability"
        public static let retryAfterMs = "retry-after-ms"
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
