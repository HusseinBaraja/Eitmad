---
title: "Evolve contracts without breaking supported peers"
description: "Classify protocol changes, preserve the compatibility window, negotiate capabilities, and ship breaking changes safely."
audience: "api"
page_type: "task"
status: "active"
owner: "Rust contract maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "the protocol version, supported release window, capability negotiation, generator, or rollout process changes"
keywords:
  - "compatibility window"
  - "backward-compatible contract change"
  - "breaking contract change"
  - "capability negotiation"
---

# Evolve contracts without breaking supported peers

Change the Rust contract authority first, then regenerate every derived artifact. A change may ship on protocol v1 only when every peer inside the declared compatibility window can still decode the messages it receives and preserve their meaning.

## Compatibility window

The released engine and native shells MUST support the current protocol minor and every minor explicitly retained by current release notes. The active foundation window is `1.0–1.2`: `1.0` remains command/query-only, subscription traffic requires negotiated protocol `1.1` and `eitmad.capability.local-ipc-subscriptions.v1`, relationship administration requires `1.2`, and authorization-policy events require both `1.2` and `eitmad.capability.authorization-policy-events.v1`.

The engine and Windows supervisor advertise `1.0–1.2`. A peer negotiated at `1.0` receives `eitmad.error.ipc-subscription-unsupported.v1` if it attempts subscription traffic, even if it incorrectly advertises the capability. A `1.1` peer cannot request authorization-policy events and never receives the `1.2`-only revocation reason. Protocol `1.2` alone is insufficient for an authorization-policy subscription: Rust returns `eitmad.error.ipc-subscription-unsupported.v1` when the policy-event capability is absent from the negotiated intersection. A minor may leave the window only after supported update paths no longer require it, rollout evidence is reviewed, and the release note records the removal. Product versions and protocol versions are independent.

Compatibility across protocol majors is not implied. A major transition requires the breaking-change process below and an explicit bridge or coordinated upgrade plan.

## Negotiate before normal traffic

After authenticating the local channel and before accepting a command, query, or subscription, each peer sends `PeerHello` with its peer kind, product version, supported protocol ranges, offered capabilities, required capabilities, and schema ranges. Rust then:

1. Selects the highest minor in the highest overlapping major.
2. Intersects offered capabilities.
3. Rejects when either peer requires a capability the other does not offer.
4. Selects the highest overlapping version for each shared schema.
5. Rejects a required schema that is missing or has no overlapping version.

An optional capability absent from the negotiated intersection disables that behavior; it does not authorize a best-effort fallback. A rejection occurs before normal traffic and must surface as a structured compatibility recovery state. The authoritative algorithm and rejection types are in `crates/contracts/src/versioning.rs`.

## Classify the change

| Change | Protocol treatment | Required evidence |
| --- | --- | --- |
| Add an optional field whose absence preserves existing meaning | Backward-compatible minor change | Old fixture decoded by new code; old peer receives no required new behavior |
| Add optional behavior | New capability and minor version | Tests with capability present and absent; sender gates messages on the negotiated capability |
| Add a tagged operation or enum variant | Capability-gated while older closed unions remain supported | Old peers never receive the new variant; C# and Swift conformance pass |
| Add an optional schema | Schema registration and capability gate when behavior depends on it | Required/optional overlap tests |
| Rename or remove a field, identifier, variant, or error parameter | Breaking major change | Major-version plan and parallel bindings |
| Make an optional field required or tighten accepted values/bounds | Breaking major change unless the restriction was already enforced and documented | Compatibility review plus boundary tests |
| Change field meaning, units, authorization meaning, default, or retry semantics | Breaking major change | Migration, security, and operational review |

Never reuse a stable identifier with new meaning. Deprecation keeps the old identifier and behavior available for the full compatibility window.

## Make a backward-compatible change

1. Update the owning Rust type and protocol catalog.
2. Add Rust tests for old input, new input, unknown fields, required capability absence, schema overlap, and relevant Arabic or mixed-direction data.
3. Add or retain cross-version fixtures for every minor in the active window.
4. Run `npm run contracts:generate --prefix crates/contracts/codegen` and review all schema, registry, fixture, documentation, C#, and Swift diffs.
5. Run `npm run contracts:verify --prefix crates/contracts/codegen`, Rust checks, and available platform conformance runners.
6. Update the release compatibility note before merging a change that expands or contracts the window.

Generated output is review evidence, not approval of compatibility.

## Ship a breaking change

1. Write an accepted ADR defining the incompatibility, new major, supported bridge, rollback limits, and retirement criteria.
2. Introduce new versioned identifiers and parallel generated artifacts; do not overwrite or reinterpret v1 identifiers.
3. Define engine, shell, server, sync-schema, storage-migration, and update-order compatibility explicitly.
4. Add old/new negotiation rejection tests and bridge tests for every supported upgrade path.
5. Roll out readers before writers where dual-read or dual-write is safe; otherwise require a coordinated stop with a recoverable user state.
6. Keep the prior major until its documented support window expires. Removal requires release evidence and a follow-up change, not the initial breaking change.

If rollback would cross an incompatible storage or sync migration, block rollback and follow the documented backup/recovery path instead of pretending protocol compatibility restores data compatibility.

## Understand CI failures

`rust-and-drift` exports fresh artifacts from Rust into a temporary directory. It fails without modifying the checkout when a tracked schema, registry, fixture, generated reference, C# binding, or Swift binding is missing or differs. It also fails when non-generated shell source contains an `eitmad.*` protocol literal or when binding identifier parity tests fail.

The `windows-binding` and `macos-binding` jobs depend on `rust-and-drift`; stale contracts block platform conformance. Once unblocked, each job compiles its generated binding and round-trips the Rust fixture. CI never regenerates and accepts files automatically. Regenerate locally, review compatibility, commit every intended output, and rerun the checks.

For exact shapes, return to [protocol v1](index.md). For a stale-output symptom, follow [resolve generated contract drift](../troubleshooting/contract-binding-drift.md).
