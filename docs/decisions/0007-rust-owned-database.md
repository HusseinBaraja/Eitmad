---
title: "ADR-0007: Rust owns all product database access"
description: "Records why only Rust may read, write, migrate, back up, or recover authoritative product storage."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "a product data path appears to require direct non-Rust database access"
---

# ADR-0007: Rust owns all product database access

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: A required data workload cannot meet correctness or performance goals through Rust-owned boundaries
> - Supersedes: None
> - Superseded by: None

## Context and problem

Direct database access from shells, reports, plugins, or utilities would bypass domain invariants, scope authorization, audit, migrations, and sync semantics. Multiple writers also make recovery and compatibility unsafe.

## Decision drivers

- Transactional enforcement of invariants and audit
- One migration and recovery authority
- Scope isolation and sync correctness

## Considered options

- **Rust-only database access:** strongest consistency and policy enforcement.
- **Read-only shell access:** appears safe, but can leak scopes and couple UI to schema.
- **Shared access:** maximum flexibility, but no reliable authority boundary.

## Decision

Only the Rust engine MUST open, query, or mutate product databases. All consumers use authorized commands, queries, subscriptions, exports, or diagnostic contracts. Rust owns schema, migrations, transactions, indexing, backup coordination, recovery, retention, deletion, and audit linkage.

## Architecture and contract impact

Read models are purpose-built and bounded. Large results use paging or streaming. Database layout is private implementation detail, not a client contract.

## Storage and sync impact

Every record has explicit scope. State mutation and its audit outcome are transactionally consistent where required. Local-first and server-authoritative stores implement the same domain and sync contracts without exposing storage handles.

## Security impact

Queries and exports are authorized in Rust and filtered by scope. Database credentials, paths, encryption keys, and raw pages are not exposed to shells or plugins.

## Arabic UX impact

Rust-owned search and collation define Arabic normalization, mixed-language matching, and stable sorting while preserving original Unicode values for display and audit.

## Consequences and tradeoffs

### Positive

- No client can bypass validation, authorization, audit, or migrations.
- Schema can evolve without coupling every shell to storage details.

### Negative

- Reporting and bulk workflows require designed APIs instead of ad hoc SQL.

### Risks

- Poor query contracts could create latency or oversized read models; mitigate with measured paging, streaming, subscriptions, and feature-owned projections.

## Verification

Foundation review confirms the direct-access prohibition. Production CI must detect shell database dependencies and tests must cover migration, transaction, backup, recovery, scope, and representative Arabic search behavior.

## Follow-up work

- Select storage technology only after the first domain's invariants, volume, and sync mode are defined.

## Related decisions and documents

- [Target architecture: Storage and event history](../architecture/target-architecture.md#storage-and-event-history)
- [ADR-0008](0008-required-unified-synchronization.md)
- [ADR-0010](0010-rebac-authorization-foundation.md)
