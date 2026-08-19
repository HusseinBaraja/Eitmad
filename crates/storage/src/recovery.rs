use std::{
    fs,
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{Connection, OpenFlags, backup::Backup};
use uuid::Uuid;

use crate::{
    AuthorityStore, CorruptionCheck, DATABASE_FILE_NAME, StorageError, make_directory_private,
    make_file_private, validate_database_file, validate_database_file_with,
};

#[derive(Clone, Debug)]
pub struct RestoreOutcome {
    pub store: AuthorityStore,
    pub previous_database: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryArtifactKind {
    PreMigration,
    PreRestore,
    FailedRestore,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryArtifact {
    pub path: PathBuf,
    pub kind: RecoveryArtifactKind,
}

impl AuthorityStore {
    pub(crate) fn backup_before_pending_migration(&self) -> Result<(), StorageError> {
        let connection = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| StorageError)?;
        let version = crate::migrations::read_version(&connection)?;
        if version >= crate::CURRENT_STORAGE_VERSION {
            return Ok(());
        }
        if version != 0 && version < crate::MIN_SUPPORTED_STORAGE_VERSION {
            return Err(StorageError);
        }
        let parent = self.path.parent().ok_or(StorageError)?;
        let destination = parent.join(format!(
            "eitmad.pre-migration-v{version}-to-v{}-{}.sqlite3",
            crate::CURRENT_STORAGE_VERSION,
            Uuid::new_v4()
        ));
        self.backup_to(destination)
    }

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
        let result = create_validated_backup(&self.path, &temporary)
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
        validate_database_file_with(source.as_ref(), CorruptionCheck::Full)
    }

    /// Lists preserved recovery artifacts without opening, deleting, or trusting them.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error when the private runtime directory cannot
    /// be enumerated safely.
    pub fn recovery_artifacts(
        runtime_directory: impl AsRef<Path>,
    ) -> Result<Vec<RecoveryArtifact>, StorageError> {
        let mut artifacts = Vec::new();
        for entry in fs::read_dir(runtime_directory).map_err(|_| StorageError)? {
            let entry = entry.map_err(|_| StorageError)?;
            if !entry.file_type().map_err(|_| StorageError)?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let kind = if name.starts_with("eitmad.pre-migration-") && name.ends_with(".sqlite3") {
                Some(RecoveryArtifactKind::PreMigration)
            } else if name.starts_with("eitmad.pre-restore-") && name.ends_with(".sqlite3") {
                Some(RecoveryArtifactKind::PreRestore)
            } else if name.starts_with("eitmad.failed-restore-") && name.ends_with(".sqlite3") {
                Some(RecoveryArtifactKind::FailedRestore)
            } else {
                None
            };
            if let Some(kind) = kind {
                artifacts.push(RecoveryArtifact {
                    path: entry.path(),
                    kind,
                });
            }
        }
        artifacts.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(artifacts)
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
        create_validated_backup(source, &candidate)?;

        let previous = live
            .is_file()
            .then(|| directory.join(format!("eitmad.pre-restore-{token}.sqlite3")));
        if let Some(previous) = &previous {
            checkpoint(&live)?;
            move_database_family(&live, previous)?;
        }
        if move_database_family(&candidate, &live).is_err() {
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
                if move_database_family(&live, &failed).is_ok() {
                    if let Some(previous) = &previous {
                        if move_database_family(previous, &live).is_err() {
                            let _ = move_database_family(&failed, &live);
                        }
                    }
                }
                Err(error)
            }
        }
    }
}

fn create_validated_backup(source: &Path, destination: &Path) -> Result<(), StorageError> {
    create_private_file(destination)?;
    let result = backup_database(source, destination)
        .and_then(|()| validate_database_file_with(destination, CorruptionCheck::Full));
    if result.is_err() {
        let _ = fs::remove_file(destination);
    }
    result
}

fn create_private_file(path: &Path) -> Result<(), StorageError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path).map_err(|_| StorageError)?;
    if make_file_private(path).is_err() {
        let _ = fs::remove_file(path);
        return Err(StorageError);
    }
    Ok(())
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
    move_database_family_with(source, destination, |from, to| fs::rename(from, to))
}

fn move_database_family_with(
    source: &Path,
    destination: &Path,
    mut rename: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<(), StorageError> {
    if source == destination {
        return Err(StorageError);
    }

    let source_paths = database_family_paths(source);
    let destination_paths = database_family_paths(destination);
    if !source_paths[0].is_file() {
        return Err(StorageError);
    }

    let mut moves = Vec::new();
    for (source_path, destination_path) in source_paths.iter().zip(&destination_paths) {
        if destination_path.try_exists().map_err(|_| StorageError)? {
            return Err(StorageError);
        }
        if source_path.try_exists().map_err(|_| StorageError)? {
            moves.push((source_path.as_path(), destination_path.as_path()));
        }
    }

    let mut completed = Vec::new();
    for &(source_path, destination_path) in &moves {
        if rename(source_path, destination_path).is_err() {
            for &(moved_source, moved_destination) in completed.iter().rev() {
                let _ = rename(moved_destination, moved_source);
            }
            return Err(StorageError);
        }
        completed.push((source_path, destination_path));
    }
    Ok(())
}

fn database_family_paths(path: &Path) -> [PathBuf; 3] {
    [
        path.to_path_buf(),
        path_with_suffix(path, "-wal"),
        path_with_suffix(path, "-shm"),
    ]
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
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

    #[test]
    fn pending_migration_creates_validated_recovery_artifact_first() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let connection = Connection::open(store.path()).unwrap();
        connection
            .execute_batch(
                "DROP TABLE identity_sessions;
                 DROP TABLE identity_workspaces;
                 DROP TABLE identity_organizations;
                 DROP TABLE identity_accounts;
                 DROP TABLE identity_tenants;
                 DROP TABLE identity_users;
                 DROP TABLE identity_devices;
                 ALTER TABLE mutation_audit DROP COLUMN session_id;
                 ALTER TABLE mutation_audit DROP COLUMN device_id;
                 DELETE FROM schema_migrations WHERE version = 5;",
            )
            .unwrap();
        drop(connection);
        drop(store);

        let migrated = AuthorityStore::open(directory.path()).unwrap();
        assert!(migrated.verify_integrity(CorruptionCheck::Full).is_ok());
        let artifacts = AuthorityStore::recovery_artifacts(directory.path()).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, RecoveryArtifactKind::PreMigration);
        AuthorityStore::validate_backup(&artifacts[0].path).unwrap();
    }

    #[test]
    fn database_family_move_rolls_back_a_partial_rename() {
        let directory = TempDir::new().unwrap();
        let source = directory.path().join("source.sqlite3");
        let destination = directory.path().join("destination.sqlite3");
        for path in database_family_paths(&source) {
            fs::write(path, b"data").unwrap();
        }
        let mut calls = 0;

        let result = move_database_family_with(&source, &destination, |from, to| {
            calls += 1;
            if calls == 2 {
                return Err(io::Error::other("injected rename failure"));
            }
            fs::rename(from, to)
        });

        assert!(result.is_err());
        assert!(
            database_family_paths(&source)
                .iter()
                .all(|path| path.is_file())
        );
        assert!(
            database_family_paths(&destination)
                .iter()
                .all(|path| !path.exists())
        );
    }

    #[test]
    fn database_family_move_preflights_all_destinations() {
        let directory = TempDir::new().unwrap();
        let source = directory.path().join("source.sqlite3");
        let destination = directory.path().join("destination.sqlite3");
        fs::write(&source, b"main").unwrap();
        fs::write(path_with_suffix(&source, "-wal"), b"wal").unwrap();
        fs::write(path_with_suffix(&destination, "-shm"), b"collision").unwrap();

        assert!(move_database_family(&source, &destination).is_err());
        assert!(source.is_file());
        assert!(path_with_suffix(&source, "-wal").is_file());
        assert!(!destination.exists());
    }
}
