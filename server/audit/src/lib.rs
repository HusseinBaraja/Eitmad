//! Complete append-only audit authority shared by every server plane.

use eitmad_contracts::{
    identity::{
        DeviceId, PrincipalId, ScopeId, ScopeKind, ScopeRef, SessionId, TenantId, WorkspaceId,
    },
    server::AuthenticatedServerSession,
    transport::{CausationId, CorrelationId, IdempotencyKey, UnixMillis},
};
use sha2::{Digest as _, Sha256};
use sqlx::{PgConnection, PgPool, Row as _};
use uuid::Uuid;

const MIGRATION_SQL: &str = include_str!("../migrations/0004_server_audit_envelope.sql");
const PREPARE_MARKER: &str = "-- eitmad:phase:prepare";
const VALIDATE_MARKER: &str = "-- eitmad:phase:validate";
const FINALIZE_MARKER: &str = "-- eitmad:phase:finalize";
const BACKFILL_BATCH_SIZE: i64 = 1_000;

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
        let mut connection = self
            .pool
            .acquire()
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_lock(1163158101)")
            .execute(&mut *connection)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        let result = migrate_locked(&mut connection, &checksum).await;
        let reset = sqlx::query("SELECT set_config('eitmad.audit_migration', '', false)")
            .execute(&mut *connection)
            .await;
        let unlock = sqlx::query("SELECT pg_advisory_unlock(1163158101)")
            .execute(&mut *connection)
            .await;
        match (result, reset, unlock) {
            (Err(error), _, _) => Err(error),
            (Ok(()), Err(error), _) | (Ok(()), Ok(_), Err(error)) => {
                Err(AuditDatabaseError::Unavailable(error))
            }
            (Ok(()), Ok(_), Ok(_)) => Ok(()),
        }
    }
}

async fn migrate_locked(
    connection: &mut PgConnection,
    checksum: &str,
) -> Result<(), AuditDatabaseError> {
    let prerequisites: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.eitmad_server_migrations WHERE version IN (1, 2, 3)",
    )
    .fetch_one(&mut *connection)
    .await
    .map_err(AuditDatabaseError::Unavailable)?;
    if prerequisites != 3 {
        return Err(AuditDatabaseError::MissingPrerequisites);
    }
    let existing =
        sqlx::query("SELECT checksum FROM public.eitmad_server_migrations WHERE version = 4")
            .fetch_optional(&mut *connection)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
    if let Some(existing) = existing {
        if existing.get::<String, _>("checksum") != checksum {
            return Err(AuditDatabaseError::MigrationChecksum);
        }
    } else {
        let (prepare, validate, finalize) = migration_phases()?;
        sqlx::raw_sql(prepare)
            .execute(&mut *connection)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        sqlx::query("SELECT set_config('eitmad.audit_migration', 'backfill-v4', false)")
            .execute(&mut *connection)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        loop {
            let rows = sqlx::query(
                "WITH batch AS (
                         SELECT ctid FROM audit.server_records
                         WHERE actor_kind IS NULL OR scope_kind IS NULL OR scope_id IS NULL
                         LIMIT $1
                     )
                     UPDATE audit.server_records AS records
                     SET actor_kind = 'user', scope_kind = 'tenant', scope_id = tenant_id
                     FROM batch WHERE records.ctid = batch.ctid
                     RETURNING records.audit_id",
            )
            .bind(BACKFILL_BATCH_SIZE)
            .fetch_all(&mut *connection)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
            if rows.len() < usize::try_from(BACKFILL_BATCH_SIZE).unwrap_or(usize::MAX) {
                break;
            }
        }
        sqlx::query("SELECT set_config('eitmad.audit_migration', '', false)")
            .execute(&mut *connection)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        sqlx::raw_sql(validate)
            .execute(&mut *connection)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        sqlx::raw_sql(finalize)
            .execute(&mut *connection)
            .await
            .map_err(AuditDatabaseError::Unavailable)?;
        sqlx::query(
            "INSERT INTO public.eitmad_server_migrations
                    (version, migration_id, checksum)
                 VALUES (4, 'server.audit-envelope.v1', $1)",
        )
        .bind(checksum)
        .execute(&mut *connection)
        .await
        .map_err(AuditDatabaseError::Unavailable)?;
    }
    Ok(())
}

fn migration_phases() -> Result<(&'static str, &'static str, &'static str), AuditDatabaseError> {
    let (_, after_prepare) = MIGRATION_SQL
        .split_once(PREPARE_MARKER)
        .ok_or(AuditDatabaseError::MigrationChecksum)?;
    let (prepare, after_validate) = after_prepare
        .split_once(VALIDATE_MARKER)
        .ok_or(AuditDatabaseError::MigrationChecksum)?;
    let (validate, finalize) = after_validate
        .split_once(FINALIZE_MARKER)
        .ok_or(AuditDatabaseError::MigrationChecksum)?;
    Ok((prepare, validate, finalize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_declares_each_staged_phase() {
        let (prepare, validate, finalize) = migration_phases().unwrap();
        assert!(!prepare.trim().is_empty());
        assert!(!validate.trim().is_empty());
        assert!(!finalize.trim().is_empty());
    }

    #[tokio::test]
    #[ignore = "requires a dedicated EITMAD_TEST_DATABASE_URL PostgreSQL database"]
    async fn migration_executes_and_preserves_append_only_records() {
        let database_url = std::env::var("EITMAD_TEST_DATABASE_URL")
            .expect("EITMAD_TEST_DATABASE_URL must name a dedicated test database");
        let pool = sqlx::PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .unwrap();
        let database_name: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(
            database_name.contains("test"),
            "refusing to reset a database without 'test' in its name"
        );

        sqlx::raw_sql(
            "DROP SCHEMA IF EXISTS administration CASCADE;
             DROP SCHEMA IF EXISTS sync CASCADE;
             DROP SCHEMA IF EXISTS publication CASCADE;
             DROP SCHEMA IF EXISTS audit CASCADE;
             DROP SCHEMA IF EXISTS control CASCADE;
             DROP TABLE IF EXISTS public.eitmad_server_migrations;",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE public.eitmad_server_migrations (
                 version bigint PRIMARY KEY,
                 migration_id text NOT NULL UNIQUE,
                 checksum text NOT NULL
             )",
        )
        .execute(&pool)
        .await
        .unwrap();
        for (version, migration_id, sql) in [
            (
                1_i64,
                "control.foundation.v1",
                include_str!("../../control-plane/migrations/0001_control_foundation.sql"),
            ),
            (
                2_i64,
                "sync.foundation.v1",
                include_str!("../../sync-plane/migrations/0002_sync_foundation.sql"),
            ),
            (
                3_i64,
                "admin.foundation.v1",
                include_str!("../../admin-plane/migrations/0003_admin_foundation.sql"),
            ),
        ] {
            sqlx::raw_sql(sql).execute(&pool).await.unwrap();
            sqlx::query(
                "INSERT INTO public.eitmad_server_migrations (version, migration_id, checksum)
                 VALUES ($1, $2, 'integration-fixture')",
            )
            .bind(version)
            .bind(migration_id)
            .execute(&pool)
            .await
            .unwrap();
        }

        let tenant_id = TenantId::new(Uuid::new_v4());
        let legacy_audit_id = Uuid::new_v4();
        sqlx::query("SELECT set_config('eitmad.tenant_id', $1, false)")
            .bind(tenant_id.value().to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO audit.server_records
                (audit_id, tenant_id, session_id, principal_id, operation, outcome,
                 target_kind, correlation_id, occurred_at)
             VALUES ($1, $2, $3, $4, 'legacy', 'succeeded', 'test', $5, 1)",
        )
        .bind(legacy_audit_id)
        .bind(tenant_id.value())
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();

        let database = AuditDatabase::from_pool(pool.clone());
        database.migrate().await.unwrap();
        let backfill = sqlx::query(
            "SELECT actor_kind, scope_kind, scope_id FROM audit.server_records
             WHERE audit_id = $1",
        )
        .bind(legacy_audit_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(backfill.get::<String, _>("actor_kind"), "user");
        assert_eq!(backfill.get::<String, _>("scope_kind"), "tenant");
        assert_eq!(backfill.get::<Uuid, _>("scope_id"), tenant_id.value());

        for kind in [
            ServerAuditActorKind::User,
            ServerAuditActorKind::Service,
            ServerAuditActorKind::Device,
            ServerAuditActorKind::System,
        ] {
            let mut transaction = pool.begin().await.unwrap();
            sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
                .bind(tenant_id.value().to_string())
                .execute(&mut *transaction)
                .await
                .unwrap();
            append(
                &mut transaction,
                &ServerAuditEnvelope {
                    actor: ServerAuditActor {
                        tenant_id,
                        workspace_id: None,
                        kind,
                        session_id: None,
                        device_id: None,
                        principal_id: None,
                    },
                    scope: tenant_scope(tenant_id),
                    operation: "integration-test",
                    outcome: ServerAuditOutcome::Succeeded,
                    target_kind: "test",
                    target_id: None,
                    correlation_id: CorrelationId::new(Uuid::new_v4()),
                    causation_id: None,
                    idempotency_key: None,
                    redacted_error: None,
                    occurred_at: UnixMillis(2),
                },
            )
            .await
            .unwrap();
            transaction.commit().await.unwrap();
        }

        assert!(
            sqlx::query("UPDATE audit.server_records SET operation = 'changed'")
                .execute(&pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM audit.server_records")
                .execute(&pool)
                .await
                .is_err()
        );
        sqlx::query(
            "UPDATE public.eitmad_server_migrations SET checksum = 'changed' WHERE version = 4",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            database.migrate().await,
            Err(AuditDatabaseError::MigrationChecksum)
        ));
    }
}
