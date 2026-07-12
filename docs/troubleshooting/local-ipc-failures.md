---
title: "Resolve local IPC connection and request failures"
description: "Diagnose unavailable engines, rejected handshakes, version mismatch, deadlines, payload limits, and shutdown fallback safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust engine and Windows platform maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "local IPC errors, timeouts, payload bounds, negotiation, or recovery changes"
keywords:
  - "EngineUnavailable"
  - "eitmad.error.ipc-session-invalid.v1"
  - "eitmad.error.ipc-deadline-exceeded.v1"
  - "eitmad.error.ipc-payload-too-large.v1"
---

# Resolve local IPC connection and request failures

Local IPC failures do not by themselves imply lost committed data. Stop new work, preserve the correlation ID and negotiated version, and use non-destructive checks before restarting the supervised engine.

## Symptoms and fast checks

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| `EngineUnavailable` | Engine absent, starting, or endpoint stale | Check the latest `LifecycleSnapshot` and supervisor generation | Wait for `Ready`; retry connection once against the current generation |
| Handshake authentication rejection | Development auth disabled or token differs | Confirm whether this is an approved development run | Enable the explicit development option only in a synthetic local environment; never enable it in production |
| `eitmad.error.protocol-incompatible.v1` | No common protocol/capability | Capture both product versions and `PeerHello` ranges | Install a compatible shell/engine pair; do not bypass negotiation |
| `eitmad.error.ipc-session-invalid.v1` | Reused/stale session or changed context | Compare engine generation and request session ID | Reconnect and rebuild envelopes from the new negotiated session |
| `eitmad.error.ipc-deadline-exceeded.v1` | Deadline passed before query completion | Compare deadline, current time, and operation type | Retry a query if still needed; retry a command only with the same idempotency key because outcome may be unknown |
| `eitmad.error.ipc-payload-too-large.v1` | Frame exceeds 8 MiB | Measure encoded frame size without logging content | Page or stream through a domain contract; do not raise the cap ad hoc |
| Forced shutdown | IPC and stdin drain did not exit in 15 seconds | Preserve lifecycle state and stable error IDs | Treat as crash recovery and diagnose the Rust component that failed to drain |

Do not delete runtime files, expose bearer tokens, copy raw frames, or include customer records, paths, authorization graphs, or process environment values in support evidence.

## Verify recovery

Confirm a new engine generation reaches `Ready`, the handshake reports protocol `1.0` and `eitmad.capability.local-ipc.v1`, one synthetic scoped query returns a correlated structured outcome, and intentional shutdown reaches `Stopping → Stopped` with exit code `0`.

Escalate with engine/shell versions, Windows version, generation, `EngineInstanceId`, stable error code, message ID, retry disposition, timestamp, and correlation ID. See [typed local IPC](../developer/subsystems/local-ipc.md), [Windows supervision](../developer/subsystems/windows-process-supervision.md), and the [threat model](../architecture/local-ipc-threat-model.md).
