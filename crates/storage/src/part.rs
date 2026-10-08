use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, StorageError, insert_audit,
    insert_idempotency, insert_publication, load_idempotency, migrations::Migration, scope_parts,
};
use eitmad_contracts::{
    identity::ScopeRef,
    material::{Material, MaterialUnit},
    part::{CompositionReference, Part, PartCategory, PartCategoryId, PartId},
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(16, "part.compositions.v1", "part",
    "CREATE TABLE part_categories (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL,
      normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,id), UNIQUE(scope_kind,scope_id,normalized_name));
     CREATE TABLE parts (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL, category_id TEXT NOT NULL,
      normalized_name TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,id),
      FOREIGN KEY(scope_kind,scope_id,category_id) REFERENCES part_categories(scope_kind,scope_id,id));
     CREATE TABLE part_compositions (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, part_id TEXT NOT NULL,
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,part_id,revision),
      FOREIGN KEY(scope_kind,scope_id,part_id) REFERENCES parts(scope_kind,scope_id,id));
     CREATE TABLE part_material_usages (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, part_id TEXT NOT NULL, composition_revision INTEGER NOT NULL,
      material_id TEXT NOT NULL, unit_id TEXT NOT NULL, quantity TEXT NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,part_id,composition_revision,material_id),
      FOREIGN KEY(scope_kind,scope_id,part_id,composition_revision) REFERENCES part_compositions(scope_kind,scope_id,part_id,revision),
      FOREIGN KEY(scope_kind,scope_id,material_id) REFERENCES materials(scope_kind,scope_id,id),
      FOREIGN KEY(scope_kind,scope_id,unit_id) REFERENCES material_units(scope_kind,scope_id,id));
     CREATE INDEX parts_search ON parts(scope_kind,scope_id,normalized_name,id);
     CREATE TRIGGER part_composition_immutable BEFORE UPDATE ON part_compositions BEGIN SELECT RAISE(ABORT,'immutable composition'); END;
     CREATE TRIGGER part_composition_no_delete BEFORE DELETE ON part_compositions BEGIN SELECT RAISE(ABORT,'immutable composition'); END;"
)];

pub struct PartTransaction<'a> {
    connection: &'a rusqlite::Connection,
}
#[derive(Clone, Copy)]
pub enum PartRecord<'a> {
    Part(&'a Part),
    Category(&'a PartCategory),
}

impl AuthorityStore {
    /// Executes validation, calculation, and durable writes on one consistent transaction.
    /// # Errors
    /// Rolls back all writes when the operation or a mandatory write fails.
    pub fn transact_parts<T, E: From<StorageError>>(
        &self,
        operation: impl FnOnce(&PartTransaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| StorageError)?;
        let result = operation(&PartTransaction {
            connection: &transaction,
        })?;
        transaction.commit().map_err(|_| StorageError)?;
        Ok(result)
    }
}

impl PartTransaction<'_> {
    /// Reads a scoped material in the same transaction as cost and revision checks.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn material(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<Material>, StorageError> {
        self.record(scope, "materials", id)
    }
    /// Reads a scoped unit.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn unit(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<MaterialUnit>, StorageError> {
        self.record(scope, "material_units", id)
    }
    /// Reads a scoped category.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn category(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<PartCategory>, StorageError> {
        self.record(scope, "part_categories", id)
    }
    /// Reads a current scoped part.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn part(&self, scope: &ScopeRef, id: uuid::Uuid) -> Result<Option<Part>, StorageError> {
        self.record(scope, "parts", id)
    }
    /// Reads only a scope-qualified record from an internally chosen table and validates stored JSON.
    fn record<T: serde::de::DeserializeOwned>(
        &self,
        scope: &ScopeRef,
        table: &str,
        id: uuid::Uuid,
    ) -> Result<Option<T>, StorageError> {
        let (kind, scope_id) = scope_parts(scope);
        let data = self
            .connection
            .query_row(
                &format!(
                    "SELECT record_json FROM {table} WHERE scope_kind=?1 AND scope_id=?2 AND id=?3"
                ),
                params![kind, scope_id, id.to_string()],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|_| StorageError)?;
        data.map(|bytes| serde_json::from_slice(&bytes).map_err(|_| StorageError))
            .transpose()
    }
    /// Loads an exact immutable reference, without using current material definitions.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn composition(
        &self,
        reference: &CompositionReference,
    ) -> Result<Option<Part>, StorageError> {
        let (kind, id) = scope_parts(&reference.scope);
        let revision = i64::try_from(reference.revision).map_err(|_| StorageError)?;
        let data=self.connection.query_row("SELECT record_json FROM part_compositions WHERE scope_kind=?1 AND scope_id=?2 AND part_id=?3 AND revision=?4",params![kind,id,reference.part_id.value().to_string(),revision],|r|r.get::<_,Vec<u8>>(0)).optional().map_err(|_|StorageError)?;
        data.map(|bytes| serde_json::from_slice(&bytes).map_err(|_| StorageError))
            .transpose()
    }
    /// Reads scoped categories.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn categories(
        &self,
        scope: &ScopeRef,
        after: Option<PartCategoryId>,
        limit: u32,
    ) -> Result<Vec<PartCategory>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let mut stmt=self.connection.prepare("SELECT record_json FROM part_categories WHERE scope_kind=?1 AND scope_id=?2 AND id>?3 ORDER BY id LIMIT ?4").map_err(|_|StorageError)?;
        stmt.query_map(
            params![
                kind,
                id,
                after.map(|v| v.value().to_string()).unwrap_or_default(),
                i64::from(limit) + 1
            ],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| StorageError)?
        .map(|r| serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError))
        .collect()
    }
    /// Checks a scoped category name without reading unbounded category records.
    /// # Errors
    /// Fails on unavailable storage.
    pub fn category_name_exists(
        &self,
        scope: &ScopeRef,
        id: PartCategoryId,
        name: &str,
    ) -> Result<bool, StorageError> {
        let (kind, scope_id) = scope_parts(scope);
        self.connection.query_row("SELECT EXISTS(SELECT 1 FROM part_categories WHERE scope_kind=?1 AND scope_id=?2 AND id<>?3 AND normalized_name=?4)",params![kind,scope_id,id.value().to_string(),name],|r|r.get(0)).map_err(|_|StorageError)
    }
    /// Reads a bounded scoped page with current category search names.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn list(
        &self,
        scope: &ScopeRef,
        term: &str,
        after: Option<PartId>,
        limit: u32,
    ) -> Result<Vec<Part>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let mut stmt=self.connection.prepare("SELECT p.record_json FROM parts p WHERE p.scope_kind=?1 AND p.scope_id=?2 AND p.id>?3
           AND (instr(p.normalized_name,?4)>0 OR EXISTS (SELECT 1 FROM part_categories c WHERE c.scope_kind=p.scope_kind AND c.scope_id=p.scope_id AND c.id=p.category_id AND instr(c.normalized_name,?4)>0)) ORDER BY p.id LIMIT ?5").map_err(|_|StorageError)?;
        stmt.query_map(
            params![
                kind,
                id,
                after.map(|v| v.value().to_string()).unwrap_or_default(),
                term,
                i64::from(limit) + 1
            ],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| StorageError)?
        .map(|r| serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError))
        .collect()
    }
    /// Reads a prior retry result in the exact scope.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn replay(
        &self,
        scope: &ScopeRef,
        value: &DurableIdempotency,
    ) -> Result<Option<DurableIdempotency>, StorageError> {
        Ok(load_idempotency(self.connection, scope, value.key)?.map(
            |(request_hash, response_json)| DurableIdempotency {
                key: value.key,
                request_hash,
                response_json,
            },
        ))
    }
    /// Writes redacted conflict or rejection evidence.
    /// # Errors
    /// Fails if mandatory audit cannot be written.
    pub fn audit(&self, value: &MutationAuditRecord) -> Result<(), StorageError> {
        insert_audit(self.connection, value)
    }
    /// Commits a record, immutable composition, usages, audit, retry result, and event together.
    /// # Errors
    /// Fails if any mandatory write fails; the caller transaction rolls back.
    pub fn persist(
        &self,
        record: PartRecord<'_>,
        normalized: &str,
        operation: &str,
        audit: &MutationAuditRecord,
        retry: &DurableIdempotency,
        publication: &DurablePublication,
    ) -> Result<(), StorageError> {
        let (scope, id, revision, json) = match record {
            PartRecord::Part(p) => (&p.scope, p.id.value(), p.revision, serde_json::to_vec(p)),
            PartRecord::Category(c) => (&c.scope, c.id.value(), c.revision, serde_json::to_vec(c)),
        };
        let json = json.map_err(|_| StorageError)?;
        let revision = i64::try_from(revision).map_err(|_| StorageError)?;
        let (kind, scope_id) = scope_parts(scope);
        match record {
            PartRecord::Category(c) => {
                self.connection.execute("INSERT INTO part_categories(scope_kind,scope_id,id,normalized_name,archived,revision,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7)
            ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived,revision=excluded.revision,record_json=excluded.record_json",params![kind,scope_id,id.to_string(),normalized,c.archived,revision,json]).map_err(|_|StorageError)?;
            }
            PartRecord::Part(p) => {
                self.connection.execute("INSERT INTO parts(scope_kind,scope_id,id,category_id,normalized_name,revision,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7)
                ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET category_id=excluded.category_id,normalized_name=excluded.normalized_name,revision=excluded.revision,record_json=excluded.record_json",params![kind,scope_id,id.to_string(),p.category_id.value().to_string(),normalized,revision,json]).map_err(|_|StorageError)?;
                self.connection.execute("INSERT INTO part_compositions(scope_kind,scope_id,part_id,revision,record_json) VALUES(?1,?2,?3,?4,?5)",params![kind,scope_id,id.to_string(),revision,json]).map_err(|_|StorageError)?;
                for row in &p.cost.rows {
                    self.connection.execute("INSERT INTO part_material_usages(scope_kind,scope_id,part_id,composition_revision,material_id,unit_id,quantity) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![kind,scope_id,id.to_string(),revision,row.usage.material_id.value().to_string(),row.usage.unit_id.value().to_string(),row.usage.quantity.as_str()]).map_err(|_|StorageError)?;
                }
            }
        }
        insert_audit(self.connection, audit)?;
        let revision = match record {
            PartRecord::Part(v) => {
                eitmad_contracts::catalog_revision::CatalogRevision::Part(Box::new(v.clone()))
            }
            PartRecord::Category(v) => {
                eitmad_contracts::catalog_revision::CatalogRevision::PartCategory(Box::new(
                    v.clone(),
                ))
            }
        };
        crate::catalog_sync::enqueue(self.connection, &revision, audit)?;
        let mut retry = retry.clone();
        retry.response_json = json;
        insert_idempotency(self.connection, scope, operation, &retry)?;
        insert_publication(self.connection, scope, retry.key, publication)
    }
}
