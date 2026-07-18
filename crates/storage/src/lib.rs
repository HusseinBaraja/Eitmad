//! Rust-owned `SQLite` access, migrations, transactions, and scoped repositories.

mod authorization;
mod configuration;
mod migrations;

use std::{
    fmt, fs,
    fs::OpenOptions,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(windows)]
use std::process::Command;

pub use authorization::{RelationshipCommitOutcome, RelationshipPageData};
pub use configuration::{ConfigurationCommitOutcome, StoredConfiguration};
use eitmad_contracts::{
    events::Event,
    identity::{ScopeId, ScopeKind, ScopeRef},
    transport::IdempotencyKey,
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{Connection, OpenFlags, OptionalExtension as _};

pub const DATABASE_FILE_NAME: &str = "eitmad.sqlite3";
pub const CURRENT_STORAGE_VERSION: u32 = 4;
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

#[derive(Clone, Debug)]
pub struct DurablePublication {
    pub event: Event,
    pub policy_changed: bool,
}

#[derive(Clone, Debug)]
pub struct PendingPublication {
    pub scope: ScopeRef,
    pub idempotency_key: IdempotencyKey,
    pub event: Event,
    pub policy_changed: bool,
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
        ensure_database_file(&path)?;
        make_file_private(&path)?;
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
            .and_then(|result| (result == "ok").then_some(()).ok_or(StorageError))?;
        let mut migration_copy = Connection::open_in_memory().map_err(|_| StorageError)?;
        {
            let backup = rusqlite::backup::Backup::new(&connection, &mut migration_copy)
                .map_err(|_| StorageError)?;
            backup
                .run_to_completion(100, Duration::from_millis(5), None)
                .map_err(|_| StorageError)?;
        }
        migrations::apply(&mut migration_copy)
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

    /// Loads a mutation event that has not completed in-process publication.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for unreadable or malformed outbox state.
    pub fn pending_publication(
        &self,
        scope: &ScopeRef,
        key: IdempotencyKey,
    ) -> Result<Option<PendingPublication>, StorageError> {
        let connection = self.open_connection()?;
        let (scope_kind, scope_id) = scope_parts(scope);
        connection
            .query_row(
                "SELECT event_json, policy_changed FROM publication_outbox
                 WHERE scope_kind = ?1 AND scope_id = ?2 AND idempotency_key = ?3",
                (scope_kind, scope_id, key.value().to_string()),
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(|_| StorageError)?
            .map(|(event, policy_changed)| {
                Ok(PendingPublication {
                    scope: scope.clone(),
                    idempotency_key: key,
                    event: serde_json::from_slice(&event).map_err(|_| StorageError)?,
                    policy_changed: match policy_changed {
                        0 => false,
                        1 => true,
                        _ => return Err(StorageError),
                    },
                })
            })
            .transpose()
    }

    /// Loads every mutation event awaiting in-process publication.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for unreadable or malformed outbox state.
    pub fn pending_publications(&self) -> Result<Vec<PendingPublication>, StorageError> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "SELECT scope_kind, scope_id, idempotency_key, event_json, policy_changed
                 FROM publication_outbox ORDER BY rowid",
            )
            .map_err(|_| StorageError)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(|_| StorageError)?;
        rows.map(|row| {
            let (scope_kind, scope_id, idempotency_key, event, policy_changed) =
                row.map_err(|_| StorageError)?;
            Ok(PendingPublication {
                scope: ScopeRef {
                    kind: ScopeKind::parse(scope_kind).map_err(|_| StorageError)?,
                    id: ScopeId::new(uuid::Uuid::parse_str(&scope_id).map_err(|_| StorageError)?),
                },
                idempotency_key: IdempotencyKey::new(
                    uuid::Uuid::parse_str(&idempotency_key).map_err(|_| StorageError)?,
                ),
                event: serde_json::from_slice(&event).map_err(|_| StorageError)?,
                policy_changed: match policy_changed {
                    0 => false,
                    1 => true,
                    _ => return Err(StorageError),
                },
            })
        })
        .collect()
    }

    /// Marks one successfully published mutation event complete.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error if the completion cannot commit.
    pub fn complete_publication(
        &self,
        scope: &ScopeRef,
        key: IdempotencyKey,
    ) -> Result<(), StorageError> {
        let connection = self.open_connection()?;
        let (scope_kind, scope_id) = scope_parts(scope);
        connection
            .execute(
                "DELETE FROM publication_outbox
                 WHERE scope_kind = ?1 AND scope_id = ?2 AND idempotency_key = ?3",
                (scope_kind, scope_id, key.value().to_string()),
            )
            .map_err(|_| StorageError)?;
        Ok(())
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

fn ensure_database_file(path: &Path) -> Result<(), StorageError> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists && path.is_file() => Ok(()),
        Err(_) => Err(StorageError),
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

fn insert_publication(
    connection: &Connection,
    scope: &ScopeRef,
    key: IdempotencyKey,
    publication: &DurablePublication,
) -> Result<(), StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    let event = serde_json::to_vec(&publication.event).map_err(|_| StorageError)?;
    connection
        .execute(
            "INSERT INTO publication_outbox
             (scope_kind, scope_id, idempotency_key, event_json, policy_changed)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                scope_kind,
                scope_id,
                key.value().to_string(),
                event,
                i64::from(publication.policy_changed),
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

#[cfg(all(not(unix), not(windows)))]
fn make_directory_private(path: &Path) -> Result<(), StorageError> {
    fs::metadata(path).map(|_| ()).map_err(|_| StorageError)
}

#[cfg(windows)]
fn make_directory_private(path: &Path) -> Result<(), StorageError> {
    make_windows_path_private(path, true)
}

#[cfg(unix)]
fn make_file_private(path: &Path) -> Result<(), StorageError> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|_| StorageError)
}

#[cfg(all(not(unix), not(windows)))]
fn make_file_private(path: &Path) -> Result<(), StorageError> {
    fs::metadata(path).map(|_| ()).map_err(|_| StorageError)
}

#[cfg(windows)]
fn make_file_private(path: &Path) -> Result<(), StorageError> {
    make_windows_path_private(path, false)
}

#[cfg(windows)]
fn make_windows_path_private(path: &Path, directory: bool) -> Result<(), StorageError> {
    let sid = current_windows_user_sid()?;
    let initial_acl = read_windows_acl(path)?;
    if windows_acl_is_private(&initial_acl) && windows_acl_contains_sid(path, &sid)? {
        return Ok(());
    }
    if windows_acl_has_explicit_entries(&initial_acl) {
        run_windows_tool(
            "icacls.exe",
            [path.as_os_str(), "/reset".as_ref(), "/q".as_ref()],
        )?;
    }
    let grant = if directory {
        format!("*{sid}:(OI)(CI)F")
    } else {
        format!("*{sid}:F")
    };
    run_windows_tool(
        "icacls.exe",
        [
            path.as_os_str(),
            "/inheritance:r".as_ref(),
            "/grant:r".as_ref(),
            grant.as_ref(),
            "/q".as_ref(),
        ],
    )?;
    let applied_acl = read_windows_acl(path)?;
    if windows_acl_is_private(&applied_acl) && windows_acl_contains_sid(path, &sid)? {
        Ok(())
    } else {
        Err(StorageError)
    }
}

#[cfg(windows)]
fn current_windows_user_sid() -> Result<String, StorageError> {
    let output = Command::new(windows_system_tool("whoami.exe"))
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(|_| StorageError)?;
    if !output.status.success() {
        return Err(StorageError);
    }
    String::from_utf8_lossy(&output.stdout)
        .split(|character: char| {
            !(character == 'S' || character == '-' || character.is_ascii_digit())
        })
        .find(|value| value.starts_with("S-1-") && value.len() > 4)
        .map(str::to_owned)
        .ok_or(StorageError)
}

#[cfg(windows)]
fn read_windows_acl(path: &Path) -> Result<String, StorageError> {
    let output = Command::new(windows_system_tool("icacls.exe"))
        .arg(path)
        .output()
        .map_err(|_| StorageError)?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
        .ok_or(StorageError)
}

#[cfg(windows)]
fn windows_acl_is_private(acl: &str) -> bool {
    acl.match_indices(":(").count() == 1 && acl.contains("(F)") && !acl.contains("(I)")
}

#[cfg(windows)]
fn windows_acl_has_explicit_entries(acl: &str) -> bool {
    acl.lines()
        .filter(|line| line.contains(":("))
        .any(|line| !line.contains("(I)"))
}

#[cfg(windows)]
fn windows_acl_contains_sid(path: &Path, sid: &str) -> Result<bool, StorageError> {
    let output = Command::new(windows_system_tool("icacls.exe"))
        .arg(path)
        .args(["/findsid", &format!("*{sid}"), "/q"])
        .output()
        .map_err(|_| StorageError)?;
    Ok(output.status.success())
}

#[cfg(windows)]
fn run_windows_tool<const N: usize>(
    tool: &str,
    arguments: [&std::ffi::OsStr; N],
) -> Result<(), StorageError> {
    Command::new(windows_system_tool(tool))
        .args(arguments)
        .output()
        .map_err(|_| StorageError)?
        .status
        .success()
        .then_some(())
        .ok_or(StorageError)
}

#[cfg(windows)]
fn windows_system_tool(name: &str) -> PathBuf {
    std::env::var_os("SystemRoot").map_or_else(
        || PathBuf::from(name),
        |root| PathBuf::from(root).join("System32").join(name),
    )
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

    #[test]
    fn compatibility_check_rejects_migration_incompatible_state_without_mutation() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(DATABASE_FILE_NAME);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY);
                 INSERT INTO schema_migrations VALUES (1);
                 CREATE TABLE configuration_scopes (
                     scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
                     schema_version INTEGER NOT NULL, revision INTEGER NOT NULL,
                     PRIMARY KEY (scope_kind, scope_id));
                 CREATE TABLE configuration_values (
                     scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
                     config_key TEXT NOT NULL, value_json TEXT NOT NULL,
                     PRIMARY KEY (scope_kind, scope_id, config_key));
                 CREATE TABLE authorization_scopes (broken TEXT);",
            )
            .unwrap();
        drop(connection);
        let before = fs::read(&path).unwrap();

        assert!(AuthorityStore::check_compatible(directory.path()).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }

    #[cfg(windows)]
    #[test]
    fn windows_private_acl_removes_broad_explicit_access() {
        let directory = TempDir::new().unwrap();
        run_windows_tool(
            "icacls.exe",
            [
                directory.path().as_os_str(),
                "/grant".as_ref(),
                "*S-1-1-0:R".as_ref(),
                "/q".as_ref(),
            ],
        )
        .unwrap();

        make_directory_private(directory.path()).unwrap();

        let acl = read_windows_acl(directory.path()).unwrap();
        assert!(windows_acl_is_private(&acl));
        assert!(
            windows_acl_contains_sid(directory.path(), &current_windows_user_sid().unwrap())
                .unwrap()
        );
    }
}
