---
title: "Upgrade local storage to version 7 synchronization state"
description: "Roll out scoped durable sync state, local-first queues, server-authoritative optimistic state, and generated sync contracts."
audience: "support"
page_type: "release"
status: "active"
owner: "Rust storage, synchronization, contract, and release maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "storage version 7, sync state, sync contracts, or rollback behavior changes"
keywords:
  - "storage version 7"
  - "sync.scoped-state.v1"
  - "sync_scopes"
  - "eitmad.sync.reconcile.v1"
---

# Upgrade local storage to version 7 synchronization state

Storage version 7 adds `sync.scoped-state.v1`: one scope-bound durable `sync_scopes` row containing a required state-schema version plus mode-specific records, snapshots, pending work, conflicts, bounded idempotency replays, processed deliveries, and metadata. Rust-generated protocol artifacts now include `eitmad.sync.reconcile.v1` and the expanded sync contract graph.

## Compatibility and migration

- Fresh storage version `0` creates version 7 directly.
- Supported versions `2` through `6` upgrade to `7` after a validated pre-migration backup.
- Version `1`, incomplete/reordered/changed history, incompatible schema drift, and versions newer than `7` fail closed.
- Migration 7 creates `sync_scopes` with `state_version INTEGER NOT NULL`; it does not infer an application mode or synthesize domain records. `SyncEngine::open` creates schema version `1` state with actor/audit context, and unknown row or payload versions fail as `UnsupportedStateVersion` instead of corruption.
- An older engine cannot open storage version 7. Do not attempt an in-place binary rollback after the new engine opens the database.

Generated sync shapes changed with this foundation implementation. Pending commands now expose principal-only `submittedBy`; snapshot-applied events carry metadata instead of full snapshots; adjacent enum payload fields are camel case; record arrays are decode-bounded. Deploy the Rust engine, JSON schema, C#/Swift bindings, identifier constants, and compatible peers together. Complete protocol/capability/schema negotiation before sync traffic.

## Rollout

1. Stop the prior engine cleanly and confirm private disk space for a database-sized snapshot.
2. Generate and verify contract artifacts.
3. Deploy the engine and bindings. Start once and confirm a validated `eitmad.pre-migration-v6-to-v7.sqlite3` artifact when upgrading version 6.
4. Configure each domain scope exactly once as local-first or server-authoritative through an authenticated bootstrap context. Confirm the initial state write has a successful audit row with revisions `0` to `1`. Do not select mode in a shell preference.
5. Run focused sync/storage/contract tests, strict workspace Clippy, all workspace tests, diagnostics, clean start/stop, and documentation audit.
6. Exercise a synthetic offline/reconnect or optimistic/denial flow for each configured mode before production data.

## Rollback and recovery

Stop the version 7 engine before rollback. If version 7 accepted no writes, restore the validated pre-migration artifact through the stopped-engine recovery path and deploy the older compatible engine/bindings. If it accepted local edits, pending commands, conflicts, or deliveries, restoring the artifact loses that work; preserve evidence and plan an explicit export/migration before any rollback.

Never copy `state_json`, clear `sync_scopes`, or downgrade `schema_migrations` manually. Follow [recover local storage](../operations/recover-local-storage.md), [sync troubleshooting](../troubleshooting/synchronization-failures.md), and [storage troubleshooting](../troubleshooting/local-storage-recovery-failures.md).

No production server sync-plane, scheduler, transport, sync UI, or operator conflict workflow ships in this checkpoint. The Rust engine state machine, contracts, local persistence, authorization boundary, and tests are the implemented foundation.
