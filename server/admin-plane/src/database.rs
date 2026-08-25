use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _, postgres::PgPoolOptions};

const FOUNDATION_SQL: &str = include_str!("../migrations/0003_admin_foundation.sql");

#[derive(Clone)]
pub struct AdminDatabase {
    pool: PgPool,
}

#[derive(Debug, thiserror::Error)]
pub enum AdminDatabaseError {
    #[error("admin database is unavailable")]
    Unavailable(#[source] sqlx::Error),
    #[error("control and sync migrations are required before the admin migration")]
    MissingPrerequisites,
    #[error("admin database migration checksum changed")]
    MigrationChecksum,
}

impl AdminDatabase {
    /// Connects to the `PostgreSQL` administration authority.
    ///
    /// # Errors
    ///
    /// Returns a sanitized connection error.
    pub async fn connect(
        database_url: &str,
        maximum_connections: u32,
    ) -> Result<Self, AdminDatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(maximum_connections)
            .connect(database_url)
            .await
            .map_err(AdminDatabaseError::Unavailable)?;
        Ok(Self { pool })
    }

    /// Applies the checksummed administration migration after control and sync.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable prerequisites or changed history.
    pub async fn migrate(&self) -> Result<(), AdminDatabaseError> {
        let checksum = format!("{:x}", Sha256::digest(FOUNDATION_SQL.as_bytes()));
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(AdminDatabaseError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock(1163158101)")
            .execute(&mut *transaction)
            .await
            .map_err(AdminDatabaseError::Unavailable)?;
        let prerequisites: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM public.eitmad_server_migrations WHERE version IN (1, 2)",
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(AdminDatabaseError::Unavailable)?;
        if prerequisites != 2 {
            return Err(AdminDatabaseError::MissingPrerequisites);
        }
        let existing =
            sqlx::query("SELECT checksum FROM public.eitmad_server_migrations WHERE version = 3")
                .fetch_optional(&mut *transaction)
                .await
                .map_err(AdminDatabaseError::Unavailable)?;
        if let Some(existing) = existing {
            if existing.get::<String, _>("checksum") != checksum {
                return Err(AdminDatabaseError::MigrationChecksum);
            }
        } else {
            sqlx::raw_sql(FOUNDATION_SQL)
                .execute(&mut *transaction)
                .await
                .map_err(AdminDatabaseError::Unavailable)?;
            sqlx::query(
                "INSERT INTO public.eitmad_server_migrations
                    (version, migration_id, checksum)
                 VALUES (3, 'server.admin-foundation.v1', $1)",
            )
            .bind(checksum)
            .execute(&mut *transaction)
            .await
            .map_err(AdminDatabaseError::Unavailable)?;
        }
        transaction
            .commit()
            .await
            .map_err(AdminDatabaseError::Unavailable)
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
    fn migration_persists_backup_and_support_status_with_forced_isolation() {
        for table in ["operations.backup_status", "operations.support_workflows"] {
            assert!(FOUNDATION_SQL.contains(table));
        }
        assert!(FOUNDATION_SQL.contains("FORCE ROW LEVEL SECURITY"));
        assert!(FOUNDATION_SQL.contains("current_setting(''eitmad.tenant_id'', true)"));
    }
}
