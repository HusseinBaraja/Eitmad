//! Immutable scoped image blobs, committed with redacted mutation audit.
use crate::{
    AuthorityStore, DurableIdempotency, StorageError, insert_audit, insert_idempotency,
    load_idempotency, migrations::Migration, scope_parts,
};
use eitmad_contracts::{catalog_image::CatalogImageRef, identity::ScopeRef};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(19, "catalog.images.v1", "catalog-image",
    "CREATE TABLE catalog_images (
       scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL,
       kind TEXT NOT NULL, sha256 TEXT NOT NULL, content BLOB NOT NULL CHECK(length(content) BETWEEN 1 AND 8388608),
       PRIMARY KEY(scope_kind,scope_id,id));
     CREATE TRIGGER catalog_image_no_update BEFORE UPDATE ON catalog_images BEGIN SELECT RAISE(ABORT,'immutable image'); END;
     CREATE TRIGGER catalog_image_no_delete BEFORE DELETE ON catalog_images BEGIN SELECT RAISE(ABORT,'retained image'); END;
     CREATE TABLE catalog_image_uploads(scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL, actor_json BLOB NOT NULL, reference_json BLOB NOT NULL, PRIMARY KEY(scope_kind,scope_id,id));"
)];

impl AuthorityStore {
    /// Reads retry evidence without storing the source path or content in the retry response.
    /// # Errors
    /// Returns only a sanitized storage error.
    pub fn catalog_image_retry(
        &self,
        scope: &ScopeRef,
        key: eitmad_contracts::transport::IdempotencyKey,
    ) -> Result<Option<DurableIdempotency>, StorageError> {
        load_idempotency(&self.open_connection()?, scope, key).map(|v| {
            v.map(|(request_hash, response_json)| DurableIdempotency {
                key,
                request_hash,
                response_json,
            })
        })
    }
    /// Reads one exact scoped reference. No caller-controlled filesystem path is used.
    /// # Errors
    /// Returns only a sanitized storage error.
    pub fn catalog_image(
        &self,
        scope: &ScopeRef,
        image: &CatalogImageRef,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let (kind, scope_id) = scope_parts(scope);
        self.open_connection()?.query_row(
            "SELECT content FROM catalog_images WHERE scope_kind=?1 AND scope_id=?2 AND id=?3 AND kind=?4 AND sha256=?5",
            params![kind, scope_id, image.id.to_string(), format!("{:?}", image.kind), image.sha256], |r| r.get(0)
        ).optional().map_err(|_| StorageError)
    }

    /// Atomically retains a validated immutable image and its audit result. Exact imports deduplicate.
    /// # Errors
    /// Rolls back when storage or mandatory audit fails.
    pub fn insert_catalog_image(
        &self,
        scope: &ScopeRef,
        image: &CatalogImageRef,
        content: &[u8],
        audit: &MutationAuditRecord,
        retry: Option<&DurableIdempotency>,
        upload_actor: Option<&eitmad_contracts::identity::AuthorizationContext>,
    ) -> Result<(), StorageError> {
        self.write_transaction(|tx| {
            if let Some(retry) = retry {
                if let Some((hash,response))=load_idempotency(tx,scope,retry.key)? {
                    return if hash==retry.request_hash && response==retry.response_json { Ok(()) } else { Err(StorageError) };
                }
            }
            let (kind, scope_id) = scope_parts(scope);
            let changed = tx.execute("INSERT OR IGNORE INTO catalog_images(scope_kind,scope_id,id,kind,sha256,content) VALUES(?1,?2,?3,?4,?5,?6)", params![kind, scope_id, image.id.to_string(), format!("{:?}", image.kind), image.sha256, content]).map_err(|_| StorageError)?;
            if changed > 0 || retry.is_some() { insert_audit(tx, audit)?; }
            if let Some(actor)=upload_actor {
                tx.execute("INSERT OR IGNORE INTO catalog_image_uploads(scope_kind,scope_id,id,actor_json,reference_json) VALUES(?1,?2,?3,?4,?5)",params![kind,scope_id,image.id.to_string(),serde_json::to_vec(actor).map_err(|_|StorageError)?,serde_json::to_vec(image).map_err(|_|StorageError)?]).map_err(|_|StorageError)?;
            }
            if let Some(retry)=retry { insert_idempotency(tx,scope,"eitmad.catalog-image.import.v1",retry)?; }
            Ok(())
        })
    }
    /// Reads at most sixteen pending upload references without reading their image bodies.
    /// # Errors
    /// Returns a sanitized storage failure.
    pub fn pending_catalog_images(
        &self,
    ) -> Result<
        Vec<(
            eitmad_contracts::identity::AuthorizationContext,
            CatalogImageRef,
        )>,
        StorageError,
    > {
        let connection = self.open_connection()?;
        let mut statement=connection.prepare("SELECT actor_json,reference_json FROM catalog_image_uploads ORDER BY scope_kind,scope_id,id LIMIT 16").map_err(|_|StorageError)?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .map_err(|_| StorageError)?
            .map(|row| {
                let (actor, image) = row.map_err(|_| StorageError)?;
                Ok((
                    serde_json::from_slice(&actor).map_err(|_| StorageError)?,
                    serde_json::from_slice(&image).map_err(|_| StorageError)?,
                ))
            })
            .collect()
    }
    /// Removes pending work only after an exact authenticated acknowledgement.
    /// # Errors
    /// Returns a sanitized storage failure.
    pub fn acknowledge_catalog_image(
        &self,
        scope: &ScopeRef,
        image: &CatalogImageRef,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        let (kind, scope_id) = scope_parts(scope);
        self.write_transaction(|tx| {
            let changed = tx.execute(
                "DELETE FROM catalog_image_uploads WHERE scope_kind=?1 AND scope_id=?2 AND id=?3",
                params![kind, scope_id, image.id.to_string()],
            )
            .map_err(|_| StorageError)?;
            if changed > 0 {
                insert_audit(tx, audit)?;
            }
            Ok(())
        })
    }
}
