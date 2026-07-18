use std::fmt::Write as _;

use rusqlite::{Connection, OptionalExtension as _, params};
use sha2::{Digest as _, Sha256};

use crate::{CURRENT_STORAGE_VERSION, StorageError, authorization, configuration};

const HISTORY_SCHEMA: &str = "CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    migration_id TEXT NOT NULL UNIQUE,
    feature TEXT NOT NULL,
    checksum TEXT NOT NULL
);";

const CORE_MIGRATIONS: &[Migration] = &[
    Migration::new(
        3,
        "storage.audit-idempotency.v1",
        "storage",
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
    ),
    Migration::new(
        4,
        "storage.publication-outbox.v1",
        "storage",
        "CREATE TABLE publication_outbox (
             scope_kind TEXT NOT NULL,
             scope_id TEXT NOT NULL,
             idempotency_key TEXT NOT NULL,
             event_json BLOB NOT NULL,
             policy_changed INTEGER NOT NULL CHECK (policy_changed IN (0, 1)),
             PRIMARY KEY (scope_kind, scope_id, idempotency_key)
         );",
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Migration {
    pub(crate) version: u32,
    pub(crate) id: &'static str,
    pub(crate) feature: &'static str,
    pub(crate) sql: &'static str,
}

impl Migration {
    pub(crate) const fn new(
        version: u32,
        id: &'static str,
        feature: &'static str,
        sql: &'static str,
    ) -> Self {
        Self {
            version,
            id,
            feature,
            sql,
        }
    }

    fn checksum(self) -> String {
        let digest = Sha256::digest(
            [
                self.id.as_bytes(),
                &[0],
                self.feature.as_bytes(),
                &[0],
                self.sql.as_bytes(),
            ]
            .concat(),
        );
        digest
            .iter()
            .fold(String::with_capacity(64), |mut value, byte| {
                write!(value, "{byte:02x}").expect("writing to a String cannot fail");
                value
            })
    }
}

fn registry() -> Vec<Migration> {
    configuration::MIGRATIONS
        .iter()
        .chain(authorization::MIGRATIONS)
        .chain(CORE_MIGRATIONS)
        .copied()
        .collect()
}

pub(crate) fn apply(connection: &mut Connection) -> Result<(), StorageError> {
    let migrations = registry();
    if migrations.len() != usize::try_from(CURRENT_STORAGE_VERSION).map_err(|_| StorageError)? {
        return Err(StorageError);
    }
    apply_registry(connection, &migrations)
}

fn apply_registry(
    connection: &mut Connection,
    migrations: &[Migration],
) -> Result<(), StorageError> {
    validate_registry(migrations)?;
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|_| StorageError)?;
    prepare_history(connection, migrations)?;
    let applied = verify_history(connection, migrations)?;
    for migration in migrations.iter().skip(applied) {
        let transaction = connection.transaction().map_err(|_| StorageError)?;
        transaction
            .execute_batch(migration.sql)
            .map_err(|_| StorageError)?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version, migration_id, feature, checksum)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    migration.version,
                    migration.id,
                    migration.feature,
                    migration.checksum()
                ],
            )
            .map_err(|_| StorageError)?;
        transaction.commit().map_err(|_| StorageError)?;
    }
    verify_history(connection, migrations)?;
    verify_schema(connection, migrations)
}

fn validate_registry(migrations: &[Migration]) -> Result<(), StorageError> {
    for (index, migration) in migrations.iter().enumerate() {
        let expected = u32::try_from(index + 1).map_err(|_| StorageError)?;
        if migration.version != expected
            || migration.id.is_empty()
            || migration.feature.is_empty()
            || migration.sql.trim().is_empty()
            || migrations[..index]
                .iter()
                .any(|previous| previous.id == migration.id)
        {
            return Err(StorageError);
        }
    }
    Ok(())
}

fn prepare_history(
    connection: &mut Connection,
    migrations: &[Migration],
) -> Result<(), StorageError> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(|_| StorageError)?
        .is_some();
    if !exists {
        return connection
            .execute_batch(HISTORY_SCHEMA)
            .map_err(|_| StorageError);
    }
    if history_has_column(connection, "migration_id")? {
        return Ok(());
    }
    let legacy_version = read_version(connection)?;
    if legacy_version > CURRENT_STORAGE_VERSION {
        return Err(StorageError);
    }
    let transaction = connection.transaction().map_err(|_| StorageError)?;
    transaction
        .execute_batch("ALTER TABLE schema_migrations RENAME TO schema_migrations_legacy;")
        .and_then(|()| transaction.execute_batch(HISTORY_SCHEMA))
        .map_err(|_| StorageError)?;
    for migration in migrations
        .iter()
        .take(usize::try_from(legacy_version).map_err(|_| StorageError)?)
    {
        transaction
            .execute(
                "INSERT INTO schema_migrations(version, migration_id, feature, checksum)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    migration.version,
                    migration.id,
                    migration.feature,
                    migration.checksum()
                ],
            )
            .map_err(|_| StorageError)?;
    }
    transaction
        .execute_batch("DROP TABLE schema_migrations_legacy;")
        .map_err(|_| StorageError)?;
    transaction.commit().map_err(|_| StorageError)
}

fn history_has_column(connection: &Connection, column: &str) -> Result<bool, StorageError> {
    let mut statement = connection
        .prepare("PRAGMA table_info(schema_migrations)")
        .map_err(|_| StorageError)?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|_| StorageError)?;
    for name in names {
        if name.map_err(|_| StorageError)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn verify_history(
    connection: &Connection,
    migrations: &[Migration],
) -> Result<usize, StorageError> {
    let mut statement = connection
        .prepare(
            "SELECT version, migration_id, feature, checksum
             FROM schema_migrations ORDER BY version",
        )
        .map_err(|_| StorageError)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| StorageError)?;
    let mut applied = 0;
    for row in rows {
        let stored = row.map_err(|_| StorageError)?;
        let expected = migrations.get(applied).ok_or(StorageError)?;
        if stored
            != (
                expected.version,
                expected.id.to_owned(),
                expected.feature.to_owned(),
                expected.checksum(),
            )
        {
            return Err(StorageError);
        }
        applied += 1;
    }
    Ok(applied)
}

fn verify_schema(connection: &Connection, migrations: &[Migration]) -> Result<(), StorageError> {
    let expected = Connection::open_in_memory().map_err(|_| StorageError)?;
    expected
        .execute_batch(HISTORY_SCHEMA)
        .map_err(|_| StorageError)?;
    for migration in migrations {
        expected
            .execute_batch(migration.sql)
            .map_err(|_| StorageError)?;
    }
    (schema_entries(connection)? == schema_entries(&expected)?)
        .then_some(())
        .ok_or(StorageError)
}

fn schema_entries(
    connection: &Connection,
) -> Result<Vec<(String, String, String, String)>, StorageError> {
    let mut statement = connection
        .prepare(
            "SELECT type, name, tbl_name, sql FROM sqlite_schema
             WHERE name NOT LIKE 'sqlite_%' AND sql IS NOT NULL
             ORDER BY type, name",
        )
        .map_err(|_| StorageError)?;
    statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                normalize_sql(&row.get::<_, String>(3)?),
            ))
        })
        .map_err(|_| StorageError)?
        .map(|row| row.map_err(|_| StorageError))
        .collect()
}

fn normalize_sql(sql: &str) -> String {
    sql.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

pub(crate) fn read_version(connection: &Connection) -> Result<u32, StorageError> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(|_| StorageError)?
        .is_some();
    if !exists {
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

    use super::*;

    #[test]
    fn migrates_legacy_history_and_preserves_configuration() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY); INSERT INTO schema_migrations VALUES (1);")
            .unwrap();
        connection
            .execute_batch(configuration::MIGRATIONS[0].sql)
            .unwrap();
        connection.execute_batch(
            "INSERT INTO configuration_scopes VALUES ('organization', 'scope-1', 1, 7);
             INSERT INTO configuration_values VALUES ('organization', 'scope-1', 'eitmad.config.locale.primary.v1', '{\"kind\":\"text\",\"value\":\"ar-YE\"}');",
        ).unwrap();

        apply(&mut connection).unwrap();

        assert_eq!(read_version(&connection).unwrap(), CURRENT_STORAGE_VERSION);
        let value: String = connection
            .query_row("SELECT value_json FROM configuration_values", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(value.contains("ar-YE"));
        let feature: String = connection
            .query_row(
                "SELECT feature FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(feature, "configuration");
    }

    #[test]
    fn rejects_changed_history_and_schema_drift() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply(&mut connection).unwrap();
        connection
            .execute(
                "UPDATE schema_migrations SET checksum = 'changed' WHERE version = 1",
                [],
            )
            .unwrap();
        assert!(apply(&mut connection).is_err());

        let mut connection = Connection::open_in_memory().unwrap();
        apply(&mut connection).unwrap();
        connection
            .execute_batch("CREATE TABLE unexpected(value TEXT);")
            .unwrap();
        assert!(apply(&mut connection).is_err());
    }

    #[test]
    fn rejects_incomplete_or_reordered_history() {
        let mut connection = Connection::open_in_memory().unwrap();
        apply(&mut connection).unwrap();
        connection
            .execute("DELETE FROM schema_migrations WHERE version = 2", [])
            .unwrap();
        assert!(apply(&mut connection).is_err());

        let invalid = [
            Migration::new(2, "test.second.v1", "test", "SELECT 1;"),
            Migration::new(1, "test.first.v1", "test", "SELECT 1;"),
        ];
        let mut connection = Connection::open_in_memory().unwrap();
        assert!(apply_registry(&mut connection, &invalid).is_err());
    }

    #[test]
    fn failed_migration_rolls_back_schema_and_history() {
        let migrations = [
            Migration::new(
                1,
                "test.first.v1",
                "test",
                "CREATE TABLE first(value TEXT);",
            ),
            Migration::new(
                2,
                "test.broken.v1",
                "test",
                "CREATE TABLE partial(value TEXT); invalid sql;",
            ),
        ];
        let mut connection = Connection::open_in_memory().unwrap();
        assert!(apply_registry(&mut connection, &migrations).is_err());
        assert_eq!(read_version(&connection).unwrap(), 1);
        let partial: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE name = 'partial'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(partial, 0);
    }
}
