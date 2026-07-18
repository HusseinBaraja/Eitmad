---
title: "Resolve local IPC connection and request failures"
description: "Diagnose unavailable engines, subscription capacity, resync, backpressure, version mismatch, deadlines, and payload limits safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust engine and Windows platform maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "local IPC errors, timeouts, payload bounds, negotiation, or recovery changes"
keywords:
  - "EngineUnavailable"
  - "eitmad.error.ipc-session-invalid.v1"
  - "eitmad.error.ipc-deadline-exceeded.v1"
  - "eitmad.error.ipc-payload-too-large.v1"
  - "eitmad.error.ipc-subscription-resync-required.v1"
  - "eitmad.error.ipc-subscription-unsupported.v1"
  - "eitmad.error.ipc-subscription-capacity-exceeded.v1"
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
| `eitmad.error.ipc-subscription-unsupported.v1` | Peer negotiated `1.0`, omitted the subscription capability, or requested a `1.2` policy stream under `1.1` | Capture negotiated protocol and capability IDs | Use a compatible engine/shell pair and required capability; do not replace subscriptions with UI polling |
| `eitmad.error.ipc-subscription-capacity-exceeded.v1` | One client connection already owns 64 active subscriptions | Count active feature subscriptions without recording payloads or scope data | Reuse or explicitly unsubscribe obsolete streams; do not raise the ceiling without a resource review |
| `eitmad.error.ipc-subscription-resync-required.v1` | Cursor expired, came from another generation, stream, or scope | Compare engine generation and subscription kind without exposing cursor ownership | Subscribe fresh, query the authoritative scoped projection, then apply buffered live events |
| `SubscriptionClosed` with `backpressure` | A discrete record, notification, or error gap exceeded replay | Preserve the last processed cursor and sanitized event kind | Allow supervision to reconnect/resubscribe; if resync is required, query current state before continuing |
| `SubscriptionClosed` with `authorizationRevoked` | Protocol `1.2` policy reauthorization denied the stream | Preserve only principal/scope identifiers approved for support | Discard later buffered state and resubscribe only after an owner restores access |
| Protocol `1.0`/`1.1` connection closes after policy change | Revocation cannot use the `1.2` close reason safely | Confirm policy revision changed in the same scope | Reconnect with compatible bindings; never continue from cached protected state |
| Repeated `SubscriptionBackpressure` | Shell consumer is slower than event production | Measure queue depth/event rate without recording payloads | Fix consumer work or reduce producer frequency; do not enlarge bounds without a resource review |
| Connection closes after a large response | Encoded response exceeded 8 MiB or response serialization failed | Check the stable request type and sanitized frame-size metric | Reduce or page the response, reconnect, and retry only when the operation's retry contract permits it |
| `local IPC server failed` or `local IPC server task failed` on stderr | The accept loop returned an I/O error or the task panicked | Preserve the sanitized message and latest lifecycle state | Stop new work, restart through the supervisor, and escalate to the Rust IPC owner if it recurs |
| Forced shutdown | IPC and stdin drain did not exit in 15 seconds | Preserve lifecycle state and stable error IDs | Treat as crash recovery and diagnose the Rust component that failed to drain |

Do not delete runtime files, expose bearer tokens, copy raw frames, or include customer records, paths, authorization graphs, or process environment values in support evidence.

## Verify recovery

Confirm a new engine generation reaches `Ready`, the handshake reports the expected version within `1.0–1.2`, and required capabilities are present. For a `1.2` client, verify one synthetic scoped subscription receives an ordered event and a revoked synthetic member receives no later configuration event. Intentional shutdown must reach `Stopping → Stopped` with exit code `0`.

Escalate with engine/shell versions, Windows version, generation, `EngineInstanceId`, stable error code, message ID, retry disposition, timestamp, and correlation ID. See [typed local IPC](../developer/subsystems/local-ipc.md), [Windows supervision](../developer/subsystems/windows-process-supervision.md), and the [threat model](../architecture/local-ipc-threat-model.md).
