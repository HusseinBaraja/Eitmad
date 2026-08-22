use eitmad_contracts::identity::TenantId;
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _, postgres::PgPoolOptions};

const FOUNDATION_SQL: &str = include_str!("../migrations/0002_sync_foundation.sql");

#[derive(Clone)]
pub struct SyncDatabase {
    pool: PgPool,
}

pub(crate) async fn tenant_transaction(
    pool: &PgPool,
    tenant_id: TenantId,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(tenant_id.value().to_string())
        .execute(&mut *transaction)
        .await?;
    Ok(transaction)
}

#[derive(Debug, thiserror::Error)]
pub enum SyncDatabaseError {
    #[error("sync database is unavailable")]
    Unavailable(#[source] sqlx::Error),
    #[error("sync database migration checksum changed")]
    MigrationChecksum,
}

impl SyncDatabase {
    /// Connects to the `PostgreSQL` sync authority.
    ///
    /// # Errors
    ///
    /// Returns a sanitized connection error.
    pub async fn connect(
        database_url: &str,
        maximum_connections: u32,
    ) -> Result<Self, SyncDatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(maximum_connections)
            .connect(database_url)
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
        Ok(Self { pool })
    }

    /// Applies the checksummed sync migration after the control migration.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable prerequisites or changed history.
    pub async fn migrate(&self) -> Result<(), SyncDatabaseError> {
        let checksum = format!("{:x}", Sha256::digest(FOUNDATION_SQL.as_bytes()));
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(1163158101)")
            .execute(&mut *transaction)
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
        let control_applied: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM public.eitmad_server_migrations WHERE version = 1
             )",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(SyncDatabaseError::Unavailable)?;
        if !control_applied {
            return Err(SyncDatabaseError::Unavailable(sqlx::Error::Protocol(
                "control migration is required".to_owned(),
            )));
        }
        let existing =
            sqlx::query("SELECT checksum FROM public.eitmad_server_migrations WHERE version = 2")
                .fetch_optional(&mut *transaction)
                .await
                .map_err(SyncDatabaseError::Unavailable)?;
        if let Some(existing) = existing {
            if existing.get::<String, _>("checksum") != checksum {
                return Err(SyncDatabaseError::MigrationChecksum);
            }
        } else {
            sqlx::raw_sql(FOUNDATION_SQL)
                .execute(&mut *transaction)
                .await
                .map_err(SyncDatabaseError::Unavailable)?;
            sqlx::query(
                "INSERT INTO public.eitmad_server_migrations
                    (version, migration_id, checksum)
                 VALUES (2, 'server.sync-foundation.v1', $1)",
            )
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(SyncDatabaseError::Unavailable)
    }

    #[must_use]
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_persists_sync_sessions_history_and_conflicts() {
        for table in [
            "sync.scopes",
            "sync.records",
            "sync.operations",
            "sync.idempotency_results",
            "sync.conflicts",
            "sync.snapshots",
            "sync.device_checkpoints",
            "sync.subscription_events",
        ] {
            assert!(FOUNDATION_SQL.contains(table), "missing sync table {table}");
        }
        assert!(
            FOUNDATION_SQL
                .contains("UNIQUE (tenant_id, scope_kind, scope_id, schema_id, idempotency_key)")
        );
    }

    #[test]
    fn migration_forces_tenant_isolation_on_every_sync_table() {
        assert!(FOUNDATION_SQL.contains("FORCE ROW LEVEL SECURITY"));
        for table in [
            "'scopes'",
            "'records'",
            "'operations'",
            "'idempotency_results'",
            "'conflicts'",
            "'snapshots'",
            "'device_checkpoints'",
            "'subscription_events'",
        ] {
            assert!(FOUNDATION_SQL.contains(table), "missing RLS table {table}");
        }
    }
}
