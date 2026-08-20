---
title: "Use the synchronization and transport contracts"
description: "Reference Rust-owned sync records, one simulation/LAN/WAN frame, streams, cancellation, delivery identities, negotiation, and cache labels."
audience: "api"
page_type: "reference"
status: "active"
owner: "Rust contract and synchronization maintainers"
last_verified: "2026-08-20"
review_triggers:
  - "a type or identifier in crates/contracts/src/sync.rs changes"
  - "the shared transport frame in crates/contracts/src/sync_transport.rs changes"
keywords:
  - "ChangeRecord"
  - "SyncSnapshot"
  - "PendingCommand"
  - "ReconciliationDelivery"
  - "SyncTransportFrame"
  - "SyncTransportPayload"
  - "eitmad.sync.reconcile.v1"
---

# Use the synchronization and transport contracts

`crates/contracts/src/sync.rs` is the canonical synchronization message contract. `crates/contracts/src/sync_transport.rs` is its single streaming frame for simulation, LAN, and WAN connections. Generated JSON, C#, Swift, identifier constants, and the [mechanical protocol listing](../_generated/contracts-v1.md) derive from Rust. Native clients and network adapters must not define parallel sync records, frames, or route-specific wire protocols.

## Core types

| Type | Required meaning |
| --- | --- |
| `SyncMode` | Immutable application strategy for one persisted scope: `LocalFirst` or `ServerAuthoritative` |
| `ChangeRecord` | Scoped upsert/tombstone with change and record identity, base/result revisions, idempotency, schema/versioned bytes, and optional merge provenance |
| `SyncSnapshot` | Scoped checkpoint projection with server generation and explicit cache validity deadline |
| `PendingCommand` | Idempotent server intent with principal-only attribution, typed bytes, and optional optimistic projection |
| `SyncMetadata` | Connection, checkpoint, last success, generation, and cache deadline for the configured mode |
| `ConflictRecord` | Local and remote inputs, open/resolved status, and optional `MergeMetadata` |
| `ReconciliationDelivery` | Idempotent delivery envelope containing optional snapshot, incremental changes, and command results |
| `RecordView` | Record plus `RecordAuthority` and `CacheFreshness`; consumers must preserve both labels |
| `SyncEvent` | Connection, queue, conflict, denial, snapshot metadata, duplicate, and status lifecycle value |
| `SyncTransportFrame` | Route-independent envelope with frame/idempotency identity, negotiated protocol, correlation, stream sequence/end, and payload |
| `SyncTransportPayload` | Existing `SyncMessage`, cancellation, heartbeat, or heartbeat acknowledgement |

An upsert without `payload` and a tombstone with `payload` are invalid. `EncodedDomainPayload.schema_id` and `schema_version` select a previously negotiated domain schema; the sync layer treats `base64` as opaque.

## Identities and revisions

`RecordId` identifies the domain record. `ChangeId` identifies one mutation and permits echo acknowledgement. `DeliveryId` identifies a received envelope. `IdempotencyKey` identifies caller intent and is checked independently from delivery identity. `Checkpoint` is opaque progress state; clients compare identity or resume from it but do not derive ordering from UUID bytes.

`base_revision` names the revision read before a mutation. `revision` names its result. A merge records `common_ancestor_revision`, both source change IDs, strategy, and time. Clock time is metadata, not the conflict winner.

## Messages

| Identifier | Payload | Use |
| --- | --- | --- |
| `eitmad.sync.negotiate.v1` | `SyncNegotiation` | Declare mode, full `PeerHello`, and resume checkpoint |
| `eitmad.sync.pull.v1` | `PullRequest` | Request at most 500 records after a checkpoint |
| `eitmad.sync.changes.v1` | `ChangeBatch` | Carry a bounded delivery and resulting checkpoint |
| `eitmad.sync.reconcile.v1` | `ReconciliationDelivery` | Apply snapshots, changes, and command outcomes idempotently |
| `eitmad.sync.acknowledge.v1` | `BatchAcknowledgement` | Acknowledge one delivery and accepted record count |
| `eitmad.sync.conflict.v1` | `ConflictNotice` | Identify a conflict and both revisions without payload disclosure |
| `eitmad.sync.backpressure.v1` | `RetryAfter` | Request bounded retry using a stable error reference |

Normal traffic starts only after `versioning::negotiate` accepts a common protocol, all required capabilities, and required schema ranges. The Rust transport core always offers and requires `eitmad.capability.sync.v1`; a peer without it is incompatible. A `SyncMode` mismatch is also incompatible. Unknown required messages fail; unknown object fields remain the additive compatibility mechanism.

## Shared streaming frame

`SyncTransportFrame` carries the same `SyncMessage` on simulation, LAN, direct WAN, and relayed WAN routes. Route choice does not appear in the frame and cannot change message meaning. Each stream starts at sequence `0`, increments by one, and closes its direction when `endOfStream` is true.

`SyncTransportPayload::Cancel` contains the stream ID, optional last accepted sequence, and one stable cancellation reason: client request, deadline, superseded work, or shutdown. Its stream ID must match the enclosing frame stream ID. Cancellation uses the same frame, ordering, correlation, version, and idempotency rules as data. Repeating local cancellation for an already cancelled or completed outgoing stream does not send another frame. Heartbeat values carry only a Unix-millisecond send time; they are connection-health signals, not reconciliation acknowledgements.

`frameId` and `idempotencyKey` are independent. The Rust transport core ignores an exact retained retry when either identity repeats. It fails closed if a retained identity is reused for different logical frame content. Adapter-local retention is bounded to 4,096 accepted pairs and 8,192 terminal stream IDs; it is not durable across adapter replacement. Domain deliveries still use `DeliveryId` plus `IdempotencyKey` and the durable engine replay rules described below.

The wire frame contains no credential, authentication proof, account, device secret, endpoint, or encryption downgrade flag. Authentication and encrypted-session evidence belong to the Rust `SyncTransport` connection boundary before negotiation. A native shell must never inject a token or secret into a frame.

## Server command outcomes

`CommandDisposition::Accepted` may include one authoritative change. `Denied` includes only an `ErrorCodeRef`. A client removes the matching pending command in both cases. It installs the authoritative result on acceptance and rebuilds optimistic state from confirmed cache plus remaining commands on denial.

Adjacent enum payload fields serialize in camel case. `SyncEvent::SnapshotApplied` contains `snapshotId`, `checkpoint`, and `records`, not the complete snapshot; authorized Rust consumers use `SyncEngine::read_last_snapshot` when they need its bounded records. `PendingCommand.submittedBy` is a `PrincipalId`. Full authorization/session context remains only at the boundary and in protected audit storage.

No outcome contains raw authorization reasoning or server error prose. Shells localize stable message IDs at their normal error boundary.

## Cache and status projection

`RecordAuthority` distinguishes `LocalDurable`, `ServerConfirmed`, and `Optimistic`. `CacheFreshness` distinguishes `Fresh` and `Stale`. A stale optimistic value may be displayed only as provisional; a stale server-confirmed value is withheld by authorized `SyncEngine::read_record`.

`SyncStatus` is a coalesced projection: offline, current checkpoint, queued count, progress, conflict count, or stable failure reference. It does not replace detailed conflict, command, or delivery records.

## Bounds, authorization, and audit

- `ChangeBatch.records`, `SyncSnapshot.records`, and `ReconciliationDelivery.changes` reject more than 500 records during deserialization; `PullRequest.maximum_records` is `1..=500`.
- Every record and snapshot has an explicit `ScopeRef`.
- Actor/tenant/workspace assertions are verified against the authenticated boundary and ReBAC policy before work.
- Accepted mutations and `SyncBoundary` audit commit atomically in Rust-owned storage.
- `SyncEngine::connect`, `disconnect`, replay, record reads, and snapshot reads require the current actor/request/audit context; authorization precedes replay lookup. Transport connection establishment does not replace this ReBAC boundary.
- Payload bytes, command bytes, tokens, customer text, and relationship graphs are forbidden in routine logs and audit identifiers.

## Generate and verify

```powershell
npm run contracts:generate --prefix crates/contracts/codegen
```

```powershell
npm run contracts:verify --prefix crates/contracts/codegen
```

The generated schema root exports `SyncMessage`, `SyncTransportFrame`, and `SyncStatus`; referenced sync types are included transitively. See [protocol v1 contracts](index.md), [sync engine and transport ownership](../developer/subsystems/synchronization.md), [sync failure recovery](../troubleshooting/synchronization-failures.md), and [contract evolution](evolve-contracts-compatibly.md).
