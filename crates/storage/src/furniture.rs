use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, StorageError, insert_audit,
    insert_idempotency, insert_publication, load_idempotency, migrations::Migration, scope_parts,
};
use eitmad_contracts::{
    furniture::{
        Furniture, FurnitureCategory, FurnitureCategoryId, FurnitureId, FurnitureReference,
    },
    identity::ScopeRef,
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(18, "furniture.definitions.v1", "furniture",
    "CREATE TABLE furniture_categories (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL,
      normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,id), UNIQUE(scope_kind,scope_id,normalized_name));
     CREATE TABLE furnitures (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL, category_id TEXT NOT NULL,
      normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,id),
      FOREIGN KEY(scope_kind,scope_id,category_id) REFERENCES furniture_categories(scope_kind,scope_id,id));
     CREATE TABLE furniture_revisions (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, furniture_id TEXT NOT NULL,
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,furniture_id,revision),
      FOREIGN KEY(scope_kind,scope_id,furniture_id) REFERENCES furnitures(scope_kind,scope_id,id));
     CREATE TABLE furniture_option_identities (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, furniture_id TEXT NOT NULL, id TEXT NOT NULL,
      normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
      PRIMARY KEY(scope_kind,scope_id,id),
      FOREIGN KEY(scope_kind,scope_id,furniture_id) REFERENCES furnitures(scope_kind,scope_id,id));
     CREATE TABLE furniture_part_compositions (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, furniture_id TEXT NOT NULL,
      furniture_revision INTEGER NOT NULL, part_id TEXT NOT NULL, part_revision INTEGER NOT NULL,
      quantity INTEGER NOT NULL CHECK(quantity>0),
      PRIMARY KEY(scope_kind,scope_id,furniture_id,furniture_revision,part_id),
      FOREIGN KEY(scope_kind,scope_id,furniture_id,furniture_revision) REFERENCES furniture_revisions(scope_kind,scope_id,furniture_id,revision),
      FOREIGN KEY(scope_kind,scope_id,part_id,part_revision) REFERENCES part_compositions(scope_kind,scope_id,part_id,revision));
     CREATE INDEX furnitures_search ON furnitures(scope_kind,scope_id,normalized_name,id);
     CREATE TRIGGER furniture_revision_immutable BEFORE UPDATE ON furniture_revisions BEGIN SELECT RAISE(ABORT,'immutable furniture revision'); END;
     CREATE TRIGGER furniture_revision_no_delete BEFORE DELETE ON furniture_revisions BEGIN SELECT RAISE(ABORT,'immutable furniture revision'); END;"
)];

pub struct FurnitureTransaction<'a> {
    pub(crate) connection: &'a rusqlite::Connection,
}
#[derive(Clone, Copy)]
pub enum FurnitureRecord<'a> {
    Furniture(&'a Furniture),
    Category(&'a FurnitureCategory),
}

impl AuthorityStore {
    /// Executes validation, calculation, and durable writes on one consistent transaction.
    /// # Errors
    /// Rolls back all writes when the operation or a mandatory write fails.
    pub fn transact_furnitures<T, E: From<StorageError>>(
        &self,
        operation: impl FnOnce(&FurnitureTransaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        self.transact_furnitures_with(rusqlite::TransactionBehavior::Immediate, operation)
    }

    /// Reads a consistent furniture snapshot without reserving the database writer lock.
    /// # Errors
    /// Fails when the snapshot cannot be opened, read, or committed.
    pub fn read_furnitures<T, E: From<StorageError>>(
        &self,
        operation: impl FnOnce(&FurnitureTransaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        self.transact_furnitures_with(rusqlite::TransactionBehavior::Deferred, operation)
    }

    fn transact_furnitures_with<T, E: From<StorageError>>(
        &self,
        behavior: rusqlite::TransactionBehavior,
        operation: impl FnOnce(&FurnitureTransaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(behavior)
            .map_err(|_| StorageError)?;
        let result = operation(&FurnitureTransaction {
            connection: &transaction,
        })?;
        transaction.commit().map_err(|_| StorageError)?;
        Ok(result)
    }
}

impl FurnitureTransaction<'_> {
    /// Reads a scoped dependency category without exposing another capability's transaction.
    /// # Errors
    /// Rejects invalid durable data.
    pub fn part_category(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<eitmad_contracts::part::PartCategory>, StorageError> {
        self.record(scope, "part_categories", id)
    }
    /// Reads the category of an immutable material snapshot.
    /// # Errors
    /// Rejects invalid durable data.
    pub fn material_category(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<eitmad_contracts::material::MaterialCategory>, StorageError> {
        self.record(scope, "material_categories", id)
    }
    /// Reads a scoped category.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn category(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<FurnitureCategory>, StorageError> {
        self.record(scope, "furniture_categories", id)
    }
    /// Reads a current scoped furniture.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn furniture(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<Furniture>, StorageError> {
        self.record(scope, "furnitures", id)
    }
    /// Reads a current scoped part while holding the Furniture transaction.
    /// # Errors
    /// Fails when stored data is invalid.
    pub fn part(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<eitmad_contracts::part::Part>, StorageError> {
        self.record(scope, "parts", id)
    }
    /// Resolves an immutable part revision on the same transaction.
    /// # Errors
    /// Fails when stored data is invalid.
    pub fn composition(
        &self,
        r: &eitmad_contracts::part::CompositionReference,
    ) -> Result<Option<eitmad_contracts::part::Part>, StorageError> {
        let (kind, id) = scope_parts(&r.scope);
        let bytes = self.connection.query_row("SELECT record_json FROM part_compositions WHERE scope_kind=?1 AND scope_id=?2 AND part_id=?3 AND revision=?4", params![kind,id,r.part_id.value().to_string(),i64::try_from(r.revision).map_err(|_|StorageError)?], |row| row.get::<_,Vec<u8>>(0)).optional().map_err(|_|StorageError)?;
        bytes
            .map(|v| serde_json::from_slice(&v).map_err(|_| StorageError))
            .transpose()
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
    /// Loads an exact immutable reference, without using current furniture definitions.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn revision(
        &self,
        reference: &FurnitureReference,
    ) -> Result<Option<Furniture>, StorageError> {
        let (kind, id) = scope_parts(&reference.scope);
        let revision = i64::try_from(reference.revision).map_err(|_| StorageError)?;
        let data=self.connection.query_row("SELECT record_json FROM furniture_revisions WHERE scope_kind=?1 AND scope_id=?2 AND furniture_id=?3 AND revision=?4",params![kind,id,reference.furniture_id.value().to_string(),revision],|r|r.get::<_,Vec<u8>>(0)).optional().map_err(|_|StorageError)?;
        data.map(|bytes| serde_json::from_slice(&bytes).map_err(|_| StorageError))
            .transpose()
    }
    /// Reads scoped categories.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn categories(
        &self,
        scope: &ScopeRef,
        after: Option<FurnitureCategoryId>,
        limit: u32,
    ) -> Result<Vec<FurnitureCategory>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let mut stmt=self.connection.prepare("SELECT record_json FROM furniture_categories WHERE scope_kind=?1 AND scope_id=?2 AND id>?3 ORDER BY id LIMIT ?4").map_err(|_|StorageError)?;
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
        id: FurnitureCategoryId,
        name: &str,
    ) -> Result<bool, StorageError> {
        let (kind, scope_id) = scope_parts(scope);
        self.connection.query_row("SELECT EXISTS(SELECT 1 FROM furniture_categories WHERE scope_kind=?1 AND scope_id=?2 AND id<>?3 AND normalized_name=?4)",params![kind,scope_id,id.value().to_string(),name],|r|r.get(0)).map_err(|_|StorageError)
    }
    /// Reads a bounded scoped page with current category search names.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn list(
        &self,
        scope: &ScopeRef,
        term: &str,
        selectable_only: bool,
        after: Option<FurnitureId>,
        limit: u32,
    ) -> Result<Vec<Furniture>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let mut stmt=self.connection.prepare("SELECT p.record_json FROM furnitures p WHERE p.scope_kind=?1 AND p.scope_id=?2 AND p.id>?3
           AND (instr(p.normalized_name,?4)>0 OR EXISTS (SELECT 1 FROM furniture_categories c WHERE c.scope_kind=p.scope_kind AND c.scope_id=p.scope_id AND c.id=p.category_id AND instr(c.normalized_name,?4)>0)) AND (?6=0 OR (p.archived=0 AND json_extract(p.record_json,'$.state')='active' AND EXISTS(SELECT 1 FROM furniture_categories c WHERE c.scope_kind=p.scope_kind AND c.scope_id=p.scope_id AND c.id=p.category_id AND c.archived=0) AND EXISTS(SELECT 1 FROM furniture_option_identities v WHERE v.scope_kind=p.scope_kind AND v.scope_id=p.scope_id AND v.furniture_id=p.id AND v.archived=0))) ORDER BY p.id LIMIT ?5").map_err(|_|StorageError)?;
        stmt.query_map(
            params![
                kind,
                id,
                after.map(|v| v.value().to_string()).unwrap_or_default(),
                term,
                i64::from(limit) + 1,
                selectable_only
            ],
            |r| r.get::<_, Vec<u8>>(0),
        )
        .map_err(|_| StorageError)?
        .map(|r| serde_json::from_slice(&r.map_err(|_| StorageError)?).map_err(|_| StorageError))
        .collect()
    }
    /// Rejects variant identities already owned by another furniture in this scope.
    /// # Errors
    /// Fails on unavailable storage.
    pub fn option_owner(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>, StorageError> {
        let (kind, scope_id) = scope_parts(scope);
        let value: Option<String> = self.connection.query_row("SELECT furniture_id FROM furniture_option_identities WHERE scope_kind=?1 AND scope_id=?2 AND id=?3",params![kind,scope_id,id.to_string()],|r|r.get(0)).optional().map_err(|_|StorageError)?;
        value
            .map(|v| uuid::Uuid::parse_str(&v).map_err(|_| StorageError))
            .transpose()
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
    /// Commits a record, immutable revision, variant identities, audit, retry result, and event together.
    /// # Errors
    /// Fails if any mandatory write fails; the caller transaction rolls back.
    pub fn persist(
        &self,
        record: FurnitureRecord<'_>,
        normalized: &str,
        operation: &str,
        audit: &MutationAuditRecord,
        retry: &DurableIdempotency,
        publication: &DurablePublication,
    ) -> Result<(), StorageError> {
        let (scope, id, revision, json) = match record {
            FurnitureRecord::Furniture(p) => {
                (&p.scope, p.id.value(), p.revision, serde_json::to_vec(p))
            }
            FurnitureRecord::Category(c) => {
                (&c.scope, c.id.value(), c.revision, serde_json::to_vec(c))
            }
        };
        let json = json.map_err(|_| StorageError)?;
        let revision = i64::try_from(revision).map_err(|_| StorageError)?;
        let (kind, scope_id) = scope_parts(scope);
        match record {
            FurnitureRecord::Category(c) => {
                self.connection.execute("INSERT INTO furniture_categories(scope_kind,scope_id,id,normalized_name,archived,revision,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7)
            ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived,revision=excluded.revision,record_json=excluded.record_json",params![kind,scope_id,id.to_string(),normalized,c.archived,revision,json]).map_err(|_|StorageError)?;
            }
            FurnitureRecord::Furniture(p) => {
                self.connection.execute("INSERT INTO furnitures(scope_kind,scope_id,id,category_id,normalized_name,archived,revision,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET category_id=excluded.category_id,normalized_name=excluded.normalized_name,archived=excluded.archived,revision=excluded.revision,record_json=excluded.record_json",params![kind,scope_id,id.to_string(),p.category_id.value().to_string(),normalized,p.state == eitmad_contracts::furniture::FurnitureState::Archived,revision,json]).map_err(|_|StorageError)?;
                self.connection.execute("INSERT INTO furniture_revisions(scope_kind,scope_id,furniture_id,revision,record_json) VALUES(?1,?2,?3,?4,?5)",params![kind,scope_id,id.to_string(),revision,json]).map_err(|_|StorageError)?;
                for usage in &p.parts {
                    self.connection.execute("INSERT INTO furniture_part_compositions(scope_kind,scope_id,furniture_id,furniture_revision,part_id,part_revision,quantity) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![kind,scope_id,id.to_string(),revision,usage.reference.part_id.value().to_string(),i64::try_from(usage.reference.revision).map_err(|_|StorageError)?,i64::from(usage.quantity)]).map_err(|_|StorageError)?;
                }
                for variant in &p.variants {
                    self.connection.execute("INSERT INTO furniture_option_identities(scope_kind,scope_id,furniture_id,id,normalized_name,archived) VALUES(?1,?2,?3,?4,?5,?6)
                    ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived WHERE furniture_id=excluded.furniture_id",params![kind,scope_id,id.to_string(),variant.id.value().to_string(),variant.name,variant.archived]).map_err(|_|StorageError)?;
                }
                for option in p.colors.iter().chain(&p.handles) {
                    self.connection.execute("INSERT INTO furniture_option_identities(scope_kind,scope_id,furniture_id,id,normalized_name,archived) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived WHERE furniture_id=excluded.furniture_id",params![kind,scope_id,id.to_string(),option.id.to_string(),option.name,option.archived]).map_err(|_|StorageError)?;
                }
            }
        }
        insert_audit(self.connection, audit)?;
        let revision = match record {
            FurnitureRecord::Furniture(v) => {
                eitmad_contracts::catalog_revision::CatalogRevision::Furniture(Box::new(v.clone()))
            }
            FurnitureRecord::Category(v) => {
                eitmad_contracts::catalog_revision::CatalogRevision::FurnitureCategory(Box::new(
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
