---
title: "Maintain the authoritative contract layer"
description: "Understand contract ownership, trust boundaries, generation, compatibility, tests, and safe extension points."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust contract maintainers"
last_verified: "2026-08-23"
review_triggers:
  - "protocol ownership, wire format, generation, compatibility, or platform binding changes"
keywords:
  - "eitmad-contracts"
  - "Rust contract authority"
  - "JSON Schema"
  - "Quicktype"
---

# Maintain the authoritative contract layer

The contracts capability gives every external peer one Rust-owned protocol without moving business execution into the contract crate. It owns shapes and compatibility helpers; engine runtime, authorization, persistence, synchronization behavior, and update policy remain in their dedicated crates.

## Boundary and data flow

```mermaid
flowchart LR
    Rust["eitmad-contracts\nRust authority"] --> Export["Draft-07 schema\nregistry · fixture"]
    Export --> CSharp["Generated C#\nSystem.Text.Json"]
    Export --> Swift["Generated Swift\nCodable"]
    CSharp --> Windows["Thin Windows shell"]
    Swift --> MacOS["Thin macOS shell"]
    Rust --> Engine["Engine runtime\ndecode · verify · authorize · execute"]
```

The generated model describes untrusted input. The engine must authenticate the channel, compare the payload session and scope with channel state, validate bounds and domain invariants, authorize with ReBAC, execute atomically, and emit the required audit outcome.

## Tagged unions in generated bindings

Rust tagged contracts (`tagged_contract!`) serialize as JSON objects that carry a `kind` discriminator string and a `payload` field. Quicktype collapses every `oneOf` of such envelopes into one class whose payload is typed as the first variant, which cannot represent the other kinds. The generator therefore:

1. Collects every top-level discriminated union from the exported Draft-07 schema (`crates/contracts/codegen/unions.mjs`).
2. Renders typed union bindings itself into `shells/windows/generated/EitmadContracts.Unions.g.cs` (a C# class with a `Kind` string, an untyped payload, one `For<Pascal>` factory, and one `As<Pascal>()` typed accessor per kind) and `shells/macos/generated/EitmadContractsUnions.generated.swift` (a Swift `enum` with associated values plus full `Codable` conformance).
3. Feeds quicktype a reduced schema in which each union definition is replaced by an empty object and every payload type stays reachable through a deterministic keep-alive container.

Codegen tests pin every union variant to its kind string and payload type in both languages, so adding a Rust variant without regenerating fails verification. Nested tagged enums that mix payload-less struct variants remain collapsed by quicktype; extending the renderer to them is deliberate follow-up work, not an accident.

## Invariants

- Rust definitions and the Rust protocol catalog are the only editable sources of wire shapes and identifiers.
- Commands, queries, subscriptions, and events remain distinct typed envelopes.
- Every operation has explicit identity, session, scope, correlation, version, and deadline context; retryable mutation also has an idempotency key.
- Unknown additive fields are tolerated. Unknown required operations fail closed.
- Open identifiers are validated during Rust deserialization and preserved when their meaning is optional and unknown.
- Error prose, localization, bidi controls, secrets, and raw permission graphs do not become protocol control data.
- Sync messages are bounded and transport-independent; domain-specific reconciliation is not implemented in this crate.

## Arabic and mixed-direction behavior

JSON is Unicode and preserves Arabic or mixed Arabic/Latin values unchanged. Canonical data contains no presentation-only bidi controls. Stable message IDs and typed parameters cross the boundary; shells render localized Arabic text and apply directionality. The deterministic fixture includes `خزانة Wardrobe 120 cm - فرع صنعاء` and `ملف عرض السعر Quote-١٢.pdf` to detect encoding loss in Rust, C#, and Swift.

### Arabic-first feature gate

| Result | Checklist area | Evidence or reason |
| --- | --- | --- |
| Pass | Unicode IPC, canonical values, stable message IDs, typed parameters, and exclusion of bidi controls | Rust serialization and compatibility tests |
| Pass | Shared Arabic, Latin, numerals, filename, and mixed-direction fixtures | Generated protocol fixture and .NET conformance runner |
| Not applicable | RTL layout, focus, keyboard input, typography, accessibility, search, reports, PDF, and printing | This capability has no UI, search behavior, or document renderer; shells remain blocked on their full pre-shell gate |
| Not applicable | Default locale, calendar, time zone, digits, currency, fonts, and fallback policy | Protocol v1 can transport canonical values but does not select product presentation policy; the registered locale key defines no default |
| Not applicable | Local Swift compiler execution | The current Windows environment has no Swift toolchain; mandatory macOS CI owns compilation and fixture execution before merge |

The missing local Swift toolchain does not authorize shell implementation or bypass CI. Its review trigger is any Swift generator, schema, or macOS toolchain change.

## Failure modes

| Failure | Safe behavior | Owning evidence |
| --- | --- | --- |
| Incompatible major or missing required capability | Reject negotiation before normal requests | `versioning::negotiate` tests |
| Invalid protocol identifier | Reject during Rust deserialization | transport tests |
| Unknown required operation | Reject the tagged union | transport compatibility test |
| Stale configuration revision | Return `eitmad.error.config-revision-conflict.v1`; do not overwrite | typed config/error contracts |
| Oversized page or sync batch | Reject against declared bounds | transport and sync tests/schema |
| A tagged Rust variant is missing from rendered union bindings | Union conformance tests fail verification | `crates/contracts/codegen/test/unions.test.mjs` |
| Generated binding differs or omits a Rust identifier | Fail deterministic drift and identifier-parity checks | codegen verification and CI |

## Tests and observability

Unit tests live beside contract behavior. Cross-language fixtures and runners live under `tests/contract-compatibility/`. Codegen tests prove missing/stale output detection and C#/Swift identifier parity. CI checks Rust formatting, strict Clippy, tests, generated drift, .NET compilation/round-trip, Swift compilation/round-trip, Unicode preservation, and forbidden handwritten shell identifiers. Platform jobs do not run until Rust drift validation succeeds.

Contracts expose correlation identifiers and safe structured errors. `ContractError::redacted_for_external_boundary` removes free text and undeclared metadata, and the IPC writer reapplies it to every nested error before serialization. Contracts do not authorize logging payloads. Diagnostics must use the [observation field contract](privacy-preserving-observability.md) and record only approved identifiers, versions, negotiation outcomes, bounds, and redacted failure metadata.

## Safe extension points

- Add a foundation concern inside this crate only when several external peers need the same stable boundary.
- Add product-domain commands and payloads in their owning vertical, then expose only the deliberate contract surface.
- Register every new operation, capability, permission, config key, schema ID, error code, message ID, and parameter name before generation.
- Use a capability for optional behavior. Increment the protocol major when an older peer cannot interpret the new meaning safely.
- Add Linux output only after its shell technology ADR; do not preselect a language through the generator.

Next, use the [protocol v1 reference](../../api/index.md) when adding a client, follow the [contract evolution process](../../api/evolve-contracts-compatibly.md) when changing a boundary, or [resolve generated contract drift](../../troubleshooting/contract-binding-drift.md) when outputs disagree.
