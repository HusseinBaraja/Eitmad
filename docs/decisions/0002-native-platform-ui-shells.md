# ADR-0002: Use native platform UI shells

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: Native shells fail platform quality, accessibility, distribution, or maintenance goals
> - Supersedes: None
> - Superseded by: None

## Context and problem

The desktop product must feel correct on each supported operating system while sharing authoritative behavior. A single cross-platform UI runtime could simplify UI reuse but may compromise native accessibility, lifecycle integration, installer behavior, or resource use.

## Decision drivers

- Native accessibility and interaction quality
- Reliable OS lifecycle and integration behavior
- Thin, replaceable presentation adapters

## Considered options

- **Native shells:** strongest platform integration, with separate UI implementations.
- **One cross-platform shell:** more visual code reuse, but adds runtime and platform compromises.
- **Browser UI:** broad portability, but weaker native integration and a larger trust surface.

## Decision

Windows MUST use a C# native shell. macOS and Linux MUST use native platform UI choices selected for platform quality and maintainability. Shells translate user intent into Rust-owned commands, queries, and subscriptions; they MUST NOT own business logic, authoritative schemas, config, database access, sync, permissions, secrets, or external product APIs.

## Architecture and contract impact

Each shell is an adapter behind versioned IPC and capability negotiation. Platform bindings are generated or mechanically validated from Rust contracts.

## Storage and sync impact

Shells show storage and sync state returned by Rust but never access the database or resolve domain conflicts independently.

## Security impact

Shells are untrusted clients. Native prompts and credential facilities adapt OS behavior but do not grant authority by themselves.

## Arabic UX impact

Each shell owns correct native RTL rendering, input, accessibility, focus, and bidirectional presentation while consuming canonical Rust-owned product semantics.

## Consequences and tradeoffs

### Positive

- High-quality platform-native UX, accessibility, and integration.
- UI technology can change without changing business state.

### Negative

- Presentation code and platform verification are repeated across operating systems.

### Risks

- Shell behavior may drift; mitigate with shared acceptance criteria, generated bindings, and contract conformance tests.

## Verification

Foundation review confirmed the ownership table in [target architecture](../architecture/target-architecture.md) assigns only presentation and OS adaptation to shells.

## Follow-up work

- Select native macOS and Linux UI technologies before implementing those shells.
- Define cross-platform Arabic and accessibility conformance scenarios.

## Related decisions and documents

- [ADR-0001](0001-rust-core-product-authority.md)
- [ADR-0003](0003-separate-supervised-engine-process.md)
- [Arabic-first UX](../product/arabic-first-ux.md)
