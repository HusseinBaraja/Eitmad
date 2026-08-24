---
title: "Extend WAN relay coordination safely"
description: "Understand relay sessions, peer and server routes, reconnect policy, tenant isolation, authorization, audit, and failure reporting."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "connectivity maintainers"
last_verified: "2026-08-24"
review_triggers:
  - "relay session, routing, reconnect, authorization, audit, quota, or deployment behavior changes"
keywords:
  - "WAN relay"
  - "RelaySessionMetadata"
  - "eitmad.capability.server-relay.v1"
  - "eitmad.error.relay-unavailable.v1"
---

# Extend WAN relay coordination safely

The relay plane brokers authenticated WAN connection metadata when a direct server route is unavailable. It does not own synchronization meaning, inspect business payloads, or grant product permissions.

## Authority and boundaries

| Concern | Rust authority |
| --- | --- |
| External metadata, route, health, and failure shapes | `crates/contracts/src/relay.rs` |
| Session lifecycle, retry timing, health, and bounded failure reports | `server/relay-plane/src/lib.rs` |
| Relationship authorization and append-only audit | `server/control-plane/src/access.rs` through `server/host/src/planes.rs` |
| Authenticated HTTP routes | `server/host/src/http.rs` |
| Direct-WAN then relay selection in the engine | `crates/sync/src/transport/wan.rs` |

`RelayRouter` is the explicit peer/server routing hook. A production router may establish or close transport resources, but it must keep `SyncTransportFrame` opaque and preserve the same authentication, encryption, ordering, idempotency, and reconciliation rules as direct WAN.

## Session lifecycle and reconnect

`RelayCoordinator::open` requires the authenticated device to match `source_device_id`, a tenant member relationship, a route whose peer device belongs to the same tenant, and a TTL from `1` ms through `3,600,000` ms. The default caller TTL is `900,000` ms. A successful session moves from `Connecting` to `Active` only after routing and audit succeed.

An active device may heartbeat its own session. A failed route uses `schedule_reconnect`, which increments a bounded attempt counter and sets `next_reconnect_at` with exponential delay. `reconnect_due` rejects early attempts with `RetryNotDue`, restores `Active` after a successful hook, and stops after eight attempts or session expiry. Close is idempotent. Administrative close is a distinct owner-only relay action.

Health is tenant scoped. It reports active, reconnecting, and failed counts plus whether the process accepts new sessions. Failure reports contain only stable phase, code, retry, time, device, tenant, session, and correlation metadata. The process retains at most 1,024 reports; it does not retain payload bytes or free-text customer content.

## Authorization, tenant isolation, and audit

Every public lifecycle or health operation calls `RelaySecurity` before state access and calls its audit method for success, denial, invalid input, or failure. Normal relay actions require tenant membership. `AdministrativeClose` requires tenant ownership. Peer routing additionally checks that the target device is registered and not revoked in the actor's tenant.

Cross-tenant session access returns denial or not-found semantics without returning metadata. Denied attempts write the audit row under the actor's tenant. Audit rows contain stable operation and target kinds, correlation ID, and redacted error IDs; they never contain relayed frames.

## Failure and recovery

| Failure | State guarantee | Safe recovery |
| --- | --- | --- |
| `Denied` | No route or metadata mutation occurs; denial audit is attempted | Repair tenant membership or ownership; do not bypass the route check |
| `Invalid` or `RetryNotDue` | Existing session metadata remains | Correct the source device, TTL, state, or wait until `next_reconnect_at` |
| `RouteUnavailable` | Pending synchronization work remains outside the relay | Restore the peer/server hook and use bounded reconnect |
| `NotFound` | No cross-tenant session metadata is disclosed | Verify the tenant-scoped session ID and correlation ID |
| `Unavailable` | The operation fails closed | Restore authorization, audit, or relay state availability before retry |

Use [server-plane troubleshooting](../../troubleshooting/server-plane-failures.md) for symptom-led checks.

## Arabic and mixed-direction behavior

The relay has no UI and treats sync payloads as opaque bytes. It does not normalize Arabic or mixed text such as `خزانة Wardrobe 120 cm`. Future shells may use the provisional Arabic term `مرحّل المزامنة`, but they must localize stable state and error IDs and isolate LTR identifiers inside RTL layouts.

## Tests and safe extension

`server/relay-plane/src/lib.rs` tests the complete open, heartbeat, reconnect, and close lifecycle; denied sessions; audit calls; tenant isolation; and tenant-scoped failure visibility. `server/host/src/http.rs` verifies that relay routes reject unauthenticated calls. The existing WAN transport tests verify that relay fallback never bypasses authentication, encryption, or compatibility failures.

Run:

```powershell
cargo test -p eitmad-relay-plane -p eitmad-server -p eitmad-sync
```

Add quotas, frame forwarding, or a separate relay process only through bounded, authenticated hooks. Preserve tenant checks, opaque payload handling, redacted audit, retry limits, and protocol `1.5` contracts. Related pages: [synchronization](synchronization.md), [server authority](server-authority.md), [ADR-0026](../../decisions/0026-compose-authorized-operational-server-planes.md), and [protocol 1.5 rollout](../../releases/protocol-1-5-operational-server-planes.md).
