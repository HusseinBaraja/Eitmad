//! Durable reference marker state and bounded local-first sync work.

use eitmad_contracts::{
    identity::ScopeRef,
    reference_marker::{
        ReferenceMarker, ReferenceMarkerId, ReferenceMarkerLabel, ReferenceMarkerPage,
        ReferenceMarkerSyncState,
    },
    sync::{ChangeId, ChangeRecord},
    transport::UnixMillis,
};
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use rusqlite::{OptionalExtension as _, params};

use crate::{
    AuthorityStore, DurableIdempotency, DurablePublication, StorageError, insert_audit,
    insert_idempotency, insert_publication, load_idempotency, migrations::Migration, scope_parts,
};

pub const MAX_REFERENCE_MARKER_SYNC_BATCH: u32 = 50;

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::additive(
    8,
    "reference-marker.initial.v1",
    "reference-marker",
    "CREATE TABLE reference_markers (
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         marker_id TEXT NOT NULL,
         label TEXT NOT NULL,
         revision INTEGER NOT NULL CHECK (revision > 0),
         updated_at INTEGER NOT NULL,
         sync_state TEXT NOT NULL CHECK (sync_state IN ('pending', 'confirmed')),
         PRIMARY KEY (scope_kind, scope_id, marker_id)
     );
     CREATE TABLE reference_marker_sync_outbox (
         change_id TEXT PRIMARY KEY,
         scope_kind TEXT NOT NULL,
         scope_id TEXT NOT NULL,
         marker_id TEXT NOT NULL,
         change_json BLOB NOT NULL,
         FOREIGN KEY (scope_kind, scope_id, marker_id)
             REFERENCES reference_markers(scope_kind, scope_id, marker_id) ON DELETE CASCADE
     );",
)];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReferenceMarkerCommitOutcome {
    Committed {
        marker: ReferenceMarker,
        changed: bool,
    },
    Replayed {
        response_json: Vec<u8>,
    },
    RevisionConflict {
        actual_revision: Option<u64>,
    },
    IdempotencyMismatch,
}

pub struct ReferenceMarkerCommit<'a> {
    pub marker: &'a ReferenceMarker,
    pub expected_revision: Option<u64>,
    pub operation: &'a str,
    pub idempotency: &'a DurableIdempotency,
    pub audit: &'a MutationAuditRecord,
    pub publication: &'a DurablePublication,
    pub change: &'a ChangeRecord,
}

impl AuthorityStore {
    /// Reads one bounded marker page for an exact scope.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error for malformed or unavailable storage.
    pub fn list_reference_markers(
        &self,
        scope: &ScopeRef,
        after: Option<ReferenceMarkerId>,
        limit: u32,
    ) -> Result<ReferenceMarkerPage, StorageError> {
        self.read_transaction(|connection| {
            let (scope_kind, scope_id) = scope_parts(scope);
            let after = after.map(|id| id.value().to_string()).unwrap_or_default();
            let fetch = i64::from(limit.checked_add(1).ok_or(StorageError)?);
            let mut statement = connection
                .prepare(
                    "SELECT marker_id, label, revision, updated_at, sync_state
                     FROM reference_markers
                     WHERE scope_kind = ?1 AND scope_id = ?2 AND marker_id > ?3
                     ORDER BY marker_id LIMIT ?4",
                )
                .map_err(|_| StorageError)?;
            let rows = statement
                .query_map(params![scope_kind, scope_id, after, fetch], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(|_| StorageError)?;
            let mut items = rows
                .map(|row| decode_marker(scope, row.map_err(|_| StorageError)?))
                .collect::<Result<Vec<_>, _>>()?;
            let has_more = items.len() > usize::try_from(limit).map_err(|_| StorageError)?;
            if has_more {
                items.pop();
            }
            let next = has_more
                .then(|| items.last().map(|marker| marker.id))
                .flatten();
            Ok(ReferenceMarkerPage { items, next })
        })
    }

    /// Atomically writes marker state, audit, idempotency, event, and sync work.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error if the transaction cannot commit.
    pub fn commit_reference_marker(
        &self,
        commit: &ReferenceMarkerCommit<'_>,
    ) -> Result<ReferenceMarkerCommitOutcome, StorageError> {
        self.write_transaction(|transaction| commit_reference_marker_on(transaction, commit))
    }

    /// Loads one bounded local-first sync batch without deleting it.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid limit or malformed durable work.
    pub fn reference_marker_sync_batch(
        &self,
        scope: &ScopeRef,
        limit: u32,
    ) -> Result<Vec<ChangeRecord>, StorageError> {
        if !(1..=MAX_REFERENCE_MARKER_SYNC_BATCH).contains(&limit) {
            return Err(StorageError);
        }
        let connection = self.open_connection()?;
        let (scope_kind, scope_id) = scope_parts(scope);
        let mut statement = connection
            .prepare(
                "SELECT change_json FROM reference_marker_sync_outbox
                 WHERE scope_kind = ?1 AND scope_id = ?2 ORDER BY rowid LIMIT ?3",
            )
            .map_err(|_| StorageError)?;
        statement
            .query_map(params![scope_kind, scope_id, limit], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .map_err(|_| StorageError)?
            .map(|row| {
                serde_json::from_slice(&row.map_err(|_| StorageError)?).map_err(|_| StorageError)
            })
            .collect()
    }

    /// Confirms one delivered local-first change and marks its marker current.
    ///
    /// # Errors
    ///
    /// Returns a sanitized error when the exact queued change cannot be confirmed.
    pub fn confirm_reference_marker_sync(
        &self,
        scope: &ScopeRef,
        change_id: ChangeId,
    ) -> Result<(), StorageError> {
        self.write_transaction(|transaction| {
            let (scope_kind, scope_id) = scope_parts(scope);
            let marker_id = transaction
                .query_row(
                    "SELECT marker_id FROM reference_marker_sync_outbox
                     WHERE change_id = ?1 AND scope_kind = ?2 AND scope_id = ?3",
                    params![change_id.value().to_string(), scope_kind, scope_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|_| StorageError)?
                .ok_or(StorageError)?;
            transaction
                .execute(
                    "DELETE FROM reference_marker_sync_outbox WHERE change_id = ?1",
                    [change_id.value().to_string()],
                )
                .map_err(|_| StorageError)?;
            let remaining = transaction
                .query_row(
                    "SELECT 1 FROM reference_marker_sync_outbox
                     WHERE scope_kind = ?1 AND scope_id = ?2 AND marker_id = ?3 LIMIT 1",
                    params![scope_kind, scope_id, marker_id],
                    |_| Ok(()),
                )
                .optional()
                .map_err(|_| StorageError)?
                .is_some();
            if !remaining {
                transaction
                    .execute(
                        "UPDATE reference_markers SET sync_state = 'confirmed'
                         WHERE scope_kind = ?1 AND scope_id = ?2 AND marker_id = ?3",
                        params![scope_kind, scope_id, marker_id],
                    )
                    .map_err(|_| StorageError)?;
            }
            Ok(())
        })
    }
}

fn commit_reference_marker_on(
    transaction: &rusqlite::Connection,
    commit: &ReferenceMarkerCommit<'_>,
) -> Result<ReferenceMarkerCommitOutcome, StorageError> {
    let scope = &commit.marker.scope;
    if let Some((stored_hash, response_json)) =
        load_idempotency(transaction, scope, commit.idempotency.key)?
    {
        if stored_hash == commit.idempotency.request_hash {
            return Ok(ReferenceMarkerCommitOutcome::Replayed { response_json });
        }
        insert_audit(
            transaction,
            &commit.audit.clone().with_outcome(
                AuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1".to_owned()),
            ),
        )?;
        return Ok(ReferenceMarkerCommitOutcome::IdempotencyMismatch);
    }

    let (scope_kind, scope_id) = scope_parts(scope);
    let marker_id = commit.marker.id.value().to_string();
    let actual_revision = transaction
        .query_row(
            "SELECT revision FROM reference_markers
             WHERE scope_kind = ?1 AND scope_id = ?2 AND marker_id = ?3",
            params![scope_kind, scope_id, marker_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| StorageError)?
        .map(u64::try_from)
        .transpose()
        .map_err(|_| StorageError)?;
    if actual_revision != commit.expected_revision {
        let mut conflict = commit.audit.clone().with_outcome(
            AuditOutcome::Conflict,
            Some("eitmad.error.reference-marker-revision-conflict.v1".to_owned()),
        );
        conflict.previous_revision = actual_revision;
        conflict.resulting_revision = actual_revision;
        insert_audit(transaction, &conflict)?;
        return Ok(ReferenceMarkerCommitOutcome::RevisionConflict { actual_revision });
    }

    transaction
        .execute(
            "INSERT INTO reference_markers
                 (scope_kind, scope_id, marker_id, label, revision, updated_at, sync_state)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending')
                 ON CONFLICT(scope_kind, scope_id, marker_id) DO UPDATE SET
                   label = excluded.label,
                   revision = excluded.revision,
                   updated_at = excluded.updated_at,
                   sync_state = excluded.sync_state",
            params![
                scope_kind,
                scope_id,
                marker_id,
                commit.marker.label.as_str(),
                i64::try_from(commit.marker.revision).map_err(|_| StorageError)?,
                commit.marker.updated_at.0,
            ],
        )
        .map_err(|_| StorageError)?;
    let encoded_change = serde_json::to_vec(commit.change).map_err(|_| StorageError)?;
    transaction
        .execute(
            "INSERT INTO reference_marker_sync_outbox
                 (change_id, scope_kind, scope_id, marker_id, change_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                commit.change.change_id.value().to_string(),
                scope_kind,
                scope_id,
                marker_id,
                encoded_change,
            ],
        )
        .map_err(|_| StorageError)?;
    let stored_marker = commit.marker.clone();

    let mut success = commit.audit.clone();
    success.outcome = AuditOutcome::Succeeded;
    success.previous_revision = actual_revision;
    success.resulting_revision = Some(stored_marker.revision);
    insert_audit(transaction, &success)?;
    insert_idempotency(transaction, scope, commit.operation, commit.idempotency)?;
    insert_publication(
        transaction,
        scope,
        commit.idempotency.key,
        commit.publication,
    )?;
    Ok(ReferenceMarkerCommitOutcome::Committed {
        marker: stored_marker,
        changed: true,
    })
}

fn decode_marker(
    scope: &ScopeRef,
    row: (String, String, i64, i64, String),
) -> Result<ReferenceMarker, StorageError> {
    Ok(ReferenceMarker {
        id: ReferenceMarkerId::new(uuid::Uuid::parse_str(&row.0).map_err(|_| StorageError)?),
        scope: scope.clone(),
        label: ReferenceMarkerLabel::parse(row.1).map_err(|_| StorageError)?,
        revision: u64::try_from(row.2).map_err(|_| StorageError)?,
        updated_at: UnixMillis(row.3),
        sync_state: match row.4.as_str() {
            "pending" => ReferenceMarkerSyncState::Pending,
            "confirmed" => ReferenceMarkerSyncState::Confirmed,
            _ => return Err(StorageError),
        },
    })
}
