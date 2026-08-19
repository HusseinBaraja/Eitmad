---
title: "Resolve queued, conflicted, denied, or stale synchronization"
description: "Diagnose sync mode, compatibility, scope, idempotency, conflict, cache, and durable-state failures without losing offline work."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "synchronization, authorization, and storage maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "SyncEngineError, sync status, queue, conflict, cache, or recovery behavior changes"
keywords:
  - "StaleCache"
  - "IdempotencyMismatch"
  - "IncompatiblePeer"
  - "sync queued"
  - "المزامنة معلقة"
  - "تعارض المزامنة"
---

# Resolve queued, conflicted, denied, or stale synchronization

The Rust sync engine preserves local-first edits offline and preserves server-authoritative commands until an explicit result. Do not delete `sync_scopes`, clear a queue, replace an idempotency key, or edit SQLite to make a symptom disappear.

## Symptoms

- Status remains queued after connectivity returns.
- A record is conflicted and local/remote values differ.
- An optimistic server-authoritative edit disappears after a denial.
- A read returns `SyncEngineError::StaleCache`.
- Reconciliation returns `IdempotencyMismatch`, `ScopeMismatch`, `IncompatibleMode`, or `IncompatiblePeer`.
- Sync state fails to open with `StorageUnavailable`, `StorageConflict`, or `CorruptState`.
- Future Arabic UI may expose reviewed equivalents of `المزامنة معلقة` or `تعارض المزامنة`; no native sync workflow exists yet.

## Fast checks

1. Record the application mode, connection state, pending count, open conflict count, checkpoint presence, cache deadline, protocol range, required capabilities, schema ranges, timestamp, and correlation ID. Do not collect payload bytes.
2. Confirm the actor, engine, records, snapshot, tenant, and optional workspace refer to the same scope.
3. Confirm version/capability/schema negotiation succeeded and both peers declared the same `SyncMode`.
4. For queued local-first work, confirm the delivery echoes the original `change_id`; matching only `record_id` does not acknowledge it.
5. For server-authoritative work, find the exact `PendingCommandId` result and whether it was accepted or denied.
6. Compare `cache_valid_until` with the engine's Unix-millisecond input. Do not treat a stale confirmed row as authoritative.
7. For duplicate or mismatch errors, compare delivery ID, idempotency key, and a safe fingerprint—not raw request bytes.
8. If storage is implicated, stop duplicate engine instances and follow local-storage diagnostics before retrying.

## Causes and resolutions

| Evidence | Cause | Resolution | Verify |
| --- | --- | --- | --- |
| Offline local-first record is readable and pending | Expected disconnected operation | Restore compatible transport; keep the queue intact | Echoed `change_id` leaves the queue and checkpoint advances |
| Remote revision advanced beyond a pending edit's base | Concurrent edit | Use the domain conflict workflow; default is defer | Conflict remains open until a truthful keep/merge decision is audited |
| Optimistic value rolled back with denied command event | Server rejected intent | Explain denial through localized authorization UI; repair permission or input through its owner | Confirmed cache value returns and denied command is absent from queue |
| `StaleCache` | Confirmed snapshot passed `valid_until` | Fetch an authorized fresh snapshot | Read returns `ServerConfirmed` plus `Fresh` |
| Stale optimistic value still visible | Pending intent overlays an expired base | Label it optimistic and stale; do not use it for authoritative decisions | Acceptance installs server value, or denial rebuilds from confirmed cache |
| `IdempotencyMismatch` | One key or delivery identity was reused for different bytes | Stop blind retry and investigate caller identity generation | Exact replay is ignored/replayed; new intent uses a new key |
| `ScopeMismatch` or authorization denial | Cross-scope/unpermitted delivery | Reject it, validate channel identity, and follow authorization troubleshooting | Original local state is unchanged; denial audit is present when storage is available |
| `IncompatibleMode` | Scope reopened under another strategy | Restore the configured mode; plan a data migration to change strategy | Engine opens without rewriting existing state |
| `IncompatiblePeer` | No common protocol, capability, or required schema | Deploy compatible peers or restore supported schema range | Negotiation succeeds before any delivery |
| `StorageConflict` | Competing writer revision | Stop duplicate authority and reopen from disk | One engine owns the scope and pending work remains |
| `CorruptState` or migration failure | Unreadable state/schema/history | Stop writes and use validated storage recovery | Integrity, history, migration 7, and engine reopen pass |

## Verify recovery

Use synthetic Arabic/mixed text such as `خزانة Wardrobe 120 cm` in a disposable scope. Verify offline reopen, reconnect acknowledgement, one conflict, exact duplicate delivery, denied optimistic rollback, stale-cache refusal then snapshot refresh, authorization denial, and incompatible version rejection.

Run the focused suite:

```powershell
cargo test -p eitmad-sync -p eitmad-storage -p eitmad-contracts
```

## Escalate safely

Collect stable enum/error names, mode, protocol/capability/schema ranges, scope kind, queue/conflict counts, revisions, checkpoint presence, cache timestamps, sanitized fingerprints, audit outcome, and correlation ID. Never attach sync state JSON, database files, payload/command bytes, customer text, principal UUIDs, relationship graphs, tokens, or raw server errors to routine tickets.

Treat any applied unauthorized or cross-scope record as a security incident. Return to [sync engine ownership](../developer/subsystems/synchronization.md), [authorization denial](authorization-boundary-denials.md), or [storage recovery](local-storage-recovery-failures.md).
