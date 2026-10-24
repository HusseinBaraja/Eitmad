---
title: "Threat-model Windows local IPC"
description: "Review trust boundaries, supervised bootstrap authentication, attacks, controls, and residual risks for local engine IPC."
audience: "architecture"
page_type: "explanation"
status: "active"
owner: "security and Rust engine maintainers"
last_verified: "2026-10-24"
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
| Replace an uncertain pairing through recovery | The onboarding capability requires the process handshake. Rust permits explicit pending-exchange abandonment only before account sign-in and before a committed local connection exists. Cleanup retains the device key, data, and trust; missing pending identity/key material fails closed. Retry reuses the signed completion | Explicit abandonment can lose receipt recovery for an already committed server approval. It does not revoke approval or permit a replacement identity; see [pairing recovery](../developer/subsystems/server-authority.md#pair-a-discovered-server-and-approve-its-device) |
| Assert another tenant or role | The handshake returns a device principal without the installation owner relation. Rust sign-in requires live server confirmation and returns the account's distinct user principal. Connected account administration also requires server Owner authority | Control-plane account import remains a trusted Rust operation |
| Replay a request on another connection | Exact engine-returned user context is bound to the signed-in connection and checked against the durable session before dispatch | The process-lifetime bootstrap token permits approved same-generation reconnect, but cannot grant user authority |
| Protocol downgrade or drift | Mandatory `PeerHello`, generated bindings, the current Rust-declared desktop protocol, and operation-specific capabilities | Older beta shells must update with the engine; negotiation does not grant account permissions |
| Reuse an expired or revoked session | Rust checks issue, expiry, closure, device, account activity, tenant, and scope before each request and event delivery; sign-out closes the durable session | A command already running at an independent revocation boundary can finish unless its product transaction rechecks session validity |
| Deliver state after access revocation | Policy-change signal plus authorization immediately before every event delivery | Storage unavailability closes the stream fail-safe and may reduce availability |
| Memory exhaustion | 8 MiB frame cap, 1,024-entry/16 MiB replay cap, and 256-event delivery queues | Repeated allowed-size traffic still consumes bounded work |
| Request starvation | Per-request deadlines, concurrent dispatch, bounded shutdown | Domain handlers must implement their own resource bounds |
| Cross-scope replay or cursor probing | Exact session/scope authorization, embedded-scope validation, and indistinguishable invalid-cursor errors | A compromised authenticated same-scope peer can still consume authorized event volume |
| Slow-consumer event loss | Replaceable state coalesces; discrete gaps close explicitly and require replay/resync | Repeated lag can reduce shell availability |
| Confuse late responses | Request correlation and pending-map removal | A timed-out command may still complete; outcome is explicitly unknown |
| Leak secrets through diagnostics | No token or payload logging; structured allowlisted errors | Debuggers and privileged local processes remain outside this control |
| Orphan or hang the engine | stdin abandonment signal, typed shutdown, 15-second Job Object fallback | Forced exit is crash recovery and cannot guarantee unfinished work |

Discount approval commands use protocol 1.20 and generated contracts. Rust authorizes every request, list, decision, and subscription. A Receptionist cannot approve by calling IPC directly or changing the shell role flag. Established server subscriptions recheck the authenticated session and current scope permissions without replaying the connection proof nonce. New connections require a fresh proof. Server decisions bind the immutable request ID, quotation revision, state revision, and commercial fingerprint; server audit, history, replay receipt, and scoped events commit atomically. Self-decision and stale terms fail closed. See the [quotation approval boundary](../developer/subsystems/quotations.md#server-confirmed-discount-approval).

## Residual release requirements

The backend imports server-verified accounts with local Argon2 password verifiers and issues eight-hour durable user sessions. Under [ADR-0029](../decisions/0029-require-live-server-authority-for-protected-access.md), protected access also requires fresh authenticated server confirmation. Missing configuration, trust failures, and disconnection fail closed; a verifier cannot authorize offline access. The IPC boundary checks protected requests and deliveries, and idle subscriptions revalidate within the [documented detection bounds](../developer/subsystems/identity-foundation.md#disconnected-authorization-limit). Failure closes the local session without deleting cached records or changing pending-work authorship. It stores no password or bearer token in shell state or logs. Shared account changes and deactivation use server authority; password rotation, hostile same-user process tests, and an explicit OS-account support policy remain required before shared-machine release. Keep Windows pipe ACL hardening as defense in depth. Do not add identity, tenant, workspace, scope, role, or permission assertions back to the shell contract.

No Arabic customer text is interpreted during authentication. Canonical UTF-8 payloads remain opaque to the transport, presentation bidi controls are not added, and structured subscription failures expose no policy graph, cursor owner, or customer data.

Review [ADR-0017](../decisions/0017-windows-named-pipe-local-ipc.md), the [local IPC subsystem](../developer/subsystems/local-ipc.md), and [zero-trust ADR-0009](../decisions/0009-zero-trust-security-model.md) before changing this boundary.
