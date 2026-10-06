//! Durable catalog transfer work and atomic, scoped read-model projection.
use crate::{
    AuthorityStore, DurablePublication, StorageError, insert_audit, insert_publication,
    migrations::Migration, scope_parts,
};
use eitmad_contracts::{
    catalog_revision::{CatalogEntry, CatalogRevision, CatalogSyncIssue},
    identity::{AuthenticatedIdentity, AuthorizationContext, ScopeRef},
    sync::{ChangeOperation, ChangeRecord, Checkpoint, LocalChangeDisposition},
    transport::SchemaId,
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};
use uuid::Uuid;

/// Fully decoded role-filtered changes and the owning capability's Arabic name normalizer.
pub struct CatalogSyncProjection<'a> {
    pub private: &'a [CatalogRevision],
    pub public: &'a [(ChangeRecord, Option<CatalogEntry>)],
    pub normalize_name: fn(&str) -> String,
}
pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(22,"catalog.synchronization.v1","catalog",
    "CREATE TABLE catalog_sync_revisions(scope_kind TEXT NOT NULL,scope_id TEXT NOT NULL,kind TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,record_json BLOB NOT NULL,actor_json BLOB,pending INTEGER NOT NULL CHECK(pending IN(0,1)),PRIMARY KEY(scope_kind,scope_id,kind,id,revision));
     CREATE TRIGGER catalog_sync_revision_no_update BEFORE UPDATE OF record_json ON catalog_sync_revisions BEGIN SELECT RAISE(ABORT,'immutable catalog revision'); END;
     CREATE TRIGGER catalog_sync_revision_no_delete BEFORE DELETE ON catalog_sync_revisions BEGIN SELECT RAISE(ABORT,'retained catalog revision'); END;
     CREATE TABLE catalog_sync_checkpoints(scope_kind TEXT NOT NULL,scope_id TEXT NOT NULL,principal_id TEXT NOT NULL,schema_id TEXT NOT NULL,checkpoint TEXT NOT NULL,PRIMARY KEY(scope_kind,scope_id,principal_id,schema_id));
     CREATE TABLE catalog_sales_records(scope_kind TEXT NOT NULL,scope_id TEXT NOT NULL,id TEXT NOT NULL,revision INTEGER NOT NULL,tombstone INTEGER NOT NULL,record_json BLOB,PRIMARY KEY(scope_kind,scope_id,id));
     CREATE TABLE catalog_sync_clients(scope_kind TEXT NOT NULL,scope_id TEXT NOT NULL,principal_id TEXT NOT NULL,actor_json BLOB NOT NULL,bootstrapped INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(scope_kind,scope_id,principal_id));"),
    Migration::new(23, "catalog.sync-exceptions.v1", "catalog",
        "CREATE TABLE catalog_sync_exceptions (
         scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, kind TEXT NOT NULL, id TEXT NOT NULL,
         revision INTEGER NOT NULL, disposition_json BLOB NOT NULL,
         PRIMARY KEY(scope_kind,scope_id,kind,id,revision),
         FOREIGN KEY(scope_kind,scope_id,kind,id,revision)
         REFERENCES catalog_sync_revisions(scope_kind,scope_id,kind,id,revision));")];
/// Retains immutable transfer work and submitting authority inside the owning save transaction.
pub(crate) fn enqueue(
    tx: &rusqlite::Connection,
    record: &CatalogRevision,
    audit: &MutationAuditRecord,
) -> Result<(), StorageError> {
    let actor = AuthorizationContext {
        session_id: audit.session_id,
        identity: AuthenticatedIdentity {
            principal_id: audit.principal_id,
            principal_kind: audit.principal_kind,
            device_id: audit.device_id,
            service_id: None,
        },
        tenant_id: audit.tenant_id,
        workspace_id: audit.workspace_id,
        scope: audit.scope.clone(),
    };
    retain(tx, record, Some(&actor))
}
/// Accepts exact typed retries and prevents replacement of one immutable catalog identity.
fn retain(
    tx: &rusqlite::Connection,
    record: &CatalogRevision,
    actor: Option<&AuthorizationContext>,
) -> Result<(), StorageError> {
    let (kind, id, revision, scope) = record.identity();
    let (scope_kind, scope_id) = scope_parts(scope);
    let revision = i64::try_from(revision).map_err(|_| StorageError)?;
    let json = serde_json::to_vec(record).map_err(|_| StorageError)?;
    let existing: Option<Vec<u8>> = tx.query_row("SELECT record_json FROM catalog_sync_revisions WHERE scope_kind=?1 AND scope_id=?2 AND kind=?3 AND id=?4 AND revision=?5",params![scope_kind,scope_id,kind,id.to_string(),revision],|r|r.get(0)).optional().map_err(|_| StorageError)?;
    if let Some(existing) = existing {
        return if serde_json::from_slice::<CatalogRevision>(&existing).map_err(|_| StorageError)?
            == *record
        {
            Ok(())
        } else {
            Err(StorageError)
        };
    }
    tx.execute(
        "INSERT INTO catalog_sync_revisions VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            scope_kind,
            scope_id,
            kind,
            id.to_string(),
            revision,
            json,
            actor
                .map(serde_json::to_vec)
                .transpose()
                .map_err(|_| StorageError)?,
            actor.is_some()
        ],
    )
    .map_err(|_| StorageError)?;
    Ok(())
}
impl AuthorityStore {
    /// Registers an authorized reader for restart-safe background refresh.
    /// # Errors
    /// Rejects failed registration or audit writes.
    pub fn register_catalog_client(
        &self,
        actor: &AuthorizationContext,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        let (sk, si) = scope_parts(&actor.scope);
        let json = serde_json::to_vec(actor).map_err(|_| StorageError)?;
        self.write_transaction(|tx| {
            let old:Option<Vec<u8>>=tx.query_row("SELECT actor_json FROM catalog_sync_clients WHERE scope_kind=?1 AND scope_id=?2 AND principal_id=?3",params![sk,si,actor.identity.principal_id.value().to_string()],|r|r.get(0)).optional().map_err(|_|StorageError)?;
            if old.as_ref()==Some(&json) {return Ok(());}
            tx.execute("INSERT INTO catalog_sync_clients(scope_kind,scope_id,principal_id,actor_json) VALUES(?1,?2,?3,?4) ON CONFLICT(scope_kind,scope_id,principal_id) DO UPDATE SET actor_json=excluded.actor_json",params![sk,si,actor.identity.principal_id.value().to_string(),json]).map_err(|_|StorageError)?; insert_audit(tx,audit)
        })
    }
    /// Captures pre-existing current definitions and immutable history, including historical cost inputs.
    /// # Errors
    /// Fails atomically for inconsistent snapshots or audit failure.
    pub fn seed_catalog_sync(
        &self,
        actor: &AuthorizationContext,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        self.write_transaction(|tx| {
            let (kind,id) = scope_parts(&actor.scope);
            let seeded:bool=tx.query_row("SELECT bootstrapped FROM catalog_sync_clients WHERE scope_kind=?1 AND scope_id=?2 AND principal_id=?3",params![kind,id,actor.identity.principal_id.value().to_string()],|r|r.get(0)).optional().map_err(|_|StorageError)?.unwrap_or(false);
            if seeded {return Ok(());}
            for (table,tag) in [("material_categories","materialCategory"),("material_units","unit"),("materials","material"),("part_categories","partCategory"),("part_compositions","part"),("product_categories","productCategory"),("product_revisions","product"),("furniture_categories","furnitureCategory"),("furniture_revisions","furniture")] {
                let mut stmt=tx.prepare(&format!("SELECT record_json FROM {table} WHERE scope_kind=?1 AND scope_id=?2")).map_err(|_| StorageError)?;
                let rows=stmt.query_map(params![kind,id],|r|r.get::<_,Vec<u8>>(0)).map_err(|_| StorageError)?;
                for row in rows {
                    let payload: serde_json::Value=serde_json::from_slice(&row.map_err(|_|StorageError)?).map_err(|_|StorageError)?;
                    let record: CatalogRevision=serde_json::from_value(serde_json::json!({"kind":tag,"payload":payload})).map_err(|_|StorageError)?;
                    if let CatalogRevision::Part(p)=&record {
                        for row in &p.cost.rows { for dep in [CatalogRevision::Unit(Box::new(row.unit.clone())),CatalogRevision::Unit(Box::new(row.cost_unit.clone())),CatalogRevision::Material(Box::new(row.material.clone()))] { enqueue(tx,&dep,audit)?; } }
                    }
                    enqueue(tx,&record,audit)?;
                }
            }
            tx.execute("UPDATE catalog_sync_clients SET bootstrapped=1 WHERE scope_kind=?1 AND scope_id=?2 AND principal_id=?3",params![kind,id,actor.identity.principal_id.value().to_string()]).map_err(|_|StorageError)?;
            insert_audit(tx,audit)
        })
    }
    /// Returns a dependency-ordered bounded page. Exact immutable retries use their retained identity.
    /// # Errors
    /// Rejects invalid durable work.
    pub fn pending_catalog_revisions(
        &self,
        scope: &ScopeRef,
        after: Option<&CatalogRevision>,
    ) -> Result<Vec<CatalogRevision>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let connection = self.open_connection()?;
        let (after_order, after_id, after_revision) = after.map_or((-1, String::new(), 0), |r| {
            let (kind, id, revision, _) = r.identity();
            (transfer_order(kind), id.to_string(), revision)
        });
        let mut stmt=connection.prepare("SELECT record_json FROM catalog_sync_revisions WHERE scope_kind=?1 AND scope_id=?2 AND pending=1 AND (CASE kind WHEN 'material-category' THEN 0 WHEN 'unit' THEN 1 WHEN 'material' THEN 2 WHEN 'part-category' THEN 3 WHEN 'part' THEN 4 WHEN 'product-category' THEN 5 WHEN 'product' THEN 6 WHEN 'furniture-category' THEN 7 ELSE 8 END,id,revision) > (?3,?4,?5) ORDER BY CASE kind WHEN 'material-category' THEN 0 WHEN 'unit' THEN 1 WHEN 'material' THEN 2 WHEN 'part-category' THEN 3 WHEN 'part' THEN 4 WHEN 'product-category' THEN 5 WHEN 'product' THEN 6 WHEN 'furniture-category' THEN 7 ELSE 8 END,id,revision LIMIT 50").map_err(|_|StorageError)?;
        stmt.query_map(
            params![
                kind,
                id,
                after_order,
                after_id,
                i64::try_from(after_revision).map_err(|_| StorageError)?
            ],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| StorageError)?
        .map(|r| serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError))
        .collect()
    }
    /// Retains the submitting identity only inside Rust storage for restart recovery.
    /// # Errors
    /// Rejects invalid durable work.
    pub fn pending_catalog_actors(&self) -> Result<Vec<AuthorizationContext>, StorageError> {
        let connection = self.open_connection()?;
        let mut stmt=connection.prepare("SELECT actor_json FROM catalog_sync_clients UNION SELECT actor_json FROM catalog_sync_revisions WHERE pending=1 AND actor_json IS NOT NULL LIMIT 16").map_err(|_|StorageError)?;
        stmt.query_map([], |r| r.get::<_, Vec<u8>>(0))
            .map_err(|_| StorageError)?
            .map(|r| {
                serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError)
            })
            .collect()
    }
    /// Removes only accepted transfer work in the same transaction as its redacted audit.
    /// # Errors
    /// Preserves pending work if mandatory evidence fails.
    pub fn acknowledge_catalog_revision(
        &self,
        record: &CatalogRevision,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        let (kind, id, revision, scope) = record.identity();
        let (sk, si) = scope_parts(scope);
        let revision = i64::try_from(revision).map_err(|_| StorageError)?;
        self.write_transaction(|tx| { tx.execute("UPDATE catalog_sync_revisions SET pending=0 WHERE scope_kind=?1 AND scope_id=?2 AND kind=?3 AND id=?4 AND revision=?5",params![sk,si,kind,id.to_string(),revision]).map_err(|_|StorageError)?; insert_audit(tx,audit) })
    }

    /// Quarantines a terminal server result atomically with its audit, without deleting the revision.
    /// # Errors
    /// Leaves work pending if the rejection or mandatory audit cannot be retained.
    pub fn reject_catalog_revision(
        &self,
        record: &CatalogRevision,
        disposition: &LocalChangeDisposition,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        if !matches!(
            disposition,
            LocalChangeDisposition::Rejected { .. } | LocalChangeDisposition::Conflicted { .. }
        ) {
            return Err(StorageError);
        }
        let (kind, id, revision, scope) = record.identity();
        if &audit.scope != scope {
            return Err(StorageError);
        }
        let (sk, si) = scope_parts(scope);
        let revision = i64::try_from(revision).map_err(|_| StorageError)?;
        let json = serde_json::to_vec(disposition).map_err(|_| StorageError)?;
        self.write_transaction(|tx| {
            tx.execute("INSERT INTO catalog_sync_exceptions VALUES(?1,?2,?3,?4,?5,?6)",
                params![sk, si, kind, id.to_string(), revision, json]).map_err(|_| StorageError)?;
            let changed = tx.execute("UPDATE catalog_sync_revisions SET pending=0 WHERE scope_kind=?1 AND scope_id=?2 AND kind=?3 AND id=?4 AND revision=?5 AND pending=1",
                params![sk, si, kind, id.to_string(), revision]).map_err(|_| StorageError)?;
            if changed != 1 { return Err(StorageError); }
            insert_audit(tx, audit)
        })
    }

    /// Checks accepted dependencies. Revision zero means an accepted current category or unit.
    /// # Errors
    /// Fails closed if retained transfer state is unavailable.
    pub fn catalog_dependencies_ready(
        &self,
        scope: &ScopeRef,
        dependencies: &[(&str, uuid::Uuid, u64)],
    ) -> Result<bool, StorageError> {
        let (sk, si) = scope_parts(scope);
        self.read_transaction(|tx| {
            for (kind, id, revision) in dependencies {
                let accepted: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM catalog_sync_revisions r WHERE r.scope_kind=?1 AND r.scope_id=?2 AND r.kind=?3 AND r.id=?4 AND (?5=0 OR r.revision=?5) AND r.pending=0 AND NOT EXISTS(SELECT 1 FROM catalog_sync_exceptions e WHERE e.scope_kind=r.scope_kind AND e.scope_id=r.scope_id AND e.kind=r.kind AND e.id=r.id AND e.revision=r.revision))",
                    params![sk, si, kind, id.to_string(), i64::try_from(*revision).map_err(|_| StorageError)?], |r| r.get(0)).map_err(|_| StorageError)?;
                if !accepted { return Ok(false); }
            }
            Ok(true)
        })
    }

    /// Reports unresolved roots without exposing private payloads or conflict responses.
    /// # Errors
    /// Rejects invalid retained transfer state. The caller must authorize Manager access.
    pub fn catalog_sync_issues(
        &self,
        scope: &ScopeRef,
    ) -> Result<Vec<CatalogSyncIssue>, StorageError> {
        let (sk, si) = scope_parts(scope);
        self.read_transaction(|tx| {
            let mut stmt = tx.prepare("SELECT r.record_json,e.disposition_json FROM catalog_sync_revisions r JOIN catalog_sync_exceptions e USING(scope_kind,scope_id,kind,id,revision) WHERE r.scope_kind=?1 AND r.scope_id=?2 AND NOT EXISTS(SELECT 1 FROM catalog_sync_revisions newer WHERE newer.scope_kind=r.scope_kind AND newer.scope_id=r.scope_id AND newer.kind=r.kind AND newer.id=r.id AND newer.revision>r.revision AND newer.pending=0 AND NOT EXISTS(SELECT 1 FROM catalog_sync_exceptions ne WHERE ne.scope_kind=newer.scope_kind AND ne.scope_id=newer.scope_id AND ne.kind=newer.kind AND ne.id=newer.id AND ne.revision=newer.revision)) ORDER BY r.kind,r.id,r.revision LIMIT 50").map_err(|_| StorageError)?;
            stmt.query_map(params![sk, si], |r| Ok((r.get::<_,Vec<u8>>(0)?,r.get::<_,Vec<u8>>(1)?))).map_err(|_| StorageError)?
                .map(|row| {
                    let (record, disposition) = row.map_err(|_| StorageError)?;
                    let record: CatalogRevision = serde_json::from_slice(&record).map_err(|_| StorageError)?;
                    let disposition: LocalChangeDisposition = serde_json::from_slice(&disposition).map_err(|_| StorageError)?;
                    let (kind, id, revision, _) = record.identity();
                    let name = serde_json::to_value(&record).map_err(|_| StorageError)?["payload"]["name"].as_str().ok_or(StorageError)?.to_owned();
                    Ok(CatalogSyncIssue { kind: kind.into(), id, revision, name, conflicted: matches!(disposition, LocalChangeDisposition::Conflicted { .. }) })
                }).collect()
        })
    }
    /// Loads a role-specific cursor. Public and private streams cannot overwrite one another.
    /// # Errors
    /// Rejects invalid durable cursors.
    pub fn catalog_checkpoint(
        &self,
        actor: &AuthorizationContext,
        schema: &SchemaId,
    ) -> Result<Option<Checkpoint>, StorageError> {
        let (sk, si) = scope_parts(&actor.scope);
        let value:Option<String>=self.open_connection()?.query_row("SELECT checkpoint FROM catalog_sync_checkpoints WHERE scope_kind=?1 AND scope_id=?2 AND principal_id=?3 AND schema_id=?4",params![sk,si,actor.identity.principal_id.value().to_string(),schema.as_str()],|r|r.get(0)).optional().map_err(|_|StorageError)?;
        value
            .map(|v| {
                uuid::Uuid::parse_str(&v)
                    .map(Checkpoint::new)
                    .map_err(|_| StorageError)
            })
            .transpose()
    }
    /// Commits a whole decoded page with its cursor and audit; interruption exposes no accepted prefix.
    /// # Errors
    /// Rejects conflicting history, foreign scope, and failed mandatory writes.
    pub fn project_catalog_page(
        &self,
        actor: &AuthorizationContext,
        schema: &SchemaId,
        checkpoint: Checkpoint,
        projection: &CatalogSyncProjection<'_>,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        self.write_transaction(|tx| {
            let (sk,si)=scope_parts(&actor.scope);
            for record in projection.private {
                let (kind,id,revision,scope)=record.identity();
                if scope!=&actor.scope {return Err(StorageError);}
                let quarantined:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM catalog_sync_exceptions WHERE scope_kind=?1 AND scope_id=?2 AND kind=?3 AND id=?4 AND revision=?5)",params![sk,si,kind,id.to_string(),i64::try_from(revision).map_err(|_|StorageError)?],|r|r.get(0)).map_err(|_|StorageError)?;
                // Preserve the local input for repair while advancing unrelated server history.
                if quarantined { continue; }
                retain(tx,record,None)?;
                import_record(tx,record,projection.normalize_name)?;
            }
            for (change,entry) in projection.public {
                if change.scope!=actor.scope || entry.as_ref().is_some_and(|e|e.price.target.scope()!=&actor.scope) {return Err(StorageError);}
                let existing:Option<(i64,bool,Option<Vec<u8>>)>=tx.query_row("SELECT revision,tombstone,record_json FROM catalog_sales_records WHERE scope_kind=?1 AND scope_id=?2 AND id=?3",params![sk,si,change.record_id.value().to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|StorageError)?;
                if let Some(entry)=entry {crate::pricing::cache_price(tx,&entry.price)?;}
                let previous=existing.as_ref().and_then(|(_,_,json)|json.as_ref()).map(|json|serde_json::from_slice::<CatalogEntry>(json).map_err(|_|StorageError)).transpose()?;
                let json=entry.as_ref().map(serde_json::to_vec).transpose().map_err(|_|StorageError)?;
                let tombstone=change.operation==ChangeOperation::Tombstone;
                let change_revision=i64::try_from(change.revision).map_err(|_|StorageError)?;
                if let Some((revision,old_tombstone,old_json))=existing { if revision>change_revision {continue;}
                if revision==change_revision {if old_tombstone!=tombstone || old_json!=json {return Err(StorageError);} continue;} }
                tx.execute("INSERT INTO catalog_sales_records VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET revision=excluded.revision,tombstone=excluded.tombstone,record_json=excluded.record_json",params![sk,si,change.record_id.value().to_string(),change_revision,tombstone,json]).map_err(|_|StorageError)?;
                if let Some(notice)=entry.as_ref().or(previous.as_ref()) { insert_publication(tx,&actor.scope,eitmad_contracts::transport::IdempotencyKey::new(uuid::Uuid::new_v4()),&DurablePublication {event:eitmad_contracts::events::Event::PriceChanged(eitmad_contracts::pricing::PriceChangeNotice{target:notice.price.target.clone(),revision:notice.price.revision}),policy_changed:false})?; }
            }
            tx.execute("INSERT INTO catalog_sync_checkpoints VALUES(?1,?2,?3,?4,?5) ON CONFLICT(scope_kind,scope_id,principal_id,schema_id) DO UPDATE SET checkpoint=excluded.checkpoint",params![sk,si,actor.identity.principal_id.value().to_string(),schema.as_str(),checkpoint.value().to_string()]).map_err(|_|StorageError)?;
            insert_audit(tx,audit)
        })
    }
    /// Returns confirmed sales entries without any private table joins.
    /// # Errors
    /// Rejects invalid durable projections.
    pub fn catalog_sales(&self, scope: &ScopeRef) -> Result<Vec<CatalogEntry>, StorageError> {
        let (sk, si) = scope_parts(scope);
        let connection = self.open_connection()?;
        let mut stmt=connection.prepare("SELECT record_json FROM catalog_sales_records WHERE scope_kind=?1 AND scope_id=?2 AND NOT tombstone ORDER BY id").map_err(|_|StorageError)?;
        stmt.query_map(params![sk, si], |r| r.get::<_, Vec<u8>>(0))
            .map_err(|_| StorageError)?
            .map(|r| {
                serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError)
            })
            .collect()
    }

    /// Reads a bounded public batch. Private definition tables are never joined.
    /// # Errors
    /// Rejects unavailable or invalid durable projections.
    pub fn catalog_sales_batch(
        &self,
        scope: &ScopeRef,
        after: Option<Uuid>,
    ) -> Result<Vec<(Uuid, CatalogEntry)>, StorageError> {
        let (sk, si) = scope_parts(scope);
        let connection = self.open_connection()?;
        let mut stmt = connection.prepare("SELECT id,record_json FROM catalog_sales_records WHERE scope_kind=?1 AND scope_id=?2 AND NOT tombstone AND id>?3 ORDER BY id LIMIT 100").map_err(|_|StorageError)?;
        stmt.query_map(
            params![sk, si, after.map(|id| id.to_string()).unwrap_or_default()],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?)),
        )
        .map_err(|_| StorageError)?
        .map(|r| {
            let (id, json) = r.map_err(|_| StorageError)?;
            Ok((
                Uuid::parse_str(&id).map_err(|_| StorageError)?,
                serde_json::from_slice(&json).map_err(|_| StorageError)?,
            ))
        })
        .collect()
    }

    /// Returns only current public variants for an item, including on clients without private definitions.
    /// # Errors
    /// Rejects unavailable storage or an oversized public definition.
    pub fn catalog_sales_item(
        &self,
        scope: &ScopeRef,
        target: &eitmad_contracts::pricing::PriceTarget,
    ) -> Result<Vec<CatalogEntry>, StorageError> {
        let (sk, si) = scope_parts(scope);
        let (kind, id, _) = target.identity();
        let path = if kind == "product" {
            "$.price.target.payload.productId"
        } else {
            "$.price.target.payload.furnitureId"
        };
        let connection = self.open_connection()?;
        let mut stmt=connection.prepare("SELECT record_json FROM catalog_sales_records WHERE scope_kind=?1 AND scope_id=?2 AND NOT tombstone AND json_extract(CAST(record_json AS TEXT),'$.price.target.kind')=?3 AND json_extract(CAST(record_json AS TEXT),?4)=?5 ORDER BY id LIMIT 101").map_err(|_|StorageError)?;
        let entries: Vec<CatalogEntry> = stmt
            .query_map(params![sk, si, kind, path, id.to_string()], |r| {
                r.get::<_, Vec<u8>>(0)
            })
            .map_err(|_| StorageError)?
            .map(|r| {
                serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError)
            })
            .collect::<Result<_, _>>()?;
        if entries.len() > 100 {
            return Err(StorageError);
        }
        Ok(entries)
    }

    /// Returns bounded category filters from active public records.
    /// # Errors
    /// Rejects unavailable projections.
    pub fn catalog_sales_categories(&self, scope: &ScopeRef) -> Result<Vec<String>, StorageError> {
        let (sk, si) = scope_parts(scope);
        let connection = self.open_connection()?;
        let mut stmt=connection.prepare("SELECT DISTINCT json_extract(CAST(record_json AS TEXT),'$.categoryName') FROM catalog_sales_records WHERE scope_kind=?1 AND scope_id=?2 AND NOT tombstone ORDER BY 1 LIMIT 100").map_err(|_|StorageError)?;
        stmt.query_map(params![sk, si], |r| r.get::<_, String>(0))
            .map_err(|_| StorageError)?
            .map(|r| r.map_err(|_| StorageError))
            .collect()
    }
}

/// Compares domain values so omitted defaults and object field order do not cause false conflicts.
fn same_record(record: &CatalogRevision, stored: &[u8]) -> Result<bool, StorageError> {
    let tag = serde_json::to_value(record).map_err(|_| StorageError)?["kind"].clone();
    let payload: serde_json::Value = serde_json::from_slice(stored).map_err(|_| StorageError)?;
    let stored: CatalogRevision =
        serde_json::from_value(serde_json::json!({"kind": tag, "payload": payload}))
            .map_err(|_| StorageError)?;
    Ok(&stored == record)
}

/// Mirrors the durable page order so skipped dependents cannot starve later unrelated work.
fn transfer_order(kind: &str) -> i32 {
    match kind {
        "material-category" => 0,
        "unit" => 1,
        "material" => 2,
        "part-category" => 3,
        "part" => 4,
        "product-category" => 5,
        "product" => 6,
        "furniture-category" => 7,
        _ => 8,
    }
}
/// Imports only newer current state while comparing same-revision domain values and retaining history.
fn import_record(
    tx: &rusqlite::Connection,
    record: &CatalogRevision,
    normalize_name: fn(&str) -> String,
) -> Result<(), StorageError> {
    let (kind, id, revision, scope) = record.identity();
    let (sk, si) = scope_parts(scope);
    let revision = i64::try_from(revision).map_err(|_| StorageError)?;
    let table = match kind {
        "unit" => "material_units",
        "material-category" => "material_categories",
        "material" => "materials",
        "part-category" => "part_categories",
        "part" => "parts",
        "product-category" => "product_categories",
        "product" => "products",
        "furniture-category" => "furniture_categories",
        "furniture" => "furnitures",
        _ => return Err(StorageError),
    };
    let payload = serde_json::to_value(record).map_err(|_| StorageError)?["payload"].clone();
    let json = serde_json::to_vec(&payload).map_err(|_| StorageError)?;
    let current:Option<(i64,Vec<u8>)>=tx.query_row(&format!("SELECT revision,record_json FROM {table} WHERE scope_kind=?1 AND scope_id=?2 AND id=?3"),params![sk,si,id.to_string()],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|StorageError)?;
    if let Some((r, bytes)) = &current {
        if *r == revision && !same_record(record, bytes)? {
            return Err(StorageError);
        }
    }
    let newer = current.is_none_or(|(r, _)| r < revision);
    if newer {
        let mut columns = vec![
            "scope_kind",
            "scope_id",
            "id",
            "normalized_name",
            "revision",
            "record_json",
        ];
        let mut values: Vec<rusqlite::types::Value> = vec![
            sk.to_owned().into(),
            si.clone().into(),
            id.to_string().into(),
            normalize_name(payload["name"].as_str().ok_or(StorageError)?).into(),
            revision.into(),
            json.clone().into(),
        ];
        if table != "parts" {
            columns.push("archived");
            values.push(
                i64::from(
                    payload["archived"]
                        .as_bool()
                        .unwrap_or(payload["state"] == "archived"),
                )
                .into(),
            );
        }
        if matches!(kind, "material" | "part" | "product" | "furniture") {
            columns.push("category_id");
            values.push(
                payload["categoryId"]
                    .as_str()
                    .ok_or(StorageError)?
                    .to_owned()
                    .into(),
            );
        }
        if kind == "material" {
            columns.push("unit_id");
            values.push(
                payload["unitId"]
                    .as_str()
                    .ok_or(StorageError)?
                    .to_owned()
                    .into(),
            );
        }
        let placeholders = vec!["?"; columns.len()].join(",");
        let updates = columns
            .iter()
            .skip(3)
            .map(|c| format!("{c}=excluded.{c}"))
            .collect::<Vec<_>>()
            .join(",");
        tx.execute(&format!("INSERT INTO {table}({}) VALUES({placeholders}) ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET {updates}",columns.join(",")),rusqlite::params_from_iter(values)).map_err(|_|StorageError)?;
    }
    import_history(tx, record, &json)?;
    import_relations(tx, record, newer, normalize_name)
}
/// Keeps Part, Product, and Furniture history immutable even when equivalent JSON encodings differ.
fn import_history(
    tx: &rusqlite::Connection,
    record: &CatalogRevision,
    json: &[u8],
) -> Result<(), StorageError> {
    let (kind, id, revision, scope) = record.identity();
    let (sk, si) = scope_parts(scope);
    let revision = i64::try_from(revision).map_err(|_| StorageError)?;
    if let Some((history, id_column)) = match kind {
        "part" => Some(("part_compositions", "part_id")),
        "product" => Some(("product_revisions", "product_id")),
        "furniture" => Some(("furniture_revisions", "furniture_id")),
        _ => None,
    } {
        let retained:Option<Vec<u8>>=tx.query_row(&format!("SELECT record_json FROM {history} WHERE scope_kind=?1 AND scope_id=?2 AND {id_column}=?3 AND revision=?4"),params![sk,si,id.to_string(),revision],|r|r.get(0)).optional().map_err(|_|StorageError)?;
        if let Some(retained) = retained {
            if !same_record(record, &retained)? {
                return Err(StorageError);
            }
        } else {
            tx.execute(
                &format!("INSERT INTO {history} VALUES(?1,?2,?3,?4,?5)"),
                params![sk, si, id.to_string(), revision, json],
            )
            .map_err(|_| StorageError)?;
        }
    }
    Ok(())
}
/// Projects exact usage and option identities with the imported definition transaction.
fn import_relations(
    tx: &rusqlite::Connection,
    record: &CatalogRevision,
    newer: bool,
    normalize_name: fn(&str) -> String,
) -> Result<(), StorageError> {
    let (_, id, revision, scope) = record.identity();
    let (sk, si) = scope_parts(scope);
    let revision = i64::try_from(revision).map_err(|_| StorageError)?;
    match record {
        CatalogRevision::Part(p) => {
            for row in &p.cost.rows {
                tx.execute(
                    "INSERT OR IGNORE INTO part_material_usages VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        sk,
                        si,
                        id.to_string(),
                        revision,
                        row.usage.material_id.value().to_string(),
                        row.usage.unit_id.value().to_string(),
                        row.usage.quantity.as_str()
                    ],
                )
                .map_err(|_| StorageError)?;
            }
        }
        CatalogRevision::Product(p) if newer => {
            for variant in &p.variants {
                let changed=tx.execute("INSERT INTO product_variants VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived WHERE product_id=excluded.product_id",params![sk,si,id.to_string(),variant.id.value().to_string(),normalize_name(&variant.name),variant.archived]).map_err(|_|StorageError)?;
                if changed != 1 {
                    return Err(StorageError);
                }
            }
        }
        CatalogRevision::Furniture(f) => {
            for usage in &f.parts {
                tx.execute("INSERT OR IGNORE INTO furniture_part_compositions VALUES(?1,?2,?3,?4,?5,?6,?7)",params![sk,si,id.to_string(),revision,usage.reference.part_id.value().to_string(),i64::try_from(usage.reference.revision).map_err(|_|StorageError)?,usage.quantity]).map_err(|_|StorageError)?;
            }
            if newer {
                let options = f
                    .variants
                    .iter()
                    .map(|v| (v.id.value(), v.name.as_str(), v.archived))
                    .chain(
                        f.colors
                            .iter()
                            .chain(&f.handles)
                            .map(|o| (o.id, o.name.as_str(), o.archived)),
                    );
                for (option, name, archived) in options {
                    let changed=tx.execute("INSERT INTO furniture_option_identities VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived WHERE furniture_id=excluded.furniture_id",params![sk,si,id.to_string(),option.to_string(),normalize_name(name),archived]).map_err(|_|StorageError)?;
                    if changed != 1 {
                        return Err(StorageError);
                    }
                }
            }
        }
        _ => (),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::{
        identity::*,
        product::*,
        transport::{CorrelationId, UnixMillis},
    };
    use eitmad_observability_audit::AuditTarget;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn actor() -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(1)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(2)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(Uuid::from_u128(3)),
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(Uuid::from_u128(3)),
            },
        }
    }
    fn evidence(actor: &AuthorizationContext) -> MutationAuditRecord {
        MutationAuditRecord::from_authorization(
            actor,
            UnixMillis(1),
            CorrelationId::new(Uuid::new_v4()),
            "eitmad.catalog.test.v1",
            AuditTarget {
                kind: "catalog".into(),
                identifiers: vec![],
            },
        )
    }
    fn category(actor: &AuthorizationContext, id: u128) -> CatalogRevision {
        CatalogRevision::ProductCategory(Box::new(ProductCategory {
            id: ProductCategoryId::new(Uuid::from_u128(id)),
            scope: actor.scope.clone(),
            name: format!("فئة اختبار {id}"),
            archived: false,
            revision: 1,
            updated_at: UnixMillis(1),
        }))
    }
    fn project(
        store: &AuthorityStore,
        actor: &AuthorizationContext,
        records: &[CatalogRevision],
    ) -> Result<(), StorageError> {
        store.project_catalog_page(
            actor,
            &SchemaId::parse("eitmad.schema.product.v1").unwrap(),
            Checkpoint::new(Uuid::new_v4()),
            &CatalogSyncProjection {
                private: records,
                public: &[],
                normalize_name: str::to_owned,
            },
            &evidence(actor),
        )
    }

    #[test]
    fn empty_notes_in_older_product_json_do_not_block_current_or_history_import() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let actor = actor();
        let category = category(&actor, 10);
        project(&store, &actor, std::slice::from_ref(&category)).unwrap();
        let product = CatalogRevision::Product(Box::new(Product {
            id: ProductId::new(Uuid::from_u128(11)),
            scope: actor.scope.clone(),
            name: "مرتبة اختبار".into(),
            category_id: ProductCategoryId::new(Uuid::from_u128(10)),
            category_name: "فئة اختبار 10".into(),
            image: None,
            description: String::new(),
            notes: String::new(),
            variants: vec![],
            archived: false,
            revision: 1,
            updated_at: UnixMillis(1),
        }));
        let mut old = serde_json::to_value(&product).unwrap()["payload"].clone();
        old["notes"] = "".into();
        let old_bytes = serde_json::to_vec(&old).unwrap();
        let scope_id = actor.scope.id.value().to_string();
        store
            .write_transaction(|tx| {
                tx.execute(
                    "INSERT INTO products VALUES('organization',?1,?2,?3,'مرتبة اختبار',0,1,?4)",
                    params![
                        scope_id,
                        Uuid::from_u128(11).to_string(),
                        Uuid::from_u128(10).to_string(),
                        old_bytes
                    ],
                )
                .map_err(|_| StorageError)?;
                tx.execute(
                    "INSERT INTO product_revisions VALUES('organization',?1,?2,1,?3)",
                    params![scope_id, Uuid::from_u128(11).to_string(), old_bytes],
                )
                .map_err(|_| StorageError)?;
                Ok(())
            })
            .unwrap();
        project(&store, &actor, std::slice::from_ref(&product)).unwrap();
        // Current and history comparisons must still reject actual changes at one immutable revision.
        let mut altered = product.clone();
        if let CatalogRevision::Product(p) = &mut altered {
            p.description = "تغيير".into();
        }
        assert_eq!(
            store.write_transaction(|tx| import_record(tx, &altered, str::to_owned)),
            Err(StorageError)
        );
        let altered_payload =
            serde_json::to_vec(&serde_json::to_value(&altered).unwrap()["payload"]).unwrap();
        assert_eq!(
            store.write_transaction(|tx| import_history(tx, &altered, &altered_payload)),
            Err(StorageError)
        );
    }

    #[test]
    fn quarantine_and_audit_are_atomic_scoped_and_survive_restart() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let actor = actor();
        let record = category(&actor, 10);
        store
            .write_transaction(|tx| enqueue(tx, &record, &evidence(&actor)))
            .unwrap();
        let disposition = LocalChangeDisposition::Rejected {
            reason: eitmad_contracts::sync::ErrorCodeRef::parse("eitmad.error.contract-invalid.v1")
                .unwrap(),
        };
        let mut invalid_audit = evidence(&actor);
        invalid_audit.operation.clear();
        assert_eq!(
            store.reject_catalog_revision(&record, &disposition, &invalid_audit),
            Err(StorageError)
        );
        assert_eq!(
            store.pending_catalog_revisions(&actor.scope, None).unwrap(),
            vec![record.clone()]
        );
        assert!(store.catalog_sync_issues(&actor.scope).unwrap().is_empty());
        store
            .reject_catalog_revision(&record, &disposition, &evidence(&actor))
            .unwrap();
        drop(store);
        let store = AuthorityStore::open(directory.path()).unwrap();
        assert!(
            store
                .pending_catalog_revisions(&actor.scope, None)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            store.catalog_sync_issues(&actor.scope).unwrap()[0].id,
            Uuid::from_u128(10)
        );
        let dependencies = [("product-category", Uuid::from_u128(10), 0)];
        assert!(
            !store
                .catalog_dependencies_ready(&actor.scope, &dependencies)
                .unwrap()
        );
        let mut foreign = actor.scope.clone();
        foreign.id = ScopeId::new(Uuid::from_u128(999));
        assert!(store.catalog_sync_issues(&foreign).unwrap().is_empty());
        let mut repaired = record.clone();
        if let CatalogRevision::ProductCategory(c) = &mut repaired {
            c.revision = 2;
        }
        store
            .write_transaction(|tx| enqueue(tx, &repaired, &evidence(&actor)))
            .unwrap();
        store
            .acknowledge_catalog_revision(&repaired, &evidence(&actor))
            .unwrap();
        assert!(
            store
                .catalog_dependencies_ready(&actor.scope, &dependencies)
                .unwrap()
        );
        assert!(
            !store
                .catalog_dependencies_ready(
                    &actor.scope,
                    &[("product-category", Uuid::from_u128(10), 1)]
                )
                .unwrap()
        );
        assert!(store.catalog_sync_issues(&actor.scope).unwrap().is_empty());
    }
}
