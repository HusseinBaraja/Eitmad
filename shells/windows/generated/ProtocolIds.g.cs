// Generated from Rust contracts. Do not edit.
using System;
using System.Linq;

namespace Eitmad.Contracts;

public static class ProtocolIds
{
    public static class Version
    {
        public const long Major = 1;
        public const long Minor = 15;
    }

    public static class IpcMessages
    {
        public const string EitmadIpcDesktopSessionResponseV1 = "eitmad.ipc.desktop-session-response.v1";
        public const string EitmadIpcDesktopSessionStateV1 = "eitmad.ipc.desktop-session-state.v1";
        public const string EitmadIpcDesktopSignInV1 = "eitmad.ipc.desktop-sign-in.v1";
        public const string EitmadIpcDesktopSignOutV1 = "eitmad.ipc.desktop-sign-out.v1";
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
        public const string EitmadPricingPublishV1 = "eitmad.pricing.publish.v1";
        public const string EitmadCatalogImageImportV1 = "eitmad.catalog-image.import.v1";
        public const string EitmadConfigUpdateV1 = "eitmad.config.update.v1";
        public const string EitmadAuthorizationRelationshipGrantV1 = "eitmad.authorization.relationship.grant.v1";
        public const string EitmadAuthorizationRelationshipRevokeV1 = "eitmad.authorization.relationship.revoke.v1";
        public const string EitmadCustomerCreateV1 = "eitmad.customer.create.v1";
        public const string EitmadCustomerUpdateV1 = "eitmad.customer.update.v1";
        public const string EitmadMaterialCategorySaveV1 = "eitmad.material-category.save.v1";
        public const string EitmadMaterialUnitSaveV1 = "eitmad.material-unit.save.v1";
        public const string EitmadMaterialSaveV1 = "eitmad.material.save.v1";
        public const string EitmadFurnitureSaveV1 = "eitmad.furniture.save.v1";
        public const string EitmadFurnitureCategorySaveV1 = "eitmad.furniture-category.save.v1";
        public const string EitmadProductSaveV1 = "eitmad.product.save.v1";
        public const string EitmadProductCategorySaveV1 = "eitmad.product-category.save.v1";
        public const string EitmadPartSaveV1 = "eitmad.part.save.v1";
        public const string EitmadPartCategorySaveV1 = "eitmad.part-category.save.v1";
        public const string EitmadDesktopAccountCreateV1 = "eitmad.desktop-account.create.v1";
        public const string EitmadDesktopAccountUpdateV1 = "eitmad.desktop-account.update.v1";
        public const string EitmadDesktopAccountDeactivateV1 = "eitmad.desktop-account.deactivate.v1";
    }

    public static class Queries
    {
        public const string EitmadPricingListV1 = "eitmad.pricing.list.v1";
        public const string EitmadPricingReviewV1 = "eitmad.pricing.review.v1";
        public const string EitmadPricingSelectionV1 = "eitmad.pricing.selection.v1";
        public const string EitmadPricingDiscountV1 = "eitmad.pricing.discount.v1";
        public const string EitmadCatalogImageGetV1 = "eitmad.catalog-image.get.v1";
        public const string EitmadConfigGetV1 = "eitmad.config.get.v1";
        public const string EitmadPermissionsGetEffectiveV1 = "eitmad.permissions.get-effective.v1";
        public const string EitmadAuthorizationRelationshipsListV1 = "eitmad.authorization.relationships.list.v1";
        public const string EitmadCustomerGetV1 = "eitmad.customer.get.v1";
        public const string EitmadCustomerSearchV1 = "eitmad.customer.search.v1";
        public const string EitmadFurnitureListV1 = "eitmad.furniture.list.v1";
        public const string EitmadFurnitureCategoryListV1 = "eitmad.furniture-category.list.v1";
        public const string EitmadFurnitureRevisionGetV1 = "eitmad.furniture-revision.get.v1";
        public const string EitmadFurnitureReviewV1 = "eitmad.furniture.review.v1";
        public const string EitmadFurnitureSelectionCheckV1 = "eitmad.furniture-selection.check.v1";
        public const string EitmadProductListV1 = "eitmad.product.list.v1";
        public const string EitmadProductCategoryListV1 = "eitmad.product-category.list.v1";
        public const string EitmadProductRevisionGetV1 = "eitmad.product-revision.get.v1";
        public const string EitmadPartListV1 = "eitmad.part.list.v1";
        public const string EitmadPartCategoryListV1 = "eitmad.part-category.list.v1";
        public const string EitmadPartCostV1 = "eitmad.part.cost.v1";
        public const string EitmadPartCompositionGetV1 = "eitmad.part-composition.get.v1";
        public const string EitmadMaterialListV1 = "eitmad.material.list.v1";
        public const string EitmadMaterialReferenceListV1 = "eitmad.material-reference.list.v1";
        public const string EitmadDesktopAccountListV1 = "eitmad.desktop-account.list.v1";
    }

    public static class Subscriptions
    {
        public const string EitmadPricingChangedSubscribeV1 = "eitmad.pricing.changed.subscribe.v1";
        public const string EitmadConfigChangedSubscribeV1 = "eitmad.config.changed.subscribe.v1";
        public const string EitmadPermissionsChangedSubscribeV1 = "eitmad.permissions.changed.subscribe.v1";
        public const string EitmadAuthorizationPolicyChangedSubscribeV1 = "eitmad.authorization.policy.changed.subscribe.v1";
        public const string EitmadCustomerChangedSubscribeV1 = "eitmad.customer.changed.subscribe.v1";
        public const string EitmadMaterialChangedSubscribeV1 = "eitmad.material.changed.subscribe.v1";
        public const string EitmadFurnitureChangedSubscribeV1 = "eitmad.furniture.changed.subscribe.v1";
        public const string EitmadProductChangedSubscribeV1 = "eitmad.product.changed.subscribe.v1";
        public const string EitmadPartChangedSubscribeV1 = "eitmad.part.changed.subscribe.v1";
    }

    public static class Events
    {
        public const string EitmadPricingChangedEventV1 = "eitmad.pricing.changed.event.v1";
        public const string EitmadConfigChangedEventV1 = "eitmad.config.changed.event.v1";
        public const string EitmadPermissionsChangedEventV1 = "eitmad.permissions.changed.event.v1";
        public const string EitmadAuthorizationPolicyChangedEventV1 = "eitmad.authorization.policy.changed.event.v1";
        public const string EitmadCustomerChangedEventV1 = "eitmad.customer.changed.event.v1";
        public const string EitmadMaterialChangedEventV1 = "eitmad.material.changed.event.v1";
        public const string EitmadFurnitureChangedEventV1 = "eitmad.furniture.changed.event.v1";
        public const string EitmadProductChangedEventV1 = "eitmad.product.changed.event.v1";
        public const string EitmadPartChangedEventV1 = "eitmad.part.changed.event.v1";
    }

    public static class SyncMessages
    {
        public const string EitmadSyncNegotiateV1 = "eitmad.sync.negotiate.v1";
        public const string EitmadSyncPullV1 = "eitmad.sync.pull.v1";
        public const string EitmadSyncChangesV1 = "eitmad.sync.changes.v1";
        public const string EitmadSyncSubmitLocalV1 = "eitmad.sync.submit-local.v1";
        public const string EitmadSyncLocalResultV1 = "eitmad.sync.local-result.v1";
        public const string EitmadSyncReconcileV1 = "eitmad.sync.reconcile.v1";
        public const string EitmadSyncAcknowledgeV1 = "eitmad.sync.acknowledge.v1";
        public const string EitmadSyncConflictV1 = "eitmad.sync.conflict.v1";
        public const string EitmadSyncBackpressureV1 = "eitmad.sync.backpressure.v1";
        public const string EitmadSyncSnapshotManifestV1 = "eitmad.sync.snapshot-manifest.v1";
        public const string EitmadSyncSnapshotChunkV1 = "eitmad.sync.snapshot-chunk.v1";
        public const string EitmadSyncSnapshotCompleteV1 = "eitmad.sync.snapshot-complete.v1";
        public const string EitmadSyncSnapshotRequiredV1 = "eitmad.sync.snapshot-required.v1";
    }

    public static class ServerMessages
    {
        public const string EitmadServerHelloV1 = "eitmad.server.hello.v1";
        public const string EitmadServerSyncV1 = "eitmad.server.sync.v1";
        public const string EitmadServerSubscribeV1 = "eitmad.server.subscribe.v1";
        public const string EitmadServerAcknowledgeV1 = "eitmad.server.acknowledge.v1";
        public const string EitmadServerHelloAcceptedV1 = "eitmad.server.hello-accepted.v1";
        public const string EitmadServerSyncMessageV1 = "eitmad.server.sync-message.v1";
        public const string EitmadServerEventV1 = "eitmad.server.event.v1";
        public const string EitmadServerFailureV1 = "eitmad.server.failure.v1";
    }

    public static class Capabilities
    {
        public const string EitmadCapabilityPricingV1 = "eitmad.capability.pricing.v1";
        public const string EitmadCapabilityEngineLifecycleV1 = "eitmad.capability.engine-lifecycle.v1";
        public const string EitmadCapabilityLocalIpcV1 = "eitmad.capability.local-ipc.v1";
        public const string EitmadCapabilityDesktopUserSessionV1 = "eitmad.capability.desktop-user-session.v1";
        public const string EitmadCapabilityLocalIpcSubscriptionsV1 = "eitmad.capability.local-ipc-subscriptions.v1";
        public const string EitmadCapabilityAuthorizationPolicyEventsV1 = "eitmad.capability.authorization-policy-events.v1";
        public const string EitmadCapabilityAuthorizationScopesV1 = "eitmad.capability.authorization-scopes.v1";
        public const string EitmadCapabilityConfigV1 = "eitmad.capability.config.v1";
        public const string EitmadCapabilityPermissionsV1 = "eitmad.capability.permissions.v1";
        public const string EitmadCapabilitySyncV1 = "eitmad.capability.sync.v1";
        public const string EitmadCapabilityServerConnectionV1 = "eitmad.capability.server-connection.v1";
        public const string EitmadCapabilityServerDeviceProofV1 = "eitmad.capability.server-device-proof.v1";
        public const string EitmadCapabilityServerSnapshotChunksV1 = "eitmad.capability.server-snapshot-chunks.v1";
        public const string EitmadCapabilityServerSubscriptionResumeV1 = "eitmad.capability.server-subscription-resume.v1";
        public const string EitmadCapabilityServerRelayV1 = "eitmad.capability.server-relay.v1";
        public const string EitmadCapabilityServerUpdateDistributionV1 = "eitmad.capability.server-update-distribution.v1";
        public const string EitmadCapabilityServerAdministrationV1 = "eitmad.capability.server-administration.v1";
        public const string EitmadCapabilityUpdateV1 = "eitmad.capability.update.v1";
        public const string EitmadCapabilityCustomerV1 = "eitmad.capability.customer.v1";
        public const string EitmadCapabilityMaterialV1 = "eitmad.capability.material.v1";
        public const string EitmadCapabilityPartV1 = "eitmad.capability.part.v1";
        public const string EitmadCapabilityCatalogImageV1 = "eitmad.capability.catalog-image.v1";
        public const string EitmadCapabilityProductV1 = "eitmad.capability.product.v1";
        public const string EitmadCapabilityFurnitureV1 = "eitmad.capability.furniture.v1";
        public const string EitmadCapabilityDesktopAccountManagementV1 = "eitmad.capability.desktop-account-management.v1";
    }

    public static class Permissions
    {
        public const string EitmadPermissionCatalogReadV1 = "eitmad.permission.catalog.read.v1";
        public const string EitmadPermissionPricingWriteV1 = "eitmad.permission.pricing.write.v1";
        public const string EitmadPermissionPricingCostReadV1 = "eitmad.permission.pricing.cost.read.v1";
        public const string EitmadPermissionConfigReadV1 = "eitmad.permission.config.read.v1";
        public const string EitmadPermissionConfigWriteV1 = "eitmad.permission.config.write.v1";
        public const string EitmadPermissionConfigImportV1 = "eitmad.permission.config.import.v1";
        public const string EitmadPermissionConfigExportV1 = "eitmad.permission.config.export.v1";
        public const string EitmadPermissionAuthorizationManageV1 = "eitmad.permission.authorization.manage.v1";
        public const string EitmadPermissionPermissionsReadV1 = "eitmad.permission.permissions.read.v1";
        public const string EitmadPermissionObservabilitySensitiveDebugV1 = "eitmad.permission.observability.sensitive-debug.v1";
        public const string EitmadPermissionSyncReadV1 = "eitmad.permission.sync.read.v1";
        public const string EitmadPermissionServerAccountsManageV1 = "eitmad.permission.server.accounts.manage.v1";
        public const string EitmadPermissionServerDevicesManageV1 = "eitmad.permission.server.devices.manage.v1";
        public const string EitmadPermissionServerLicenseReadV1 = "eitmad.permission.server.license.read.v1";
        public const string EitmadPermissionServerUpdateChannelManageV1 = "eitmad.permission.server.update-channel.manage.v1";
        public const string EitmadPermissionServerRelayConnectV1 = "eitmad.permission.server.relay.connect.v1";
        public const string EitmadPermissionServerRelayHealthReadV1 = "eitmad.permission.server.relay.health.read.v1";
        public const string EitmadPermissionServerRelayFailureReportV1 = "eitmad.permission.server.relay.failure.report.v1";
        public const string EitmadPermissionServerRelayAdminCloseV1 = "eitmad.permission.server.relay.admin-close.v1";
        public const string EitmadPermissionServerUpdateManifestPublishV1 = "eitmad.permission.server.update-manifest.publish.v1";
        public const string EitmadPermissionServerAdminDiagnosticsReadV1 = "eitmad.permission.server.admin.diagnostics.read.v1";
        public const string EitmadPermissionServerAdminHealthReadV1 = "eitmad.permission.server.admin.health.read.v1";
        public const string EitmadPermissionServerAdminBackupReadV1 = "eitmad.permission.server.admin.backup.read.v1";
        public const string EitmadPermissionServerAdminMigrationReadV1 = "eitmad.permission.server.admin.migration.read.v1";
        public const string EitmadPermissionServerAdminAuditReadV1 = "eitmad.permission.server.admin.audit.read.v1";
        public const string EitmadPermissionServerAdminTenantReadV1 = "eitmad.permission.server.admin.tenant.read.v1";
        public const string EitmadPermissionServerAdminDeviceReadV1 = "eitmad.permission.server.admin.device.read.v1";
        public const string EitmadPermissionServerAdminSupportExecuteV1 = "eitmad.permission.server.admin.support.execute.v1";
        public const string EitmadPermissionUpdateReadV1 = "eitmad.permission.update.read.v1";
        public const string EitmadPermissionUpdateReportInstallerV1 = "eitmad.permission.update.report-installer.v1";
        public const string EitmadPermissionCustomerReadV1 = "eitmad.permission.customer.read.v1";
        public const string EitmadPermissionCustomerWriteV1 = "eitmad.permission.customer.write.v1";
        public const string EitmadPermissionProductReadV1 = "eitmad.permission.product.read.v1";
        public const string EitmadPermissionFurnitureReadV1 = "eitmad.permission.furniture.read.v1";
        public const string EitmadPermissionProductWriteV1 = "eitmad.permission.product.write.v1";
        public const string EitmadPermissionFurnitureWriteV1 = "eitmad.permission.furniture.write.v1";
        public const string EitmadPermissionProductCostReadV1 = "eitmad.permission.product.cost.read.v1";
        public const string EitmadPermissionPartReadV1 = "eitmad.permission.part.read.v1";
        public const string EitmadPermissionPartWriteV1 = "eitmad.permission.part.write.v1";
        public const string EitmadPermissionMaterialReadV1 = "eitmad.permission.material.read.v1";
        public const string EitmadPermissionMaterialWriteV1 = "eitmad.permission.material.write.v1";
        public const string EitmadPermissionMaterialUnitManageV1 = "eitmad.permission.material-unit.manage.v1";
        public const string EitmadPermissionCatalogDraftWriteV1 = "eitmad.permission.catalog.draft.write.v1";
        public const string EitmadPermissionQuotationDraftWriteV1 = "eitmad.permission.quotation.draft.write.v1";
        public const string EitmadPermissionDesktopAccountsManageV1 = "eitmad.permission.desktop-accounts.manage.v1";
    }

    public static class ConfigKeys
    {
        public const string EitmadConfigLocalePrimaryV1 = "eitmad.config.locale.primary.v1";
    }

    public static class Relations
    {
        public const string EitmadRelationOrganizationConfigManagerV1 = "eitmad.relation.organization.config-manager.v1";
        public const string EitmadRelationOrganizationMemberV1 = "eitmad.relation.organization.member.v1";
        public const string EitmadRelationOrganizationManagerV1 = "eitmad.relation.organization.manager.v1";
        public const string EitmadRelationOrganizationReceptionistV1 = "eitmad.relation.organization.receptionist.v1";
        public const string EitmadRelationOrganizationOwnerV1 = "eitmad.relation.organization.owner.v1";
    }

    public static class SchemaIds
    {
        public const string EitmadSchemaPricingV1 = "eitmad.schema.pricing.v1";
        public const string EitmadSchemaProtocolV1 = "eitmad.schema.protocol.v1";
        public const string EitmadSchemaCustomerV1 = "eitmad.schema.customer.v1";
        public const string EitmadSchemaMaterialV1 = "eitmad.schema.material.v1";
        public const string EitmadSchemaPartV1 = "eitmad.schema.part.v1";
        public const string EitmadSchemaCatalogImageV1 = "eitmad.schema.catalog-image.v1";
        public const string EitmadSchemaProductV1 = "eitmad.schema.product.v1";
        public const string EitmadSchemaFurnitureV1 = "eitmad.schema.furniture.v1";
    }

    public static class ErrorCodes
    {
        public const string EitmadErrorPricingInvalidV1 = "eitmad.error.pricing-invalid.v1";
        public const string EitmadErrorPricingReferenceInvalidV1 = "eitmad.error.pricing-reference-invalid.v1";
        public const string EitmadErrorPricingRevisionConflictV1 = "eitmad.error.pricing-revision-conflict.v1";
        public const string EitmadErrorPricingBelowCostV1 = "eitmad.error.pricing-below-cost.v1";
        public const string EitmadErrorPricingUnconfirmedV1 = "eitmad.error.pricing-unconfirmed.v1";
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
        public const string EitmadErrorDesktopAuthenticationFailedV1 = "eitmad.error.desktop-authentication-failed.v1";
        public const string EitmadErrorIpcSubscriptionResyncRequiredV1 = "eitmad.error.ipc-subscription-resync-required.v1";
        public const string EitmadErrorIpcSubscriptionUnsupportedV1 = "eitmad.error.ipc-subscription-unsupported.v1";
        public const string EitmadErrorIpcDeadlineExceededV1 = "eitmad.error.ipc-deadline-exceeded.v1";
        public const string EitmadErrorProtocolIncompatibleV1 = "eitmad.error.protocol-incompatible.v1";
        public const string EitmadErrorSyncBackpressureV1 = "eitmad.error.sync-backpressure.v1";
        public const string EitmadErrorServerAuthenticationFailedV1 = "eitmad.error.server-authentication-failed.v1";
        public const string EitmadErrorServerBootstrapFailedV1 = "eitmad.error.server-bootstrap-failed.v1";
        public const string EitmadErrorServerClientIncompatibleV1 = "eitmad.error.server-client-incompatible.v1";
        public const string EitmadErrorServerConfigInvalidV1 = "eitmad.error.server-config-invalid.v1";
        public const string EitmadErrorServerDatabaseUnavailableV1 = "eitmad.error.server-database-unavailable.v1";
        public const string EitmadErrorServerDeviceProofInvalidV1 = "eitmad.error.server-device-proof-invalid.v1";
        public const string EitmadErrorServerIdempotencyMismatchV1 = "eitmad.error.server-idempotency-mismatch.v1";
        public const string EitmadErrorServerLicenseRequiredV1 = "eitmad.error.server-license-required.v1";
        public const string EitmadErrorServerMigrationFailedV1 = "eitmad.error.server-migration-failed.v1";
        public const string EitmadErrorServerRuntimeFailedV1 = "eitmad.error.server-runtime-failed.v1";
        public const string EitmadErrorServerSnapshotRequiredV1 = "eitmad.error.server-snapshot-required.v1";
        public const string EitmadErrorServerTokenExpiredV1 = "eitmad.error.server-token-expired.v1";
        public const string EitmadErrorServerTokenReuseV1 = "eitmad.error.server-token-reuse.v1";
        public const string EitmadErrorRelaySessionNotFoundV1 = "eitmad.error.relay-session-not-found.v1";
        public const string EitmadErrorRelayUnavailableV1 = "eitmad.error.relay-unavailable.v1";
        public const string EitmadErrorUpdateManifestInvalidV1 = "eitmad.error.update-manifest-invalid.v1";
        public const string EitmadErrorUpdateManifestNotFoundV1 = "eitmad.error.update-manifest-not-found.v1";
        public const string EitmadErrorUpdateDistributionUnavailableV1 = "eitmad.error.update-distribution-unavailable.v1";
        public const string EitmadErrorAdminUnavailableV1 = "eitmad.error.admin-unavailable.v1";
        public const string EitmadErrorUpdateInstallerFailedV1 = "eitmad.error.update-installer-failed.v1";
        public const string EitmadErrorCustomerNotFoundV1 = "eitmad.error.customer-not-found.v1";
        public const string EitmadErrorCustomerRevisionConflictV1 = "eitmad.error.customer-revision-conflict.v1";
        public const string EitmadErrorCustomerUnavailableV1 = "eitmad.error.customer-unavailable.v1";
        public const string EitmadErrorMaterialInvalidV1 = "eitmad.error.material-invalid.v1";
        public const string EitmadErrorPartInvalidV1 = "eitmad.error.part-invalid.v1";
        public const string EitmadErrorCatalogImageInvalidV1 = "eitmad.error.catalog-image-invalid.v1";
        public const string EitmadErrorCatalogImageNotFoundV1 = "eitmad.error.catalog-image-not-found.v1";
        public const string EitmadErrorCatalogImageUnavailableV1 = "eitmad.error.catalog-image-unavailable.v1";
        public const string EitmadErrorProductInvalidV1 = "eitmad.error.product-invalid.v1";
        public const string EitmadErrorFurnitureInvalidV1 = "eitmad.error.furniture-invalid.v1";
        public const string EitmadErrorPartNotFoundV1 = "eitmad.error.part-not-found.v1";
        public const string EitmadErrorProductNotFoundV1 = "eitmad.error.product-not-found.v1";
        public const string EitmadErrorFurnitureNotFoundV1 = "eitmad.error.furniture-not-found.v1";
        public const string EitmadErrorPartRevisionConflictV1 = "eitmad.error.part-revision-conflict.v1";
        public const string EitmadErrorProductRevisionConflictV1 = "eitmad.error.product-revision-conflict.v1";
        public const string EitmadErrorFurnitureRevisionConflictV1 = "eitmad.error.furniture-revision-conflict.v1";
        public const string EitmadErrorPartReferenceInvalidV1 = "eitmad.error.part-reference-invalid.v1";
        public const string EitmadErrorProductReferenceInvalidV1 = "eitmad.error.product-reference-invalid.v1";
        public const string EitmadErrorFurnitureReferenceInvalidV1 = "eitmad.error.furniture-reference-invalid.v1";
        public const string EitmadErrorPartUnavailableV1 = "eitmad.error.part-unavailable.v1";
        public const string EitmadErrorProductUnavailableV1 = "eitmad.error.product-unavailable.v1";
        public const string EitmadErrorFurnitureUnavailableV1 = "eitmad.error.furniture-unavailable.v1";
        public const string EitmadErrorMaterialNotFoundV1 = "eitmad.error.material-not-found.v1";
        public const string EitmadErrorMaterialRevisionConflictV1 = "eitmad.error.material-revision-conflict.v1";
        public const string EitmadErrorMaterialReferenceInvalidV1 = "eitmad.error.material-reference-invalid.v1";
        public const string EitmadErrorMaterialUnavailableV1 = "eitmad.error.material-unavailable.v1";
        public const string EitmadErrorDesktopAccountInvalidV1 = "eitmad.error.desktop-account-invalid.v1";
        public const string EitmadErrorDesktopAccountRevisionConflictV1 = "eitmad.error.desktop-account-revision-conflict.v1";
        public const string EitmadErrorDesktopAccountLastManagerV1 = "eitmad.error.desktop-account-last-manager.v1";
        public const string EitmadErrorDesktopAccountUnavailableV1 = "eitmad.error.desktop-account-unavailable.v1";
    }

    public static class MessageIds
    {
        public const string EitmadMessagePricingInvalidV1 = "eitmad.message.pricing-invalid.v1";
        public const string EitmadMessagePricingReferenceInvalidV1 = "eitmad.message.pricing-reference-invalid.v1";
        public const string EitmadMessagePricingRevisionConflictV1 = "eitmad.message.pricing-revision-conflict.v1";
        public const string EitmadMessagePricingBelowCostV1 = "eitmad.message.pricing-below-cost.v1";
        public const string EitmadMessagePricingUnconfirmedV1 = "eitmad.message.pricing-unconfirmed.v1";
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
        public const string EitmadMessageDesktopAuthenticationFailedV1 = "eitmad.message.desktop-authentication-failed.v1";
        public const string EitmadMessageIpcSubscriptionResyncRequiredV1 = "eitmad.message.ipc-subscription-resync-required.v1";
        public const string EitmadMessageIpcSubscriptionUnsupportedV1 = "eitmad.message.ipc-subscription-unsupported.v1";
        public const string EitmadMessageIpcDeadlineExceededV1 = "eitmad.message.ipc-deadline-exceeded.v1";
        public const string EitmadMessageObservabilitySensitiveDebugWarningV1 = "eitmad.message.observability-sensitive-debug-warning.v1";
        public const string EitmadMessageProtocolIncompatibleV1 = "eitmad.message.protocol-incompatible.v1";
        public const string EitmadMessageSyncBackpressureV1 = "eitmad.message.sync-backpressure.v1";
        public const string EitmadMessageServerAuthenticationFailedV1 = "eitmad.message.server-authentication-failed.v1";
        public const string EitmadMessageServerBootstrapFailedV1 = "eitmad.message.server-bootstrap-failed.v1";
        public const string EitmadMessageServerClientIncompatibleV1 = "eitmad.message.server-client-incompatible.v1";
        public const string EitmadMessageServerConfigInvalidV1 = "eitmad.message.server-config-invalid.v1";
        public const string EitmadMessageServerDatabaseUnavailableV1 = "eitmad.message.server-database-unavailable.v1";
        public const string EitmadMessageServerDeviceProofInvalidV1 = "eitmad.message.server-device-proof-invalid.v1";
        public const string EitmadMessageServerIdempotencyMismatchV1 = "eitmad.message.server-idempotency-mismatch.v1";
        public const string EitmadMessageServerLicenseRequiredV1 = "eitmad.message.server-license-required.v1";
        public const string EitmadMessageServerMigrationFailedV1 = "eitmad.message.server-migration-failed.v1";
        public const string EitmadMessageServerRuntimeFailedV1 = "eitmad.message.server-runtime-failed.v1";
        public const string EitmadMessageServerSnapshotRequiredV1 = "eitmad.message.server-snapshot-required.v1";
        public const string EitmadMessageServerTokenExpiredV1 = "eitmad.message.server-token-expired.v1";
        public const string EitmadMessageServerTokenReuseV1 = "eitmad.message.server-token-reuse.v1";
        public const string EitmadMessageRelaySessionNotFoundV1 = "eitmad.message.relay-session-not-found.v1";
        public const string EitmadMessageRelayUnavailableV1 = "eitmad.message.relay-unavailable.v1";
        public const string EitmadMessageUpdateManifestInvalidV1 = "eitmad.message.update-manifest-invalid.v1";
        public const string EitmadMessageUpdateManifestNotFoundV1 = "eitmad.message.update-manifest-not-found.v1";
        public const string EitmadMessageUpdateDistributionUnavailableV1 = "eitmad.message.update-distribution-unavailable.v1";
        public const string EitmadMessageAdminUnavailableV1 = "eitmad.message.admin-unavailable.v1";
        public const string EitmadMessageUpdateInstallerFailedV1 = "eitmad.message.update-installer-failed.v1";
        public const string EitmadMessageCustomerNotFoundV1 = "eitmad.message.customer-not-found.v1";
        public const string EitmadMessageCustomerRevisionConflictV1 = "eitmad.message.customer-revision-conflict.v1";
        public const string EitmadMessageCustomerUnavailableV1 = "eitmad.message.customer-unavailable.v1";
        public const string EitmadMessageMaterialInvalidV1 = "eitmad.message.material-invalid.v1";
        public const string EitmadMessagePartInvalidV1 = "eitmad.message.part-invalid.v1";
        public const string EitmadMessageCatalogImageInvalidV1 = "eitmad.message.catalog-image-invalid.v1";
        public const string EitmadMessageCatalogImageNotFoundV1 = "eitmad.message.catalog-image-not-found.v1";
        public const string EitmadMessageCatalogImageUnavailableV1 = "eitmad.message.catalog-image-unavailable.v1";
        public const string EitmadMessageProductInvalidV1 = "eitmad.message.product-invalid.v1";
        public const string EitmadMessageFurnitureInvalidV1 = "eitmad.message.furniture-invalid.v1";
        public const string EitmadMessagePartNotFoundV1 = "eitmad.message.part-not-found.v1";
        public const string EitmadMessageProductNotFoundV1 = "eitmad.message.product-not-found.v1";
        public const string EitmadMessageFurnitureNotFoundV1 = "eitmad.message.furniture-not-found.v1";
        public const string EitmadMessagePartRevisionConflictV1 = "eitmad.message.part-revision-conflict.v1";
        public const string EitmadMessageProductRevisionConflictV1 = "eitmad.message.product-revision-conflict.v1";
        public const string EitmadMessageFurnitureRevisionConflictV1 = "eitmad.message.furniture-revision-conflict.v1";
        public const string EitmadMessagePartReferenceInvalidV1 = "eitmad.message.part-reference-invalid.v1";
        public const string EitmadMessageProductReferenceInvalidV1 = "eitmad.message.product-reference-invalid.v1";
        public const string EitmadMessageFurnitureReferenceInvalidV1 = "eitmad.message.furniture-reference-invalid.v1";
        public const string EitmadMessagePartUnavailableV1 = "eitmad.message.part-unavailable.v1";
        public const string EitmadMessageProductUnavailableV1 = "eitmad.message.product-unavailable.v1";
        public const string EitmadMessageFurnitureUnavailableV1 = "eitmad.message.furniture-unavailable.v1";
        public const string EitmadMessageMaterialNotFoundV1 = "eitmad.message.material-not-found.v1";
        public const string EitmadMessageMaterialRevisionConflictV1 = "eitmad.message.material-revision-conflict.v1";
        public const string EitmadMessageMaterialReferenceInvalidV1 = "eitmad.message.material-reference-invalid.v1";
        public const string EitmadMessageMaterialUnavailableV1 = "eitmad.message.material-unavailable.v1";
        public const string EitmadMessageDesktopAccountInvalidV1 = "eitmad.message.desktop-account-invalid.v1";
        public const string EitmadMessageDesktopAccountRevisionConflictV1 = "eitmad.message.desktop-account-revision-conflict.v1";
        public const string EitmadMessageDesktopAccountLastManagerV1 = "eitmad.message.desktop-account-last-manager.v1";
        public const string EitmadMessageDesktopAccountUnavailableV1 = "eitmad.message.desktop-account-unavailable.v1";
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
