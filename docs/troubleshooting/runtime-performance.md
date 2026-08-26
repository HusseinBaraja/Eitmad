---
title: "Diagnose startup, idle, IPC, and sync performance"
description: "Measure engine readiness, idle resource use, common IPC latency, and background sync against explicit bounded expectations."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "runtime, IPC, synchronization, and Windows adapter maintainers"
last_verified: "2026-08-26"
review_triggers:
  - "performance expectations, diagnostics, IPC limits, retry policy, sync batches, or startup work changes"
keywords:
  - "performance diagnostics"
  - "elapsedMicros"
  - "idle CPU"
  - "IPC latency"
  - "background sync"
  - "بطء المحرك"
---

# Diagnose startup, idle, IPC, and sync performance

Use this path when startup feels slow, the idle app consumes resources, normal local IPC calls lag, or synchronization creates sustained work. The values below are engineering regression expectations on a supported release workstation, not remote-network service-level guarantees.

## Built-in diagnostic fields and expectations

Run the non-mutating diagnostic command:

```powershell
cargo run -q -p eitmad-engine-cli -- diagnose
```

`DiagnosticReport.elapsedMicros` is total preflight time. Every `checks[]` item includes `elapsedMicros`. `performanceExpectations` reports:

| Metric | Expectation |
| --- | ---: |
| Ready startup | at most 3,000 ms |
| Idle CPU | at most 50 basis points (0.50%) after stabilization |
| Idle working set | at most 150 MiB |
| Common local query p95 | at most 50 ms |
| Common local command p95 | at most 100 ms |
| Background sync batch | at most 50 reference records |
| Background sync cycle | at most 1,000 ms of active work before yielding |

The diagnostic report does not contain paths, product payloads, secrets, relationship graphs, or raw errors. Record stable check IDs, elapsed values, product/protocol version, correlation IDs, and sanitized process counters only.

## Fast checks

1. Run `diagnose` three times against the same stopped runtime directory. Separate one-time OS cache effects from repeatable slow checks.
2. Start the Windows app and wait two minutes without interaction or pending sync. Compare process CPU time over a five-minute window and record working set. The engine and shell use blocking pipe, channel, subscription, signal, and process waits; there is no product-state polling loop to tune.
3. Exercise 100 synthetic `GetConfiguration` or `ListReferenceMarkers` queries and 100 synthetic marker commands through the named pipe. Use unique create IDs or correct expected revisions and idempotency keys. Record p50, p95, maximum, error count, and frame size. Do not use customer scopes.
4. Queue synthetic marker work offline, then reconnect. Confirm batches never exceed 50, transport attempts stop after five failures, and cancellation stops reconnect delay promptly.
5. Compare results with a release build. Debug builds are useful for functional diagnosis but not performance acceptance.

## Evidence and resolution

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| One readiness check dominates `elapsedMicros` | Slow check or storage/environment fault | Match its stable check ID to the owning subsystem | Repair that check; the started-store check must reuse the open SQLite authority instead of copying it |
| Startup stops at event recovery | More than 1,024 durable publications or a malformed row | Count sanitized outbox rows while the engine is stopped through an approved diagnostic | Escalate to runtime/storage maintainers; do not clear the outbox |
| Idle CPU grows with no work | New timer, polling loop, retry loop, or busy channel consumer | Search production code for `interval`, `Task.Delay`, and loops without blocking waits | Replace polling with subscriptions/signals or add bounded cancellation and backoff |
| Working set grows with events or requests | Unbounded in-flight work, replay, queue, or payload | Check 64 in-flight IPC, 1,024 replay event, 2,048 sync queue, and frame limits | Restore the bound or reject work with backpressure; do not increase it without evidence |
| Common IPC p95 exceeds its budget | Chatty call sequence, repeated JSON conversion, large frame, disk wait, or lock contention | Compare request count, frame size, and command/query split | Use typed command results, parallel independent queries, one-pass conversion, compact events, and paged reads |
| Sync consumes disk or network continuously | Oversized batch, retry loop, duplicate scheduler, or unacknowledged work | Check batch count, attempt count, cancellation, and idempotency reuse | Keep batches at 50, yield each cycle, stop after five attempts, and run one scheduler per scope |

## Current hard limits

- Local IPC frame: 8 MiB.
- Concurrent command/query work per connection: 64.
- Active subscriptions per connection: 64.
- Local IPC replay: 1,024 events and 16 MiB.
- Startup publication recovery: pages of 64, maximum 1,024 events.
- Reference marker page: 100 items; sync batch: 50 records.
- General sync pending local changes and pending commands: 2,048 each.
- Sync transport default retry attempts: 5, with capped backoff.

These limits prevent excessive idle memory, disk, and network amplification. Reaching a bound is a diagnostic event, not permission to add an independent retry or load the full table.

## Verify and escalate safely

After a fix, rerun release diagnostics, common IPC sampling, one idle window, background sync, focused tests, and clean app start/stop. Escalate with versions, stable IDs, aggregate timings, counts, and sanitized resource samples. Do not attach database files, raw frames, labels, bearer tokens, paths, or customer data.

Related pages: [engine startup](engine-startup-failures.md), [local IPC failures](local-ipc-failures.md), [synchronization failures](synchronization-failures.md), [engine runtime](../developer/subsystems/engine-runtime.md), and [reference marker](../developer/subsystems/reference-marker.md).
