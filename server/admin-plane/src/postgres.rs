use std::sync::Arc;

use async_trait::async_trait;
use eitmad_contracts::{
    administration::{
        AdministrativeAuditId, AdministrativeAuditRecord, AdministrativeFailureCode, BackupState,
        BackupStatus, DeviceVisibility, DiagnosticSummary, MigrationState, MigrationStatus,
        ServiceComponentId, ServiceHealth, ServiceHealthState, SupportWorkflow,
        SupportWorkflowState, TenantVisibility,
    },
    identity::{DeviceId, PrincipalId, TenantId},
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use sqlx::{PgPool, Row as _};

use crate::{AdministrationDataSource, AdministrativeError, RelayMetricsSource};

#[async_trait]
pub trait SupportWorkflowExecutor: Send + Sync {
    async fn execute(
        &self,
        actor: &AuthenticatedServerSession,
        workflow: &SupportWorkflow,
    ) -> Result<(), AdministrativeError>;
}

#[derive(Clone)]
pub struct PostgresAdministrationDataSource {
    pool: PgPool,
    support: Arc<dyn SupportWorkflowExecutor>,
    relay_metrics: Arc<dyn RelayMetricsSource>,
}

impl PostgresAdministrationDataSource {
    #[must_use]
    pub const fn new(
        pool: PgPool,
        support: Arc<dyn SupportWorkflowExecutor>,
        relay_metrics: Arc<dyn RelayMetricsSource>,
    ) -> Self {
        Self {
            pool,
            support,
            relay_metrics,
        }
    }

    async fn transaction(
        &self,
        tenant_id: TenantId,
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, AdministrativeError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
            .bind(tenant_id.value().to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        Ok(transaction)
    }
}

#[async_trait]
impl AdministrationDataSource for PostgresAdministrationDataSource {
    async fn diagnostics(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<DiagnosticSummary, AdministrativeError> {
        let services = self.health(tenant_id).await?;
        let active_relay_sessions = self
            .relay_metrics
            .active_sessions(actor, correlation_id, now)
            .await?;
        let mut transaction = self.transaction(tenant_id).await?;
        let pending_support_workflows: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM operations.support_workflows
             WHERE tenant_id = $1 AND state IN ('pending', 'running')",
        )
        .bind(tenant_id.value())
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        Ok(DiagnosticSummary {
            generated_at: now,
            correlation_id,
            services,
            active_relay_sessions,
            pending_support_workflows: u32::try_from(pending_support_workflows)
                .map_err(|_| AdministrativeError::Unavailable)?,
        })
    }

    async fn health(&self, tenant_id: TenantId) -> Result<Vec<ServiceHealth>, AdministrativeError> {
        let mut transaction = self.transaction(tenant_id).await?;
        sqlx::query("SELECT 1")
            .execute(&mut *transaction)
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        Ok(vec![ServiceHealth {
            component: ServiceComponentId::parse("database")
                .map_err(|_| AdministrativeError::Unavailable)?,
            state: ServiceHealthState::Healthy,
            checked_at: crate::postgres::unix_millis_now(),
            failure_code: None,
        }])
    }

    async fn backup_status(
        &self,
        tenant_id: TenantId,
    ) -> Result<BackupStatus, AdministrativeError> {
        let mut transaction = self.transaction(tenant_id).await?;
        let row = sqlx::query(
            "SELECT state, last_success_at, last_verified_at, next_scheduled_at,
                    recovery_point_age_ms, failure_code
             FROM operations.backup_status WHERE tenant_id = $1",
        )
        .bind(tenant_id.value())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        let Some(row) = row else {
            return Ok(BackupStatus {
                state: BackupState::NotConfigured,
                last_success_at: None,
                last_verified_at: None,
                next_scheduled_at: None,
                recovery_point_age_ms: None,
                failure_code: None,
            });
        };
        Ok(BackupStatus {
            state: parse_backup_state(&row.get::<String, _>("state"))?,
            last_success_at: row.get::<Option<i64>, _>("last_success_at").map(UnixMillis),
            last_verified_at: row
                .get::<Option<i64>, _>("last_verified_at")
                .map(UnixMillis),
            next_scheduled_at: row
                .get::<Option<i64>, _>("next_scheduled_at")
                .map(UnixMillis),
            recovery_point_age_ms: row
                .get::<Option<i64>, _>("recovery_point_age_ms")
                .map(u64::try_from)
                .transpose()
                .map_err(|_| AdministrativeError::Unavailable)?,
            failure_code: parse_failure(row.get("failure_code"))?,
        })
    }

    async fn migration_status(
        &self,
        tenant_id: TenantId,
    ) -> Result<MigrationStatus, AdministrativeError> {
        let mut transaction = self.transaction(tenant_id).await?;
        let current: i32 = sqlx::query_scalar(
            "SELECT coalesce(max(version), 0) FROM public.eitmad_server_migrations",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        let current_version =
            u32::try_from(current).map_err(|_| AdministrativeError::Unavailable)?;
        let required_version = 3;
        Ok(MigrationStatus {
            state: if current_version >= required_version {
                MigrationState::Current
            } else {
                MigrationState::Pending
            },
            current_version,
            required_version,
            pending_migration_ids: if current_version < required_version {
                vec!["server.admin-foundation.v1".to_owned()]
            } else {
                Vec::new()
            },
            failure_code: None,
        })
    }

    async fn audit_records(
        &self,
        tenant_id: TenantId,
        limit: u32,
    ) -> Result<Vec<AdministrativeAuditRecord>, AdministrativeError> {
        let mut transaction = self.transaction(tenant_id).await?;
        let rows = sqlx::query(
            "SELECT audit_id, tenant_id, principal_id, operation, outcome, target_kind,
                    correlation_id, occurred_at, redacted_error
             FROM audit.server_records WHERE tenant_id = $1
             ORDER BY occurred_at DESC, audit_id DESC LIMIT $2",
        )
        .bind(tenant_id.value())
        .bind(i64::from(limit))
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        rows.iter().map(audit_record).collect()
    }

    async fn tenant_visibility(
        &self,
        tenant_id: TenantId,
    ) -> Result<TenantVisibility, AdministrativeError> {
        let mut transaction = self.transaction(tenant_id).await?;
        let row = sqlx::query(
            "SELECT EXISTS (SELECT 1 FROM control.tenants WHERE tenant_id = $1) AS enabled,
                    (SELECT count(DISTINCT device_id) FROM control.account_devices
                     WHERE tenant_id = $1 AND revoked_at IS NULL) AS active_device_count,
                    (SELECT count(*) FROM control.sessions
                     WHERE tenant_id = $1 AND revoked_at IS NULL) AS active_session_count,
                    (SELECT max(last_seen_at) FROM control.sessions
                     WHERE tenant_id = $1) AS last_seen_at",
        )
        .bind(tenant_id.value())
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        Ok(TenantVisibility {
            tenant_id,
            enabled: row.get("enabled"),
            active_device_count: count_to_u32(row.get("active_device_count"))?,
            active_session_count: count_to_u32(row.get("active_session_count"))?,
            last_seen_at: row.get::<Option<i64>, _>("last_seen_at").map(UnixMillis),
        })
    }

    async fn device_visibility(
        &self,
        tenant_id: TenantId,
    ) -> Result<Vec<DeviceVisibility>, AdministrativeError> {
        let mut transaction = self.transaction(tenant_id).await?;
        let rows = sqlx::query(
            "SELECT ad.device_id, d.label,
                    bool_and(ad.revoked_at IS NOT NULL OR d.revoked_at IS NOT NULL) AS revoked,
                    max(s.last_seen_at) AS last_seen_at
             FROM control.account_devices ad
             JOIN control.devices d ON d.device_id = ad.device_id
             LEFT JOIN control.sessions s ON s.tenant_id = ad.tenant_id
                  AND s.device_id = ad.device_id
             WHERE ad.tenant_id = $1
             GROUP BY ad.device_id, d.label ORDER BY ad.device_id",
        )
        .bind(tenant_id.value())
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        Ok(rows
            .into_iter()
            .map(|row| DeviceVisibility {
                tenant_id,
                device_id: DeviceId::new(row.get("device_id")),
                label: row.get("label"),
                revoked: row.get("revoked"),
                last_seen_at: row.get::<Option<i64>, _>("last_seen_at").map(UnixMillis),
            })
            .collect())
    }

    async fn execute_support(
        &self,
        actor: &AuthenticatedServerSession,
        mut workflow: SupportWorkflow,
    ) -> Result<SupportWorkflow, AdministrativeError> {
        let mut transaction = self.transaction(workflow.tenant_id).await?;
        sqlx::query(
            "INSERT INTO operations.support_workflows
                (tenant_id, workflow_id, action_json, reason_code, state, requested_at)
             VALUES ($1, $2, $3, $4, 'running', $5)",
        )
        .bind(workflow.tenant_id.value())
        .bind(workflow.workflow_id.value())
        .bind(serde_json::to_value(&workflow.action).map_err(|_| AdministrativeError::Invalid)?)
        .bind(&workflow.reason_code)
        .bind(workflow.requested_at.0)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;

        let workflow_error = self.support.execute(actor, &workflow).await.err();
        let now = unix_millis_now();
        let mut transaction = self.transaction(workflow.tenant_id).await?;
        let (state, failure_code) = match workflow_error {
            None => ("succeeded", None),
            Some(AdministrativeError::Denied) => {
                ("failed", Some("eitmad.error.authorization-denied.v1"))
            }
            Some(AdministrativeError::Invalid) => {
                ("failed", Some("eitmad.error.contract-invalid.v1"))
            }
            Some(AdministrativeError::Unavailable) => {
                ("failed", Some("eitmad.error.config-unavailable.v1"))
            }
        };
        sqlx::query(
            "UPDATE operations.support_workflows
             SET state = $3, completed_at = $4, failure_code = $5
             WHERE tenant_id = $1 AND workflow_id = $2",
        )
        .bind(workflow.tenant_id.value())
        .bind(workflow.workflow_id.value())
        .bind(state)
        .bind(now.0)
        .bind(failure_code)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AdministrativeError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AdministrativeError::Unavailable)?;
        workflow.state = if state == "succeeded" {
            SupportWorkflowState::Succeeded
        } else {
            SupportWorkflowState::Failed
        };
        workflow.completed_at = Some(now);
        workflow.failure_code = parse_failure(failure_code.map(str::to_owned))?;
        if let Some(error) = workflow_error {
            return Err(error);
        }
        Ok(workflow)
    }
}

fn audit_record(row: &sqlx::PgRow) -> Result<AdministrativeAuditRecord, AdministrativeError> {
    Ok(AdministrativeAuditRecord {
        audit_id: AdministrativeAuditId::new(row.get("audit_id")),
        tenant_id: TenantId::new(row.get("tenant_id")),
        principal_id: PrincipalId::new(row.get("principal_id")),
        operation: row.get("operation"),
        outcome: row.get("outcome"),
        target_kind: row.get("target_kind"),
        correlation_id: CorrelationId::new(row.get("correlation_id")),
        occurred_at: UnixMillis(row.get("occurred_at")),
        redacted_error: parse_failure(row.get("redacted_error"))?,
    })
}

fn parse_failure(
    value: Option<String>,
) -> Result<Option<AdministrativeFailureCode>, AdministrativeError> {
    value
        .map(AdministrativeFailureCode::parse)
        .transpose()
        .map_err(|_| AdministrativeError::Unavailable)
}

fn parse_backup_state(value: &str) -> Result<BackupState, AdministrativeError> {
    match value {
        "current" => Ok(BackupState::Current),
        "stale" => Ok(BackupState::Stale),
        "running" => Ok(BackupState::Running),
        "failed" => Ok(BackupState::Failed),
        "not_configured" => Ok(BackupState::NotConfigured),
        _ => Err(AdministrativeError::Unavailable),
    }
}

fn count_to_u32(value: i64) -> Result<u32, AdministrativeError> {
    u32::try_from(value).map_err(|_| AdministrativeError::Unavailable)
}

fn unix_millis_now() -> UnixMillis {
    let milliseconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    UnixMillis(i64::try_from(milliseconds).unwrap_or(i64::MAX))
}
