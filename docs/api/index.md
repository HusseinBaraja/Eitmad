---
title: "Use Eitmad protocol v1 contracts"
description: "Consume Rust-owned JSON commands, queries, subscriptions, errors, and generated native bindings without protocol drift."
audience: "api"
page_type: "reference"
status: "active"
owner: "Rust contract maintainers"
last_verified: "2026-07-12"
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

Generated files have a `Do not edit` header. Linux bindings remain blocked on the Linux UI technology decision; a future Linux shell must consume the Rust crate directly or add a generated/validated binding through a new decision.

## Interaction contracts

| Interaction | Required context | Foundation operations |
| --- | --- | --- |
| Command | Version, request/correlation/causation IDs, authenticated session, scope, deadline, idempotency key | Update configuration, cancel operation, report installer outcome |
| Query | Version, request/correlation/causation IDs, authenticated session, scope, deadline | Read configuration, effective permissions, update state, sync status |
| Subscription | Version, request/correlation IDs, authenticated session, scope, optional resume cursor | Configuration, permission, update, and sync state changes |
| Event | Subscription/correlation IDs, sequence, cursor, occurrence time | Typed configuration, permission, update, and sync values |

The identity and scope fields are assertions to verify against the authenticated channel, not credentials and not proof of authorization. Rust must authorize every operation and audit every state-changing command in the runtime that executes it.

## Wire and compatibility rules

- Protocol v1 uses UTF-8 JSON with camel-case fields and explicit `kind`/`payload` tags.
- UUIDs are lowercase hyphenated strings. Times are Unix milliseconds. Canonical values remain locale-independent.
- Unknown object fields are accepted for additive minor-version evolution.
- Unknown required operation variants are rejected; they are never guessed or treated as a known command.
- Open identifiers preserve unknown valid values so optional future capabilities and errors can be handled safely.
- Configuration is revisioned and typed. It does not carry arbitrary JSON or secret values; secret references and redacted reads are explicit variants.
- Sync domain payloads are registered schema/version identifiers plus encoded bytes. A domain vertical must define the payload schema before use.

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

Protocol `1.0` is the current version. Additive optional fields and capabilities use a minor version or capability gate. Renamed fields, changed meaning, removed variants, or incompatible identifier behavior require a new major version.

## Structured failures

`ContractError` carries a stable error code, localization message ID, typed parameters, retry disposition, correlation ID, and an optional safe detail. Shells localize the message ID and render parameters; they never parse prose. Current codes and message IDs are listed in the [generated reference](../_generated/contracts-v1.md).

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
npm run contracts:check --prefix crates/contracts/codegen
```

The check also rejects `eitmad.*` protocol literals in non-generated shell source. Windows CI compiles and round-trips the .NET binding; macOS CI compiles and round-trips the Swift binding.

## Extend protocol v1 safely

1. Add the Rust type beside its owning contract concern and register its identifier in the Rust catalog.
2. Add success, denial, failure, unknown-field, compatibility, and Arabic/mixed-direction tests that apply.
3. Regenerate all outputs; never edit a generated failure directly.
4. Review whether the change is additive, capability-gated, or major-version breaking.
5. Update this reference, the owning subsystem page, and troubleshooting knowledge.

For ownership and failure flows, see [Maintain the authoritative contract layer](../developer/subsystems/contract-layer.md). For drift failures, see [Resolve generated contract drift](../troubleshooting/contract-binding-drift.md).
