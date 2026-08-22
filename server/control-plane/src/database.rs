use eitmad_contracts::identity::TenantId;
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Postgres, Row as _, Transaction, postgres::PgPoolOptions};

const FOUNDATION_SQL: &str = include_str!("../migrations/0001_control_foundation.sql");

#[derive(Clone)]
pub struct ControlDatabase {
    pool: PgPool,
}

#[derive(Debug, thiserror::Error)]
pub enum ControlDatabaseError {
    #[error("server database is unavailable")]
    Unavailable(#[source] sqlx::Error),
    #[error("server database migration failed")]
    Migration(#[source] sqlx::Error),
    #[error("server database migration checksum changed")]
    MigrationChecksum,
}

impl ControlDatabase {
    /// Connects to `PostgreSQL` with a bounded pool.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error when `PostgreSQL` is unavailable.
    pub async fn connect(
        database_url: &str,
        maximum_connections: u32,
    ) -> Result<Self, ControlDatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(maximum_connections)
            .connect(database_url)
            .await
            .map_err(ControlDatabaseError::Unavailable)?;
        Ok(Self { pool })
    }

    /// Applies embedded, checksummed `SQLx` migrations.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error when a migration cannot complete.
    pub async fn migrate(&self) -> Result<(), ControlDatabaseError> {
        let checksum = format!("{:x}", Sha256::digest(FOUNDATION_SQL.as_bytes()));
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(ControlDatabaseError::Migration)?;
        sqlx::query("SELECT pg_advisory_xact_lock(1163158101)")
            .execute(&mut *transaction)
            .await
            .map_err(ControlDatabaseError::Migration)?;
        sqlx::raw_sql(
            "CREATE TABLE IF NOT EXISTS public.eitmad_server_migrations (
                version integer PRIMARY KEY,
                migration_id text NOT NULL UNIQUE,
                checksum text NOT NULL,
                applied_at timestamptz NOT NULL DEFAULT now()
            )",
        )
        .execute(&mut *transaction)
        .await
        .map_err(ControlDatabaseError::Migration)?;
        let existing =
            sqlx::query("SELECT checksum FROM public.eitmad_server_migrations WHERE version = 1")
                .fetch_optional(&mut *transaction)
                .await
                .map_err(ControlDatabaseError::Migration)?;
        if let Some(existing) = existing {
            if existing.get::<String, _>("checksum") != checksum {
                return Err(ControlDatabaseError::MigrationChecksum);
            }
        } else {
            sqlx::raw_sql(FOUNDATION_SQL)
                .execute(&mut *transaction)
                .await
                .map_err(ControlDatabaseError::Migration)?;
            sqlx::query(
                "INSERT INTO public.eitmad_server_migrations
                    (version, migration_id, checksum)
                 VALUES (1, 'server.control-foundation.v1', $1)",
            )
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(ControlDatabaseError::Migration)?;
        }
        transaction
            .commit()
            .await
            .map_err(ControlDatabaseError::Migration)
    }

    #[must_use]
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }
}

pub(crate) async fn tenant_transaction(
    pool: &PgPool,
    tenant_id: TenantId,
) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(tenant_id.value().to_string())
        .execute(&mut *transaction)
        .await?;
    Ok(transaction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_persists_the_complete_identity_topology() {
        for table in [
            "control.tenants",
            "control.users",
            "control.accounts",
            "control.organizations",
            "control.devices",
            "control.account_devices",
            "control.sessions",
            "control.token_families",
            "control.licenses",
            "control.update_assignments",
            "audit.server_records",
        ] {
            assert!(
                FOUNDATION_SQL.contains(table),
                "missing persistent table {table}"
            );
        }
        assert!(FOUNDATION_SQL.contains("server_audit_no_update"));
        assert!(FOUNDATION_SQL.contains("server_audit_no_delete"));
    }

    #[test]
    fn migration_forces_tenant_isolation_on_scoped_control_tables() {
        assert!(FOUNDATION_SQL.contains("FORCE ROW LEVEL SECURITY"));
        for table in [
            "'users'",
            "'accounts'",
            "'organizations'",
            "'account_devices'",
            "'sessions'",
            "'relationship_tuples'",
            "'licenses'",
            "'update_assignments'",
        ] {
            assert!(FOUNDATION_SQL.contains(table), "missing RLS table {table}");
        }
        assert!(FOUNDATION_SQL.contains("current_setting(''eitmad.tenant_id'', true)"));
    }
}
