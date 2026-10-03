---
title: "Extend privacy-preserving observability safely"
description: "Trace metadata-only logs, correlation, and IPC error redaction without exposing product data or secrets."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust reliability and security maintainers"
last_verified: "2026-10-03"
review_triggers:
  - "diagnostic fields, sinks, error contracts, or redaction changes"
keywords:
  - "eitmad-observability-audit"
  - "metadata-only logging"
  - "correlation ID"
---

# Extend privacy-preserving observability safely

`eitmad-observability-audit` applies Rust-owned field contracts before operational logs are serialized. Only declared metadata is included. Sensitive and secret fields are always redacted; there is no temporary mode that reveals them.

## Authority and boundary

`crates/contracts/src/observability.rs` owns event, component, field, severity, classification, and value-kind types. `crates/observability-audit/src/diagnostics.rs` checks the allowlist, field types, duplicate declarations, and duplicate emitted values. The engine CLI uses this path for operational failures.

`ObservationContract::redact` creates a `StructuredLog` with private fields, a stable event ID, occurrence time, component, severity, correlation ID, and checked values. Callers cannot bypass classification by filling a public output map.

| Classification | Serialized output |
| --- | --- |
| `Metadata` | Included after field and type validation |
| `Sensitive` | `redacted` |
| `Secret` | `redacted` |

Local IPC uses `ContractError::redacted_for_external_boundary` immediately before serialization. That projection retains stable codes, message IDs, retry policy, correlation, safe numeric details, and allowlisted metadata. It removes free text, mismatched parameter kinds, and compatibility reasons even if an internal dispatcher constructed an unsafe error.

## Security and correlation

Product payloads, customer text, credentials, authorization graphs, and raw causes must not enter logs or audit. A correlation ID links operational failures; it does not authorize cross-scope joins or product access.

Mutation audit remains separate from diagnostic output. State changes and their redacted audit records commit together in the owning storage boundary. The historical `SensitiveDebugMode` audit tag remains readable, but no diagnostic controller or permission enables that mode.

There is no custom structured-error metadata builder, crash-report pipeline, persistent diagnostic sink, upload destination, retention job, or support-bundle exporter. Add one only for a current workflow with explicit access, quotas, privacy, scope, and deletion rules.

## Failure and recovery

| Failure | Behavior | Repair |
| --- | --- | --- |
| Unknown, duplicate, or wrong-kind field | Event construction fails | Fix the owning field contract |
| Raw internal error text | IPC strips it | Replace the raw cause with stable typed metadata |
| Serialization failure | CLI emits only a stable fallback event and correlation ID | Fix the schema without printing the value |
| Suspected sensitive output | Treat as a privacy incident | Follow [diagnostic leakage recovery](../../troubleshooting/privacy-and-secret-leakage.md) |

Rust identifiers are language-neutral. Future Arabic diagnostic UI must isolate correlation and error identifiers in LTR runs inside RTL layout. No customer text may be collected for readability.

## Verification

Focused tests cover sensitive and secret sentinels, field allowlists, duplicate and wrong-kind values, mandatory audit metadata, external error projection, IPC serialization, and structured CLI failures. Run the affected crate tests; use the [focused check table](../index.md#choose-the-smallest-normal-proof) for cross-boundary changes.

Related authority: [ADR-0012](../../decisions/0012-privacy-preserving-observability.md), [typed local IPC](local-ipc.md), and [secret storage](secret-storage.md).
