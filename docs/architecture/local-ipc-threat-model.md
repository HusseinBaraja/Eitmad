---
title: "Threat-model Windows local IPC"
description: "Review trust boundaries, supervised bootstrap authentication, attacks, controls, and residual risks for local engine IPC."
audience: "architecture"
page_type: "explanation"
status: "active"
owner: "security and Rust engine maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "local peer authentication, pipe discovery, identity, authorization, transport, or production packaging changes"
keywords:
  - "local IPC threat model"
  - "named pipe authentication"
  - "IPC bootstrap token"
---

# Threat-model Windows local IPC

The named pipe is an untrusted process boundary. The supervised bootstrap proves possession of the inherited parent-child channel, and Rust owns identity plus every command, query, and subscription authorization decision.

## Assets and actors

Protected assets are domain data, scope boundaries, session identity, command integrity, audit causation, availability, and ephemeral bootstrap tokens. Actors include the intended shell, Rust engine, other same-user processes, elevated processes, malware, stale engine generations, and accidental incompatible clients.

## Threats and implemented controls

| Threat | Current control | Residual risk |
| --- | --- | --- |
| Connect to a guessed pipe | Unique endpoint plus 256-bit token sent only through inherited standard input | Same-account malware or a privileged debugger can inspect parent memory or handles |
| Assert another tenant or role | Protocol 1.6 has no client authorization fields; Rust verifies the persisted installation identity and owner relation | The installation owner is not a human sign-in session |
| Replay a request on another connection | Exact engine-returned session and authorization context are required | The process-lifetime bootstrap token permits approved same-generation reconnect |
| Protocol downgrade or drift | Mandatory `PeerHello`, highest common version across `1.0–1.6`, required authorization-scope capability, operation/capability gates, generated bindings | The 1.6 handshake shape requires a coordinated engine and shell rollout |
| Deliver state after access revocation | Policy-change signal plus authorization immediately before every event delivery | Storage unavailability closes the stream fail-safe and may reduce availability |
| Memory exhaustion | 8 MiB frame cap, 1,024-entry/16 MiB replay cap, and 256-event delivery queues | Repeated allowed-size traffic still consumes bounded work |
| Request starvation | Per-request deadlines, concurrent dispatch, bounded shutdown | Domain handlers must implement their own resource bounds |
| Cross-scope replay or cursor probing | Exact session/scope authorization, embedded-scope validation, and indistinguishable invalid-cursor errors | A compromised authenticated same-scope peer can still consume authorized event volume |
| Slow-consumer event loss | Replaceable state coalesces; discrete gaps close explicitly and require replay/resync | Repeated lag can reduce shell availability |
| Confuse late responses | Request correlation and pending-map removal | A timed-out command may still complete; outcome is explicitly unknown |
| Leak secrets through diagnostics | No token or payload logging; structured allowlisted errors | Debuggers and privileged local processes remain outside this control |
| Orphan or hang the engine | stdin abandonment signal, typed shutdown, 15-second Job Object fallback | Forced exit is crash recovery and cannot guarantee unfinished work |

## Residual release requirements

The base app no longer trusts shell identity. Before a shared-machine or multi-user product release, add human sign-in, session rotation and revocation, hostile same-user process tests, and an explicit OS-account support policy. Keep Windows pipe ACL hardening as defense in depth. Do not add identity, tenant, workspace, scope, role, or permission assertions back to the shell contract.

No Arabic customer text is interpreted during authentication. Canonical UTF-8 payloads remain opaque to the transport, presentation bidi controls are not added, and structured subscription failures expose no policy graph, cursor owner, or customer data.

Review [ADR-0017](../decisions/0017-windows-named-pipe-local-ipc.md), the [local IPC subsystem](../developer/subsystems/local-ipc.md), and [zero-trust ADR-0009](../decisions/0009-zero-trust-security-model.md) before changing this boundary.
