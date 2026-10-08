use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, StorageError, insert_audit,
    insert_idempotency, insert_publication, load_idempotency, migrations::Migration, scope_parts,
};
use eitmad_contracts::{
    identity::ScopeRef,
    product::{Product, ProductCategory, ProductCategoryId, ProductId, ProductReference},
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(17, "product.definitions.v1", "product",
    "CREATE TABLE product_categories (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL,
      normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,id), UNIQUE(scope_kind,scope_id,normalized_name));
     CREATE TABLE products (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL, category_id TEXT NOT NULL,
      normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,id),
      FOREIGN KEY(scope_kind,scope_id,category_id) REFERENCES product_categories(scope_kind,scope_id,id));
     CREATE TABLE product_revisions (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, product_id TEXT NOT NULL,
      revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
      PRIMARY KEY(scope_kind,scope_id,product_id,revision),
      FOREIGN KEY(scope_kind,scope_id,product_id) REFERENCES products(scope_kind,scope_id,id));
     CREATE TABLE product_variants (
      scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, product_id TEXT NOT NULL, id TEXT NOT NULL,
      normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
      PRIMARY KEY(scope_kind,scope_id,id),
      FOREIGN KEY(scope_kind,scope_id,product_id) REFERENCES products(scope_kind,scope_id,id));
     CREATE INDEX products_search ON products(scope_kind,scope_id,normalized_name,id);
     CREATE TRIGGER product_revision_immutable BEFORE UPDATE ON product_revisions BEGIN SELECT RAISE(ABORT,'immutable product revision'); END;
     CREATE TRIGGER product_revision_no_delete BEFORE DELETE ON product_revisions BEGIN SELECT RAISE(ABORT,'immutable product revision'); END;"
)];

pub struct ProductTransaction<'a> {
    pub(crate) connection: &'a rusqlite::Connection,
}
#[derive(Clone, Copy)]
pub enum ProductRecord<'a> {
    Product(&'a Product),
    Category(&'a ProductCategory),
}

impl AuthorityStore {
    /// Executes validation, calculation, and durable writes on one consistent transaction.
    /// # Errors
    /// Rolls back all writes when the operation or a mandatory write fails.
    pub fn transact_products<T, E: From<StorageError>>(
        &self,
        operation: impl FnOnce(&ProductTransaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        self.transact_products_with(rusqlite::TransactionBehavior::Immediate, operation)
    }

    /// Reads a consistent product snapshot without reserving the database writer lock.
    /// # Errors
    /// Fails when the snapshot cannot be opened, read, or committed.
    pub fn read_products<T, E: From<StorageError>>(
        &self,
        operation: impl FnOnce(&ProductTransaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        self.transact_products_with(rusqlite::TransactionBehavior::Deferred, operation)
    }

    fn transact_products_with<T, E: From<StorageError>>(
        &self,
        behavior: rusqlite::TransactionBehavior,
        operation: impl FnOnce(&ProductTransaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(behavior)
            .map_err(|_| StorageError)?;
        let result = operation(&ProductTransaction {
            connection: &transaction,
        })?;
        transaction.commit().map_err(|_| StorageError)?;
        Ok(result)
    }
}

impl ProductTransaction<'_> {
    /// Reads a scoped category.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn category(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<ProductCategory>, StorageError> {
        self.record(scope, "product_categories", id)
    }
    /// Reads a current scoped product.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn product(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<Product>, StorageError> {
        self.record(scope, "products", id)
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
    /// Loads an exact immutable reference, without using current product definitions.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn revision(&self, reference: &ProductReference) -> Result<Option<Product>, StorageError> {
        let (kind, id) = scope_parts(&reference.scope);
        let revision = i64::try_from(reference.revision).map_err(|_| StorageError)?;
        let data=self.connection.query_row("SELECT record_json FROM product_revisions WHERE scope_kind=?1 AND scope_id=?2 AND product_id=?3 AND revision=?4",params![kind,id,reference.product_id.value().to_string(),revision],|r|r.get::<_,Vec<u8>>(0)).optional().map_err(|_|StorageError)?;
        data.map(|bytes| serde_json::from_slice(&bytes).map_err(|_| StorageError))
            .transpose()
    }
    /// Reads scoped categories.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn categories(
        &self,
        scope: &ScopeRef,
        after: Option<ProductCategoryId>,
        limit: u32,
    ) -> Result<Vec<ProductCategory>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let mut stmt=self.connection.prepare("SELECT record_json FROM product_categories WHERE scope_kind=?1 AND scope_id=?2 AND id>?3 ORDER BY id LIMIT ?4").map_err(|_|StorageError)?;
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
        id: ProductCategoryId,
        name: &str,
    ) -> Result<bool, StorageError> {
        let (kind, scope_id) = scope_parts(scope);
        self.connection.query_row("SELECT EXISTS(SELECT 1 FROM product_categories WHERE scope_kind=?1 AND scope_id=?2 AND id<>?3 AND normalized_name=?4)",params![kind,scope_id,id.value().to_string(),name],|r|r.get(0)).map_err(|_|StorageError)
    }
    /// Reads a bounded scoped page with current category search names.
    /// # Errors
    /// Fails on invalid durable data.
    pub fn list(
        &self,
        scope: &ScopeRef,
        term: &str,
        selectable_only: bool,
        after: Option<ProductId>,
        limit: u32,
    ) -> Result<Vec<Product>, StorageError> {
        let (kind, id) = scope_parts(scope);
        let mut stmt=self.connection.prepare("SELECT p.record_json FROM products p WHERE p.scope_kind=?1 AND p.scope_id=?2 AND p.id>?3
           AND (instr(p.normalized_name,?4)>0 OR EXISTS (SELECT 1 FROM product_categories c WHERE c.scope_kind=p.scope_kind AND c.scope_id=p.scope_id AND c.id=p.category_id AND instr(c.normalized_name,?4)>0)) AND (?6=0 OR (p.archived=0 AND EXISTS(SELECT 1 FROM product_categories c WHERE c.scope_kind=p.scope_kind AND c.scope_id=p.scope_id AND c.id=p.category_id AND c.archived=0) AND EXISTS(SELECT 1 FROM product_variants v WHERE v.scope_kind=p.scope_kind AND v.scope_id=p.scope_id AND v.product_id=p.id AND v.archived=0))) ORDER BY p.id LIMIT ?5").map_err(|_|StorageError)?;
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
    /// Rejects variant identities already owned by another product in this scope.
    /// # Errors
    /// Fails on unavailable storage.
    pub fn variant_owner(
        &self,
        scope: &ScopeRef,
        id: uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>, StorageError> {
        let (kind, scope_id) = scope_parts(scope);
        let value: Option<String> = self.connection.query_row("SELECT product_id FROM product_variants WHERE scope_kind=?1 AND scope_id=?2 AND id=?3",params![kind,scope_id,id.to_string()],|r|r.get(0)).optional().map_err(|_|StorageError)?;
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
        record: ProductRecord<'_>,
        normalized: &str,
        operation: &str,
        audit: &MutationAuditRecord,
        retry: &DurableIdempotency,
        publication: &DurablePublication,
    ) -> Result<(), StorageError> {
        let (scope, id, revision, json) = match record {
            ProductRecord::Product(p) => {
                (&p.scope, p.id.value(), p.revision, serde_json::to_vec(p))
            }
            ProductRecord::Category(c) => {
                (&c.scope, c.id.value(), c.revision, serde_json::to_vec(c))
            }
        };
        let json = json.map_err(|_| StorageError)?;
        let revision = i64::try_from(revision).map_err(|_| StorageError)?;
        let (kind, scope_id) = scope_parts(scope);
        match record {
            ProductRecord::Category(c) => {
                self.connection.execute("INSERT INTO product_categories(scope_kind,scope_id,id,normalized_name,archived,revision,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7)
            ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived,revision=excluded.revision,record_json=excluded.record_json",params![kind,scope_id,id.to_string(),normalized,c.archived,revision,json]).map_err(|_|StorageError)?;
            }
            ProductRecord::Product(p) => {
                self.connection.execute("INSERT INTO products(scope_kind,scope_id,id,category_id,normalized_name,archived,revision,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET category_id=excluded.category_id,normalized_name=excluded.normalized_name,archived=excluded.archived,revision=excluded.revision,record_json=excluded.record_json",params![kind,scope_id,id.to_string(),p.category_id.value().to_string(),normalized,p.archived,revision,json]).map_err(|_|StorageError)?;
                self.connection.execute("INSERT INTO product_revisions(scope_kind,scope_id,product_id,revision,record_json) VALUES(?1,?2,?3,?4,?5)",params![kind,scope_id,id.to_string(),revision,json]).map_err(|_|StorageError)?;
                for variant in &p.variants {
                    self.connection.execute("INSERT INTO product_variants(scope_kind,scope_id,product_id,id,normalized_name,archived) VALUES(?1,?2,?3,?4,?5,?6)
                    ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived WHERE product_id=excluded.product_id",params![kind,scope_id,id.to_string(),variant.id.value().to_string(),variant.name,variant.archived]).map_err(|_|StorageError)?;
                }
            }
        }
        insert_audit(self.connection, audit)?;
        let revision = match record {
            ProductRecord::Product(v) => {
                eitmad_contracts::catalog_revision::CatalogRevision::Product(Box::new(v.clone()))
            }
            ProductRecord::Category(v) => {
                eitmad_contracts::catalog_revision::CatalogRevision::ProductCategory(Box::new(
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
