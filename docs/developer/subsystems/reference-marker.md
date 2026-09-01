---
title: "Trace the reference marker through every architecture boundary"
description: "Use the small product-neutral reference marker to understand contracts, Rust authority, storage, authorization, audit, sync, and the Windows RTL shell."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "reference marker maintainers"
last_verified: "2026-09-01"
review_triggers:
  - "reference marker contracts, permissions, migration, sync projection, event, or Windows view changes"
keywords:
  - "reference marker"
  - "الميزة المرجعية"
  - "مرجع REF-١٢"
  - "eitmad.reference-marker.upsert.v1"
  - "reference-marker.initial.v1"
---

# Trace the reference marker through every architecture boundary

The reference marker is an intentionally small, product-neutral feature that proves the complete desktop architecture. It stores one or more short labels in an organization scope. It is a reference implementation, not a furniture, sales, or workflow concept.

## Purpose and ownership

The primary reader is an engineer who needs a safe example before adding a real vertical feature. The feature shows the required seams without introducing product policy.

| Concern | Authority |
| --- | --- |
| Domain orchestration, sync projection, and audit construction | `crates/reference-marker` |
| Typed command, query, subscription, event, values, capability, schema, permissions, and errors | `crates/contracts` |
| ReBAC permission decision | `crates/authorization` |
| SQLite migration and atomic repository | `crates/storage/src/reference_marker.rs` |
| IPC dispatch and durable event publication | `crates/engine-runtime` |
| Generated C# binding and Windows process/IPC adapter | `shells/windows/generated` and `platform-adapters/windows` |
| Arabic display and ephemeral input state | `shells/windows/Features/Operations` and `MainWindow.xaml` |

Rust remains authoritative. The Windows shell cannot validate labels, decide permission, open SQLite, create sync records, or write audit rows.

## Contracts and bounds

Capability `eitmad.capability.reference-marker.v1` and schema `eitmad.schema.reference-marker.v1` are optional negotiated features at protocol `1.5`. Current Windows and engine peers advertise schema version `1`.

| Interaction | Identifier | Result or payload |
| --- | --- | --- |
| Command | `eitmad.reference-marker.upsert.v1` | `ReferenceMarkerUpserted(ReferenceMarker)` |
| Query | `eitmad.reference-marker.list.v1` | `ReferenceMarkerPage` |
| Subscription | `eitmad.reference-marker.changed.subscribe.v1` | resumable discrete stream |
| Event | `eitmad.reference-marker.changed.event.v1` | compact ID, scope, revision, time, and change ID |

`ReferenceMarkerLabel` preserves Unicode bytes and mixed direction. It rejects an empty value, surrounding whitespace, control characters, bidirectional formatting controls, and UTF-8 data above 256 bytes. Arabic letters and join controls remain valid. `ListReferenceMarkers` requires `limit` from `1` through `100`; the repository reads `limit + 1` to produce `next`. The event omits the label so subscription traffic cannot become a large inline state copy. The shell queries the bounded page after a change notice.

Representative accepted text is `مرجع REF-١٢`. Rust does not insert bidi controls or normalize this value.

## Command, permission, storage, and audit flow

1. The shell sends `UpsertReferenceMarker` with a non-empty idempotency key and `expectedRevision` (`null` for create).
2. `ProductDispatcher` creates the mandatory mutation context.
3. `ReferenceMarkerService` requires `eitmad.permission.reference-marker.write.v1`. Organization owner and configuration-manager relationships receive write permission; a member receives only `eitmad.permission.reference-marker.read.v1`.
4. Rust builds revision `1` for create or `expectedRevision + 1` for update. SQLite compares the exact stored revision.
5. One immediate transaction writes `reference_markers`, `mutation_audit`, `idempotency_records`, `publication_outbox`, and `reference_marker_sync_outbox`.
6. After commit, the dispatcher publishes the compact event and completes its durable publication row. A crash before completion leaves the event for bounded startup recovery.
7. The command response contains the committed marker. The shell applies it directly and does not issue a redundant query.

A denial writes an audit result without storing the label. A revision mismatch writes a conflict audit and returns `eitmad.error.reference-marker-revision-conflict.v1`. Reusing an idempotency key with different bytes fails as an invalid contract. Audit targets contain the marker UUID and the changed field name, not the label.

## Storage migration and recovery

Migration `reference-marker.initial.v1` is storage version `8`. It creates:

- `reference_markers`, keyed by scope kind, scope ID, and marker ID;
- `reference_marker_sync_outbox`, keyed by change ID with an exact marker foreign key.

Supported versions `2` through `7` upgrade after the normal validated pre-migration backup. An engine older than storage version `8` cannot open the migrated database. Do not edit `schema_migrations`, marker rows, or outbox rows manually. Follow the [storage version 8 release note](../../releases/storage-v8-reference-marker.md) and [local storage recovery](../../operations/recover-local-storage.md).

## Local-first sync behavior

Every committed marker mutation creates one shared `ChangeRecord` with `ChangeOperation::Upsert`, the marker UUID as `RecordId`, base and resulting revisions, the command idempotency key, and a base64 JSON payload under `eitmad.schema.reference-marker.v1` version `1`. The marker remains `Pending` until the exact change ID is confirmed. Confirmation removes that work and marks the marker `Confirmed` only when no later change for the same marker remains.

`reference_marker_sync_batch` accepts `1..=50`; it never loads the entire outbox. Failed transport does not delete work. This slice supplies durable local-first behavior and the common sync record. It does not add a production scheduler, remote domain registration, or conflict UI. A future connector must use the shared sync transport and preserve ordering, idempotency, scope, cancellation, retry, and conflict rules.

## Windows Arabic and RTL behavior

The operations center displays **الميزة المرجعية**, **حفظ العلامة**, **بانتظار المزامنة**, and **متزامن**. The input and label use RTL flow. Technical constraints use isolated LTR text. The mixed example `مرجع REF-١٢` verifies Arabic letters, Latin letters, and Arabic-Indic digits without changing stored text.

On connection, the coordinator issues configuration, sync, update, and marker snapshot queries concurrently. It owns seven resumable subscriptions. A marker event updates the visible revision status and triggers one bounded marker query. A successful marker command uses its typed result immediately. The shell does not claim server confirmation while `syncState` is `pending`.

Keyboard traversal, Arabic screen-reader announcements, high contrast, and 200% text scaling still require release-candidate verification. This gap is owned by Windows UI maintainers.

## Failure modes

| Failure | Preserved state | Safe recovery |
| --- | --- | --- |
| Permission denial | No marker or sync work is stored; denial audit is attempted | Repair the scoped relationship, then send new intent with a new key |
| Revision conflict | Existing marker and outboxes remain unchanged | Query a bounded fresh page, review the value, then submit against its revision |
| Command timeout | Outcome can be unknown | Retry only the same command with the same idempotency key |
| Event publication failure | Committed marker, audit, idempotency, and event outbox remain durable | Restart; bounded startup recovery republishes at most 1,024 events |
| Sync transport failure | Marker remains `Pending`; durable sync work remains | Restore the route and retry under the shared bounded retry policy |
| Storage or audit failure | Transaction rolls back or the request fails closed | Preserve the runtime directory and follow storage troubleshooting |
| Unsupported capability or schema | Peer must not use the feature | Upgrade the engine and generated binding together |

## Tests and verification

The evidence map is:

- contract bounds and Arabic/mixed text: `crates/contracts/src/reference_marker.rs`;
- permission mapping: `crates/authorization/src/lib.rs`;
- atomic storage, page, and sync behavior: `crates/storage/src/reference_marker.rs` and `crates/reference-marker/src/lib.rs` tests;
- dispatcher, audit, query, event, and outbox recovery: `crates/engine-runtime/src/dispatcher.rs` tests;
- C# codec, supervision, command seam, reconnect, and cancellation: `platform-adapters/windows/tests`;
- RTL display, typed state mapping, reconnect, resync, and ownership: `shells/windows/tests`;
- generated contract compatibility: `crates/contracts/codegen` and `tests/contract-compatibility`.

Run:

```powershell
cargo test -p eitmad-reference-marker -p eitmad-storage -p eitmad-engine-runtime
```

```powershell
npm run contracts:verify --prefix crates/contracts/codegen
```

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo
```

## Tradeoffs and extension points

The label is stored as text instead of arbitrary JSON so the example remains bounded and reviewable. The event carries a notice instead of the label to reduce IPC and replay memory. Marker writes always advance revision, even when the new label text matches; this keeps command, audit, event, and sync behavior explicit in the reference path.

Add real product features as sibling verticals. Reuse the typed envelope, authorization, audit, storage transaction, publication outbox, shared sync record, and thin-shell patterns. Do not add product-specific fields to the reference marker or treat it as a generic key/value store.

Related pages: [contract reference](../../api/index.md), [local storage](local-storage.md), [synchronization](synchronization.md), [Windows shell](windows-native-shell.md), [runtime performance](../../troubleshooting/runtime-performance.md), and [Arabic-first checklist](../contributing/arabic-first-feature-checklist.md).
