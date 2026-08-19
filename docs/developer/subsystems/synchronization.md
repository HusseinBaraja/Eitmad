---
title: "Extend the dual-mode synchronization engine safely"
description: "Understand Rust ownership, local-first reconciliation, server-authoritative queues, conflicts, authorization, persistence, and extension points."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust synchronization maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "sync contracts, reconciliation, persistence, authorization, cache, or conflict behavior changes"
keywords:
  - "eitmad-sync"
  - "local-first"
  - "server-authoritative"
  - "pending command queue"
  - "optimistic rollback"
  - "SyncEngine"
  - "المزامنة"
---

# Extend the dual-mode synchronization engine safely

`eitmad-sync` is the single Rust authority for local-first and server-authoritative synchronization. A scope chooses one `SyncMode` when `SyncEngine::open` creates its durable state. Reopening that scope under another mode fails; changing a domain's mode requires a reviewed migration, not a runtime toggle.

## Ownership and boundaries

| Concern | Authority |
| --- | --- |
| Wire records, snapshots, queues, conflicts, metadata, negotiation, and events | `crates/contracts/src/sync.rs` |
| Mode behavior, reconciliation, deduplication, cache reads, and conflict hooks | `crates/sync/src/engine.rs` |
| ReBAC decision and denied audit | `crates/authorization/src/boundary.rs` through `SyncAuthorization` |
| Scoped durable state and accepted mutation audit transaction | `crates/storage/src/sync_state.rs` |
| Behavior tests across contracts, authorization, and SQLite | `crates/sync/tests/sync_engine.rs` |

Native shells display returned state and localized recovery choices. They must not open SQLite, alter queues, merge payloads, retry denied work, infer freshness, or reproduce these contracts. LAN and WAN transports carry the same typed records; transport choice cannot change reconciliation semantics.

The current server sync-plane executable remains an empty deployment boundary. `SyncEngine` implements the device/engine state machine and contracts; server persistence, authentication, transport, scheduling, and production conflict workflows still require their own vertical work.

## Shared state model

Every `ChangeRecord` has a stable change ID, record ID, exact scope, operation, base and resulting revisions, occurrence time, idempotency key, registered payload schema/version, and optional `MergeMetadata`. Tombstones have no payload; upserts require one. Payload bytes remain domain-owned, so the sync engine never interprets Arabic text, furniture rules, or business truth.

`SyncSnapshot` names a checkpoint, server generation, creation time, cache validity deadline, and bounded scoped records. `PendingCommand` retains the authenticated actor and typed command envelope needed for safe replay plus an optional optimistic change. `ConflictRecord` preserves both inputs, resolution status, and merge provenance. `SyncMetadata` records application mode, connectivity, checkpoint, last successful exchange, generation, and cache validity.

SQLite migration `sync.scoped-state.v1` stores one complete serialized state per scope in `sync_scopes`. Compare-and-swap storage revisions prevent two engine instances from silently overwriting each other. Accepted mutations commit state and a `SyncBoundary` audit row in one transaction. Denials are audited by `AuthorizationGate` before state code runs.

## Local-first flow

1. `apply_local_change` checks mode, actor scope, operation shape, idempotency, and ReBAC authorization.
2. Rust assigns the next record revision, writes the visible local record, appends the outgoing change, stores the replay result, and audits the transaction.
3. Reads remain available from durable local state while `ConnectionState::Offline`; a shell may describe this as queued, not synchronized.
4. `connect` negotiates protocol, capabilities, and schemas before reconciliation.
5. `reconcile` acknowledges an echoed change by `change_id`, applies newer non-conflicting remote records, or creates an explicit conflict when a remote revision advances beyond a pending edit's base.

The default `ConflictHook` returns `Defer`; this preserves the local visible value and an open conflict. A domain may implement `KeepLocal`, `KeepRemote`, or `Merge(payload)` only when its invariant makes that result truthful. Keep-local and domain-merge resolutions rebase onto the remote revision and record both source change IDs in `MergeMetadata`. Never add a generic last-write-wins hook for quotation, order, ledger, permission, or other truth-sensitive data.

## Server-authoritative flow

1. A server snapshot populates confirmed cache state, checkpoint, generation, and `valid_until`.
2. `queue_command` authorizes and durably queues a typed command. An optional optimistic change is projected over confirmed state and is returned as `RecordAuthority::Optimistic`.
3. Accepted command results remove the queue item and install any returned authoritative change.
4. Denied results remove the item and rebuild visible records from confirmed cache plus remaining commands. This rolls back only the denied optimistic projection; it does not fabricate a server result.
5. A confirmed cache read after `cache_valid_until` returns `SyncEngineError::StaleCache`. A pending optimistic value may remain visible, but its `CacheFreshness::Stale` and `RecordAuthority::Optimistic` labels are mandatory.

Consumers must never use stale confirmed cache as proof for pricing, permission, inventory, accounting, or another authoritative decision. Fetch an authorized snapshot first. Optimistic UI must be visibly provisional and must handle denial without presenting the rejected value as saved.

## Idempotency and duplicate delivery

Local changes and server commands retain a SHA-256 fingerprint beside their idempotency key. Repeating the same key and same request returns the stored result; reusing the key for different bytes returns `SyncEngineError::IdempotencyMismatch`.

Each `ReconciliationDelivery` has both a delivery ID and idempotency key. Repeating either identity with the same fingerprint emits `DuplicateDeliveryIgnored` and makes no state change. Reusing either identity for different content fails closed. The checkpoint advances only after an authorized delivery and its audit commit succeed.

## Authorization, scope, and security invariants

- The actor scope, engine scope, every change scope, and snapshot scope must match exactly.
- `connect` must succeed before delivery processing. Mode, protocol, required capability, and required schema incompatibility stop normal traffic.
- `SyncAuthorization` runs `AuthorizationGate` with `BoundaryKind::Sync`; missing relationships and tenant/workspace mismatch deny by default.
- Validation and authorization complete before a mutable state copy is persisted. A failed storage commit restores the in-memory copy.
- Accepted state and audit are atomic. Audit targets contain synthetic or redacted identifiers, never payload bytes.
- Pending actor context is identity metadata for authorized replay, not a credential. Tokens and secret material never belong in sync state or payload diagnostics.

## Sync events and shell projection

`drain_events` returns typed `SyncEvent` values for connection changes, queued edits/commands, detected/resolved conflicts, denied commands, applied snapshots, duplicate deliveries, and coalesced status. Existing IPC `SyncStatusChanged` and `RecordChanged` subscriptions are the shell-facing projection boundary; wiring the new lifecycle events into the runtime dispatcher remains future work.

Future Arabic UI should use reviewed labels for offline, queued, optimistic, stale, conflicted, denied, and current states. Mixed text such as `خزانة Wardrobe 120 cm` remains opaque UTF-8 payload data. The engine adds no bidi controls and does not normalize or merge text.

## Failure modes and recovery

| Failure | State guarantee | Recovery |
| --- | --- | --- |
| `WrongMode` or `IncompatibleMode` | No cross-strategy mutation | Open the configured domain mode or ship a reviewed migration |
| `ScopeMismatch` | No cross-scope data applied | Correct authenticated scope and reject the suspect delivery |
| `Authorization(Denied)` | Protected callback did not mutate state; denial audit attempted | Repair the relationship through the owning workflow |
| `IncompatiblePeer` | No normal delivery accepted | Upgrade or select compatible protocol/capability/schema ranges |
| `IdempotencyMismatch` | Original replay state remains | Stop blind retry; create a new key only for genuinely new intent |
| `StaleCache` | Stale confirmed value is withheld | Obtain an authorized fresh snapshot |
| `StorageConflict` | Competing writer was not overwritten | Stop duplicate engine authority and reopen from persisted state |
| `StorageUnavailable` or `CorruptState` | Engine fails closed | Follow local-storage recovery; do not edit `sync_scopes` manually |

For symptom-led checks, use [synchronization failures](../../troubleshooting/synchronization-failures.md).

## Tests and safe extension

`crates/sync/tests/sync_engine.rs` uses the real ReBAC gate and a temporary SQLite authority store. It covers offline edit durability/reopen, reconnect acknowledgement, deferred conflict creation, duplicate delivery, denied optimistic rollback, stale-cache refusal and refresh, unauthorized delivery, and incompatible protocol rejection. `crates/storage/src/sync_state.rs` tests scoped revision checks, while storage recovery tests cover migration 7 backup behavior.

Run:

```powershell
cargo test -p eitmad-sync -p eitmad-storage -p eitmad-contracts
```

Then run strict workspace Clippy, all workspace tests, contract generation/verification, engine diagnostics, and the documentation audit. A new domain must define its mode, payload schema, authorization action/object, conflict policy, stale-read tolerance, Arabic UI states, and audit target before using the engine.

Related references: [sync contracts](../../api/synchronization-contracts.md), [local storage](local-storage.md), [authorization](authorization.md), [ADR-0008](../../decisions/0008-required-unified-synchronization.md), and [storage version 7 rollout](../../releases/storage-v7-sync-state.md).
