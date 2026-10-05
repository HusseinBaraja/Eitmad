//! Durable catalog transfer work and atomic, scoped read-model projection.
use crate::{
    AuthorityStore, DurablePublication, StorageError, insert_audit, insert_publication,
    migrations::Migration, scope_parts,
};
use eitmad_contracts::{
    catalog_revision::{CatalogEntry, CatalogRevision},
    identity::{AuthenticatedIdentity, AuthorizationContext, ScopeRef},
    sync::{ChangeOperation, ChangeRecord, Checkpoint},
    transport::SchemaId,
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

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
     CREATE TABLE catalog_sync_clients(scope_kind TEXT NOT NULL,scope_id TEXT NOT NULL,principal_id TEXT NOT NULL,actor_json BLOB NOT NULL,bootstrapped INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(scope_kind,scope_id,principal_id));")];

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
    ) -> Result<Vec<CatalogRevision>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let connection = self.open_connection()?;
        let mut stmt=connection.prepare("SELECT record_json FROM catalog_sync_revisions WHERE scope_kind=?1 AND scope_id=?2 AND pending=1 ORDER BY CASE kind WHEN 'material-category' THEN 0 WHEN 'unit' THEN 1 WHEN 'material' THEN 2 WHEN 'part-category' THEN 3 WHEN 'part' THEN 4 WHEN 'product-category' THEN 5 WHEN 'product' THEN 6 WHEN 'furniture-category' THEN 7 ELSE 8 END,id,revision LIMIT 50").map_err(|_|StorageError)?;
        stmt.query_map(params![kind, id], |r| r.get::<_, Vec<u8>>(0))
            .map_err(|_| StorageError)?
            .map(|r| {
                serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError)
            })
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
            for record in projection.private { if record.identity().3!=&actor.scope {return Err(StorageError);} retain(tx,record,None)?; import_record(tx,record,projection.normalize_name)?; }
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
}

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
        if *r == revision
            && serde_json::from_slice::<serde_json::Value>(bytes).map_err(|_| StorageError)?
                != payload
        {
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
    import_history(tx, record, &payload, &json)?;
    import_relations(tx, record, newer, normalize_name)
}

fn import_history(
    tx: &rusqlite::Connection,
    record: &CatalogRevision,
    payload: &serde_json::Value,
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
            if serde_json::from_slice::<serde_json::Value>(&retained).map_err(|_| StorageError)?
                != *payload
            {
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
