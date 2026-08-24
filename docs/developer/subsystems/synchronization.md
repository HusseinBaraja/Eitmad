---
title: "Extend synchronization and its shared transports safely"
description: "Understand Rust ownership, dual-mode reconciliation, one simulation/LAN/WAN transport interface, connection safety, and extension points."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust synchronization maintainers"
last_verified: "2026-08-24"
review_triggers:
  - "sync contracts, reconciliation, transport, persistence, authorization, cache, or conflict behavior changes"
keywords:
  - "eitmad-sync"
  - "local-first"
  - "server-authoritative"
  - "pending command queue"
  - "optimistic rollback"
  - "SyncEngine"
  - "SyncTransport"
  - "LAN sync"
  - "WAN relay"
  - "المزامنة"
---

# Extend synchronization and its shared transports safely

`eitmad-sync` is the single Rust authority for local-first and server-authoritative synchronization. A scope chooses one `SyncMode` when `SyncEngine::open` creates its durable state. Reopening that scope under another mode fails; changing a domain's mode requires a reviewed migration, not a runtime toggle.

## Ownership and boundaries

| Concern | Authority |
| --- | --- |
| Wire records, snapshots, queues, conflicts, metadata, negotiation, and events | `crates/contracts/src/sync.rs` |
| Shared stream frame, cancellation, and heartbeat wire shape | `crates/contracts/src/sync_transport.rs` |
| Mode behavior, reconciliation, deduplication, cache reads, and conflict hooks | `crates/sync/src/engine.rs` |
| Authentication/encryption policy, negotiation, ordering, cancellation, retry, health, and duplicate-frame safety | `crates/sync/src/transport/core.rs` |
| Local simulation, LAN discovery, and WAN server/relay route policy | `crates/sync/src/transport/simulation.rs`, `lan.rs`, and `wan.rs` |
| ReBAC decision and denied audit | `crates/authorization/src/boundary.rs` through `SyncAuthorization` |
| Scoped durable state and accepted mutation audit transaction | `crates/storage/src/sync_state.rs` |
| Behavior tests across contracts, transport failures, authorization, and SQLite | `crates/sync/tests/sync_transport.rs` and `sync_engine.rs` |

Native shells display returned state and localized recovery choices. They must not open SQLite, alter queues, merge payloads, retry denied work, infer freshness, or reproduce these contracts. LAN and WAN transports carry the same typed records; transport choice cannot change reconciliation semantics.

The PostgreSQL server sync plane implements registered domain handlers, durable operations, idempotency, conflicts, checkpoints, snapshot-backed history, and resumable subscriptions. The combined host supplies HTTPS, one WebSocket boundary, and authenticated relay coordination. Its domain registry remains empty until the first production vertical registers a schema and handler. LAN discovery sources, production relay payload routing, scheduling, and conflict UI still require vertical implementations. A production driver must not move authentication, secret access, authorization, or sync semantics into a native shell. See [WAN relay coordination](wan-relay-coordination.md).

## One transport interface and wire protocol

`SyncTransport` is the only high-level connection interface for local simulation, LAN, and WAN routes. Every implementation exposes `connect`, `disconnect`, ordered `send`/`receive`, `cancel`, `health`, and the negotiated session. The core offers and requires `eitmad.capability.sync.v1` on every connection. `SyncTransportFrame` always carries the existing `SyncMessage`; LAN and WAN must not add route-specific message enums, acknowledgements, retry meanings, or reconciliation behavior.

Each frame has a frame ID, independent idempotency key, negotiated protocol version, correlation ID, stream ID, zero-based sequence, end-of-stream marker, and one payload. Payloads are a sync message, cancellation, heartbeat, or heartbeat acknowledgement. The generated JSON schema and C#/Swift bindings contain the same frame definition.

The core requires sequence `0` for a new stream and then accepts only the next sequence. An end-of-stream frame closes that direction. Cancellation is itself an end-of-stream frame and prevents later local sends on the stream. The cancellation payload stream ID must match the frame stream ID; a mismatch returns `StreamOutOfOrder` in the cancellation phase without cancelling either stream. Repeating local cancellation after the stream is cancelled or its outgoing direction is complete succeeds without sending another frame. Callers use another stream ID for new work; they must not reset sequence on an existing ID.

Incoming duplicate safety covers both `SyncFrameId` and `IdempotencyKey`. An exact retained retry is ignored before sequence evaluation. Reusing either retained identity with different logical content returns `DuplicateConflict`. The in-memory transport window retains at most 4,096 accepted frame/key pairs and 8,192 terminal stream IDs in insertion order. Evicting the oldest terminal stream also removes its sequence and completion state, so callers must use a new stream ID instead of retrying beyond this adapter-local window. Both windows survive reconnect while the adapter instance lives. They complement, but do not replace, the durable 2,048-entry engine delivery/replay windows.

## Authentication and encrypted-session expectations

| Route | Required local authentication | Required session evidence | Allowed degraded behavior |
| --- | --- | --- | --- |
| Simulation | `Simulation` | Explicit `IsolatedSimulation`; never valid for LAN/WAN | Scripted disconnects and failures only |
| LAN | Device ID plus a Rust-owned `SecretId` reference | Authenticated encryption, authenticated peer, and forward secrecy | Connect to a reachable discovered peer when other interfaces or peers fail |
| WAN | Account ID, device ID, and a Rust-owned `SecretId` reference | Authenticated encryption, authenticated peer, and forward secrecy | Use the configured relay only when direct server connectivity fails |

`SecretId` is a non-secret reference. The connector may resolve it only through an approved Rust secret-storage integration; the frame and failure types never carry token bytes. `EstablishedConnection.authenticated_as` must exactly match the requested device or account/device identity. A mismatch is terminal. LAN/WAN reject isolated plaintext or a connection that lacks peer authentication or forward secrecy.

Simulation isolation and LAN/WAN encryption are explicit route policies, not two strengths of one optional-encryption flag. A simulation driver that returns an authenticated-encryption session is the wrong driver for that isolated route and fails as `DriverUnavailable` in the encryption phase. An isolated session remains invalid for LAN and WAN and fails as `EncryptionRequired`.

Authentication, encryption, version, required-capability, and required-schema failures do not fall back to another WAN route. Transport authentication establishes a peer; it does not authorize records. `SyncEngine` still checks ReBAC, scope, and audit requirements before protected sync reads or mutations.

## Discovery, reconnect, backoff, and health

`LanAdapter` calls `LanDiscovery` on every connection attempt, selects the reachable peer with the lowest numeric priority, and breaks equal-priority ties by `peer_id`. It reports `NoLanPeer` when discovery completed without a peer. If discovery reports failed interfaces or peers but returns one reachable peer, the connection succeeds with `HealthStatus::Degraded` and `PartialNetwork`. If no peer remains during a partial discovery, the attempt fails as `PartialNetwork` and preserves pending engine work.

`WanAdapter` tries the configured server first. It uses the configured relay only after `Disconnected`, `PartialNetwork`, `ServerUnavailable`, or `DriverUnavailable`. A relay connection is healthy enough to transfer frames but is reported as degraded with the direct-server failure retained. If both routes fail, the result is `RelayUnavailable`. A deployment without a relay reports `ServerUnavailable`.

The shared core closes a failed driver connection, retains stream and duplicate state, and schedules exponential retry from `RetryPolicy.initial_delay_ms` up to `maximum_delay_ms`. It stops after `maximum_attempts`. `RetryNotReady` reports the remaining delay when a caller reconnects early. Terminal security and compatibility failures have `RetryAdvice::Never`; an operator must correct credentials, session security, or peer compatibility instead of retrying.

`ConnectionHealth` exposes offline, connecting, healthy, or degraded state; selected target; last success; structured last failure; consecutive failures; next retry time; and optional round-trip time. `last_success_at` changes only after a successful connect, send, received frame, or delivered cancellation. Disconnect changes the status to offline but preserves the last successful exchange time. These fields are operational state, not proof that an engine reconciliation or server mutation committed.

## Shared state model

Every `ChangeRecord` has a stable change ID, record ID, exact scope, operation, base and resulting revisions, occurrence time, idempotency key, registered payload schema/version, and optional `MergeMetadata`. Tombstones have no payload; upserts require one. Payload bytes remain domain-owned, so the sync engine never interprets Arabic text, furniture rules, or business truth.

`SyncSnapshot` names a checkpoint, server generation, creation time, cache validity deadline, and bounded scoped records. `PendingCommand` retains only the submitting `PrincipalId`, exact scope, typed command envelope, and optional optimistic change; session, device, service, tenant, and workspace context remains in the audit record instead of durable queue or event payloads. `ConflictRecord` preserves both inputs, resolution status, and merge provenance. `SyncMetadata` records application mode, connectivity, checkpoint, last successful exchange, generation, and cache validity.

SQLite migration `sync.scoped-state.v1` stores one complete serialized state per scope in `sync_scopes`. Both the row and serialized `EngineState` carry schema version `1`; an unknown version returns `UnsupportedStateVersion` before decoding. Compare-and-swap storage revisions prevent two engine instances from silently overwriting each other. Every state write, including first-scope bootstrap and connection changes, requires a successful `SyncBoundary` audit in the same transaction. Denials are audited by `AuthorizationGate` before state code runs.

## Local-first flow

1. `apply_local_change` checks mode, actor scope, operation shape, idempotency, and ReBAC authorization.
2. Rust assigns the next record revision, writes the visible local record, appends the outgoing change, stores the replay result, and audits the transaction.
3. Reads remain available from durable local state while `ConnectionState::Offline`; a shell may describe this as queued, not synchronized.
4. Authorized `connect` negotiates protocol, capabilities, and schemas before reconciliation; authorized `disconnect` durably marks transport loss.
5. `reconcile` acknowledges an echoed change by `change_id`, keeps any newer queued edit to the same record visible, applies newer non-conflicting remote records, or creates an explicit conflict when a remote revision advances beyond the newest pending edit's base.

The default `ConflictHook` returns `Defer`; this preserves the local visible value and an open conflict. A domain may implement `KeepLocal`, `KeepRemote`, or `Merge(payload)` only when its invariant makes that result truthful. Keep-local and domain-merge resolutions rebase onto the remote revision and record both source change IDs in `MergeMetadata`. Never add a generic last-write-wins hook for quotation, order, ledger, permission, or other truth-sensitive data.

## Server-authoritative flow

1. A server snapshot populates confirmed cache state, checkpoint, generation, and `valid_until`.
2. `queue_command` authorizes and durably queues a typed command. An optional optimistic change is projected over confirmed state and is returned as `RecordAuthority::Optimistic`.
3. Accepted command results remove the queue item and install any returned authoritative change.
4. Denied results remove the item and rebuild visible records from confirmed cache plus remaining commands. This rolls back only the denied optimistic projection; it does not fabricate a server result.
5. `read_record` and `read_last_snapshot` require actor, request, and audit context and execute through the authorized read boundary. A confirmed cache read after `cache_valid_until` returns `SyncEngineError::StaleCache`. A pending optimistic value may remain visible, but its `CacheFreshness::Stale` and `RecordAuthority::Optimistic` labels are mandatory.

Consumers must never use stale confirmed cache as proof for pricing, permission, inventory, accounting, or another authoritative decision. Fetch an authorized snapshot first. Optimistic UI must be visibly provisional and must handle denial without presenting the rejected value as saved.

## Idempotency and duplicate delivery

Local changes and server commands retain a versioned SHA-256 fingerprint over an explicit ordered field set beside their idempotency key. Authorization runs before replay lookup. Repeating a retained key and the same request returns the stored result; reusing it for different content returns `SyncEngineError::IdempotencyMismatch`.

Each `ReconciliationDelivery` has both a delivery ID and idempotency key. Repeating either retained identity with the same fingerprint emits `DuplicateDeliveryIgnored` and makes no state change. Reusing either identity for different content fails closed. Replay and processed-delivery maps use keyed lookup and retain at most 2,048 entries per category, pruning the oldest entries after mutations and successful reconciliation. Keys older than this explicit idempotency window are no longer replay identities; callers must not retry indefinitely. The checkpoint advances only after an authorized delivery and its audit commit succeed.

## Authorization, scope, and security invariants

- The actor scope, engine scope, every change scope, and snapshot scope must match exactly.
- `connect` must succeed before delivery processing. Mode, protocol, required capability, and required schema incompatibility stop normal traffic.
- `SyncAuthorization` runs `AuthorizationGate` with `BoundaryKind::Sync`; missing relationships and tenant/workspace mismatch deny by default.
- Validation and authorization complete before replay lookup or mutation. A reconciliation error or failed storage commit restores the in-memory copy.
- Accepted state and audit are atomic. Audit targets contain synthetic or redacted identifiers, never payload bytes.
- Pending commands expose only principal attribution. Tokens, session/device/service identifiers, tenant/workspace context, and secret material never belong in queue events or payload diagnostics.

## Sync events and shell projection

`drain_events` returns typed `SyncEvent` values for connection changes, queued edits/commands, detected/resolved conflicts, denied commands, applied snapshots, duplicate deliveries, and coalesced status. Reconciliation buffers events until the state/audit commit succeeds, so failed work cannot reach subscribers. `SnapshotApplied` carries only snapshot ID, checkpoint, and record count; authorized consumers query `read_last_snapshot` for the full payload. Existing IPC `SyncStatusChanged` and `RecordChanged` subscriptions are the shell-facing projection boundary; wiring the new lifecycle events into the runtime dispatcher remains future work.

Future Arabic UI should use reviewed labels for offline, queued, optimistic, stale, conflicted, denied, and current states. Mixed text such as `خزانة Wardrobe 120 cm` remains opaque UTF-8 payload data. The engine adds no bidi controls and does not normalize or merge text.

## Failure modes and recovery

| Failure | State guarantee | Recovery |
| --- | --- | --- |
| `WrongMode` or `IncompatibleMode` | No cross-strategy mutation | Open the configured domain mode or ship a reviewed migration |
| `ScopeMismatch` | No cross-scope data applied | Correct authenticated scope and reject the suspect delivery |
| `Authorization(Denied)` | Protected callback did not mutate state; denial audit attempted | Repair the relationship through the owning workflow |
| `IncompatiblePeer` | No normal delivery accepted | Upgrade or select compatible protocol/capability/schema ranges |
| `Disconnected` | Offline delivery was not misclassified as protocol failure | Restore transport, reconnect, then retry the retained delivery |
| `IdempotencyMismatch` | Original replay state remains | Stop blind retry; create a new key only for genuinely new intent |
| `StaleCache` | Stale confirmed value is withheld | Obtain an authorized fresh snapshot |
| `StorageConflict` | Competing writer was not overwritten | Stop duplicate engine authority and reopen from persisted state |
| `UnsupportedStateVersion` | Unknown durable state is not decoded | Run a supported migration or compatible engine; do not edit the version marker |
| `StorageUnavailable` or `CorruptState` | Engine fails closed | Follow local-storage recovery; do not edit `sync_scopes` manually |
| `AuthenticationFailed` or `EncryptionRequired` | No frame traffic and no relay bypass | Repair the Rust-owned credential reference or secure connector; never downgrade encryption |
| `VersionMismatch`, `CapabilityMismatch`, or `SchemaMismatch` | Connection closes before normal traffic | Deploy compatible peers; do not translate messages in an adapter |
| `RetryNotReady` | Pending engine work stays intact | Wait until `next_retry_at`; do not add an independent retry loop |
| `DuplicateConflict` or `StreamOutOfOrder` | Suspect frame is not delivered to reconciliation | Stop the stream and inspect frame/key/sequence generation without collecting payload bytes |
| `PartialNetwork` | Reachable LAN peer may continue as degraded | Repair failed interfaces/peers and reconnect; do not clear queues |
| `ServerUnavailable` or `RelayUnavailable` | WAN work remains offline/pending | Restore the named route; retry only under reported backoff |

For symptom-led checks, use [synchronization failures](../../troubleshooting/synchronization-failures.md).

## Tests and safe extension

`crates/sync/tests/sync_transport.rs` uses the simulation adapter and scripted connection drivers. It covers disconnect/reconnect and last-success preservation, retry gating, authentication failure, exact and conflicting duplicates, new-frame-ID idempotent retry, ordered/end-marked streams, cancellation idempotency and stream-ID validation, protocol mismatch, deterministic LAN priority selection, LAN partial discovery, LAN encryption rejection, WAN relay degradation, relay unavailability, and refusal to route around authentication failure. Unit tests in `crates/sync/src/transport/core.rs` verify bounded terminal-stream retirement and explicit simulation isolation.

`crates/sync/tests/sync_engine.rs` uses the real ReBAC gate and a temporary SQLite authority store. It covers offline edit durability/reopen, multiple-edit acknowledgement, all conflict outcomes and rebase overflow, duplicate/mismatched replay, replay authorization, denied optimistic rollback, stale-cache refusal and refresh, authorized reads, offline/incompatible errors, reconciliation rollback, stale-event suppression, and incompatible protocol rejection. `crates/storage/src/sync_state.rs` tests scope isolation, mode/version persistence, revision checks, mandatory audit revisions, and invalid modes; storage recovery tests cover migration 7 backup behavior.

Run:

```powershell
cargo test -p eitmad-sync -p eitmad-storage -p eitmad-contracts
```

Then run strict workspace Clippy, all workspace tests, contract generation/verification, engine diagnostics, and the documentation audit. A new production connector must define credential resolution, peer verification, encrypted-session implementation, route discovery, timeouts, backpressure integration, safe diagnostics, and deployment ownership without changing `SyncTransportFrame` or `SyncMessage`. A new domain must define its mode, payload schema, authorization action/object, conflict policy, stale-read tolerance, Arabic UI states, and audit target before using the engine.

Related references: [sync contracts](../../api/synchronization-contracts.md), [server authority](server-authority.md), [WAN relay](wan-relay-coordination.md), [local storage](local-storage.md), [authorization](authorization.md), [ADR-0008](../../decisions/0008-required-unified-synchronization.md), [storage version 7 rollout](../../releases/storage-v7-sync-state.md), and [protocol 1.5 rollout](../../releases/protocol-1-5-operational-server-planes.md).
