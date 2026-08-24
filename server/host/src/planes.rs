use async_trait::async_trait;
use eitmad_admin_plane::{
    AdministrativeAction, AdministrativeAuditOutcome, AdministrativeError, AdministrativeSecurity,
    RelayMetricsSource, SupportWorkflowExecutor,
};
use eitmad_contracts::{
    administration::{SupportAction, SupportWorkflow},
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
}

impl ServerPlaneSecurity {
    #[must_use]
    pub const fn new(access: ServerAccessService) -> Self {
        Self { access }
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
        _: UpdatePlaneAction,
        _: &UpdateChannelId,
    ) -> Result<(), UpdatePlaneError> {
        self.access
            .authorize(actor, actor.tenant_id, None, AccessRequirement::TenantOwner)
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
            SupportAction::CollectDiagnostics | SupportAction::VerifyBackup => Ok(()),
            SupportAction::RetryMigration => Err(AdministrativeError::Invalid),
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
