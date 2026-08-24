//! Least-privilege administration, diagnostics, and support workflows.

use std::sync::Arc;

use async_trait::async_trait;
use eitmad_contracts::{
    administration::{
        AdministrativeAuditRecord, BackupStatus, DeviceVisibility, DiagnosticSummary,
        MigrationStatus, ServiceHealth, StartSupportWorkflow, SupportWorkflow, SupportWorkflowId,
        SupportWorkflowState, TenantVisibility,
    },
    identity::TenantId,
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use uuid::Uuid;

mod database;
mod postgres;

pub use database::{AdminDatabase, AdminDatabaseError};
pub use postgres::{PostgresAdministrationDataSource, SupportWorkflowExecutor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdministrativeAction {
    ReadDiagnostics,
    ReadHealth,
    ReadBackupStatus,
    ReadMigrationStatus,
    ReadAudit,
    ReadTenants,
    ReadDevices,
    StartSupportWorkflow,
}

impl AdministrativeAction {
    #[must_use]
    pub const fn operation(self) -> &'static str {
        match self {
            Self::ReadDiagnostics => "eitmad.admin.diagnostics.read.v1",
            Self::ReadHealth => "eitmad.admin.health.read.v1",
            Self::ReadBackupStatus => "eitmad.admin.backup-status.read.v1",
            Self::ReadMigrationStatus => "eitmad.admin.migration-status.read.v1",
            Self::ReadAudit => "eitmad.admin.audit.read.v1",
            Self::ReadTenants => "eitmad.admin.tenants.read.v1",
            Self::ReadDevices => "eitmad.admin.devices.read.v1",
            Self::StartSupportWorkflow => "eitmad.admin.support.start.v1",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdministrativeAuditOutcome {
    Succeeded,
    Denied,
    Invalid,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AdministrativeError {
    #[error("administrative action is denied")]
    Denied,
    #[error("administrative request is invalid")]
    Invalid,
    #[error("administrative data is unavailable")]
    Unavailable,
}

#[async_trait]
pub trait AdministrativeSecurity: Send + Sync {
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        action: AdministrativeAction,
        tenant_id: TenantId,
    ) -> Result<(), AdministrativeError>;

    async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        action: AdministrativeAction,
        outcome: AdministrativeAuditOutcome,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), AdministrativeError>;
}

#[async_trait]
pub trait RelayMetricsSource: Send + Sync {
    async fn active_sessions(
        &self,
        actor: &AuthenticatedServerSession,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<u32, AdministrativeError>;
}

#[async_trait]
pub trait AdministrationDataSource: Send + Sync {
    async fn diagnostics(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<DiagnosticSummary, AdministrativeError>;
    async fn health(&self, tenant_id: TenantId) -> Result<Vec<ServiceHealth>, AdministrativeError>;
    async fn backup_status(&self, tenant_id: TenantId)
    -> Result<BackupStatus, AdministrativeError>;
    async fn migration_status(
        &self,
        tenant_id: TenantId,
    ) -> Result<MigrationStatus, AdministrativeError>;
    async fn audit_records(
        &self,
        tenant_id: TenantId,
        limit: u32,
    ) -> Result<Vec<AdministrativeAuditRecord>, AdministrativeError>;
    async fn tenant_visibility(
        &self,
        tenant_id: TenantId,
    ) -> Result<TenantVisibility, AdministrativeError>;
    async fn device_visibility(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<DeviceVisibility>, AdministrativeError>;
    async fn execute_support(
        &self,
        actor: &AuthenticatedServerSession,
        workflow: SupportWorkflow,
    ) -> Result<SupportWorkflow, AdministrativeError>;
}

#[derive(Clone)]
pub struct AdministrationService {
    security: Arc<dyn AdministrativeSecurity>,
    data: Arc<dyn AdministrationDataSource>,
}

impl AdministrationService {
    #[must_use]
    pub const fn new(
        security: Arc<dyn AdministrativeSecurity>,
        data: Arc<dyn AdministrationDataSource>,
    ) -> Self {
        Self { security, data }
    }

    /// Returns redacted tenant diagnostics after authorization and audit.
    ///
    /// # Errors
    ///
    /// Returns a denial or a provider/audit availability error.
    pub async fn diagnostics(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<DiagnosticSummary, AdministrativeError> {
        self.execute_read(
            actor,
            AdministrativeAction::ReadDiagnostics,
            tenant_id,
            correlation_id,
            now,
            || self.data.diagnostics(actor, tenant_id, correlation_id, now),
        )
        .await
    }

    /// Returns component health after authorization and audit.
    ///
    /// # Errors
    ///
    /// Returns a denial or a provider/audit availability error.
    pub async fn health(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<Vec<ServiceHealth>, AdministrativeError> {
        self.execute_read(
            actor,
            AdministrativeAction::ReadHealth,
            tenant_id,
            correlation_id,
            now,
            || self.data.health(tenant_id),
        )
        .await
    }

    /// Returns the tenant backup status after authorization and audit.
    ///
    /// # Errors
    ///
    /// Returns a denial or a provider/audit availability error.
    pub async fn backup_status(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<BackupStatus, AdministrativeError> {
        self.execute_read(
            actor,
            AdministrativeAction::ReadBackupStatus,
            tenant_id,
            correlation_id,
            now,
            || self.data.backup_status(tenant_id),
        )
        .await
    }

    /// Returns the server migration status after authorization and audit.
    ///
    /// # Errors
    ///
    /// Returns a denial or a provider/audit availability error.
    pub async fn migration_status(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<MigrationStatus, AdministrativeError> {
        self.execute_read(
            actor,
            AdministrativeAction::ReadMigrationStatus,
            tenant_id,
            correlation_id,
            now,
            || self.data.migration_status(tenant_id),
        )
        .await
    }

    /// Returns a bounded tenant audit page after authorization and audit.
    ///
    /// # Errors
    ///
    /// Returns an invalid limit, denial, or provider/audit availability error.
    pub async fn audit_records(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        limit: u32,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<Vec<AdministrativeAuditRecord>, AdministrativeError> {
        let action = AdministrativeAction::ReadAudit;
        self.authorize(actor, action, tenant_id, correlation_id, now)
            .await?;
        if limit == 0 || limit > 500 {
            self.audit(
                actor,
                action,
                AdministrativeAuditOutcome::Invalid,
                tenant_id,
                correlation_id,
                now,
            )
            .await?;
            return Err(AdministrativeError::Invalid);
        }
        match self.data.audit_records(tenant_id, limit).await {
            Ok(records) => {
                self.audit(
                    actor,
                    action,
                    AdministrativeAuditOutcome::Succeeded,
                    tenant_id,
                    correlation_id,
                    now,
                )
                .await?;
                Ok(records)
            }
            Err(error) => {
                self.audit(
                    actor,
                    action,
                    outcome_for(error),
                    tenant_id,
                    correlation_id,
                    now,
                )
                .await?;
                Err(error)
            }
        }
    }

    /// Returns one tenant visibility summary after authorization and audit.
    ///
    /// # Errors
    ///
    /// Returns a denial or a provider/audit availability error.
    pub async fn tenant_visibility(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<TenantVisibility, AdministrativeError> {
        self.execute_read(
            actor,
            AdministrativeAction::ReadTenants,
            tenant_id,
            correlation_id,
            now,
            || self.data.tenant_visibility(tenant_id),
        )
        .await
    }

    /// Returns tenant-scoped device visibility after authorization and audit.
    ///
    /// # Errors
    ///
    /// Returns a denial or a provider/audit availability error.
    pub async fn device_visibility(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<Vec<DeviceVisibility>, AdministrativeError> {
        self.execute_read(
            actor,
            AdministrativeAction::ReadDevices,
            tenant_id,
            correlation_id,
            now,
            || self.data.device_visibility(tenant_id),
        )
        .await
    }

    /// Starts one reason-coded tenant support workflow.
    ///
    /// # Errors
    ///
    /// Returns an invalid request, denial, provider, or audit error.
    pub async fn start_support_workflow(
        &self,
        actor: &AuthenticatedServerSession,
        request: &StartSupportWorkflow,
        now: UnixMillis,
    ) -> Result<SupportWorkflow, AdministrativeError> {
        let action = AdministrativeAction::StartSupportWorkflow;
        self.authorize(
            actor,
            action,
            request.tenant_id,
            request.correlation_id,
            now,
        )
        .await?;
        if !valid_reason_code(&request.reason_code) {
            self.audit(
                actor,
                action,
                AdministrativeAuditOutcome::Invalid,
                request.tenant_id,
                request.correlation_id,
                now,
            )
            .await?;
            return Err(AdministrativeError::Invalid);
        }
        let workflow = SupportWorkflow {
            workflow_id: SupportWorkflowId::new(Uuid::new_v4()),
            tenant_id: request.tenant_id,
            action: request.action.clone(),
            reason_code: request.reason_code.clone(),
            state: SupportWorkflowState::Running,
            requested_at: now,
            completed_at: None,
            failure_code: None,
        };
        match self.data.execute_support(actor, workflow).await {
            Ok(workflow) => {
                self.audit(
                    actor,
                    action,
                    AdministrativeAuditOutcome::Succeeded,
                    request.tenant_id,
                    request.correlation_id,
                    now,
                )
                .await?;
                Ok(workflow)
            }
            Err(error) => {
                self.audit(
                    actor,
                    action,
                    outcome_for(error),
                    request.tenant_id,
                    request.correlation_id,
                    now,
                )
                .await?;
                Err(error)
            }
        }
    }

    async fn execute_read<T, Future>(
        &self,
        actor: &AuthenticatedServerSession,
        action: AdministrativeAction,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
        read: impl FnOnce() -> Future,
    ) -> Result<T, AdministrativeError>
    where
        Future: std::future::Future<Output = Result<T, AdministrativeError>>,
    {
        self.authorize(actor, action, tenant_id, correlation_id, now)
            .await?;
        match read().await {
            Ok(value) => {
                self.audit(
                    actor,
                    action,
                    AdministrativeAuditOutcome::Succeeded,
                    tenant_id,
                    correlation_id,
                    now,
                )
                .await?;
                Ok(value)
            }
            Err(error) => {
                self.audit(
                    actor,
                    action,
                    outcome_for(error),
                    tenant_id,
                    correlation_id,
                    now,
                )
                .await?;
                Err(error)
            }
        }
    }

    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        action: AdministrativeAction,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), AdministrativeError> {
        let result = if actor.tenant_id == tenant_id {
            self.security.authorize(actor, action, tenant_id).await
        } else {
            Err(AdministrativeError::Denied)
        };
        if let Err(error) = result {
            self.audit(
                actor,
                action,
                outcome_for(error),
                tenant_id,
                correlation_id,
                now,
            )
            .await?;
            return Err(error);
        }
        Ok(())
    }

    async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        action: AdministrativeAction,
        outcome: AdministrativeAuditOutcome,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), AdministrativeError> {
        self.security
            .audit(actor, action, outcome, tenant_id, correlation_id, now)
            .await
    }
}

fn valid_reason_code(value: &str) -> bool {
    (3..=64).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
        })
}

const fn outcome_for(error: AdministrativeError) -> AdministrativeAuditOutcome {
    match error {
        AdministrativeError::Denied => AdministrativeAuditOutcome::Denied,
        AdministrativeError::Invalid => AdministrativeAuditOutcome::Invalid,
        AdministrativeError::Unavailable => AdministrativeAuditOutcome::Failed,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use eitmad_contracts::{
        administration::{
            BackupState, MigrationState, ServiceComponentId, ServiceHealthState, SupportAction,
        },
        identity::{AccountId, DeviceId, PrincipalId, SessionId, UserId},
    };

    use super::*;

    #[derive(Default)]
    struct TestSecurity {
        allowed: AtomicBool,
        audits: Mutex<Vec<(AdministrativeAction, AdministrativeAuditOutcome)>>,
    }

    #[async_trait]
    impl AdministrativeSecurity for TestSecurity {
        async fn authorize(
            &self,
            _: &AuthenticatedServerSession,
            _: AdministrativeAction,
            _: TenantId,
        ) -> Result<(), AdministrativeError> {
            self.allowed
                .load(Ordering::SeqCst)
                .then_some(())
                .ok_or(AdministrativeError::Denied)
        }
        async fn audit(
            &self,
            _: &AuthenticatedServerSession,
            action: AdministrativeAction,
            outcome: AdministrativeAuditOutcome,
            _: TenantId,
            _: CorrelationId,
            _: UnixMillis,
        ) -> Result<(), AdministrativeError> {
            self.audits.lock().unwrap().push((action, outcome));
            Ok(())
        }
    }

    struct TestData;

    #[async_trait]
    impl AdministrationDataSource for TestData {
        async fn diagnostics(
            &self,
            _: &AuthenticatedServerSession,
            _: TenantId,
            correlation_id: CorrelationId,
            now: UnixMillis,
        ) -> Result<DiagnosticSummary, AdministrativeError> {
            Ok(DiagnosticSummary {
                generated_at: now,
                correlation_id,
                services: self.health(TenantId::new(Uuid::from_u128(1))).await?,
                active_relay_sessions: 2,
                pending_support_workflows: 0,
            })
        }
        async fn health(&self, _: TenantId) -> Result<Vec<ServiceHealth>, AdministrativeError> {
            Ok(vec![ServiceHealth {
                component: ServiceComponentId::parse("database").unwrap(),
                state: ServiceHealthState::Healthy,
                checked_at: UnixMillis(10),
                failure_code: None,
            }])
        }
        async fn backup_status(&self, _: TenantId) -> Result<BackupStatus, AdministrativeError> {
            Ok(BackupStatus {
                state: BackupState::Current,
                last_success_at: Some(UnixMillis(8)),
                last_verified_at: Some(UnixMillis(9)),
                next_scheduled_at: Some(UnixMillis(20)),
                recovery_point_age_ms: Some(2),
                failure_code: None,
            })
        }
        async fn migration_status(
            &self,
            _: TenantId,
        ) -> Result<MigrationStatus, AdministrativeError> {
            Ok(MigrationStatus {
                state: MigrationState::Current,
                current_version: 4,
                required_version: 4,
                pending_migration_ids: Vec::new(),
                failure_code: None,
            })
        }
        async fn audit_records(
            &self,
            tenant_id: TenantId,
            _: u32,
        ) -> Result<Vec<AdministrativeAuditRecord>, AdministrativeError> {
            Ok(vec![AdministrativeAuditRecord {
                audit_id: eitmad_contracts::administration::AdministrativeAuditId::new(
                    Uuid::new_v4(),
                ),
                tenant_id,
                principal_id: PrincipalId::new(Uuid::new_v4()),
                operation: "eitmad.test.v1".to_owned(),
                outcome: "succeeded".to_owned(),
                target_kind: "test".to_owned(),
                correlation_id: CorrelationId::new(Uuid::new_v4()),
                occurred_at: UnixMillis(1),
                redacted_error: None,
            }])
        }
        async fn tenant_visibility(
            &self,
            tenant_id: TenantId,
        ) -> Result<TenantVisibility, AdministrativeError> {
            Ok(TenantVisibility {
                tenant_id,
                enabled: true,
                active_device_count: 1,
                active_session_count: 1,
                last_seen_at: Some(UnixMillis(10)),
            })
        }
        async fn device_visibility(
            &self,
            tenant_id: TenantId,
        ) -> Result<Vec<DeviceVisibility>, AdministrativeError> {
            Ok(vec![DeviceVisibility {
                tenant_id,
                device_id: DeviceId::new(Uuid::new_v4()),
                label: "workshop".to_owned(),
                revoked: false,
                last_seen_at: Some(UnixMillis(10)),
            }])
        }
        async fn execute_support(
            &self,
            _: &AuthenticatedServerSession,
            mut workflow: SupportWorkflow,
        ) -> Result<SupportWorkflow, AdministrativeError> {
            workflow.state = SupportWorkflowState::Succeeded;
            workflow.completed_at = Some(workflow.requested_at);
            Ok(workflow)
        }
    }

    fn actor(tenant: u128) -> AuthenticatedServerSession {
        AuthenticatedServerSession {
            session_id: SessionId::new(Uuid::new_v4()),
            account_id: AccountId::new(Uuid::new_v4()),
            user_id: UserId::new(Uuid::new_v4()),
            device_id: DeviceId::new(Uuid::new_v4()),
            tenant_id: TenantId::new(Uuid::from_u128(tenant)),
            issued_at: UnixMillis(0),
            expires_at: UnixMillis(i64::MAX),
        }
    }

    fn service(allowed: bool) -> (AdministrationService, Arc<TestSecurity>) {
        let security = Arc::new(TestSecurity::default());
        security.allowed.store(allowed, Ordering::SeqCst);
        (
            AdministrationService::new(security.clone(), Arc::new(TestData)),
            security,
        )
    }

    #[tokio::test]
    async fn backup_status_is_authorized_audited_and_complete() {
        let (service, security) = service(true);
        let actor = actor(1);
        let status = service
            .backup_status(
                &actor,
                actor.tenant_id,
                CorrelationId::new(Uuid::new_v4()),
                UnixMillis(10),
            )
            .await
            .unwrap();
        assert_eq!(status.state, BackupState::Current);
        assert_eq!(status.last_verified_at, Some(UnixMillis(9)));
        assert_eq!(
            security.audits.lock().unwrap().as_slice(),
            &[(
                AdministrativeAction::ReadBackupStatus,
                AdministrativeAuditOutcome::Succeeded
            )]
        );
    }

    #[tokio::test]
    async fn administrative_authorization_denies_before_data_access_and_audits() {
        let (service, security) = service(false);
        let actor = actor(1);
        assert_eq!(
            service
                .health(
                    &actor,
                    actor.tenant_id,
                    CorrelationId::new(Uuid::new_v4()),
                    UnixMillis(10)
                )
                .await,
            Err(AdministrativeError::Denied)
        );
        assert_eq!(
            security.audits.lock().unwrap().as_slice(),
            &[(
                AdministrativeAction::ReadHealth,
                AdministrativeAuditOutcome::Denied
            )]
        );

        security.audits.lock().unwrap().clear();
        assert_eq!(
            service
                .audit_records(
                    &actor,
                    actor.tenant_id,
                    0,
                    CorrelationId::new(Uuid::new_v4()),
                    UnixMillis(10)
                )
                .await,
            Err(AdministrativeError::Denied)
        );
        assert_eq!(
            security.audits.lock().unwrap().as_slice(),
            &[(
                AdministrativeAction::ReadAudit,
                AdministrativeAuditOutcome::Denied
            )]
        );
    }

    #[tokio::test]
    async fn administrative_tenant_isolation_denies_cross_tenant_visibility_and_support() {
        let (service, security) = service(true);
        let actor = actor(1);
        let other = TenantId::new(Uuid::from_u128(2));
        assert_eq!(
            service
                .device_visibility(
                    &actor,
                    other,
                    CorrelationId::new(Uuid::new_v4()),
                    UnixMillis(10)
                )
                .await,
            Err(AdministrativeError::Denied)
        );
        let request = StartSupportWorkflow {
            tenant_id: other,
            action: SupportAction::VerifyBackup,
            reason_code: "incident.backup".to_owned(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
        };
        assert_eq!(
            service
                .start_support_workflow(&actor, &request, UnixMillis(10))
                .await,
            Err(AdministrativeError::Denied)
        );
        assert_eq!(security.audits.lock().unwrap().len(), 2);
    }
}
