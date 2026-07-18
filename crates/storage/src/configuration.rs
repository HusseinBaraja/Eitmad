use eitmad_contracts::{
    config::{ConfigChange, ConfigKey, ConfigWriteValue},
    identity::ScopeRef,
};
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};

use crate::migrations::Migration;
use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, StorageError, insert_audit,
    insert_idempotency, insert_publication, load_idempotency, scope_parts,
};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(
    1,
    "configuration.initial.v1",
    "configuration",
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
)];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredConfiguration {
    pub schema_version: u32,
    pub revision: u64,
    pub values: Vec<ConfigChange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigurationCommitOutcome {
    Committed { revision: u64 },
    Replayed { response_json: Vec<u8> },
    RevisionConflict { actual_revision: u64 },
    IdempotencyMismatch,
}

impl AuthorityStore {
    /// Reads persisted configuration overrides for one exact scope.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error for unreadable or malformed state.
    pub fn read_configuration(
        &self,
        scope: &ScopeRef,
    ) -> Result<StoredConfiguration, StorageError> {
        self.read_transaction(|connection| read_configuration_on(connection, scope))
    }

    /// Atomically applies validated configuration overrides, audit, and idempotency.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage error if the transaction cannot commit.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_configuration(
        &self,
        scope: &ScopeRef,
        expected_revision: u64,
        schema_version: u32,
        changes: &[ConfigChange],
        effective_change: bool,
        operation: &str,
        idempotency: &DurableIdempotency,
        audit: &MutationAuditRecord,
        publication: Option<&DurablePublication>,
    ) -> Result<ConfigurationCommitOutcome, StorageError> {
        self.write_transaction(|transaction| {
            commit_configuration_on(
                transaction,
                scope,
                expected_revision,
                schema_version,
                changes,
                effective_change,
                operation,
                idempotency,
                audit,
                publication,
            )
        })
    }
}

fn read_configuration_on(
    connection: &rusqlite::Connection,
    scope: &ScopeRef,
) -> Result<StoredConfiguration, StorageError> {
    let (scope_kind, scope_id) = scope_parts(scope);
    let metadata = connection
        .query_row(
            "SELECT schema_version, revision FROM configuration_scopes
                 WHERE scope_kind = ?1 AND scope_id = ?2",
            (&scope_kind, &scope_id),
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| StorageError)?;
    let Some((schema_version, revision)) = metadata else {
        return Ok(StoredConfiguration {
            schema_version: 1,
            revision: 0,
            values: Vec::new(),
        });
    };
    let mut statement = connection
        .prepare(
            "SELECT config_key, value_json FROM configuration_values
                 WHERE scope_kind = ?1 AND scope_id = ?2 ORDER BY config_key",
        )
        .map_err(|_| StorageError)?;
    let rows = statement
        .query_map((&scope_kind, &scope_id), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| StorageError)?;
    let mut values = Vec::new();
    for row in rows {
        let (key, value) = row.map_err(|_| StorageError)?;
        values.push(ConfigChange {
            key: ConfigKey::parse(key).map_err(|_| StorageError)?,
            value: serde_json::from_str::<ConfigWriteValue>(&value).map_err(|_| StorageError)?,
        });
    }
    Ok(StoredConfiguration {
        schema_version,
        revision: u64::try_from(revision).map_err(|_| StorageError)?,
        values,
    })
}

#[allow(clippy::too_many_arguments)]
fn commit_configuration_on(
    transaction: &rusqlite::Connection,
    scope: &ScopeRef,
    expected_revision: u64,
    schema_version: u32,
    changes: &[ConfigChange],
    effective_change: bool,
    operation: &str,
    idempotency: &DurableIdempotency,
    audit: &MutationAuditRecord,
    publication: Option<&DurablePublication>,
) -> Result<ConfigurationCommitOutcome, StorageError> {
    if let Some((stored_hash, response_json)) =
        load_idempotency(transaction, scope, idempotency.key)?
    {
        if stored_hash == idempotency.request_hash {
            return Ok(ConfigurationCommitOutcome::Replayed { response_json });
        }
        let invalid = audit.clone().with_outcome(
            AuditOutcome::Invalid,
            Some("eitmad.error.contract-invalid.v1".to_owned()),
        );
        insert_audit(transaction, &invalid)?;
        return Ok(ConfigurationCommitOutcome::IdempotencyMismatch);
    }
    let (scope_kind, scope_id) = scope_parts(scope);
    let actual = transaction
        .query_row(
            "SELECT revision FROM configuration_scopes
                 WHERE scope_kind = ?1 AND scope_id = ?2",
            (&scope_kind, &scope_id),
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| StorageError)?
        .map(u64::try_from)
        .transpose()
        .map_err(|_| StorageError)?
        .unwrap_or(0);
    if actual != expected_revision {
        let mut conflict = audit.clone().with_outcome(
            AuditOutcome::Conflict,
            Some("eitmad.error.config-revision-conflict.v1".to_owned()),
        );
        conflict.previous_revision = Some(actual);
        conflict.resulting_revision = Some(actual);
        insert_audit(transaction, &conflict)?;
        return Ok(ConfigurationCommitOutcome::RevisionConflict {
            actual_revision: actual,
        });
    }
    let resulting_revision = if effective_change {
        expected_revision.checked_add(1).ok_or(StorageError)?
    } else {
        expected_revision
    };
    if effective_change {
        let resulting_revision_sql = i64::try_from(resulting_revision).map_err(|_| StorageError)?;
        transaction
            .execute(
                "INSERT INTO configuration_scopes
                     (scope_kind, scope_id, schema_version, revision) VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(scope_kind, scope_id) DO UPDATE SET
                       schema_version = excluded.schema_version,
                       revision = excluded.revision",
                params![scope_kind, scope_id, schema_version, resulting_revision_sql],
            )
            .map_err(|_| StorageError)?;
        for change in changes {
            let encoded = serde_json::to_string(&change.value).map_err(|_| StorageError)?;
            transaction
                .execute(
                    "INSERT INTO configuration_values
                         (scope_kind, scope_id, config_key, value_json) VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(scope_kind, scope_id, config_key) DO UPDATE SET
                           value_json = excluded.value_json",
                    params![scope_kind, scope_id, change.key.as_str(), encoded],
                )
                .map_err(|_| StorageError)?;
        }
    }
    let mut success = audit.clone();
    success.outcome = AuditOutcome::Succeeded;
    success.previous_revision = Some(expected_revision);
    success.resulting_revision = Some(resulting_revision);
    insert_audit(transaction, &success)?;
    insert_idempotency(transaction, scope, operation, idempotency)?;
    if effective_change {
        insert_publication(
            transaction,
            scope,
            idempotency.key,
            publication.ok_or(StorageError)?,
        )?;
    }
    Ok(ConfigurationCommitOutcome::Committed {
        revision: resulting_revision,
    })
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        config::ConfigWriteValue,
        identity::{PrincipalId, PrincipalKind, ScopeId, ScopeKind},
        transport::{CorrelationId, IdempotencyKey, UnixMillis},
    };
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    fn scope() -> ScopeRef {
        ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(Uuid::from_u128(1)),
        }
    }

    fn audit() -> MutationAuditRecord {
        MutationAuditRecord {
            audit_id: Uuid::new_v4(),
            occurred_at: UnixMillis(1),
            principal_id: PrincipalId::new(Uuid::from_u128(2)),
            principal_kind: PrincipalKind::User,
            scope: scope(),
            correlation_id: CorrelationId::new(Uuid::from_u128(3)),
            causation_id: None,
            idempotency_key: Some(IdempotencyKey::new(Uuid::from_u128(4))),
            operation: "eitmad.config.update.v1".to_owned(),
            outcome: AuditOutcome::Succeeded,
            previous_revision: None,
            resulting_revision: None,
            changed_identifiers: vec!["eitmad.config.locale.primary.v1".to_owned()],
            error_code: None,
        }
    }

    #[test]
    fn configuration_commit_is_atomic_revisioned_and_idempotent() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let change = ConfigChange {
            key: ConfigKey::parse("eitmad.config.locale.primary.v1").unwrap(),
            value: ConfigWriteValue::Text("ar-YE".to_owned()),
        };
        let idempotency = DurableIdempotency {
            key: IdempotencyKey::new(Uuid::from_u128(4)),
            request_hash: [7; 32],
            response_json: br#"{"revision":1}"#.to_vec(),
        };

        let first = store
            .commit_configuration(
                &scope(),
                0,
                1,
                std::slice::from_ref(&change),
                true,
                "eitmad.config.update.v1",
                &idempotency,
                &audit(),
                Some(&DurablePublication {
                    event: eitmad_contracts::events::Event::ConfigurationChanged(
                        eitmad_contracts::config::ConfigSnapshot {
                            schema_version: 1,
                            revision: 1,
                            scope: scope(),
                            entries: Vec::new(),
                        },
                    ),
                    policy_changed: false,
                }),
            )
            .unwrap();
        assert_eq!(first, ConfigurationCommitOutcome::Committed { revision: 1 });
        let replay = store
            .commit_configuration(
                &scope(),
                0,
                1,
                &[change],
                true,
                "eitmad.config.update.v1",
                &idempotency,
                &audit(),
                None,
            )
            .unwrap();
        assert_eq!(
            replay,
            ConfigurationCommitOutcome::Replayed {
                response_json: br#"{"revision":1}"#.to_vec()
            }
        );
        assert_eq!(store.read_configuration(&scope()).unwrap().revision, 1);
    }

    #[test]
    fn revision_conflict_does_not_apply_values() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let outcome = store
            .commit_configuration(
                &scope(),
                8,
                1,
                &[],
                false,
                "eitmad.config.update.v1",
                &DurableIdempotency {
                    key: IdempotencyKey::new(Uuid::new_v4()),
                    request_hash: [1; 32],
                    response_json: Vec::new(),
                },
                &audit(),
                None,
            )
            .unwrap();
        assert_eq!(
            outcome,
            ConfigurationCommitOutcome::RevisionConflict { actual_revision: 0 }
        );
        assert_eq!(store.read_configuration(&scope()).unwrap().revision, 0);
    }
}
