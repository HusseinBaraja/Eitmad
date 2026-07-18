use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{Connection, OpenFlags, backup::Backup};
use uuid::Uuid;

use crate::{
    AuthorityStore, DATABASE_FILE_NAME, StorageError, make_directory_private, make_file_private,
    validate_database_file,
};

#[derive(Clone, Debug)]
pub struct RestoreOutcome {
    pub store: AuthorityStore,
    pub previous_database: Option<PathBuf>,
}

impl AuthorityStore {
    /// Creates a consistent `SQLite` backup, including committed WAL state.
    ///
    /// The destination must not already exist.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error when backup creation or validation fails.
    pub fn backup_to(&self, destination: impl AsRef<Path>) -> Result<(), StorageError> {
        let destination = destination.as_ref();
        if destination.exists() || destination == self.path {
            return Err(StorageError);
        }
        let parent = destination.parent().ok_or(StorageError)?;
        fs::create_dir_all(parent).map_err(|_| StorageError)?;
        let temporary = parent.join(format!(".eitmad-backup-{}.sqlite3", Uuid::new_v4()));
        let result = backup_database(&self.path, &temporary)
            .and_then(|()| make_file_private(&temporary))
            .and_then(|()| validate_database_file(&temporary))
            .and_then(|()| fs::rename(&temporary, destination).map_err(|_| StorageError));
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// Validates a backup without changing it.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for corrupt, drifted, or incompatible state.
    pub fn validate_backup(source: impl AsRef<Path>) -> Result<(), StorageError> {
        validate_database_file(source.as_ref())
    }

    /// Restores a validated backup while the caller holds exclusive engine authority.
    ///
    /// A replaced database is preserved beside the live database for manual recovery.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error and restores the previous database when possible.
    pub fn restore_from_backup(
        runtime_directory: impl AsRef<Path>,
        source: impl AsRef<Path>,
    ) -> Result<RestoreOutcome, StorageError> {
        let directory = runtime_directory.as_ref();
        let source = source.as_ref();
        validate_database_file(source)?;
        fs::create_dir_all(directory).map_err(|_| StorageError)?;
        make_directory_private(directory)?;

        let live = directory.join(DATABASE_FILE_NAME);
        let token = Uuid::new_v4();
        let candidate = directory.join(format!("eitmad.restore-{token}.sqlite3"));
        backup_database(source, &candidate)?;
        make_file_private(&candidate)?;
        validate_database_file(&candidate)?;

        let previous = live
            .is_file()
            .then(|| directory.join(format!("eitmad.pre-restore-{token}.sqlite3")));
        if let Some(previous) = &previous {
            checkpoint(&live)?;
            move_database_family(&live, previous)?;
        }
        if fs::rename(&candidate, &live).is_err() {
            if let Some(previous) = &previous {
                let _ = move_database_family(previous, &live);
            }
            let _ = fs::remove_file(&candidate);
            return Err(StorageError);
        }

        match AuthorityStore::open(directory) {
            Ok(store) => Ok(RestoreOutcome {
                store,
                previous_database: previous,
            }),
            Err(error) => {
                let failed = directory.join(format!("eitmad.failed-restore-{token}.sqlite3"));
                let _ = move_database_family(&live, &failed);
                if let Some(previous) = &previous {
                    let _ = move_database_family(previous, &live);
                }
                Err(error)
            }
        }
    }
}

fn backup_database(source: &Path, destination: &Path) -> Result<(), StorageError> {
    let source = Connection::open_with_flags(
        source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| StorageError)?;
    let mut destination = Connection::open(destination).map_err(|_| StorageError)?;
    let backup = Backup::new(&source, &mut destination).map_err(|_| StorageError)?;
    backup
        .run_to_completion(100, Duration::from_millis(5), None)
        .map_err(|_| StorageError)
}

fn checkpoint(path: &Path) -> Result<(), StorageError> {
    let connection = Connection::open(path).map_err(|_| StorageError)?;
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|_| StorageError)
}

fn move_database_family(source: &Path, destination: &Path) -> Result<(), StorageError> {
    fs::rename(source, destination).map_err(|_| StorageError)?;
    for suffix in ["-wal", "-shm"] {
        let source_companion = PathBuf::from(format!("{}{suffix}", source.display()));
        if source_companion.exists() {
            let destination_companion = PathBuf::from(format!("{}{suffix}", destination.display()));
            fs::rename(source_companion, destination_companion).map_err(|_| StorageError)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn backup_captures_committed_wal_data_and_restore_preserves_previous_database() {
        let first = TempDir::new().unwrap();
        let store = AuthorityStore::open(first.path()).unwrap();
        let connection = Connection::open(store.path()).unwrap();
        connection
            .execute("INSERT INTO mutation_audit (audit_id, occurred_at, principal_id, principal_kind, scope_kind, scope_id, correlation_id, operation, outcome, changed_identifiers) VALUES ('backup-row', 1, 'p', '\"user\"', 'organization', 's', 'c', 'test', '\"succeeded\"', '[]')", [])
            .unwrap();

        let backup = first.path().join("backup.sqlite3");
        store.backup_to(&backup).unwrap();
        AuthorityStore::validate_backup(&backup).unwrap();

        let second = TempDir::new().unwrap();
        let old = AuthorityStore::open(second.path()).unwrap();
        drop(old);
        let restored = AuthorityStore::restore_from_backup(second.path(), &backup).unwrap();
        assert!(restored.previous_database.unwrap().is_file());
        let restored_connection = Connection::open(restored.store.path()).unwrap();
        let count: i64 = restored_connection
            .query_row(
                "SELECT COUNT(*) FROM mutation_audit WHERE audit_id = 'backup-row'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn corrupt_restore_does_not_replace_live_database() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let corrupt = directory.path().join("corrupt.sqlite3");
        fs::write(&corrupt, b"not sqlite").unwrap();
        assert!(AuthorityStore::restore_from_backup(directory.path(), &corrupt).is_err());
        assert!(AuthorityStore::check_compatible(directory.path()).is_ok());
        assert_eq!(store.path(), directory.path().join(DATABASE_FILE_NAME));
    }
}
