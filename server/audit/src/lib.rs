//! Complete append-only audit authority shared by every server plane.

use eitmad_contracts::{
    identity::{
        DeviceId, PrincipalId, ScopeId, ScopeKind, ScopeRef, SessionId, TenantId, WorkspaceId,
    },
    server::AuthenticatedServerSession,
    transport::{CausationId, CorrelationId, IdempotencyKey, UnixMillis},
};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

const MIGRATION_SQL: &str = include_str!("../migrations/0004_server_audit_envelope.sql");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerAuditOutcome {
    Succeeded,
    Denied,
    Invalid,
    Conflict,
    Failed,
}

impl ServerAuditOutcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Denied => "denied",
            Self::Invalid => "invalid",
            Self::Conflict => "conflict",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerAuditActorKind {
    User,
    Service,
    Device,
    System,
}

impl ServerAuditActorKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Service => "service",
            Self::Device => "device",
            Self::System => "system",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerAuditActor {
    pub tenant_id: TenantId,
    pub workspace_id: Option<WorkspaceId>,
    pub kind: ServerAuditActorKind,
    pub session_id: Option<SessionId>,
    pub device_id: Option<DeviceId>,
    pub principal_id: Option<PrincipalId>,
}

impl ServerAuditActor {
    #[must_use]
    pub const fn from_session(session: &AuthenticatedServerSession) -> Self {
        Self {
            tenant_id: session.tenant_id,
            workspace_id: None,
            kind: ServerAuditActorKind::User,
            session_id: Some(session.session_id),
            device_id: Some(session.device_id),
            principal_id: Some(PrincipalId::new(session.user_id.value())),
        }
    }

    #[must_use]
    pub const fn system(tenant_id: TenantId) -> Self {
        Self {
            tenant_id,
            workspace_id: None,
            kind: ServerAuditActorKind::System,
            session_id: None,
            device_id: None,
            principal_id: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ServerAuditEvent<'a> {
    pub operation: &'a str,
    pub outcome: ServerAuditOutcome,
    pub target_kind: &'a str,
    pub target_id: Option<Uuid>,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub redacted_error: Option<&'a str>,
    pub occurred_at: UnixMillis,
}

#[derive(Clone, Debug)]
pub struct ServerAuditEnvelope<'a> {
    pub actor: ServerAuditActor,
    pub scope: ScopeRef,
    pub operation: &'a str,
    pub outcome: ServerAuditOutcome,
    pub target_kind: &'a str,
    pub target_id: Option<Uuid>,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub redacted_error: Option<&'a str>,
    pub occurred_at: UnixMillis,
}

impl<'a> ServerAuditEnvelope<'a> {
    #[must_use]
    pub const fn from_session(
        session: &AuthenticatedServerSession,
        scope: ScopeRef,
        event: ServerAuditEvent<'a>,
    ) -> Self {
        Self {
            actor: ServerAuditActor::from_session(session),
            scope,
            operation: event.operation,
            outcome: event.outcome,
            target_kind: event.target_kind,
            target_id: event.target_id,
            correlation_id: event.correlation_id,
            causation_id: event.causation_id,
            idempotency_key: event.idempotency_key,
            redacted_error: event.redacted_error,
            occurred_at: event.occurred_at,
        }
    }

    #[must_use]
    pub fn for_tenant_session(
        session: &AuthenticatedServerSession,
        event: ServerAuditEvent<'a>,
    ) -> Self {
        Self::from_session(session, tenant_scope(session.tenant_id), event)
    }
}

/// Returns the canonical tenant scope.
///
/// # Panics
///
/// Panics only if the static `tenant` scope identifier becomes invalid.
#[must_use]
pub fn tenant_scope(tenant_id: TenantId) -> ScopeRef {
    ScopeRef {
        kind: ScopeKind::parse("tenant").expect("static tenant scope kind must be valid"),
        id: ScopeId::new(tenant_id.value()),
    }
}

/// Appends one complete audit outcome inside the caller's state transaction.
///
/// # Errors
///
/// Returns a `PostgreSQL` error. The caller must roll back authoritative work.
pub async fn append(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    envelope: &ServerAuditEnvelope<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit.server_records
            (audit_id, tenant_id, workspace_id, actor_kind, session_id, device_id, principal_id,
             scope_kind, scope_id, operation, outcome, target_kind, target_id,
             correlation_id, causation_id, idempotency_key, redacted_error, occurred_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                 $14, $15, $16, $17, $18)",
    )
    .bind(Uuid::new_v4())
    .bind(envelope.actor.tenant_id.value())
    .bind(envelope.actor.workspace_id.map(WorkspaceId::value))
    .bind(envelope.actor.kind.as_str())
    .bind(envelope.actor.session_id.map(SessionId::value))
    .bind(envelope.actor.device_id.map(DeviceId::value))
    .bind(envelope.actor.principal_id.map(PrincipalId::value))
    .bind(envelope.scope.kind.as_str())
    .bind(envelope.scope.id.value())
    .bind(envelope.operation)
    .bind(envelope.outcome.as_str())
    .bind(envelope.target_kind)
    .bind(envelope.target_id)
    .bind(envelope.correlation_id.value())
    .bind(envelope.causation_id.map(CausationId::value))
    .bind(envelope.idempotency_key.map(IdempotencyKey::value))
    .bind(envelope.redacted_error)
    .bind(envelope.occurred_at.0)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[derive(Clone)]
pub struct AuditDatabase {
    pool: PgPool,
}

#[derive(Debug, thiserror::Error)]
pub enum AuditDatabaseError {
    #[error("server audit database is unavailable")]
    Unavailable(#[source] sqlx::Error),
    #[error("control, sync, and administration migrations are required")]
    MissingPrerequisites,
    #[error("server audit migration checksum changed")]
    MigrationChecksum,
}

impl AuditDatabase {
    #[must_use]
    pub const fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Adds the complete server audit envelope after foundation migrations.
    ///
    /// # Errors
    ///
    /// Returns an error for missing prerequisites, changed history, or `PostgreSQL` failure.
    pub async fn migrate(&self) -> Result<(), AuditDatabaseError> {
        let checksum = format!("{:x}", Sha256::digest(MIGRATION_SQL.as_bytes()));
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(1163158101)")
            .execute(&mut *transaction)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        let prerequisites: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public.eitmad_server_migrations WHERE version IN (1, 2, 3)",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(AuditDatabaseError::Unavailable)?;
        if prerequisites != 3 {
            return Err(AuditDatabaseError::MissingPrerequisites);
        }
        let existing =
            sqlx::query("SELECT checksum FROM public.eitmad_server_migrations WHERE version = 4")
                .fetch_optional(&mut *transaction)
                .await
                .map_err(AuditDatabaseError::Unavailable)?;
        if let Some(existing) = existing {
            if existing.get::<String, _>("checksum") != checksum {
                return Err(AuditDatabaseError::MigrationChecksum);
            }
        } else {
            sqlx::raw_sql(MIGRATION_SQL)
                .execute(&mut *transaction)
                .await
                .map_err(AuditDatabaseError::Unavailable)?;
            sqlx::query(
                "INSERT INTO public.eitmad_server_migrations
                    (version, migration_id, checksum)
                 VALUES (4, 'server.audit-envelope.v1', $1)",
            )
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(AuditDatabaseError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_backfills_complete_scope_and_restores_append_only_triggers() {
        for column in [
            "workspace_id",
            "actor_kind",
            "scope_kind",
            "scope_id",
            "target_id",
            "causation_id",
            "idempotency_key",
        ] {
            assert!(
                MIGRATION_SQL.contains(column),
                "missing audit column {column}"
            );
        }
        assert!(MIGRATION_SQL.contains("scope_kind = 'tenant', scope_id = tenant_id"));
        assert!(MIGRATION_SQL.contains("ALTER COLUMN scope_kind SET NOT NULL"));
        assert!(MIGRATION_SQL.contains("CREATE TRIGGER server_audit_no_update"));
        assert!(MIGRATION_SQL.contains("CREATE TRIGGER server_audit_no_delete"));
    }
}
