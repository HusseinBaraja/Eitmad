use std::{
    fs,
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use eitmad_observability_audit::MutationAuditRecord;
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
            "eitmad.pre-migration-v{version}-to-v{}.sqlite3",
            crate::CURRENT_STORAGE_VERSION
        ));
        if destination.is_file() {
            return Self::validate_backup(destination);
        }
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
            .and_then(|()| fs::hard_link(&temporary, destination).map_err(|_| StorageError));
        // Publishing a hard link fails atomically if the destination appeared meanwhile.
        let _ = fs::remove_file(&temporary);
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
        restore(runtime_directory.as_ref(), source.as_ref(), None)
    }

    /// Restores a candidate with the recovery audit and closed operator session evidence
    /// committed before installation. Newer credentials and permissions are not restored.
    /// The caller must hold exclusive stopped-engine authority and authorize the request.
    ///
    /// # Errors
    /// Returns a sanitized error without replacing the live database if staging or audit fails.
    pub fn restore_from_backup_with_audit(
        runtime_directory: impl AsRef<Path>,
        source: impl AsRef<Path>,
        audit: &MutationAuditRecord,
    ) -> Result<RestoreOutcome, StorageError> {
        restore(runtime_directory.as_ref(), source.as_ref(), Some(audit))
    }

    /// Checks installation identity and scope topology without changing the candidate.
    ///
    /// # Errors
    /// Returns a sanitized error for unreadable or incomplete identity state.
    pub fn backup_matches_authority(&self, source: impl AsRef<Path>) -> Result<bool, StorageError> {
        let source = Connection::open_with_flags(source.as_ref(), OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| StorageError)?;
        self.read_transaction(|live| Ok(recovery_identity(live)? == recovery_identity(&source)?))
    }

    /// Lists every tenant root affected by a whole-database recovery operation.
    ///
    /// # Errors
    /// Fails closed if persisted scope attribution cannot be read.
    pub fn recovery_tenants(
        &self,
    ) -> Result<Vec<eitmad_contracts::identity::TenantId>, StorageError> {
        self.read_transaction(|connection| {
            let mut statement = connection.prepare(
                "SELECT tenant_id FROM identity_tenants
                 UNION SELECT scope_id FROM authorization_scopes WHERE scope_kind='organization'
                 UNION SELECT tenant_id FROM mutation_audit WHERE tenant_id IS NOT NULL AND outcome='\"succeeded\"'"
            ).map_err(|_| StorageError)?;
            statement.query_map([], |row| row.get::<_, String>(0)).map_err(|_| StorageError)?
                .map(|row| {
                    let value = row.map_err(|_| StorageError)?;
                    Ok(eitmad_contracts::identity::TenantId::new(Uuid::parse_str(&value).map_err(|_| StorageError)?))
                }).collect()
        })
    }
}

fn recovery_identity(connection: &Connection) -> Result<Vec<Vec<String>>, StorageError> {
    let mut identity = Vec::new();
    for query in [
        "SELECT tenant_id, organization_id, device_id FROM local_installation_authority ORDER BY singleton",
        "SELECT scope_kind, scope_id FROM authorization_scopes ORDER BY scope_kind, scope_id",
        "SELECT tenant_id, branch_id FROM local_desktop_branch ORDER BY singleton",
        "SELECT tenant_id FROM identity_tenants ORDER BY tenant_id",
    ] {
        let mut statement = connection.prepare(query).map_err(|_| StorageError)?;
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|column| row.get::<_, String>(column))
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| StorageError)?;
        identity.push(vec![query.to_owned()]);
        for row in rows {
            identity.push(row.map_err(|_| StorageError)?);
        }
    }
    Ok(identity)
}

fn restore(
    directory: &Path,
    source: &Path,
    audit: Option<&MutationAuditRecord>,
) -> Result<RestoreOutcome, StorageError> {
    let live = directory.join(DATABASE_FILE_NAME);
    for path in database_family_paths(&live) {
        match fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.file_type().is_file() => return Err(StorageError),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(StorageError),
        }
    }
    if source.canonicalize().map_err(|_| StorageError)?
        == live.canonicalize().unwrap_or(live.clone())
    {
        return Err(StorageError);
    }
    validate_database_file(source)?;
    fs::create_dir_all(directory).map_err(|_| StorageError)?;
    make_directory_private(directory)?;

    let token = Uuid::new_v4();
    let candidate = directory.join(format!("eitmad.restore-{token}.sqlite3"));
    create_validated_backup(source, &candidate)?;
    if let Some(audit) = audit {
        let staged = AuthorityStore {
            path: candidate.clone(),
        };
        let result = (|| {
            // Check the private staged copy, so a changed source cannot bypass identity checks.
            if !(AuthorityStore { path: live.clone() }).backup_matches_authority(&candidate)? {
                return Err(StorageError);
            }
            let mut connection = staged.open_connection()?;
            crate::migrations::apply(&mut connection)?;
            connection
                .execute(
                    "ATTACH DATABASE ?1 AS recovery_live",
                    [live.to_str().ok_or(StorageError)?],
                )
                .map_err(|_| StorageError)?;
            let transaction = connection.transaction().map_err(|_| StorageError)?;
            crate::identity::preserve_recovery_session(&transaction, audit)?;
            crate::insert_audit(&transaction, audit)?;
            transaction.commit().map_err(|_| StorageError)?;
            drop(connection);
            checkpoint(&candidate)?;
            AuthorityStore::validate_backup(&candidate)
        })();
        if result.is_err() {
            for path in database_family_paths(&candidate) {
                let _ = fs::remove_file(path);
            }
            return Err(StorageError);
        }
    }

    let previous = live
        .is_file()
        .then(|| directory.join(format!("eitmad.pre-restore-{token}.sqlite3")));
    if let Some(previous) = &previous {
        if checkpoint(&live)
            .and_then(|()| move_database_family(&live, previous))
            .is_err()
        {
            for path in database_family_paths(&candidate) {
                let _ = fs::remove_file(path);
            }
            return Err(StorageError);
        }
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
    let mut output = Connection::open(destination).map_err(|_| StorageError)?;
    // A new private snapshot has no previous state to recover. Keep its rollback
    // journal in memory so a public destination directory cannot expose sidecars.
    output
        .execute_batch("PRAGMA journal_mode=MEMORY;")
        .map_err(|_| StorageError)?;
    let backup = Backup::new(&source, &mut output).map_err(|_| StorageError)?;
    backup
        .run_to_completion(100, Duration::from_millis(5), None)
        .map_err(|_| StorageError)?;
    drop(backup);
    drop(output);
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(destination)
        .map_err(|_| StorageError)?
        .sync_all()
        .map_err(|_| StorageError)
}

fn checkpoint(path: &Path) -> Result<(), StorageError> {
    let connection = Connection::open(path).map_err(|_| StorageError)?;
    let busy: i64 = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |row| row.get(0))
        .map_err(|_| StorageError)?;
    if busy == 0 { Ok(()) } else { Err(StorageError) }
}

fn move_database_family(source: &Path, destination: &Path) -> Result<(), StorageError> {
    move_database_family_with(source, destination, |from, to| {
        fs::hard_link(from, to)?;
        if let Err(error) = fs::remove_file(from) {
            let _ = fs::remove_file(to);
            return Err(error);
        }
        Ok(())
    })
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
    fn restore_checkpoint_failure_removes_candidate_and_preserves_live_database() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        store
            .local_authorization_context(eitmad_contracts::transport::UnixMillis(1))
            .unwrap();
        let backup = directory.path().join("checkpoint-backup.sqlite3");
        store.backup_to(&backup).unwrap();
        let reader = Connection::open(store.path()).unwrap();
        reader
            .execute_batch("BEGIN; SELECT * FROM mutation_audit;")
            .unwrap();
        let writer = Connection::open(store.path()).unwrap();
        writer
            .execute_batch("UPDATE local_installation_authority SET device_id=device_id;")
            .unwrap();
        let before = fs::read(store.path()).unwrap();
        let wal = path_with_suffix(store.path(), "-wal");
        let before_wal = fs::read(&wal).unwrap();
        assert!(AuthorityStore::restore_from_backup(directory.path(), &backup).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), before);
        assert_eq!(fs::read(&wal).unwrap(), before_wal);
        assert!(fs::read_dir(directory.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("eitmad.restore-")
        }));
        assert!(
            AuthorityStore::recovery_artifacts(directory.path())
                .unwrap()
                .is_empty()
        );
        reader.execute_batch("ROLLBACK;").unwrap();
        drop(writer);
        drop(reader);
        store.verify_integrity(CorruptionCheck::Full).unwrap();
        AuthorityStore::validate_backup(&backup).unwrap();
    }

    #[test]
    fn failed_restore_audit_cannot_replace_the_live_database() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let actor = store
            .local_authorization_context(eitmad_contracts::transport::UnixMillis(1))
            .unwrap();
        let backup = directory.path().join("audit-backup.sqlite3");
        store.backup_to(&backup).unwrap();
        let before = fs::read(store.path()).unwrap();
        let invalid = MutationAuditRecord::from_authorization(
            &actor,
            eitmad_contracts::transport::UnixMillis(2),
            eitmad_contracts::transport::CorrelationId::new(Uuid::new_v4()),
            "",
            eitmad_observability_audit::AuditTarget {
                kind: "local-authority".into(),
                identifiers: vec![],
            },
        );
        assert!(
            AuthorityStore::restore_from_backup_with_audit(directory.path(), &backup, &invalid)
                .is_err()
        );
        assert!(before == fs::read(store.path()).unwrap());
        assert!(
            AuthorityStore::recovery_artifacts(directory.path())
                .unwrap()
                .is_empty()
        );
    }

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
                "DROP TABLE work_order_confirmed_history;
                 DELETE FROM schema_migrations WHERE version = 28;
                 DROP TABLE order_confirmed_history;
                 DROP TABLE order_pending;
                 DELETE FROM schema_migrations WHERE version = 27;
                 DROP TABLE quotation_confirmed_history;
                 DELETE FROM schema_migrations WHERE version = 26;
                 DROP TABLE quotation_draft_outbox;
                 DROP TABLE quotation_drafts;
                 DELETE FROM schema_migrations WHERE version = 25;
                 DELETE FROM schema_migrations WHERE version = 24;
                 DROP TABLE catalog_sync_clients;
                 DROP TABLE catalog_sales_records;
                 DROP TABLE catalog_sync_checkpoints;
                 DROP TABLE catalog_sync_exceptions;
                 DROP TABLE catalog_sync_revisions;
                 DELETE FROM schema_migrations WHERE version = 23;
                 DELETE FROM schema_migrations WHERE version = 22;
                 DROP TABLE pricing_intents;
                 DROP TABLE pricing_revisions;
                 DELETE FROM schema_migrations WHERE version = 21;
                 DELETE FROM schema_migrations WHERE version = 20;
                 DELETE FROM schema_migrations WHERE version = 19;
                 DROP TABLE catalog_image_uploads;
                 DROP TABLE catalog_images;
                 DELETE FROM schema_migrations WHERE version = 18;
                 DROP TABLE furniture_part_compositions;
                 DROP TABLE furniture_option_identities;
                 DROP TABLE furniture_revisions;
                 DROP TABLE furnitures;
                 DROP TABLE furniture_categories;
                 DELETE FROM schema_migrations WHERE version = 17;
                 DROP TABLE product_variants;
                 DROP TABLE product_revisions;
                 DROP TABLE products;
                 DROP TABLE product_categories;
                 DELETE FROM schema_migrations WHERE version = 16;
                 DROP TABLE part_material_usages;
                 DROP TABLE part_compositions;
                 DROP TABLE parts;
                 DROP TABLE part_categories;
                 DELETE FROM schema_migrations WHERE version = 15;
                 DROP TABLE materials;
                 DROP TABLE material_units;
                 DROP TABLE material_categories;
                 DELETE FROM schema_migrations WHERE version = 14;
                 DROP TABLE customer_sync_exceptions;
                 DELETE FROM schema_migrations WHERE version = 13;
                 DROP TABLE local_desktop_branch;
                 DELETE FROM schema_migrations WHERE version = 12;
                 DROP TABLE customer_sync_outbox;
                 DROP TABLE customers;
                 DELETE FROM schema_migrations WHERE version = 11;
                 DROP TABLE desktop_accounts;
                 DELETE FROM schema_migrations WHERE version = 10;
                 DROP TABLE local_installation_authority;
                 DELETE FROM schema_migrations WHERE version = 9;
                 DROP TABLE reference_marker_sync_outbox;
                 DROP TABLE reference_markers;
                 DELETE FROM schema_migrations WHERE version = 8;
                 DROP TABLE sync_scopes;
                 DELETE FROM schema_migrations WHERE version = 7;
                 ALTER TABLE mutation_audit DROP COLUMN extension_points;
                 ALTER TABLE mutation_audit DROP COLUMN redacted_error;
                 ALTER TABLE mutation_audit DROP COLUMN target;
                 ALTER TABLE mutation_audit DROP COLUMN workspace_id;
                 ALTER TABLE mutation_audit DROP COLUMN tenant_id;
                 DELETE FROM schema_migrations WHERE version = 6;
                 DROP TABLE identity_sessions;
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

        store.backup_before_pending_migration().unwrap();
        store.backup_before_pending_migration().unwrap();
        let pending = AuthorityStore::recovery_artifacts(directory.path()).unwrap();
        assert_eq!(pending.len(), 1);
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
