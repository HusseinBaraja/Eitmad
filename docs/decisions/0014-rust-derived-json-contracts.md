---
title: "ADR-0014: Derive JSON contracts and native bindings from Rust"
description: "Records the JSON Schema, C#, Swift, fixture, and drift-validation toolchain for Rust-owned protocol v1."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture and Rust contract maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "JSON or Quicktype cannot represent a required compatible contract or platform language"
---

# ADR-0014: Derive JSON contracts and native bindings from Rust

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-12
> - Decision owners: Architecture and Rust contract maintainers
> - Last verified: 2026-07-12
> - Review trigger: JSON or Quicktype cannot represent a required compatible contract or platform language
> - Supersedes: None
> - Superseded by: None

## Context and problem

ADR-0005 requires generated or mechanically validated platform contracts but intentionally left the schema and toolchain open. Protocol v1 needs readable diagnostics, additive compatibility, Arabic Unicode preservation, C# and Swift support, and deterministic drift detection while keeping Rust as authority.

## Decision drivers

- Rust remains the only editable contract authority.
- Unknown additive fields survive compatible evolution.
- Windows and macOS receive idiomatic native models.
- Fixtures expose cross-language encoding and generator defects before release.

## Considered options

- **Rust-derived JSON Schema and generated bindings:** readable, evolvable, and compatible with current native targets.
- **Binary Serde reflection:** compact and multi-language, but stricter field-evolution behavior.
- **Protocol Buffers:** mature generators, but a `.proto` file would become the practical authority.
- **Handwritten native DTOs:** prohibited because drift is silent.

## Decision

Protocol v1 uses UTF-8 JSON serialized from Rust Serde types. Schemars emits deterministic draft-07 JSON Schema. Pinned Quicktype generates C# `System.Text.Json` and Swift `Codable` models. A Rust catalog generates every stable protocol identifier into both platforms. Deterministic fixtures and CI compile and round-trip each binding. Non-generated shells may not contain `eitmad.*` protocol literals.

Linux output is deferred until the Linux UI technology decision. The future choice must consume Rust directly or use generated/mechanically validated bindings from the same schema and fixtures.

## Architecture and contract impact

Minor evolution may add optional fields and capability-gated behavior. Unknown required operation tags fail explicitly. Breaking meaning, removal, or representation changes require a new protocol major. Generated schemas describe input but do not replace Rust runtime validation, authentication, authorization, or domain rules.

## Storage and sync impact

IPC schema compatibility does not imply storage or synchronized payload compatibility. Sync domain payloads carry separately registered schema IDs and versions.

## Security impact

Generated decoders process untrusted input. Rust verifies identity, session, scope, bounds, permissions, and domain invariants after decoding. Fixtures and errors contain synthetic, non-secret data only. Shells cannot use generated DTOs to bypass engine authority.

## Arabic UX impact

UTF-8 JSON preserves Arabic and mixed-direction values without presentation bidi controls. Stable message IDs and typed parameters support localized shell rendering. C# and Swift fixtures verify representative Arabic filenames and mixed text.

## Consequences and tradeoffs

### Positive

- Schemas, identifiers, fixtures, native types, and reference documentation change together.
- JSON is inspectable in diagnostics and tolerant of safe additive fields.
- CI detects generator regressions and shell-owned protocol strings.

### Negative

- JSON is larger than a compact binary encoding.
- Checked-in generated files and Node-based generation add review and toolchain cost.
- Quicktype output quality constrains representable schema patterns and requires conformance tests.

## Verification

Rust tests cover identifiers, tagged operations, bounds, negotiation, and Unicode. Deterministic generation checks all outputs. Windows compiles and round-trips .NET models; macOS compiles and round-trips Swift models.

## Related decisions and documents

- [ADR-0005](0005-generated-or-validated-contracts.md)
- [Protocol v1 reference](../api/index.md)
- [Contract subsystem](../developer/subsystems/contract-layer.md)
