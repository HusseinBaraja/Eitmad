//! Rust-owned `SQLite` access, migrations, transactions, and scoped repositories.

mod authorization;
mod configuration;
mod migrations;

use std::{
    fmt, fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub use authorization::{RelationshipCommitOutcome, RelationshipPageData};
pub use configuration::{ConfigurationCommitOutcome, StoredConfiguration};
use eitmad_contracts::{identity::ScopeRef, transport::IdempotencyKey};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{Connection, OpenFlags, OptionalExtension as _};

pub const DATABASE_FILE_NAME: &str = "eitmad.sqlite3";
pub const CURRENT_STORAGE_VERSION: u32 = 3;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
pub struct AuthorityStore {
    path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageError;

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("authoritative storage is unavailable")
    }
}

impl std::error::Error for StorageError {}

#[derive(Clone, Debug)]
pub struct DurableIdempotency {
    pub key: IdempotencyKey,
    pub request_hash: [u8; 32],
    pub response_json: Vec<u8>,
}

type StoredIdempotency = ([u8; 32], Vec<u8>);

impl AuthorityStore {
    /// Opens and migrates the Rust-owned authority database.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error when the directory, database, or a
    /// migration cannot be opened or committed.
    pub fn open(runtime_directory: impl AsRef<Path>) -> Result<Self, StorageError> {
        let directory = runtime_directory.as_ref();
        fs::create_dir_all(directory).map_err(|_| StorageError)?;
        make_directory_private(directory)?;
        let path = directory.join(DATABASE_FILE_NAME);
        let store = Self { path };
        let mut connection = store.open_connection()?;
        migrations::apply(&mut connection)?;
        make_file_private(&store.path)?;
        Ok(store)
    }

    /// Checks that an existing database is readable and not newer than this engine.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for a missing, unreadable, corrupt, or
    /// newer database. This check never creates or migrates state.
    pub fn check_compatible(runtime_directory: impl AsRef<Path>) -> Result<(), StorageError> {
        let path = runtime_directory.as_ref().join(DATABASE_FILE_NAME);
        if !path.is_file() {
            return Err(StorageError);
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| StorageError)?;
        let version = migrations::read_version(&connection)?;
        if version > CURRENT_STORAGE_VERSION {
            return Err(StorageError);
        }
        connection
            .query_row("PRAGMA quick_check(1)", [], |row| row.get::<_, String>(0))
            .map_err(|_| StorageError)
            .and_then(|result| (result == "ok").then_some(()).ok_or(StorageError))
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends a sanitized mutation audit record.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error if the append cannot commit.
    pub fn append_audit(&self, record: &MutationAuditRecord) -> Result<(), StorageError> {
        let connection = self.open_connection()?;
        insert_audit(&connection, record)
    }

    fn open_connection(&self) -> Result<Connection, StorageError> {
        let connection = Connection::open(&self.path).map_err(|_| StorageError)?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .map_err(|_| StorageError)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(|_| StorageError)?;
        Ok(connection)
    }
}

fn scope_parts(scope: &ScopeRef) -> (&str, String) {
    (scope.kind.as_str(), scope.id.value().to_string())
}

fn load_idempotency(
    connection: &Connection,
    scope: &ScopeRef,
    key: IdempotencyKey,
) -> Result<Option<StoredIdempotency>, StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    let stored = connection
        .query_row(
            "SELECT request_hash, response_json FROM idempotency_records
             WHERE scope_kind = ?1 AND scope_id = ?2 AND idempotency_key = ?3",
            (scope_kind, scope_id, key.value().to_string()),
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
        )
        .optional()
        .map_err(|_| StorageError)?;
    stored
        .map(|(hash, response)| {
            let hash: [u8; 32] = hash.try_into().map_err(|_| StorageError)?;
            Ok((hash, response))
        })
        .transpose()
}

fn insert_idempotency(
    connection: &Connection,
    scope: &ScopeRef,
    operation: &str,
    idempotency: &DurableIdempotency,
) -> Result<(), StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    connection
        .execute(
            "INSERT INTO idempotency_records
             (scope_kind, scope_id, idempotency_key, operation, request_hash, response_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                scope_kind,
                scope_id,
                idempotency.key.value().to_string(),
                operation,
                idempotency.request_hash.as_slice(),
                idempotency.response_json.as_slice(),
            ),
        )
        .map_err(|_| StorageError)?;
    Ok(())
}

fn insert_audit(connection: &Connection, record: &MutationAuditRecord) -> Result<(), StorageError> {
    let (scope_kind, scope_id) = scope_parts(&record.scope);
    let principal_kind = serde_json::to_string(&record.principal_kind).map_err(|_| StorageError)?;
    let changed = serde_json::to_string(&record.changed_identifiers).map_err(|_| StorageError)?;
    let outcome = serde_json::to_string(&record.outcome).map_err(|_| StorageError)?;
    connection
        .execute(
            "INSERT INTO mutation_audit
             (audit_id, occurred_at, principal_id, principal_kind, scope_kind, scope_id,
              correlation_id, causation_id, idempotency_key, operation, outcome,
              previous_revision, resulting_revision, changed_identifiers, error_code)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                record.audit_id.to_string(),
                record.occurred_at.0,
                record.principal_id.value().to_string(),
                principal_kind,
                scope_kind,
                scope_id,
                record.correlation_id.value().to_string(),
                record.causation_id.map(|id| id.value().to_string()),
                record.idempotency_key.map(|id| id.value().to_string()),
                record.operation,
                outcome,
                record
                    .previous_revision
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| StorageError)?,
                record
                    .resulting_revision
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| StorageError)?,
                changed,
                record.error_code,
            ],
        )
        .map_err(|_| StorageError)?;
    Ok(())
}

#[cfg(unix)]
fn make_directory_private(path: &Path) -> Result<(), StorageError> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| StorageError)
}

#[cfg(not(unix))]
fn make_directory_private(path: &Path) -> Result<(), StorageError> {
    fs::metadata(path).map(|_| ()).map_err(|_| StorageError)
}

#[cfg(unix)]
fn make_file_private(path: &Path) -> Result<(), StorageError> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|_| StorageError)
}

#[cfg(not(unix))]
fn make_file_private(path: &Path) -> Result<(), StorageError> {
    fs::metadata(path).map(|_| ()).map_err(|_| StorageError)
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        identity::{PrincipalId, PrincipalKind, ScopeId, ScopeKind, ScopeRef},
        transport::{CorrelationId, UnixMillis},
    };
    use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
    use rusqlite::Connection;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn audit_rows_are_append_only() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        store
            .append_audit(&MutationAuditRecord {
                audit_id: Uuid::from_u128(1),
                occurred_at: UnixMillis(1),
                principal_id: PrincipalId::new(Uuid::from_u128(2)),
                principal_kind: PrincipalKind::User,
                scope: ScopeRef {
                    kind: ScopeKind::parse("organization").unwrap(),
                    id: ScopeId::new(Uuid::from_u128(3)),
                },
                correlation_id: CorrelationId::new(Uuid::from_u128(4)),
                causation_id: None,
                idempotency_key: None,
                operation: "test".to_owned(),
                outcome: AuditOutcome::Succeeded,
                previous_revision: None,
                resulting_revision: None,
                changed_identifiers: Vec::new(),
                error_code: None,
            })
            .unwrap();

        let connection = Connection::open(store.path()).unwrap();
        assert!(
            connection
                .execute("DELETE FROM mutation_audit", [])
                .is_err()
        );
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM mutation_audit", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
