use std::collections::BTreeSet;

use async_trait::async_trait;
use eitmad_admin_plane::{
    AdministrativeAction, AdministrativeAuditOutcome, AdministrativeError, AdministrativeSecurity,
    RelayMetricsSource, SupportWorkflowExecutor,
};
use eitmad_contracts::{
    administration::{SupportAction, SupportWorkflow},
    catalog::UPDATE_MANIFEST_PUBLISH_PERMISSION,
    identity::TenantId,
    relay::{RelayRoute, RelaySessionId, RelaySessionMetadata},
    server::{AuthenticatedServerSession, UpdateChannelId},
    transport::{CorrelationId, UnixMillis},
    updates::UpdateManifestId,
};
use eitmad_control_plane::{
    AccessError, AccessRequirement, ServerAccessService, ServerAuditEntry, ServerAuditOutcome,
};
use eitmad_relay_plane::{
    RelayAction, RelayAuditOutcome, RelayCoordinator, RelayError, RelayRouter, RelaySecurity,
};
use eitmad_update_plane::{
    UpdateAuditOutcome, UpdatePlaneAction, UpdatePlaneError, UpdatePublicationSecurity,
};

#[derive(Clone)]
pub struct ServerPlaneSecurity {
    access: ServerAccessService,
    update_scope: UpdateDistributionScope,
}

impl ServerPlaneSecurity {
    #[must_use]
    pub fn new(access: ServerAccessService, update_operator_tenant_id: TenantId) -> Self {
        Self {
            access,
            update_scope: UpdateDistributionScope::new(update_operator_tenant_id),
        }
    }
}

#[derive(Clone)]
struct UpdateDistributionScope {
    operator_tenant_id: TenantId,
    channels: BTreeSet<UpdateChannelId>,
}

impl UpdateDistributionScope {
    fn new(operator_tenant_id: TenantId) -> Self {
        Self {
            operator_tenant_id,
            channels: ["stable", "beta", "canary"]
                .into_iter()
                .map(|channel| {
                    UpdateChannelId::parse(channel).expect("built-in update channel must be valid")
                })
                .collect(),
        }
    }

    fn allows(
        &self,
        actor: &AuthenticatedServerSession,
        action: UpdatePlaneAction,
        channel: &UpdateChannelId,
    ) -> bool {
        actor.tenant_id == self.operator_tenant_id
            && action == UpdatePlaneAction::PublishManifest
            && self.channels.contains(channel)
    }
}

#[async_trait]
impl RelaySecurity for ServerPlaneSecurity {
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        action: RelayAction,
        tenant_id: TenantId,
        route: Option<&RelayRoute>,
    ) -> Result<(), RelayError> {
        let target_device = route.and_then(|route| match route {
            RelayRoute::Peer { target_device_id } => Some(*target_device_id),
            RelayRoute::Server { .. } => None,
        });
        let requirement = if action == RelayAction::AdministrativeClose {
            AccessRequirement::TenantOwner
        } else {
            AccessRequirement::TenantMember
        };
        self.access
            .authorize(actor, tenant_id, target_device, requirement)
            .await
            .map_err(map_relay_access)
    }

    async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        action: RelayAction,
        outcome: RelayAuditOutcome,
        correlation_id: CorrelationId,
        _: Option<RelaySessionId>,
        now: UnixMillis,
    ) -> Result<(), RelayError> {
        let redacted_error = match outcome {
            RelayAuditOutcome::Denied => Some("eitmad.error.authorization-denied.v1"),
            RelayAuditOutcome::Invalid => Some("eitmad.error.contract-invalid.v1"),
            RelayAuditOutcome::Failed => Some("eitmad.error.relay-unavailable.v1"),
            RelayAuditOutcome::Succeeded => None,
        };
        self.access
            .audit(
                actor,
                &ServerAuditEntry {
                    operation: action.operation(),
                    outcome: server_outcome(outcome),
                    target_kind: "relay_session",
                    redacted_error,
                    correlation_id,
                    now,
                },
            )
            .await
            .map_err(map_relay_access)
    }
}

#[async_trait]
impl UpdatePublicationSecurity for ServerPlaneSecurity {
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        action: UpdatePlaneAction,
        channel: &UpdateChannelId,
    ) -> Result<(), UpdatePlaneError> {
        if !self.update_scope.allows(actor, action, channel) {
            return Err(UpdatePlaneError::Denied);
        }
        self.access
            .authorize(
                actor,
                actor.tenant_id,
                None,
                AccessRequirement::TenantPermission(UPDATE_MANIFEST_PUBLISH_PERMISSION),
            )
            .await
            .map_err(map_update_access)
    }

    async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        action: UpdatePlaneAction,
        outcome: UpdateAuditOutcome,
        correlation_id: CorrelationId,
        _: UpdateManifestId,
        now: UnixMillis,
    ) -> Result<(), UpdatePlaneError> {
        let (server_outcome, error) = match outcome {
            UpdateAuditOutcome::Succeeded => (ServerAuditOutcome::Succeeded, None),
            UpdateAuditOutcome::Denied => (
                ServerAuditOutcome::Denied,
                Some("eitmad.error.authorization-denied.v1"),
            ),
            UpdateAuditOutcome::Invalid => (
                ServerAuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1"),
            ),
            UpdateAuditOutcome::Failed => (
                ServerAuditOutcome::Failed,
                Some("eitmad.error.update-distribution-unavailable.v1"),
            ),
        };
        self.access
            .audit(
                actor,
                &ServerAuditEntry {
                    operation: action.operation(),
                    outcome: server_outcome,
                    target_kind: "update_manifest",
                    redacted_error: error,
                    correlation_id,
                    now,
                },
            )
            .await
            .map_err(map_update_access)
    }
}

#[async_trait]
impl AdministrativeSecurity for ServerPlaneSecurity {
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        _: AdministrativeAction,
        tenant_id: TenantId,
    ) -> Result<(), AdministrativeError> {
        self.access
            .authorize(actor, tenant_id, None, AccessRequirement::TenantOwner)
            .await
            .map_err(map_admin_access)
    }

    async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        action: AdministrativeAction,
        outcome: AdministrativeAuditOutcome,
        _: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), AdministrativeError> {
        let (server_outcome, error) = match outcome {
            AdministrativeAuditOutcome::Succeeded => (ServerAuditOutcome::Succeeded, None),
            AdministrativeAuditOutcome::Denied => (
                ServerAuditOutcome::Denied,
                Some("eitmad.error.authorization-denied.v1"),
            ),
            AdministrativeAuditOutcome::Invalid => (
                ServerAuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1"),
            ),
            AdministrativeAuditOutcome::Failed => (
                ServerAuditOutcome::Failed,
                Some("eitmad.error.admin-unavailable.v1"),
            ),
        };
        self.access
            .audit(
                actor,
                &ServerAuditEntry {
                    operation: action.operation(),
                    outcome: server_outcome,
                    target_kind: "administrative_operation",
                    redacted_error: error,
                    correlation_id,
                    now,
                },
            )
            .await
            .map_err(map_admin_access)
    }
}

#[derive(Default)]
pub struct MetadataRelayRouter;

#[async_trait]
impl RelayRouter for MetadataRelayRouter {
    async fn connect_peer(&self, _: &RelaySessionMetadata) -> Result<(), RelayError> {
        Ok(())
    }

    async fn connect_server(&self, _: &RelaySessionMetadata) -> Result<(), RelayError> {
        Ok(())
    }

    async fn disconnect(&self, _: &RelaySessionMetadata) -> Result<(), RelayError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct ServerRelayMetrics {
    relay: RelayCoordinator,
}

impl ServerRelayMetrics {
    #[must_use]
    pub const fn new(relay: RelayCoordinator) -> Self {
        Self { relay }
    }
}

#[async_trait]
impl RelayMetricsSource for ServerRelayMetrics {
    async fn active_sessions(
        &self,
        actor: &AuthenticatedServerSession,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<u32, AdministrativeError> {
        self.relay
            .health(actor, correlation_id, now)
            .await
            .map(|health| health.active_sessions)
            .map_err(|error| match error {
                RelayError::Denied => AdministrativeError::Denied,
                RelayError::Invalid | RelayError::NotFound | RelayError::RetryNotDue => {
                    AdministrativeError::Invalid
                }
                RelayError::RouteUnavailable | RelayError::Unavailable => {
                    AdministrativeError::Unavailable
                }
            })
    }
}

#[derive(Clone)]
pub struct ServerSupportExecutor {
    relay: RelayCoordinator,
    access: ServerAccessService,
}

impl ServerSupportExecutor {
    #[must_use]
    pub const fn new(relay: RelayCoordinator, access: ServerAccessService) -> Self {
        Self { relay, access }
    }
}

#[async_trait]
impl SupportWorkflowExecutor for ServerSupportExecutor {
    async fn execute(
        &self,
        actor: &AuthenticatedServerSession,
        workflow: &SupportWorkflow,
    ) -> Result<(), AdministrativeError> {
        match workflow.action {
            SupportAction::CollectDiagnostics
            | SupportAction::VerifyBackup
            | SupportAction::RetryMigration => Err(AdministrativeError::Invalid),
            SupportAction::DisconnectRelaySession { relay_session_id } => self
                .relay
                .administrative_close(
                    actor,
                    relay_session_id,
                    CorrelationId::new(workflow.workflow_id.value()),
                    workflow.requested_at,
                )
                .await
                .map(|_| ())
                .map_err(|error| match error {
                    RelayError::Denied => AdministrativeError::Denied,
                    RelayError::Invalid | RelayError::NotFound | RelayError::RetryNotDue => {
                        AdministrativeError::Invalid
                    }
                    RelayError::RouteUnavailable | RelayError::Unavailable => {
                        AdministrativeError::Unavailable
                    }
                }),
            SupportAction::RevokeDeviceSessions { device_id } => self
                .access
                .revoke_device_sessions(
                    actor,
                    device_id,
                    CorrelationId::new(workflow.workflow_id.value()),
                    workflow.requested_at,
                )
                .await
                .map_err(map_admin_access),
        }
    }
}

const fn map_relay_access(error: AccessError) -> RelayError {
    match error {
        AccessError::Denied => RelayError::Denied,
        AccessError::Unavailable => RelayError::Unavailable,
    }
}

const fn map_update_access(error: AccessError) -> UpdatePlaneError {
    match error {
        AccessError::Denied => UpdatePlaneError::Denied,
        AccessError::Unavailable => UpdatePlaneError::Unavailable,
    }
}

const fn map_admin_access(error: AccessError) -> AdministrativeError {
    match error {
        AccessError::Denied => AdministrativeError::Denied,
        AccessError::Unavailable => AdministrativeError::Unavailable,
    }
}

const fn server_outcome(outcome: RelayAuditOutcome) -> ServerAuditOutcome {
    match outcome {
        RelayAuditOutcome::Succeeded => ServerAuditOutcome::Succeeded,
        RelayAuditOutcome::Denied => ServerAuditOutcome::Denied,
        RelayAuditOutcome::Invalid => ServerAuditOutcome::Invalid,
        RelayAuditOutcome::Failed => ServerAuditOutcome::Failed,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eitmad_contracts::{
        identity::{AccountId, DeviceId, SessionId, UserId},
        server::AuthenticatedServerSession,
    };
    use uuid::Uuid;

    use super::*;

    fn actor(tenant_id: TenantId) -> AuthenticatedServerSession {
        AuthenticatedServerSession {
            session_id: SessionId::new(Uuid::from_u128(1)),
            account_id: AccountId::new(Uuid::from_u128(2)),
            user_id: UserId::new(Uuid::from_u128(3)),
            device_id: DeviceId::new(Uuid::from_u128(4)),
            tenant_id,
            issued_at: UnixMillis(0),
            expires_at: UnixMillis(i64::MAX),
        }
    }

    #[test]
    fn update_distribution_scope_rejects_foreign_tenants_and_actions() {
        let operator = TenantId::new(Uuid::from_u128(5));
        let scope = UpdateDistributionScope::new(operator);
        let stable = UpdateChannelId::parse("stable").unwrap();

        assert!(scope.allows(
            &actor(operator),
            UpdatePlaneAction::PublishManifest,
            &stable
        ));
        assert!(!scope.allows(
            &actor(TenantId::new(Uuid::from_u128(6))),
            UpdatePlaneAction::PublishManifest,
            &stable,
        ));
        assert!(!scope.allows(&actor(operator), UpdatePlaneAction::RevokeManifest, &stable));
    }

    #[tokio::test]
    async fn unimplemented_support_actions_fail_instead_of_reporting_success() {
        let pool = eitmad_postgres_support::PgPoolOptions::new()
            .connect_lazy("postgresql://unreachable.invalid/eitmad")
            .unwrap();
        let access = ServerAccessService::new(pool);
        let security = Arc::new(ServerPlaneSecurity::new(
            access.clone(),
            TenantId::new(Uuid::from_u128(9)),
        ));
        let executor = ServerSupportExecutor::new(
            RelayCoordinator::new(security, Arc::new(MetadataRelayRouter)),
            access,
        );
        let actor = actor(TenantId::new(Uuid::from_u128(5)));

        for action in [
            SupportAction::CollectDiagnostics,
            SupportAction::VerifyBackup,
            SupportAction::RetryMigration,
        ] {
            let workflow = SupportWorkflow {
                workflow_id: eitmad_contracts::administration::SupportWorkflowId::new(
                    Uuid::new_v4(),
                ),
                tenant_id: actor.tenant_id,
                action,
                reason_code: "synthetic.readiness-audit".to_owned(),
                state: eitmad_contracts::administration::SupportWorkflowState::Running,
                requested_at: UnixMillis(1),
                completed_at: None,
                failure_code: None,
            };
            assert_eq!(
                executor.execute(&actor, &workflow).await,
                Err(AdministrativeError::Invalid)
            );
        }
    }
}
