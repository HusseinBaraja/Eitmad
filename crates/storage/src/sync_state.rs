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
         revision INTEGER NOT NULL,
         state_json BLOB NOT NULL,
         PRIMARY KEY (scope_kind, scope_id)
     );",
)];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredSyncState {
    pub application_mode: String,
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
                    "SELECT application_mode, revision, state_json FROM sync_scopes
                     WHERE scope_kind = ?1 AND scope_id = ?2",
                    (&scope_kind, &scope_id),
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, Vec<u8>>(2)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| StorageError)?
                .map(|(application_mode, revision, state_json)| {
                    Ok(StoredSyncState {
                        application_mode,
                        revision: u64::try_from(revision).map_err(|_| StorageError)?,
                        state_json,
                    })
                })
                .transpose()
        })
    }

    /// Commits one complete sync state and optional successful mutation audit.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error if the transaction cannot commit.
    pub fn commit_sync_state(
        &self,
        scope: &ScopeRef,
        application_mode: &str,
        expected_revision: u64,
        state_json: &[u8],
        audit: Option<&MutationAuditRecord>,
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
                         (scope_kind, scope_id, application_mode, revision, state_json)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(scope_kind, scope_id) DO UPDATE SET
                         application_mode = excluded.application_mode,
                         revision = excluded.revision,
                         state_json = excluded.state_json",
                    params![
                        scope_kind,
                        scope_id,
                        application_mode,
                        i64::try_from(revision).map_err(|_| StorageError)?,
                        state_json
                    ],
                )
                .map_err(|_| StorageError)?;
            if let Some(audit) = audit {
                let mut success = audit.clone();
                success.outcome = AuditOutcome::Succeeded;
                success.previous_revision = Some(actual_revision);
                success.resulting_revision = Some(revision);
                insert_audit(connection, &success)?;
            }
            Ok(SyncStateCommitOutcome::Committed { revision })
        })
    }
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::identity::{ScopeId, ScopeKind};
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    fn scope() -> ScopeRef {
        ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(Uuid::from_u128(1)),
        }
    }

    #[test]
    fn state_commit_is_scoped_and_revision_checked() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        assert_eq!(store.read_sync_state(&scope()).unwrap(), None);
        assert_eq!(
            store
                .commit_sync_state(&scope(), "local-first", 0, b"{}", None)
                .unwrap(),
            SyncStateCommitOutcome::Committed { revision: 1 }
        );
        assert_eq!(
            store
                .commit_sync_state(&scope(), "local-first", 0, b"bad", None)
                .unwrap(),
            SyncStateCommitOutcome::RevisionConflict { actual_revision: 1 }
        );
        assert_eq!(
            store.read_sync_state(&scope()).unwrap().unwrap().state_json,
            b"{}"
        );
    }
}
