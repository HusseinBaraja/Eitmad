use rusqlite::{Connection, OptionalExtension as _};

use crate::{CURRENT_STORAGE_VERSION, StorageError};

const MIGRATIONS: &[&str] = &[
    "CREATE TABLE configuration_scopes (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         schema_version INTEGER NOT NULL,
         revision INTEGER NOT NULL,
         PRIMARY KEY (scope_kind, scope_id)
     );
     CREATE TABLE configuration_values (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         config_key TEXT NOT NULL,
         value_json TEXT NOT NULL,
         PRIMARY KEY (scope_kind, scope_id, config_key),
         FOREIGN KEY (scope_kind, scope_id)
             REFERENCES configuration_scopes(scope_kind, scope_id) ON DELETE CASCADE
     );",
    "CREATE TABLE authorization_scopes (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         policy_schema_version INTEGER NOT NULL,
         policy_version INTEGER NOT NULL,
         PRIMARY KEY (scope_kind, scope_id)
     );
     CREATE TABLE scope_relationships (
         relationship_id TEXT PRIMARY KEY,
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         principal_id TEXT NOT NULL,
         principal_kind TEXT NOT NULL,
         relation TEXT NOT NULL,
         UNIQUE (scope_kind, scope_id, principal_id, principal_kind, relation),
         FOREIGN KEY (scope_kind, scope_id)
             REFERENCES authorization_scopes(scope_kind, scope_id) ON DELETE CASCADE
     );
     CREATE INDEX scope_relationships_page
         ON scope_relationships(scope_kind, scope_id, relationship_id);",
    "CREATE TABLE mutation_audit (
         audit_id TEXT PRIMARY KEY,
         occurred_at INTEGER NOT NULL,
         principal_id TEXT NOT NULL,
         principal_kind TEXT NOT NULL,
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         correlation_id TEXT NOT NULL,
         causation_id TEXT,
         idempotency_key TEXT,
         operation TEXT NOT NULL,
         outcome TEXT NOT NULL,
         previous_revision INTEGER,
         resulting_revision INTEGER,
         changed_identifiers TEXT NOT NULL,
         error_code TEXT
     );
     CREATE TRIGGER mutation_audit_no_update
         BEFORE UPDATE ON mutation_audit BEGIN SELECT RAISE(ABORT, 'audit is append-only'); END;
     CREATE TRIGGER mutation_audit_no_delete
         BEFORE DELETE ON mutation_audit BEGIN SELECT RAISE(ABORT, 'audit is append-only'); END;
     CREATE TABLE idempotency_records (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         idempotency_key TEXT NOT NULL,
         operation TEXT NOT NULL,
         request_hash BLOB NOT NULL,
         response_json BLOB NOT NULL,
         PRIMARY KEY (scope_kind, scope_id, idempotency_key)
     );",
    "CREATE TABLE publication_outbox (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         idempotency_key TEXT NOT NULL,
         event_json BLOB NOT NULL,
         policy_changed INTEGER NOT NULL CHECK (policy_changed IN (0, 1)),
         PRIMARY KEY (scope_kind, scope_id, idempotency_key)
     );",
];

pub(crate) fn apply(connection: &mut Connection) -> Result<(), StorageError> {
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS schema_migrations (
                 version INTEGER PRIMARY KEY
             );",
        )
        .map_err(|_| StorageError)?;
    let current = read_version(connection)?;
    if current > CURRENT_STORAGE_VERSION {
        return Err(StorageError);
    }
    for version in (current + 1)..=CURRENT_STORAGE_VERSION {
        let migration = MIGRATIONS
            .get(usize::try_from(version - 1).map_err(|_| StorageError)?)
            .ok_or(StorageError)?;
        let transaction = connection.transaction().map_err(|_| StorageError)?;
        transaction
            .execute_batch(migration)
            .map_err(|_| StorageError)?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version) VALUES (?1)",
                [version],
            )
            .map_err(|_| StorageError)?;
        transaction.commit().map_err(|_| StorageError)?;
    }
    Ok(())
}

pub(crate) fn read_version(connection: &Connection) -> Result<u32, StorageError> {
    let table_exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(|_| StorageError)?
        .is_some();
    if !table_exists {
        return Ok(0);
    }
    connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|_| StorageError)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn migrates_a_version_one_database_without_losing_configuration() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("legacy.sqlite3");
        let mut connection = Connection::open(&path).unwrap();
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
                     PRIMARY KEY (scope_kind, scope_id, config_key),
                     FOREIGN KEY (scope_kind, scope_id)
                       REFERENCES configuration_scopes(scope_kind, scope_id) ON DELETE CASCADE);
                 INSERT INTO configuration_scopes VALUES ('organization', 'scope-1', 1, 7);
                 INSERT INTO configuration_values VALUES
                   ('organization', 'scope-1', 'eitmad.config.locale.primary.v1',
                    '{\"kind\":\"text\",\"value\":\"ar-YE\"}');",
            )
            .unwrap();

        apply(&mut connection).unwrap();

        assert_eq!(read_version(&connection).unwrap(), CURRENT_STORAGE_VERSION);
        let value: String = connection
            .query_row("SELECT value_json FROM configuration_values", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(value.contains("ar-YE"));
    }

    #[test]
    fn rejects_a_newer_database_without_mutating_it() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY);
                 INSERT INTO schema_migrations VALUES (999);",
            )
            .unwrap();
        assert!(apply(&mut connection).is_err());
        assert_eq!(read_version(&connection).unwrap(), 999);
    }

    #[test]
    fn failed_migration_rolls_back_its_version_and_partial_schema() {
        let mut connection = Connection::open_in_memory().unwrap();
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

        assert!(apply(&mut connection).is_err());
        assert_eq!(read_version(&connection).unwrap(), 1);
        let relationships_exist: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name = 'scope_relationships'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(relationships_exist, 0);
    }
}
