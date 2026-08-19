//! Durable append-only storage for the common authorization audit envelope.

use crate::migrations::Migration;

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::additive(
    6,
    "audit.authorization-envelope.v2",
    "audit",
    "ALTER TABLE mutation_audit ADD COLUMN tenant_id TEXT;
     ALTER TABLE mutation_audit ADD COLUMN workspace_id TEXT;
     ALTER TABLE mutation_audit ADD COLUMN target TEXT;
     ALTER TABLE mutation_audit ADD COLUMN redacted_error TEXT;
     ALTER TABLE mutation_audit ADD COLUMN extension_points TEXT;",
)];
