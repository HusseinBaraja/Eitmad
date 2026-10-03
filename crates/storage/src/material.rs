//! Atomic, organization-scoped material definitions and references.

use eitmad_contracts::{
    identity::ScopeRef,
    material::{
        Material, MaterialCategory, MaterialId, MaterialPage, MaterialReferences, MaterialUnit,
    },
};
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};

use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, StorageError, insert_audit,
    insert_idempotency, insert_publication, load_idempotency, migrations::Migration, scope_parts,
};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(
    15, "material.definitions.v1", "material",
    "CREATE TABLE material_categories (
        scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL,
        normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
        revision INTEGER NOT NULL CHECK(revision > 0), record_json BLOB NOT NULL,
        PRIMARY KEY(scope_kind,scope_id,id), UNIQUE(scope_kind,scope_id,normalized_name)
     );
     CREATE TABLE material_units (
        scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL,
        normalized_name TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN (0,1)),
        revision INTEGER NOT NULL CHECK(revision > 0), record_json BLOB NOT NULL,
        PRIMARY KEY(scope_kind,scope_id,id), UNIQUE(scope_kind,scope_id,normalized_name)
     );
     CREATE TABLE materials (
        scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, id TEXT NOT NULL,
        normalized_name TEXT NOT NULL, category_id TEXT NOT NULL, unit_id TEXT NOT NULL,
        archived INTEGER NOT NULL CHECK(archived IN (0,1)),
        revision INTEGER NOT NULL CHECK(revision > 0), record_json BLOB NOT NULL,
        PRIMARY KEY(scope_kind,scope_id,id),
        FOREIGN KEY(scope_kind,scope_id,category_id) REFERENCES material_categories(scope_kind,scope_id,id),
        FOREIGN KEY(scope_kind,scope_id,unit_id) REFERENCES material_units(scope_kind,scope_id,id)
     );
     CREATE INDEX materials_search ON materials(scope_kind,scope_id,normalized_name,id);",
)];

#[derive(Clone, Copy)]
pub enum MaterialRecord<'a> {
    Material(&'a Material),
    Category(&'a MaterialCategory),
    Unit(&'a MaterialUnit),
}

impl MaterialRecord<'_> {
    fn table(self) -> &'static str {
        match self {
            Self::Material(_) => "materials",
            Self::Category(_) => "material_categories",
            Self::Unit(_) => "material_units",
        }
    }
    fn id(self) -> String {
        match self {
            Self::Material(x) => x.id.value(),
            Self::Category(x) => x.id.value(),
            Self::Unit(x) => x.id.value(),
        }
        .to_string()
    }
    fn scope(self) -> ScopeRef {
        match self {
            Self::Material(x) => &x.scope,
            Self::Category(x) => &x.scope,
            Self::Unit(x) => &x.scope,
        }
        .clone()
    }
    fn revision(self) -> u64 {
        match self {
            Self::Material(x) => x.revision,
            Self::Category(x) => x.revision,
            Self::Unit(x) => x.revision,
        }
    }
    fn archived(self) -> bool {
        match self {
            Self::Material(x) => x.archived,
            Self::Category(x) => x.archived,
            Self::Unit(x) => x.archived,
        }
    }
    fn json(self) -> Result<Vec<u8>, StorageError> {
        match self {
            Self::Material(x) => serde_json::to_vec(x),
            Self::Category(x) => serde_json::to_vec(x),
            Self::Unit(x) => serde_json::to_vec(x),
        }
        .map_err(|_| StorageError)
    }
}

pub struct MaterialCommit<'a> {
    pub record: MaterialRecord<'a>,
    pub normalized_name: &'a str,
    pub expected_revision: Option<u64>,
    pub operation: &'a str,
    pub idempotency: &'a DurableIdempotency,
    pub audit: &'a MutationAuditRecord,
    pub publication: &'a DurablePublication,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialCommitOutcome {
    Committed,
    Replayed(Vec<u8>),
    RevisionConflict(Option<u64>),
    InvalidReference,
    ReferencedUnitDefinition,
    DuplicateName,
    IdempotencyMismatch,
}

impl AuthorityStore {
    /// Reads all scoped reference definitions, including archived ones.
    /// # Errors
    /// Returns an unavailable error for invalid durable data.
    pub fn material_references(
        &self,
        scope: &ScopeRef,
    ) -> Result<MaterialReferences, StorageError> {
        let connection = self.open_connection()?;
        Ok(MaterialReferences {
            categories: read_records(&connection, scope, "material_categories")?,
            units: read_records(&connection, scope, "material_units")?,
        })
    }

    /// Searches material names in one exact scope with a stable UUID cursor.
    /// # Errors
    /// Returns an unavailable error for invalid durable data.
    pub fn list_materials(
        &self,
        scope: &ScopeRef,
        term: &str,
        after: Option<MaterialId>,
        limit: u32,
    ) -> Result<MaterialPage, StorageError> {
        let connection = self.open_connection()?;
        let (kind, id) = scope_parts(scope);
        let mut statement = connection.prepare("SELECT m.record_json FROM materials m WHERE m.scope_kind=?1 AND m.scope_id=?2
            AND (instr(m.normalized_name,?3)>0 OR EXISTS
              (SELECT 1 FROM material_categories c WHERE c.scope_kind=m.scope_kind AND c.scope_id=m.scope_id
               AND c.id=m.category_id AND instr(c.normalized_name,?3)>0)
              OR EXISTS (SELECT 1 FROM material_units u WHERE u.scope_kind=m.scope_kind AND u.scope_id=m.scope_id
               AND u.id=m.unit_id AND instr(u.normalized_name,?3)>0))
            AND m.id>?4 ORDER BY m.id LIMIT ?5").map_err(|_| StorageError)?;
        let items = statement
            .query_map(
                params![
                    kind,
                    id,
                    term,
                    after.map(|v| v.value().to_string()).unwrap_or_default(),
                    i64::from(limit) + 1
                ],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .map_err(|_| StorageError)?
            .map(|row| {
                serde_json::from_slice::<Material>(&row.map_err(|_| StorageError)?)
                    .map_err(|_| StorageError)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut items = items;
        let next = if items.len() > limit as usize {
            items.truncate(limit as usize);
            items.last().map(|v| v.id)
        } else {
            None
        };
        Ok(MaterialPage { items, next })
    }

    /// Commits one validated record, mandatory audit, idempotency result, and event in one transaction.
    /// # Errors
    /// Returns an unavailable error if any mandatory write fails.
    pub fn commit_material(
        &self,
        commit: &MaterialCommit<'_>,
    ) -> Result<MaterialCommitOutcome, StorageError> {
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| StorageError)?;
        let outcome = commit_on(&transaction, commit)?;
        transaction.commit().map_err(|_| StorageError)?;
        Ok(outcome)
    }
}

fn read_records<T: serde::de::DeserializeOwned>(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    table: &str,
) -> Result<Vec<T>, StorageError> {
    let (kind, id) = scope_parts(scope);
    let mut statement = connection
        .prepare(&format!(
            "SELECT record_json FROM {table} WHERE scope_kind=?1 AND scope_id=?2 ORDER BY id"
        ))
        .map_err(|_| StorageError)?;
    statement
        .query_map(params![kind, id], |row| row.get::<_, Vec<u8>>(0))
        .map_err(|_| StorageError)?
        .map(|row| {
            serde_json::from_slice(&row.map_err(|_| StorageError)?).map_err(|_| StorageError)
        })
        .collect()
}

fn get_record<T: serde::de::DeserializeOwned>(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
    table: &str,
    record_id: &str,
) -> Result<Option<T>, StorageError> {
    let (kind, id) = scope_parts(scope);
    let bytes = connection
        .query_row(
            &format!(
                "SELECT record_json FROM {table} WHERE scope_kind=?1 AND scope_id=?2 AND id=?3"
            ),
            params![kind, id, record_id],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(|_| StorageError)?;
    bytes
        .map(|v| serde_json::from_slice(&v).map_err(|_| StorageError))
        .transpose()
}

fn commit_on(
    tx: &rusqlite::Connection,
    commit: &MaterialCommit<'_>,
) -> Result<MaterialCommitOutcome, StorageError> {
    let record = commit.record;
    let scope = record.scope();
    if let Some((hash, response)) = load_idempotency(tx, &scope, commit.idempotency.key)? {
        if hash == commit.idempotency.request_hash {
            return Ok(MaterialCommitOutcome::Replayed(response));
        }
        insert_audit(
            tx,
            &commit.audit.clone().with_outcome(
                AuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1".to_owned()),
            ),
        )?;
        return Ok(MaterialCommitOutcome::IdempotencyMismatch);
    }
    let (kind, scope_id) = scope_parts(&scope);
    let id = record.id();
    let actual: Option<i64> = tx
        .query_row(
            &format!(
                "SELECT revision FROM {} WHERE scope_kind=?1 AND scope_id=?2 AND id=?3",
                record.table()
            ),
            params![kind, scope_id, id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| StorageError)?;
    let actual = actual
        .map(u64::try_from)
        .transpose()
        .map_err(|_| StorageError)?;
    if actual != commit.expected_revision {
        let mut audit = commit.audit.clone().with_outcome(
            AuditOutcome::Conflict,
            Some("eitmad.error.material-revision-conflict.v1".to_owned()),
        );
        audit.previous_revision = actual;
        audit.resulting_revision = actual;
        insert_audit(tx, &audit)?;
        return Ok(MaterialCommitOutcome::RevisionConflict(actual));
    }
    if let Some(outcome) = check_references(tx, commit, &scope, kind, &scope_id, &id)? {
        return Ok(outcome);
    }
    let existing_name: Option<String> = tx
        .query_row(
            &format!(
                "SELECT id FROM {} WHERE scope_kind=?1 AND scope_id=?2 AND normalized_name=?3",
                record.table()
            ),
            params![kind, scope_id, commit.normalized_name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| StorageError)?;
    if existing_name.is_some_and(|other| other != id)
        && !matches!(record, MaterialRecord::Material(_))
    {
        insert_audit(
            tx,
            &commit.audit.clone().with_outcome(
                AuditOutcome::Invalid,
                Some("eitmad.error.material-invalid.v1".to_owned()),
            ),
        )?;
        return Ok(MaterialCommitOutcome::DuplicateName);
    }
    persist_record(tx, commit, kind, &scope_id, &id)?;
    let mut audit = commit.audit.clone();
    audit.outcome = AuditOutcome::Succeeded;
    audit.previous_revision = actual;
    audit.resulting_revision = Some(record.revision());
    insert_audit(tx, &audit)?;
    let mut idempotency = commit.idempotency.clone();
    idempotency.response_json = record.json()?;
    insert_idempotency(tx, &scope, commit.operation, &idempotency)?;
    insert_publication(tx, &scope, commit.idempotency.key, commit.publication)?;
    Ok(MaterialCommitOutcome::Committed)
}

fn check_references(
    tx: &rusqlite::Connection,
    commit: &MaterialCommit<'_>,
    scope: &ScopeRef,
    kind: &str,
    scope_id: &str,
    id: &str,
) -> Result<Option<MaterialCommitOutcome>, StorageError> {
    if let MaterialRecord::Material(material) = commit.record {
        let previous = get_record::<Material>(tx, scope, "materials", id)?;
        for (table, ref_id, unchanged) in [
            (
                "material_categories",
                material.category_id.value(),
                previous
                    .as_ref()
                    .is_some_and(|v| v.category_id == material.category_id),
            ),
            (
                "material_units",
                material.unit_id.value(),
                previous
                    .as_ref()
                    .is_some_and(|v| v.unit_id == material.unit_id),
            ),
        ] {
            let archived: Option<bool> = tx
                .query_row(
                    &format!(
                        "SELECT archived FROM {table} WHERE scope_kind=?1 AND scope_id=?2 AND id=?3"
                    ),
                    params![kind, scope_id, ref_id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| StorageError)?;
            if archived.is_none_or(|value| value && !unchanged) {
                invalid_reference_audit(tx, commit)?;
                return Ok(Some(MaterialCommitOutcome::InvalidReference));
            }
        }
    }
    if let MaterialRecord::Unit(unit) = commit.record {
        if let Some(previous) = get_record::<MaterialUnit>(tx, scope, "material_units", id)? {
            let changed = previous.dimension != unit.dimension
                || previous.numerator != unit.numerator
                || previous.denominator != unit.denominator;
            let referenced: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM materials WHERE scope_kind=?1 AND scope_id=?2 AND unit_id=?3) OR EXISTS(SELECT 1 FROM part_material_usages WHERE scope_kind=?1 AND scope_id=?2 AND unit_id=?3)",
                params![kind, scope_id, id], |row| row.get(0),
            ).map_err(|_| StorageError)?;
            if changed && referenced {
                invalid_reference_audit(tx, commit)?;
                return Ok(Some(MaterialCommitOutcome::ReferencedUnitDefinition));
            }
        }
    }
    Ok(None)
}

fn invalid_reference_audit(
    tx: &rusqlite::Connection,
    commit: &MaterialCommit<'_>,
) -> Result<(), StorageError> {
    insert_audit(
        tx,
        &commit.audit.clone().with_outcome(
            AuditOutcome::Invalid,
            Some("eitmad.error.material-reference-invalid.v1".to_owned()),
        ),
    )
}

fn persist_record(
    tx: &rusqlite::Connection,
    commit: &MaterialCommit<'_>,
    kind: &str,
    scope_id: &str,
    id: &str,
) -> Result<(), StorageError> {
    let record = commit.record;
    let json = record.json()?;
    let revision = i64::try_from(record.revision()).map_err(|_| StorageError)?;
    match record {
        MaterialRecord::Material(material) => {
            tx.execute("INSERT INTO materials(scope_kind,scope_id,id,normalized_name,category_id,unit_id,archived,revision,record_json)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET
            normalized_name=excluded.normalized_name,category_id=excluded.category_id,unit_id=excluded.unit_id,archived=excluded.archived,revision=excluded.revision,record_json=excluded.record_json",
            params![kind,scope_id,id,commit.normalized_name,material.category_id.value().to_string(),material.unit_id.value().to_string(),record.archived(),revision,json]).map_err(|_| StorageError)?;
        }
        _ => {
            tx.execute(&format!("INSERT INTO {}(scope_kind,scope_id,id,normalized_name,archived,revision,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7)
            ON CONFLICT(scope_kind,scope_id,id) DO UPDATE SET normalized_name=excluded.normalized_name,archived=excluded.archived,revision=excluded.revision,record_json=excluded.record_json",record.table()),
            params![kind,scope_id,id,commit.normalized_name,record.archived(),revision,json]).map_err(|_| StorageError)?;
        }
    }
    Ok(())
}
