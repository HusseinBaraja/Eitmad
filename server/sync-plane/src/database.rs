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
        let media_sql = include_str!("../migrations/0006_catalog_images.sql");
        let media_checksum = format!("{:x}", Sha256::digest(media_sql.as_bytes()));
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT checksum FROM public.eitmad_server_migrations WHERE version=6",
        )
        .fetch_optional(&mut *transaction)
        .await
        .map_err(SyncDatabaseError::Unavailable)?;
        match existing {
            Some(value) if value != media_checksum => {
                return Err(SyncDatabaseError::MigrationChecksum);
            }
            Some(_) => (),
            None => {
                sqlx::raw_sql(media_sql)
                    .execute(&mut *transaction)
                    .await
                    .map_err(SyncDatabaseError::Unavailable)?;
                sqlx::query("INSERT INTO public.eitmad_server_migrations(version,migration_id,checksum) VALUES(6,'server.catalog-images.v1',$1)")
                    .bind(media_checksum).execute(&mut *transaction).await.map_err(SyncDatabaseError::Unavailable)?;
            }
        }
        let pricing_sql = include_str!("../migrations/0007_pricing.sql");
        let pricing_checksum = format!("{:x}", Sha256::digest(pricing_sql.as_bytes()));
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT checksum FROM public.eitmad_server_migrations WHERE version=7",
        )
        .fetch_optional(&mut *transaction)
        .await
        .map_err(SyncDatabaseError::Unavailable)?;
        match existing {
            Some(value) if value != pricing_checksum => {
                return Err(SyncDatabaseError::MigrationChecksum);
            }
            Some(_) => (),
            None => {
                sqlx::raw_sql(pricing_sql)
                    .execute(&mut *transaction)
                    .await
                    .map_err(SyncDatabaseError::Unavailable)?;
                sqlx::query("INSERT INTO public.eitmad_server_migrations(version,migration_id,checksum) VALUES(7,'server.pricing.v1',$1)")
                    .bind(pricing_checksum).execute(&mut *transaction).await.map_err(SyncDatabaseError::Unavailable)?;
            }
        }
        apply_domain_migrations(&mut transaction).await?;
        transaction
            .commit()
            .await
            .map_err(SyncDatabaseError::Unavailable)
    }

    /// Wraps an already-connected pool, for composed or diagnostic hosts.
    #[must_use]
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }
}

/// Applies the immutable catalog migration within the existing migration lock.
async fn apply_catalog_migration(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), SyncDatabaseError> {
    let catalog_sql = include_str!("../migrations/0008_catalog_revisions.sql");
    let catalog_checksum = format!("{:x}", Sha256::digest(catalog_sql.as_bytes()));
    let existing: Option<String> =
        sqlx::query_scalar("SELECT checksum FROM public.eitmad_server_migrations WHERE version=8")
            .fetch_optional(&mut **transaction)
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
    match existing {
        Some(value) if value != catalog_checksum => {
            return Err(SyncDatabaseError::MigrationChecksum);
        }
        Some(_) => (),
        None => {
            sqlx::raw_sql(catalog_sql)
                .execute(&mut **transaction)
                .await
                .map_err(SyncDatabaseError::Unavailable)?;
            sqlx::query("INSERT INTO public.eitmad_server_migrations(version,migration_id,checksum) VALUES(8,'server.catalog-revisions.v1',$1)")
                    .bind(catalog_checksum).execute(&mut **transaction).await.map_err(SyncDatabaseError::Unavailable)?;
        }
    }
    Ok(())
}

/// Applies the catalog sync constraints without changing retained migration bytes.
async fn apply_synchronization_migration(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), SyncDatabaseError> {
    let sql = include_str!("../migrations/0009_catalog_synchronization.sql");
    let checksum = format!("{:x}", Sha256::digest(sql.as_bytes()));
    let existing: Option<String> =
        sqlx::query_scalar("SELECT checksum FROM public.eitmad_server_migrations WHERE version=9")
            .fetch_optional(&mut **transaction)
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
    match existing {
        Some(value) if value != checksum => return Err(SyncDatabaseError::MigrationChecksum),
        Some(_) => (),
        None => {
            sqlx::raw_sql(sql)
                .execute(&mut **transaction)
                .await
                .map_err(SyncDatabaseError::Unavailable)?;
            sqlx::query("INSERT INTO public.eitmad_server_migrations(version,migration_id,checksum) VALUES(9,'server.catalog-synchronization.v1',$1)").bind(checksum).execute(&mut **transaction).await.map_err(SyncDatabaseError::Unavailable)?;
        }
    }
    Ok(())
}

/// Adds indexed public image references and preserves readable pre-existing publications.
async fn apply_image_reference_migration(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), SyncDatabaseError> {
    let sql = include_str!("../migrations/0010_catalog_image_references.sql");
    let checksum = format!("{:x}", Sha256::digest(sql.as_bytes()));
    let existing: Option<String> =
        sqlx::query_scalar("SELECT checksum FROM public.eitmad_server_migrations WHERE version=10")
            .fetch_optional(&mut **transaction)
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
    match existing {
        Some(value) if value != checksum => return Err(SyncDatabaseError::MigrationChecksum),
        Some(_) => (),
        None => {
            sqlx::raw_sql(sql)
                .execute(&mut **transaction)
                .await
                .map_err(SyncDatabaseError::Unavailable)?;
            // Keyset pages keep migration memory bounded, including malformed retained entries.
            let tenants = sqlx::query_scalar::<_, uuid::Uuid>(
                "SELECT tenant_id FROM control.tenants ORDER BY tenant_id",
            )
            .fetch_all(&mut **transaction)
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
            for tenant_id in tenants {
                sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
                    .bind(tenant_id.to_string())
                    .execute(&mut **transaction)
                    .await
                    .map_err(SyncDatabaseError::Unavailable)?;
                let mut after: Option<(uuid::Uuid, String, uuid::Uuid, uuid::Uuid)> = None;
                loop {
                    let rows = sqlx::query("SELECT tenant_id,scope_kind,scope_id,record_id,change_json FROM sync.records WHERE schema_id='eitmad.schema.catalog-public.v1' AND NOT tombstone AND ($1::uuid IS NULL OR (tenant_id,scope_kind,scope_id,record_id)>($1,$2,$3,$4)) ORDER BY tenant_id,scope_kind,scope_id,record_id LIMIT 100")
                    .bind(after.as_ref().map(|v| v.0)).bind(after.as_ref().map(|v| v.1.as_str()))
                    .bind(after.as_ref().map(|v| v.2)).bind(after.as_ref().map(|v| v.3))
                    .fetch_all(&mut **transaction).await.map_err(SyncDatabaseError::Unavailable)?;
                    if rows.is_empty() {
                        break;
                    }
                    for row in rows {
                        let tenant: uuid::Uuid = row.get("tenant_id");
                        let kind: String = row.get("scope_kind");
                        let scope: uuid::Uuid = row.get("scope_id");
                        let id: uuid::Uuid = row.get("record_id");
                        after = Some((tenant, kind.clone(), scope, id));
                        let Ok(change) = serde_json::from_value::<
                            eitmad_contracts::sync::ChangeRecord,
                        >(row.get("change_json")) else {
                            continue;
                        };
                        if change.scope.kind.as_str() != kind
                            || change.scope.id.value() != scope
                            || change.record_id.value() != id
                        {
                            continue;
                        }
                        if let Some((image, digest)) =
                            crate::catalog_sync::public_image_reference(&change)
                        {
                            sqlx::query("UPDATE sync.records SET public_image_id=$5,public_image_sha256=$6 WHERE tenant_id=$1 AND scope_kind=$2 AND scope_id=$3 AND record_id=$4 AND schema_id='eitmad.schema.catalog-public.v1'")
                            .bind(tenant).bind(kind).bind(scope).bind(id).bind(image).bind(digest)
                            .execute(&mut **transaction).await.map_err(SyncDatabaseError::Unavailable)?;
                        }
                    }
                }
            }
            sqlx::query("INSERT INTO public.eitmad_server_migrations(version,migration_id,checksum) VALUES(10,'server.catalog-image-references.v1',$1)")
                .bind(checksum).execute(&mut **transaction).await.map_err(SyncDatabaseError::Unavailable)?;
        }
    }
    Ok(())
}

async fn apply_quotation_draft_migration(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), SyncDatabaseError> {
    let sql = include_str!("../migrations/0011_quotation_drafts.sql");
    let checksum = format!("{:x}", Sha256::digest(sql.as_bytes()));
    let existing: Option<String> =
        sqlx::query_scalar("SELECT checksum FROM public.eitmad_server_migrations WHERE version=11")
            .fetch_optional(&mut **tx)
            .await
            .map_err(SyncDatabaseError::Unavailable)?;
    match existing {
        Some(value) if value != checksum => return Err(SyncDatabaseError::MigrationChecksum),
        Some(_) => (),
        None => {
            sqlx::raw_sql(sql)
                .execute(&mut **tx)
                .await
                .map_err(SyncDatabaseError::Unavailable)?;
            sqlx::query("INSERT INTO public.eitmad_server_migrations(version,migration_id,checksum) VALUES(11,'server.quotation-drafts.v1',$1)").bind(checksum).execute(&mut **tx).await.map_err(SyncDatabaseError::Unavailable)?;
        }
    }
    Ok(())
}

async fn apply_domain_migrations(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), SyncDatabaseError> {
    apply_catalog_migration(tx).await?;
    apply_synchronization_migration(tx).await?;
    apply_image_reference_migration(tx).await?;
    apply_quotation_draft_migration(tx).await
}
