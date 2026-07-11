# ADR-0001: Rust core is the product authority

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: A product capability cannot be implemented behind the Rust boundary without unacceptable platform or operational cost
> - Supersedes: None
> - Superseded by: None

## Context and problem

The product needs one authoritative implementation of business rules across desktop platforms, headless workflows, storage, sync, and future services. Allowing each shell to own behavior would create drift and inconsistent audit and authorization outcomes.

## Decision drivers

- Correctness across all clients and deployment modes
- One testable location for domain invariants and security controls
- Replaceable UI and transport technologies

## Considered options

- **Rust authority:** one portable, efficient authority with explicit boundaries.
- **Shell-owned logic:** closer to UI frameworks, but duplicates rules by platform.
- **Shared authority:** flexible in the short term, but creates ambiguous ownership.

## Decision

The Rust core MUST define, validate, execute, persist, and test authoritative product behavior. Native shells MAY own presentation and ephemeral view state only. When ownership is unclear, it defaults to Rust unless a later ADR defines a narrow exception.

## Architecture and contract impact

All authoritative operations cross Rust-owned, typed contracts. Rust owns domain rules, identity, permissions, sync, updates, audit, external services, and background jobs.

## Storage and sync impact

Rust is the only product component permitted to mutate authoritative state or apply sync conflict policy.

## Security impact

Authentication, authorization, validation, scope enforcement, secrets, and audit are enforced in Rust. Shell behavior is never a security boundary.

## Arabic UX impact

Domain terminology, validation outcomes, search semantics, and stable localized message identifiers originate from the Rust authority; shells render them correctly in native Arabic-first interfaces.

## Consequences and tradeoffs

### Positive

- Platform behavior remains consistent and independently testable.
- Shells and transports can be replaced without migrating domain logic.

### Negative

- UI work requires explicit IPC and cannot take direct data shortcuts.

### Risks

- An oversized core could become monolithic; mitigate with vertical domain modules and narrow public APIs.

## Verification

Foundation review confirmed consistency with [target architecture](../architecture/target-architecture.md). Production CI must later reject authoritative logic and direct storage access in shells.

## Follow-up work

- Define the first vertical domain module and its public contract before production implementation.

## Related decisions and documents

- [Target architecture](../architecture/target-architecture.md)
- [ADR-0002](0002-native-platform-ui-shells.md)
- [ADR-0005](0005-generated-or-validated-contracts.md)
