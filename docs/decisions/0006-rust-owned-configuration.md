# ADR-0006: Rust owns product configuration

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: A platform setting cannot safely be represented as an adapter-owned exception
> - Supersedes: None
> - Superseded by: None

## Context and problem

Configuration affects behavior, compatibility, security, sync, and recovery. If shells parse or edit product config directly, validation and defaults can diverge and partial writes can bypass audit or migration rules.

## Decision drivers

- Consistent validation, defaults, and migration
- Atomic, recoverable writes
- No secret or policy duplication in shells

## Considered options

- **Rust-owned config:** one authoritative schema and lifecycle.
- **Shell-owned config:** convenient settings UI, but duplicates product rules.
- **Shared file access:** simple initially, but creates concurrent writers and weak boundaries.

## Decision

Rust MUST own product configuration schemas, defaults, parsing, validation, persistence, migration, and change notifications. Shells MUST read and change settings through typed commands, queries, and subscriptions. Adapter-only presentation preferences MAY be shell-owned when they cannot influence authoritative product behavior, security, sync, or data interpretation.

## Architecture and contract impact

Config contracts are versioned Rust-owned contracts with capability discovery and structured validation errors. Secrets use approved protected storage behind Rust or a narrowly defined platform adapter contract, never ordinary config files.

## Storage and sync impact

Each setting declares scope, persistence, sync eligibility, conflict behavior, and recovery defaults. Configuration changes affecting state or policy are audited.

## Security impact

Sensitive values are redacted from IPC, logs, support bundles, and UI. Authorization applies to config reads and writes by setting and scope.

## Arabic UX impact

Settings expose stable localized labels, descriptions, values, validation, and restart requirements. Canonical values remain locale-independent.

## Consequences and tradeoffs

### Positive

- All modes interpret configuration identically.
- Invalid, stale, or partial config cannot be normalized differently by each shell.

### Negative

- Even simple product settings require an engine round trip and contract definition.

### Risks

- The config module can become a catch-all; mitigate with setting ownership in vertical capabilities and a focused configuration infrastructure boundary.

## Verification

Foundation review confirms shells are prohibited from product config file access. Production tests must cover defaults, invalid data, atomic writes, migration, authorization, and redaction.

## Follow-up work

- Define config scopes and protected-secret integration before the first persisted setting.

## Related decisions and documents

- [ADR-0001](0001-rust-core-product-authority.md)
- [ADR-0005](0005-generated-or-validated-contracts.md)
- [ADR-0012](0012-privacy-preserving-observability.md)
