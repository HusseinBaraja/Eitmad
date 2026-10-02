//! Immutable schema history for the retired reference-marker demo.

use crate::migrations::Migration;

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(
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
