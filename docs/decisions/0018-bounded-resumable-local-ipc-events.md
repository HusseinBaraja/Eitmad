---
title: "ADR-0018: Use bounded resumable local IPC event streams"
description: "Records replay lifetime, ordering, backpressure, scope isolation, and reconnect behavior for engine-to-shell events."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture and Rust engine maintainers"
last_verified: "2026-07-13"
review_triggers:
  - "local event replay, ordering, backpressure, scope, or reconnect semantics change"
keywords:
  - "ADR-0018"
  - "IPC subscription replay"
  - "event backpressure"
---

# ADR-0018: Use bounded resumable local IPC event streams

> - Status: Accepted
> - Date: 2026-07-13
> - Decision owners: Architecture, Rust engine, and Windows platform maintainers
> - Supersedes: None
> - Superseded by: None

## Context

Thin shells need engine-driven configuration, permission, sync, record, job, notification, update, and error changes without polling. A slow or restarted shell must not block Rust authority, silently lose discrete changes, or replay another scope's data.

## Decision

Protocol `1.1` adds capability-gated subscribe, unsubscribe, event, and subscription-closed messages. Rust authorizes the exact session and scope before cursor lookup. Replay is in-memory for one engine generation, bounded to 1,024 entries and 16 MiB. Cursors are opaque; invalid, expired, wrong-stream, and wrong-scope cursors share one resync-required failure.

Ordering is per subscription. Replay precedes live delivery, sequence starts at `1` for each subscription instance, and delivery is at least once from the last cursor acknowledged after shell processing. There is no global order across streams or request responses.

Replaceable configuration, permission, sync, update, and per-job state may coalesce to the newest retained value. Record changes, notifications, and errors never drop silently. An unreplayable discrete gap closes the stream with backpressure; Windows reconnects and resubscribes. Engine restart requires a fresh subscription and authoritative query.

## Consequences

Authoritative producers remain independent of shell speed, resource use is bounded, and duplicate delivery is explicit. Replay does not survive engine restart, transient notification/error history can be unavailable, and every consuming vertical must provide an authoritative query before depending on record invalidations or state resync.

Subscription operations are read-only and add no audit record. State-producing commands and jobs retain their owning vertical's ReBAC, scope, and audit duties. Unicode is preserved without presentation bidi controls; shells localize message IDs and control Arabic dynamic announcements.

## Alternatives

- Durable replay across restarts was rejected for this foundation because it would add storage schema, retention, migration, backup, and recovery obligations.
- Dropping oldest events was rejected because it can leave shell projections silently stale.
- Blocking producers was rejected because shell responsiveness must not stall authoritative engine work.
- Full record payload events were rejected because they expand privacy, schema, frame-size, and stale-truth risks.

## Verification and related guidance

Rust tests cover protocol gating, scope mismatch, replay, ordering, coalescing, and discrete overflow. Windows tests cover bounded queues, processed cursors, and stable reattachment. See [typed local IPC](../developer/subsystems/local-ipc.md), the [threat model](../architecture/local-ipc-threat-model.md), and [recovery guidance](../troubleshooting/local-ipc-failures.md).
