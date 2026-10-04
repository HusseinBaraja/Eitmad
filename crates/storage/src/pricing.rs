//! Scoped, immutable confirmed prices and durable unresolved publication intents.
use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, FurnitureTransaction,
    ProductTransaction, StorageError, insert_audit, insert_idempotency, insert_publication,
    load_idempotency, migrations::Migration, scope_parts,
};
use eitmad_contracts::{
    identity::ScopeRef,
    pricing::{ConfirmPrice, PriceTarget, PublishedPrice},
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(21, "pricing.confirmed-prices.v1", "pricing",
    "CREATE TABLE pricing_revisions (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, kind TEXT NOT NULL, entry_id TEXT NOT NULL, variant_id TEXT NOT NULL,
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,kind,entry_id,variant_id,revision));
     CREATE TRIGGER pricing_revision_no_update BEFORE UPDATE ON pricing_revisions BEGIN SELECT RAISE(ABORT,'immutable price'); END;
     CREATE TRIGGER pricing_revision_no_delete BEFORE DELETE ON pricing_revisions BEGIN SELECT RAISE(ABORT,'immutable price'); END;
     CREATE TABLE pricing_intents (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, request_hash BLOB NOT NULL, command_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,request_hash));")];

pub struct PricingTransaction<'a> {
    connection: &'a rusqlite::Connection,
}
impl AuthorityStore {
    /// Keeps catalog validation, price state, audit, retries, and events atomic.
    /// # Errors
    /// Rolls back on any failed mandatory write.
    pub fn transact_pricing<T, E: From<StorageError>>(
        &self,
        write: bool,
        operation: impl FnOnce(&PricingTransaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut connection = self.open_connection()?;
        let tx = connection
            .transaction_with_behavior(if write {
                rusqlite::TransactionBehavior::Immediate
            } else {
                rusqlite::TransactionBehavior::Deferred
            })
            .map_err(|_| StorageError)?;
        let result = operation(&PricingTransaction { connection: &tx })?;
        tx.commit().map_err(|_| StorageError)?;
        Ok(result)
    }
}
impl PricingTransaction<'_> {
    /// Stores a newer authenticated server cache revision with mutation audit.
    /// # Errors
    /// Rejects conflicting history or failed audit writes.
    pub fn cache(
        &self,
        price: &PublishedPrice,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        self.insert_price(price)?;
        insert_audit(self.connection, audit)
    }
    fn insert_price(&self, price: &PublishedPrice) -> Result<(), StorageError> {
        let (scope_kind, scope_id) = scope_parts(price.target.scope());
        let (kind, entry, variant) = price.target.identity();
        self.connection
            .execute(
                "INSERT INTO pricing_revisions VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    scope_kind,
                    scope_id,
                    kind,
                    entry.to_string(),
                    variant.to_string(),
                    i64::try_from(price.revision).map_err(|_| StorageError)?,
                    serde_json::to_vec(price).map_err(|_| StorageError)?
                ],
            )
            .map_err(|_| StorageError)?;
        Ok(())
    }
    #[must_use]
    pub const fn products(&self) -> ProductTransaction<'_> {
        ProductTransaction {
            connection: self.connection,
        }
    }
    #[must_use]
    pub const fn furnitures(&self) -> FurnitureTransaction<'_> {
        FurnitureTransaction {
            connection: self.connection,
        }
    }
    /// Returns a bounded stable combined cursor without merging catalog records.
    /// # Errors
    /// Rejects unavailable storage.
    pub fn catalog_ids(
        &self,
        scope: &ScopeRef,
        after: &str,
        limit: u32,
    ) -> Result<Vec<(String, uuid::Uuid)>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let mut stmt=self.connection.prepare("SELECT kind,id FROM (SELECT 'product' AS kind,id FROM products WHERE scope_kind=?1 AND scope_id=?2 UNION ALL SELECT 'furniture',id FROM furnitures WHERE scope_kind=?1 AND scope_id=?2) WHERE kind || ':' || id >= ?3 ORDER BY kind,id LIMIT ?4").map_err(|_| StorageError)?;
        stmt.query_map(params![kind, id, after, i64::from(limit)], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|_| StorageError)?
        .map(|r| {
            let (kind, id) = r.map_err(|_| StorageError)?;
            Ok((kind, uuid::Uuid::parse_str(&id).map_err(|_| StorageError)?))
        })
        .collect()
    }
    /// Loads the latest immutable revision for a stable variant identity.
    /// # Errors
    /// Rejects invalid durable data.
    pub fn latest(&self, target: &PriceTarget) -> Result<Option<PublishedPrice>, StorageError> {
        let (scope_kind, scope_id) = scope_parts(target.scope());
        let (kind, entry, variant) = target.identity();
        let data=self.connection.query_row("SELECT record_json FROM pricing_revisions WHERE scope_kind=?1 AND scope_id=?2 AND kind=?3 AND entry_id=?4 AND variant_id=?5 ORDER BY revision DESC LIMIT 1",params![scope_kind,scope_id,kind,entry.to_string(),variant.to_string()],|r|r.get::<_,Vec<u8>>(0)).optional().map_err(|_|StorageError)?;
        data.map(|v| serde_json::from_slice(&v).map_err(|_| StorageError))
            .transpose()
    }
    /// Loads an unresolved intent by actor-bound exact request hash.
    /// # Errors
    /// Rejects invalid durable data.
    pub fn intent(
        &self,
        scope: &ScopeRef,
        hash: &[u8; 32],
    ) -> Result<Option<ConfirmPrice>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let bytes=self.connection.query_row("SELECT command_json FROM pricing_intents WHERE scope_kind=?1 AND scope_id=?2 AND request_hash=?3",params![kind,id,hash],|r|r.get::<_,Vec<u8>>(0)).optional().map_err(|_|StorageError)?;
        bytes
            .map(|v| serde_json::from_slice(&v).map_err(|_| StorageError))
            .transpose()
    }
    /// Saves an immutable intent before contacting the server, with redacted audit.
    /// # Errors
    /// Rejects failed mandatory writes.
    pub fn prepare(
        &self,
        request: &ConfirmPrice,
        hash: &[u8; 32],
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        let (kind, id) = scope_parts(request.command.target.scope());
        self.connection
            .execute(
                "INSERT INTO pricing_intents VALUES(?1,?2,?3,?4)",
                params![
                    kind,
                    id,
                    hash,
                    serde_json::to_vec(request).map_err(|_| StorageError)?
                ],
            )
            .map_err(|_| StorageError)?;
        insert_audit(self.connection, audit)
    }
    /// Removes a resolved intent; keeps it for unknown outcomes.
    /// # Errors
    /// Rejects unavailable storage.
    pub fn resolve(&self, scope: &ScopeRef, hash: &[u8; 32]) -> Result<(), StorageError> {
        let (kind, id) = scope_parts(scope);
        self.connection.execute("DELETE FROM pricing_intents WHERE scope_kind=?1 AND scope_id=?2 AND request_hash=?3",params![kind,id,hash]).map_err(|_|StorageError)?;
        Ok(())
    }
    /// Reads the actor-bound original result for a completed retry.
    /// # Errors
    /// Rejects invalid durable data.
    pub fn replay(
        &self,
        scope: &ScopeRef,
        retry: &DurableIdempotency,
    ) -> Result<Option<DurableIdempotency>, StorageError> {
        Ok(load_idempotency(self.connection, scope, retry.key)?.map(
            |(request_hash, response_json)| DurableIdempotency {
                key: retry.key,
                request_hash,
                response_json,
            },
        ))
    }
    /// Writes only redacted rejection evidence.
    /// # Errors
    /// Rejects failed audit writes.
    pub fn audit(&self, record: &MutationAuditRecord) -> Result<(), StorageError> {
        insert_audit(self.connection, record)
    }
    /// Commits a server receipt, audit, retry, and public invalidation notice atomically.
    /// # Errors
    /// Rejects unavailable storage and conflicting immutable revisions.
    pub fn persist(
        &self,
        price: &PublishedPrice,
        audit: &MutationAuditRecord,
        retry: &DurableIdempotency,
        publication: &DurablePublication,
    ) -> Result<(), StorageError> {
        let json = serde_json::to_vec(price).map_err(|_| StorageError)?;
        let (scope_kind, scope_id) = scope_parts(price.target.scope());
        let (kind, entry, variant) = price.target.identity();
        let retained = self.connection.query_row("SELECT record_json FROM pricing_revisions WHERE scope_kind=?1 AND scope_id=?2 AND kind=?3 AND entry_id=?4 AND variant_id=?5 AND revision=?6", params![scope_kind, scope_id, kind, entry.to_string(), variant.to_string(), i64::try_from(price.revision).map_err(|_| StorageError)?], |r| r.get::<_, Vec<u8>>(0)).optional().map_err(|_| StorageError)?;
        if let Some(retained) = retained {
            if retained != json {
                return Err(StorageError);
            }
        } else {
            self.insert_price(price)?;
        }
        insert_audit(self.connection, audit)?;
        let mut retry = retry.clone();
        retry.response_json = json;
        insert_idempotency(
            self.connection,
            price.target.scope(),
            "eitmad.pricing.publish.v1",
            &retry,
        )?;
        insert_publication(
            self.connection,
            price.target.scope(),
            retry.key,
            publication,
        )
    }
}
