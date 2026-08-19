---
title: "Resolve synchronization connection, queue, conflict, or cache failures"
description: "Diagnose simulation/LAN/WAN connection health, authentication, encryption, retry, relay, stream, queue, conflict, and cache failures safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "synchronization, authorization, and storage maintainers"
last_verified: "2026-08-20"
review_triggers:
  - "SyncEngineError, TransportFailure, connection health, queue, conflict, cache, or recovery behavior changes"
keywords:
  - "StaleCache"
  - "IdempotencyMismatch"
  - "IncompatiblePeer"
  - "Disconnected"
  - "AuthenticationFailed"
  - "RelayUnavailable"
  - "RetryNotReady"
  - "PartialNetwork"
  - "UnsupportedStateVersion"
  - "sync queued"
  - "المزامنة معلقة"
  - "تعارض المزامنة"
---

# Resolve synchronization connection, queue, conflict, or cache failures

The Rust sync engine preserves local-first edits offline and preserves server-authoritative commands until an explicit result. Do not delete `sync_scopes`, clear a queue, replace an idempotency key, or edit SQLite to make a symptom disappear.

## Symptoms

- Status remains queued after connectivity returns.
- A record is conflicted and local/remote values differ.
- An optimistic server-authoritative edit disappears after a denial.
- A read returns `SyncEngineError::StaleCache`.
- Reconciliation returns `Disconnected`, `IdempotencyMismatch`, `ScopeMismatch`, `IncompatibleMode`, or `IncompatiblePeer`.
- Transport connection or streaming returns `AuthenticationFailed`, `EncryptionRequired`, `VersionMismatch`, `CapabilityMismatch`, `SchemaMismatch`, `RetryNotReady`, `DuplicateConflict`, or `StreamOutOfOrder`.
- LAN health is `Degraded` with `PartialNetwork`, or discovery returns `NoLanPeer`.
- WAN health is `Degraded` on a relay, or connection returns `ServerUnavailable` or `RelayUnavailable`.
- Sync state fails to open with `StorageUnavailable`, `StorageConflict`, `CorruptState`, or `UnsupportedStateVersion`.
- Future Arabic UI may expose reviewed equivalents of `المزامنة معلقة` or `تعارض المزامنة`; no native sync workflow exists yet.

## Fast checks

1. Record the application mode, transport kind, health status, selected target class (not a secret-bearing URL), consecutive failures, `next_retry_at`, pending count, open conflict count, checkpoint presence, cache deadline, protocol range, required capabilities, schema ranges, timestamp, and correlation ID. Do not collect credentials or payload bytes.
2. Confirm the actor, engine, records, snapshot, tenant, and optional workspace refer to the same scope.
3. Confirm transport authentication and encrypted-session validation succeeded before version/capability/schema negotiation, and confirm both peers declared the same `SyncMode`.
4. For queued local-first work, confirm the delivery echoes the original `change_id`; matching only `record_id` does not acknowledge it.
5. For server-authoritative work, find the exact `PendingCommandId` result and whether it was accepted or denied.
6. Compare `cache_valid_until` with the engine's Unix-millisecond input. Do not treat a stale confirmed row as authoritative.
7. For duplicate or mismatch errors, compare delivery ID, idempotency key, and a safe fingerprint—not raw request bytes. Confirm the retry remains inside the retained 2,048-entry replay window.
8. If state version is implicated, compare the `sync_scopes.state_version` read through diagnostics with the engine-supported version. Do not edit either the row or JSON marker.
9. If storage is implicated, stop duplicate engine instances and follow local-storage diagnostics before retrying.
10. For LAN, compare reachable-peer count with `partial_failures`. For WAN, confirm whether the selected route is direct server or relay. Do not route around an authentication, encryption, capability, schema, or version failure.

## Causes and resolutions

| Evidence | Cause | Resolution | Verify |
| --- | --- | --- | --- |
| Offline local-first record is readable and pending | Expected disconnected operation | Restore compatible transport; keep the queue intact | Echoed `change_id` leaves the queue and checkpoint advances |
| `Disconnected` during reconciliation | Transport is offline, not protocol-incompatible | Restore transport and run authorized `connect` before retrying | Reconciliation no longer returns `Disconnected` |
| `AuthenticationFailed` | Device or account/device identity did not match the established session | Repair the Rust-owned `SecretId` reference or server/device registration; do not copy token bytes into diagnostics | Authenticated identity exactly matches and negotiation starts |
| `EncryptionRequired` | LAN/WAN session lacks authenticated encryption, peer authentication, or forward secrecy | Repair the production connector or certificate/session setup; never select isolated simulation security | Connection establishes with all encrypted-session expectations true |
| `VersionMismatch`, `CapabilityMismatch`, or `SchemaMismatch` | Peers cannot negotiate the required contract | Deploy compatible peers or restore the required capability/schema range; do not retry through a relay | Negotiation succeeds before frame traffic |
| `RetryNotReady` | A caller reconnected before exponential backoff elapsed | Wait until `next_retry_at`; remove any second independent retry loop | Next due attempt runs and failure count resets after success |
| LAN `Degraded` plus `PartialNetwork` | At least one discovery interface/peer failed but one peer was reachable | Continue only if degraded sync is acceptable; repair the failed LAN path and reconnect | Health becomes `Healthy` after discovery reports no partial failures |
| `NoLanPeer` or failed `PartialNetwork` | Discovery found no usable LAN peer | Check network/interface and peer service state without deleting pending work | A discovered authenticated peer connects and pending work remains |
| WAN relay route plus `Degraded` | Direct server connectivity failed and the configured relay connected | Treat the relay as temporary degraded service; repair direct connectivity | A later connection selects `WanServer` and health is `Healthy` |
| `ServerUnavailable` | Direct WAN server failed and no relay is configured | Restore server/DNS/network service and follow reported backoff | Direct server connection negotiates normally |
| `RelayUnavailable` | Both direct server and configured relay failed | Restore either route; do not alter sync frames or clear queues | One route connects and retained work transfers idempotently |
| `DuplicateConflict` | A retained frame ID or idempotency key was reused for different logical content | Stop the stream and fix identity generation; never assign a new key to hide uncertain intent | Exact retries are `DuplicateIgnored`; new intent has new frame and key identities |
| `StreamOutOfOrder` | Sequence skipped/repeated outside duplicate rules, started above zero, or continued after end-of-stream | Resume with retained stream state when supported or start genuinely new work on a new stream ID | Frames arrive from zero in order and stop at end-of-stream/cancellation |
| Remote revision advanced beyond a pending edit's base | Concurrent edit | Use the domain conflict workflow; default is defer | Conflict remains open until a truthful keep/merge decision is audited |
| Optimistic value rolled back with denied command event | Server rejected intent | Explain denial through localized authorization UI; repair permission or input through its owner | Confirmed cache value returns and denied command is absent from queue |
| `StaleCache` | Confirmed snapshot passed `valid_until` | Fetch an authorized fresh snapshot | Read returns `ServerConfirmed` plus `Fresh` |
| Stale optimistic value still visible | Pending intent overlays an expired base | Label it optimistic and stale; do not use it for authoritative decisions | Acceptance installs server value, or denial rebuilds from confirmed cache |
| `IdempotencyMismatch` | One retained key or delivery identity was reused for different content | Stop blind retry and investigate caller identity generation | Exact retained replay is ignored/replayed; new intent uses a new key |
| `ScopeMismatch` or authorization denial | Cross-scope/unpermitted delivery | Reject it, validate channel identity, and follow authorization troubleshooting | Original local state is unchanged; denial audit is present when storage is available |
| `IncompatibleMode` | Scope reopened under another strategy | Restore the configured mode; plan a data migration to change strategy | Engine opens without rewriting existing state |
| `IncompatiblePeer` | No common protocol, capability, or required schema | Deploy compatible peers or restore supported schema range | Negotiation succeeds before any delivery |
| `StorageConflict` | Competing writer revision | Stop duplicate authority and reopen from disk | One engine owns the scope and pending work remains |
| `UnsupportedStateVersion` | Row or serialized state belongs to an unsupported engine schema | Deploy a compatible engine or explicit state migration | Engine opens without manual marker or JSON edits |
| `CorruptState` or migration failure | Unreadable state/schema/history | Stop writes and use validated storage recovery | Integrity, history, migration 7, and engine reopen pass |

## Verify recovery

Use synthetic Arabic/mixed text such as `خزانة Wardrobe 120 cm` in a disposable scope. Verify disconnect/reconnect, retry gating, authentication and encryption rejection, exact and conflicting duplicate frames, cancellation, ordered end-of-stream behavior, version mismatch, LAN partial discovery, WAN relay degradation/unavailability, offline reopen, reconnect acknowledgement, conflict outcomes, denied optimistic rollback, stale-cache refusal then refresh, authorization denial, unsupported state-version rejection, and reconciliation rollback without emitted events.

Run the focused suite:

```powershell
cargo test -p eitmad-sync -p eitmad-storage -p eitmad-contracts
```

## Escalate safely

Collect stable enum/error names, failure phase, retry advice, transport kind, target class, health status, failure count, next retry time, mode, protocol/capability/schema ranges, scope kind, queue/conflict counts, revisions, checkpoint presence, cache timestamps, sanitized fingerprints, audit outcome, and correlation ID. Never attach endpoint query strings, `SecretId` values, sync state JSON, database files, payload/command bytes, customer text, account/device/principal UUIDs, relationship graphs, tokens, certificates, or raw server errors to routine tickets.

Treat any applied unauthorized or cross-scope record as a security incident. Return to [sync engine ownership](../developer/subsystems/synchronization.md), [authorization denial](authorization-boundary-denials.md), or [storage recovery](local-storage-recovery-failures.md).
