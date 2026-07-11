# ADR-0005: Generate or mechanically validate platform contracts

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: Contract tooling cannot represent a required boundary or supported platform language
> - Supersedes: None
> - Superseded by: None

## Context and problem

Separate native shells need compatible commands, queries, subscriptions, errors, versions, and capabilities. Handwritten copies of Rust DTOs and enums can drift silently and turn compatibility failures into runtime defects.

## Decision drivers

- One canonical contract definition
- Compile-time or CI-visible drift detection
- Explicit compatibility and capability behavior

## Considered options

- **Generated bindings:** strongest consistency when target tooling is adequate.
- **Mechanically validated bindings:** permits native representations while proving conformance.
- **Handwritten contracts:** flexible, but drift-prone and prohibited.

## Decision

Rust MUST own canonical external contracts. Platform bindings MUST be generated from those definitions or mechanically validated against them in CI. Every boundary is versioned and negotiates capabilities. Unknown optional fields are tolerated where safe; missing required behavior fails explicitly. Machine-readable errors and stable message identifiers cross the boundary instead of shell-parsed prose.

## Architecture and contract impact

Contracts cover commands, queries, subscriptions, events, errors, identity, permissions, config, sync, updates, versions, and capabilities. Compatibility ranges, cancellation, deadlines, paging or streaming, backpressure, correlation, and reconnection are explicit.

## Storage and sync impact

Persisted and synchronized formats require independent version and migration rules; generated IPC bindings do not imply storage compatibility.

## Security impact

Generation does not make input trusted. Rust validates every request after decoding, and generated code must avoid unsafe logging and permissive fallback behavior.

## Arabic UX impact

Contracts carry Unicode values, stable localization identifiers, and structured parameters. They do not concatenate translated fragments or encode visual direction into canonical data.

## Consequences and tradeoffs

### Positive

- Contract drift becomes detectable before release.
- Each platform retains idiomatic bindings without redefining authority.

### Negative

- Tooling, compatibility fixtures, and generated-code review add maintenance cost.

### Risks

- A generator bug can affect all clients; mitigate with golden fixtures, schema review, and cross-language conformance tests.

## Verification

Foundation review confirms canonical ownership and required negotiation. Production verification must include generated-diff cleanliness and bidirectional compatibility fixtures.

## Follow-up work

- Select the contract schema and binding toolchain with the first vertical slice.
- Define compatibility and capability negotiation matrices.

## Related decisions and documents

- [Target architecture: IPC model](../architecture/target-architecture.md#ipc-model)
- [ADR-0001](0001-rust-core-product-authority.md)
- [ADR-0008](0008-required-unified-synchronization.md)
