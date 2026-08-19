//! Durable opaque state for the Rust-owned synchronization vertical.

use eitmad_contracts::identity::ScopeRef;
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};

use crate::{AuthorityStore, StorageError, insert_audit, migrations::Migration, scope_parts};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::additive(
    7,
    "sync.scoped-state.v1",
    "sync",
    "CREATE TABLE sync_scopes (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         application_mode TEXT NOT NULL CHECK (
             application_mode IN ('local-first', 'server-authoritative')
         ),
         state_version INTEGER NOT NULL,
         revision INTEGER NOT NULL,
         state_json BLOB NOT NULL,
         PRIMARY KEY (scope_kind, scope_id)
     );",
)];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredSyncState {
    pub application_mode: String,
    pub state_version: u32,
    pub revision: u64,
    pub state_json: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncStateCommitOutcome {
    Committed { revision: u64 },
    RevisionConflict { actual_revision: u64 },
}

impl AuthorityStore {
    /// Reads the opaque sync state for one exact scope.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error when state cannot be read or is malformed.
    pub fn read_sync_state(
        &self,
        scope: &ScopeRef,
    ) -> Result<Option<StoredSyncState>, StorageError> {
        self.read_transaction(|connection| {
            let (scope_kind, scope_id) = scope_parts(scope);
            connection
                .query_row(
                    "SELECT application_mode, state_version, revision, state_json FROM sync_scopes
                     WHERE scope_kind = ?1 AND scope_id = ?2",
                    (&scope_kind, &scope_id),
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, Vec<u8>>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| StorageError)?
                .map(|(application_mode, state_version, revision, state_json)| {
                    Ok(StoredSyncState {
                        application_mode,
                        state_version: u32::try_from(state_version).map_err(|_| StorageError)?,
                        revision: u64::try_from(revision).map_err(|_| StorageError)?,
                        state_json,
                    })
                })
                .transpose()
        })
    }

    /// Commits one complete sync state and its successful mutation audit.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error if the transaction cannot commit.
    pub fn commit_sync_state(
        &self,
        scope: &ScopeRef,
        application_mode: &str,
        state_version: u32,
        expected_revision: u64,
        state_json: &[u8],
        audit: &MutationAuditRecord,
    ) -> Result<SyncStateCommitOutcome, StorageError> {
        self.write_transaction(|connection| {
            let (scope_kind, scope_id) = scope_parts(scope);
            let actual_revision = connection
                .query_row(
                    "SELECT revision FROM sync_scopes
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
            if actual_revision != expected_revision {
                return Ok(SyncStateCommitOutcome::RevisionConflict { actual_revision });
            }
            let revision = actual_revision.checked_add(1).ok_or(StorageError)?;
            connection
                .execute(
                    "INSERT INTO sync_scopes
                         (scope_kind, scope_id, application_mode, state_version, revision, state_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(scope_kind, scope_id) DO UPDATE SET
                         application_mode = excluded.application_mode,
                         state_version = excluded.state_version,
                         revision = excluded.revision,
                         state_json = excluded.state_json",
                    params![
                        scope_kind,
                        scope_id,
                        application_mode,
                        i64::from(state_version),
                        i64::try_from(revision).map_err(|_| StorageError)?,
                        state_json
                    ],
                )
                .map_err(|_| StorageError)?;
            let mut success = audit.clone();
            success.outcome = AuditOutcome::Succeeded;
            success.previous_revision = Some(actual_revision);
            success.resulting_revision = Some(revision);
            insert_audit(connection, &success)?;
            Ok(SyncStateCommitOutcome::Committed { revision })
        })
    }
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        identity::{PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId, TenantId},
        transport::{CorrelationId, UnixMillis},
    };
    use eitmad_observability_audit::AuditTarget;
    use rusqlite::Connection;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    fn scope(value: u128) -> ScopeRef {
        ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(Uuid::from_u128(value)),
        }
    }

    fn audit(scope: ScopeRef, value: u128) -> MutationAuditRecord {
        MutationAuditRecord {
            audit_id: Uuid::from_u128(value),
            occurred_at: UnixMillis(i64::try_from(value).unwrap()),
            principal_id: PrincipalId::new(Uuid::from_u128(value + 100)),
            principal_kind: PrincipalKind::User,
            session_id: SessionId::new(Uuid::from_u128(value + 200)),
            device_id: None,
            tenant_id: TenantId::new(Uuid::from_u128(value + 300)),
            workspace_id: None,
            scope,
            correlation_id: CorrelationId::new(Uuid::from_u128(value + 400)),
            causation_id: None,
            idempotency_key: None,
            operation: "eitmad.sync.test.v1".to_owned(),
            target: AuditTarget {
                kind: "organization".to_owned(),
                identifiers: vec!["organization:synthetic".to_owned()],
            },
            outcome: AuditOutcome::Succeeded,
            previous_revision: None,
            resulting_revision: None,
            changed_identifiers: Vec::new(),
            redacted_error: None,
            extension_points: Vec::new(),
        }
    }

    #[test]
    fn state_commit_is_scoped_and_revision_checked() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let first_scope = scope(1);
        let second_scope = scope(2);
        let first_audit = audit(first_scope.clone(), 10);
        let second_audit = audit(second_scope.clone(), 11);
        assert_eq!(store.read_sync_state(&first_scope).unwrap(), None);
        assert_eq!(
            store
                .commit_sync_state(&first_scope, "local-first", 1, 0, b"{}", &first_audit)
                .unwrap(),
            SyncStateCommitOutcome::Committed { revision: 1 }
        );
        assert_eq!(
            store
                .commit_sync_state(
                    &second_scope,
                    "server-authoritative",
                    2,
                    0,
                    b"{\"second\":true}",
                    &second_audit,
                )
                .unwrap(),
            SyncStateCommitOutcome::Committed { revision: 1 }
        );
        assert_eq!(
            store
                .commit_sync_state(&first_scope, "local-first", 1, 0, b"bad", &first_audit)
                .unwrap(),
            SyncStateCommitOutcome::RevisionConflict { actual_revision: 1 }
        );
        let first = store.read_sync_state(&first_scope).unwrap().unwrap();
        assert_eq!(first.application_mode, "local-first");
        assert_eq!(first.state_version, 1);
        assert_eq!(first.revision, 1);
        assert_eq!(first.state_json, b"{}");
        let second = store.read_sync_state(&second_scope).unwrap().unwrap();
        assert_eq!(second.application_mode, "server-authoritative");
        assert_eq!(second.state_version, 2);
        assert_eq!(second.revision, 1);
        assert_eq!(second.state_json, b"{\"second\":true}");

        let connection = Connection::open(store.path()).unwrap();
        let revisions: (i64, i64) = connection
            .query_row(
                "SELECT previous_revision, resulting_revision FROM mutation_audit
                 WHERE audit_id = ?1",
                [first_audit.audit_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(revisions, (0, 1));
        assert!(
            connection
                .execute(
                    "INSERT INTO sync_scopes
                 (scope_kind, scope_id, application_mode, state_version, revision, state_json)
                 VALUES ('organization', 'invalid', 'invalid', 1, 1, X'00')",
                    [],
                )
                .is_err()
        );
    }
}
