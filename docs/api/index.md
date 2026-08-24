---
title: "Use Eitmad protocol v1 contracts"
description: "Consume Rust-owned JSON commands, queries, subscriptions, errors, and generated native bindings without protocol drift."
audience: "api"
page_type: "reference"
status: "active"
owner: "Rust contract maintainers"
last_verified: "2026-08-22"
review_triggers:
  - "a command, query, subscription, error, version, capability, or generator changes"
keywords:
  - "IPC contract"
  - "protocol v1"
  - "capability negotiation"
  - "contract drift"
---

# Use Eitmad protocol v1 contracts

`eitmad-contracts` is the sole authority for protocol v1. It defines UTF-8 JSON shapes, identifiers, compatibility rules, and generated C# and Swift bindings. Shell code must consume generated types and `ProtocolIds`; it must not declare DTO copies or protocol strings.

## Authority and generated outputs

| Need | Canonical source |
| --- | --- |
| Rust types and invariants | `crates/contracts/src/` |
| JSON wire schema | `crates/contracts/generated/contract-v1.schema.json` |
| Registered identifiers | `crates/contracts/generated/protocol-v1.json` |
| C# binding | `shells/windows/generated/` |
| Swift binding | `shells/macos/generated/` |
| Cross-language fixture | `tests/contract-compatibility/fixtures/protocol-v1.json` |
| Mechanical identifier listing | [Protocol v1 identifier reference](../_generated/contracts-v1.md) |

The sync contract graph has a focused [synchronization and transport reference](synchronization-contracts.md) covering change records, one simulation/LAN/WAN streaming frame, cancellation, snapshots, pending command queues, conflicts, delivery idempotency, cache labels, and lifecycle events.

The remote server contract adds tenant/account/device authentication shapes, session policy, license state, effective update assignment, server client/server messages, chunked snapshots, and resumable subscription events. Its ownership and invariants are in [the modular server authority](../developer/subsystems/server-authority.md).

Generated files have a `Do not edit` header. Linux bindings remain blocked on the Linux UI technology decision; a future Linux shell must consume the Rust crate directly or add a generated/validated binding through a new decision.

## Interaction contracts

| Interaction | Required context | Foundation operations |
| --- | --- | --- |
| Command | Version, request/correlation/causation IDs, authenticated session, tenant, optional workspace, scope, deadline, idempotency key | Update configuration; grant/revoke scoped relationships; placeholder operation/update commands |
| Query | Version, request/correlation/causation IDs, authenticated session, tenant, optional workspace, scope, deadline | Read configuration/effective permissions/relationships; placeholder update and sync state |
| Subscription | Version, request/correlation IDs, authenticated session, tenant, optional workspace, scope, optional resume cursor | Configuration, permission, authorization-policy, sync, record, job, notification, update, and error streams |
| Event | Subscription/correlation IDs, sequence, cursor, occurrence time | Typed state, metadata, progress, notification, and error values |

The identity, tenant, workspace, and scope fields are assertions to verify against the authenticated channel, not credentials and not proof of authorization. Rust must authorize and audit every boundary operation; state-changing verticals keep state and audit atomic.

## Engine supervision contracts

`EngineProcessIdentity`, `LifecycleSnapshot`, health-check results, and `DiagnosticReport` are Rust-owned protocol v1 shapes under capability `eitmad.capability.engine-lifecycle.v1`. Lifecycle states are `starting`, `ready`, `stopping`, `stopped`, and `failed`. Readiness is explicit and must not be inferred from a live PID or successful process launch.

The foreground CLI emits lifecycle snapshots as newline-delimited JSON on child stdout. That launch-status stream remains separate from typed product traffic and must not carry requests, credentials, or product data. Windows command, query, and subscription traffic uses the negotiated named-pipe protocol documented in [typed local IPC](../developer/subsystems/local-ipc.md). Process PID and instance metadata support correlation only.

## Wire and compatibility rules

- Protocol v1 uses UTF-8 JSON with camel-case fields and explicit `kind`/`payload` tags. The current minor is `1.4`.
- Local IPC frames add a four-byte little-endian length and enforce an 8 MiB maximum.
- UUIDs are lowercase hyphenated strings. Times are Unix milliseconds. Canonical values remain locale-independent.
- Unknown object fields are accepted for additive minor-version evolution.
- Unknown required operation variants are rejected; they are never guessed or treated as a known command.
- Open identifiers preserve unknown valid values so optional future capabilities and errors can be handled safely.
- Configuration is revisioned and typed. It does not carry arbitrary JSON or secret values; secret references and redacted reads are explicit variants.
- `SecretId` and secret lifecycle are Rust-internal authority types, not protocol-v1 operations. Shells never receive secret material or call an OS credential store for product secrets.
- Sync domain payloads are registered schema/version identifiers plus encoded bytes. A domain vertical must define the payload schema before use.
- Sync deliveries carry independent delivery IDs and idempotency keys. Consumers preserve record authority and cache freshness labels instead of presenting optimistic or stale data as canonical.
- Simulation, LAN, direct WAN, and relay WAN carry the same `SyncTransportFrame` and complete `SyncTransportPayload`; message payloads use the shared `SyncMessage`. Route adapters cannot define another wire protocol or change reconciliation meaning.
- Observation event, field, component, severity, classification, and value-kind contracts are exported in the JSON schema and exercised by the C# and Swift conformance fixture. Diagnostic values still reach sinks only through the Rust-owned redaction boundary.

This is a minimal valid query body; the complete scoped envelope is in `tests/contract-compatibility/fixtures/protocol-v1.json`:

| JSON field | Value |
| --- | --- |
| `kind` | `eitmad.config.get.v1` |
| `payload` | Empty object |

The same body with `kind` set to unregistered `eitmad.config.remove.v1` is rejected rather than treated as an optional extension.

## Version and capability negotiation

Each peer sends supported protocol major/minor ranges, available and required capabilities, and supported schema ranges. Rust selects the highest overlapping minor in the highest common major. Negotiation rejects:

- no common protocol major/minor;
- a capability required by either peer but absent from the other;
- a required schema with no overlapping version.

The encoded window is `1.0–1.4`. Protocol `1.0` supports command/query traffic, `1.1` adds local IPC subscriptions, `1.2` adds relationship administration and authorization-policy events, and `1.3` adds decodable tenant/workspace fields. Protocol `1.4` adds the remote server boundary, registered-device proof, snapshot chunks, and resumable server subscriptions. Local IPC requires `eitmad.capability.authorization-scopes.v1` and an assigned tenant for every accepted version. A remote server connection requires protocol `1.4` plus sync, server-connection, device-proof, snapshot-chunk, and subscription-resume capabilities. See the [protocol 1.4 release](../releases/protocol-1-4-server-authority.md) for coordinated binding and database rollout.

Event cursors are opaque, scoped, and valid only in the current engine generation's bounded replay window. Per-subscription sequence numbers order delivered events but do not establish global order. When a close envelope can be delivered, `clientRequested` follows explicit unsubscribe, `engineStopping` precedes shutdown, and `backpressure` identifies an unreplayable discrete gap. For authorization revocation, protocol `1.2` sends `SubscriptionClosed` with `authorizationRevoked`; revoked `1.0` and `1.1` connections terminate without a close envelope. See [typed local IPC](../developer/subsystems/local-ipc.md) for replay, duplicate delivery, backpressure, reauthorization, and resync rules.

## Implemented configuration and authorization authority

The real engine dispatcher currently executes configuration query/update, effective-permission query, relationship grant/revoke/list, and configuration/authorization subscriptions. Existing unrelated placeholder operations return `eitmad.error.contract-invalid.v1` until their Rust verticals exist. Configuration import/export remain Rust service APIs and have no shell IPC operation.

Configuration snapshots are revisioned, redacted, stable-key projections. Relationship mutations use a separate optimistic policy revision. Rust also defines scoped object, tuple, condition, permission-rule, request, and decision types for engine-owned boundaries, but they are not part of the current generated IPC root and no generic tuple-management operation exists. Read [configuration authority](../developer/subsystems/configuration.md) and [authorization/audit authority](../developer/subsystems/authorization.md) before consuming these contracts.

The active compatibility window, capability rules, change classification, and major-version rollout process are defined in [Evolve contracts without breaking supported peers](evolve-contracts-compatibly.md).

## Implemented server boundary

The combined server exposes bounded JSON activation, login, refresh, and effective update-assignment routes plus one authenticated WebSocket. The socket accepts hello, sync frames, subscription requests, and cursor acknowledgements. It returns a negotiated hello, sync messages, durable events, or structured failures. A client authenticates its device and negotiates before normal traffic. The host automatically sends a snapshot manifest, chunks, and completion when retained operation history cannot satisfy the requested checkpoint.

The executable has no registered production domain. Domain-specific operations remain unavailable until a vertical supplies one `DomainHandler`, schema, mode, authorization policy, validation or command execution, and conflict policy.

## Structured failures

`ContractError` carries a stable error code, localization message ID, typed parameters, retry disposition, correlation ID, and an optional safe detail. Before external serialization, Rust retains only allowlisted identifier/integer metadata and removes free text, mismatched parameter kinds, and compatibility reasons. The local IPC writer repeats this projection for every nested error. Shells localize the message ID and render parameters; they never parse prose. Current codes and message IDs are listed in the [generated reference](../_generated/contracts-v1.md).

## Generate or validate bindings

Install the pinned generator once after checkout:

```powershell
npm ci --ignore-scripts --prefix crates/contracts/codegen
```

Regenerate schemas, fixtures, reference, C#, Swift, and identifier constants after changing Rust contracts:

```powershell
npm run contracts:generate --prefix crates/contracts/codegen
```

Check for drift without changing tracked files:

```powershell
npm run contracts:verify --prefix crates/contracts/codegen
```

Verification compares every generated file with a fresh Rust export, checks that C# and Swift contain every Rust-exported identifier, and rejects `eitmad.*` protocol literals in non-generated shell source. Windows CI compiles and round-trips the .NET binding; macOS CI compiles and round-trips the Swift binding.

## Extend protocol v1 safely

1. Add the Rust type beside its owning contract concern and register its identifier in the Rust catalog.
2. Add success, denial, failure, unknown-field, compatibility, and Arabic/mixed-direction tests that apply.
3. Regenerate all outputs; never edit a generated failure directly.
4. Follow the [contract evolution process](evolve-contracts-compatibly.md) to classify the change and preserve the supported window.
5. Update this reference, the owning subsystem page, and troubleshooting knowledge.

For ownership and failure flows, see [Maintain the authoritative contract layer](../developer/subsystems/contract-layer.md). For drift failures, see [Resolve generated contract drift](../troubleshooting/contract-binding-drift.md).
