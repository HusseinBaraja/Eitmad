---
title: "Threat-model Windows local IPC"
description: "Review trust boundaries, current development authentication, attacks, controls, and production blockers for local engine IPC."
audience: "architecture"
page_type: "explanation"
status: "active"
owner: "security and Rust engine maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "local peer authentication, pipe discovery, identity, authorization, transport, or production packaging changes"
keywords:
  - "local IPC threat model"
  - "named pipe authentication"
  - "development bearer token"
---

# Threat-model Windows local IPC

The named pipe is an untrusted process boundary. Current authentication is development-only and blocks production release until replaced; the engine still owns every command, query, and subscription authorization decision.

## Assets and actors

Protected assets are domain data, scope boundaries, session identity, command integrity, audit causation, availability, and development bearer tokens. Actors include the intended shell, Rust engine, other same-user processes, elevated processes, malware, stale engine generations, and accidental incompatible clients.

## Threats and implemented controls

| Threat | Current control | Residual risk |
| --- | --- | --- |
| Connect to a guessed pipe | Unique random endpoint plus 256-bit bearer token | A process able to inspect the child environment may recover the token |
| Replay a request on another connection | Engine-issued session bound to one connection and exact context | Development identity itself is asserted, not verified |
| Protocol downgrade or drift | Mandatory `PeerHello`, highest common version across `1.0–1.2`, operation/capability gates, generated bindings | A malicious client may send known newer shapes under an older version; dispatcher minor gates fail them explicitly |
| Deliver state after access revocation | Policy-change signal plus authorization immediately before every event delivery | Storage unavailability closes the stream fail-safe and may reduce availability |
| Memory exhaustion | 8 MiB frame cap, 1,024-entry/16 MiB replay cap, and 256-event delivery queues | Repeated allowed-size traffic still consumes bounded work |
| Request starvation | Per-request deadlines, concurrent dispatch, bounded shutdown | Domain handlers must implement their own resource bounds |
| Cross-scope replay or cursor probing | Exact session/scope authorization, embedded-scope validation, and indistinguishable invalid-cursor errors | A compromised authenticated same-scope peer can still consume authorized event volume |
| Slow-consumer event loss | Replaceable state coalesces; discrete gaps close explicitly and require replay/resync | Repeated lag can reduce shell availability |
| Confuse late responses | Request correlation and pending-map removal | A timed-out command may still complete; outcome is explicitly unknown |
| Leak secrets through diagnostics | No token or payload logging; structured allowlisted errors | Debuggers and privileged local processes remain outside this control |
| Orphan or hang the engine | stdin abandonment signal, typed shutdown, 15-second Job Object fallback | Forced exit is crash recovery and cannot guarantee unfinished work |

## Production blockers

Production must not enable `--allow-insecure-development-auth`. The real configuration dispatcher now applies direct scoped ReBAC and audited mutation transactions, but development identity and scope remain asserted. Before release, replace that handshake with reviewed local peer authentication, derive identity and scope from trusted Rust-owned session establishment, define credential rotation and revocation, and test hostile same-user processes. Windows pipe ACL hardening is defense in depth, not a replacement for peer authentication.

No Arabic customer text is interpreted during authentication. Canonical UTF-8 payloads remain opaque to the transport, presentation bidi controls are not added, and structured subscription failures expose no policy graph, cursor owner, or customer data.

Review [ADR-0017](../decisions/0017-windows-named-pipe-local-ipc.md), the [local IPC subsystem](../developer/subsystems/local-ipc.md), and [zero-trust ADR-0009](../decisions/0009-zero-trust-security-model.md) before changing this boundary.
