---
title: "Threat-model Windows local IPC"
description: "Review trust boundaries, supervised bootstrap authentication, attacks, controls, and residual risks for local engine IPC."
audience: "architecture"
page_type: "explanation"
status: "active"
owner: "security and Rust engine maintainers"
last_verified: "2026-09-20"
review_triggers:
  - "local peer authentication, pipe discovery, identity, authorization, transport, or production packaging changes"
keywords:
  - "local IPC threat model"
  - "named pipe authentication"
  - "IPC bootstrap token"
---

# Threat-model Windows local IPC

The named pipe is an untrusted process boundary. The supervised bootstrap proves possession of the inherited parent-child channel. It does not authenticate a person. Rust owns user sign-in, session validity, and every command, query, and subscription authorization decision.

## Assets and actors

Protected assets are domain data, scope boundaries, session identity, command integrity, audit causation, availability, and ephemeral bootstrap tokens. Actors include the intended shell, Rust engine, other same-user processes, elevated processes, malware, stale engine generations, and accidental incompatible clients.

## Threats and implemented controls

| Threat | Current control | Residual risk |
| --- | --- | --- |
| Connect to a guessed pipe | Unique endpoint plus 256-bit token sent only through inherited standard input | Same-account malware or a privileged debugger can inspect parent memory or handles |
| Assert another tenant or role | The handshake returns a device principal without the installation owner relation. Protocol 1.7 sign-in resolves a provisioned account in Rust and returns its distinct user principal. Protocol 1.8 account operations authorize the Manager relationship in Rust | Control-plane account import remains a trusted Rust operation |
| Replay a request on another connection | Exact engine-returned user context is bound to the signed-in connection and checked against the durable session before dispatch | The process-lifetime bootstrap token permits approved same-generation reconnect, but cannot grant user authority |
| Protocol downgrade or drift | Mandatory `PeerHello`, generated bindings, a protocol 1.7 gate on desktop sessions, and a protocol 1.8 gate on account administration | Old shell versions need an update before account administration |
| Reuse an expired or revoked session | Rust checks issue, expiry, closure, device, account activity, tenant, and scope before each request and event delivery; sign-out closes the durable session | A command already running at an independent revocation boundary can finish unless its product transaction rechecks session validity |
| Deliver state after access revocation | Policy-change signal plus authorization immediately before every event delivery | Storage unavailability closes the stream fail-safe and may reduce availability |
| Memory exhaustion | 8 MiB frame cap, 1,024-entry/16 MiB replay cap, and 256-event delivery queues | Repeated allowed-size traffic still consumes bounded work |
| Request starvation | Per-request deadlines, concurrent dispatch, bounded shutdown | Domain handlers must implement their own resource bounds |
| Cross-scope replay or cursor probing | Exact session/scope authorization, embedded-scope validation, and indistinguishable invalid-cursor errors | A compromised authenticated same-scope peer can still consume authorized event volume |
| Slow-consumer event loss | Replaceable state coalesces; discrete gaps close explicitly and require replay/resync | Repeated lag can reduce shell availability |
| Confuse late responses | Request correlation and pending-map removal | A timed-out command may still complete; outcome is explicitly unknown |
| Leak secrets through diagnostics | No token or payload logging; structured allowlisted errors | Debuggers and privileged local processes remain outside this control |
| Orphan or hang the engine | stdin abandonment signal, typed shutdown, 15-second Job Object fallback | Forced exit is crash recovery and cannot guarantee unfinished work |

## Residual release requirements

The backend accepts locally provisioned Argon2 password verifiers and issues eight-hour durable user sessions. Offline sign-in requires the password; it never resumes a session silently. It stores no password or bearer token in shell state or logs. A production path that imports verified server accounts, account disabling and password rotation, hostile same-user process tests, and an explicit OS-account support policy are still required before shared-machine release. Keep Windows pipe ACL hardening as defense in depth. Do not add identity, tenant, workspace, scope, role, or permission assertions back to the shell contract.

No Arabic customer text is interpreted during authentication. Canonical UTF-8 payloads remain opaque to the transport, presentation bidi controls are not added, and structured subscription failures expose no policy graph, cursor owner, or customer data.

Review [ADR-0017](../decisions/0017-windows-named-pipe-local-ipc.md), the [local IPC subsystem](../developer/subsystems/local-ipc.md), and [zero-trust ADR-0009](../decisions/0009-zero-trust-security-model.md) before changing this boundary.
