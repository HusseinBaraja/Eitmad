---
title: "Upgrade the server audit envelope to migration 4"
description: "Roll out the canonical server audit crate, complete scoped records, mandatory denial evidence, and migration 4 safely."
audience: "operations"
page_type: "reference"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "server audit fields, migration 4, sync authorization, or rollback behavior changes"
keywords:
  - "server audit migration 4"
  - "control.audit_log"
  - "server/audit"
---

# Upgrade the server audit envelope to migration 4

Migration `4` replaces the duplicated control-plane and sync-plane audit writers with the Rust-owned `server/audit` authority. It adds actor kind, optional workspace, exact scope, target ID, optional causation, and optional idempotency fields to `audit.server_records`. Existing rows are backfilled as user actors in their tenant scope. The migration keeps append-only triggers active for normal sessions, backfills in bounded batches, validates constraints separately, and applies final nullability metadata in a short stage.

## Compatibility and rollout

The network protocol and generated bindings do not change. Deploy migration `4` before a binary that reports required migration version `4`. Use the immutable checksum in `deploy/migrations.sha256`. Stop the rollout if the stored checksum differs or migration `1`, `2`, or `3` is absent.

After rollout, verify these conditions:

1. The server starts and `/readyz` reports ready.
2. Migration status reports version `4` current.
3. A successful control or sync mutation stores tenant, scope, target, actor, correlation, outcome, and time.
4. A denied or invalid sync request stores a redacted row and does not run product work.
5. An audit storage failure withholds the denied result and returns unavailable.
6. `UPDATE` and `DELETE` on `audit.server_records` remain rejected.

## Recovery and rollback

Take a complete PostgreSQL backup before the migration. The new columns are additive, so an older binary can read its old projection, but it does not write the complete envelope and is not an approved steady state. If rollback is necessary, stop traffic and restore the complete pre-migration backup. Do not remove columns, rewrite historical audit rows, or disable append-only triggers by hand.

Related pages: [server authority](../developer/subsystems/server-authority.md), [authorization and audit](../developer/subsystems/authorization.md), [server administration](../developer/subsystems/server-administration.md), and [server operations](../operations/run-server-authority.md).
