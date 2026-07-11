---
title: "ADR-0010: Use ReBAC as the authorization foundation"
description: "Records why Rust derives permissions from scoped relationships and authorizes every command and query."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture and security maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "relationship authorization cannot express a required permission or scale target"
---

# ADR-0010: Use ReBAC as the authorization foundation

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: Product relationships cannot express a required access policy safely or efficiently
> - Supersedes: None
> - Superseded by: None

## Context and problem

Access depends on relationships among people, roles, teams, sites, organizations, and scoped records. Flat roles alone cannot safely express delegation, ownership, temporary access, or resource-specific authority without role explosion or scattered exceptions.

## Decision drivers

- Policies match real organizational relationships
- Central, explainable, testable authorization
- Explicit scope and revocation behavior

## Considered options

- **Relationship-based access control (ReBAC):** expressive foundation for scoped product relationships.
- **Role-based access control only:** simple, but insufficient for resource relationships and delegation.
- **Attribute-based access control only:** flexible, but can become opaque and difficult to explain.
- **Ad hoc checks:** fast initially, but inconsistent and unauditable.

## Decision

Rust MUST authorize every command and query using ReBAC as the foundation. The authorization model relates actors, roles, teams, sites, organizations, and records within explicit scopes. Roles and attributes MAY contribute to relationship evaluation, but do not replace the centralized policy model. Deny is the default. UI visibility is only a usability projection of engine decisions.

## Architecture and contract impact

Authorization exposes typed decision, capability, and explainability contracts without leaking the full permission graph. Policy and relationship schema are versioned and migrated with compatibility controls.

## Storage and sync impact

Relationship tuples and policy state are authoritative, scoped, auditable, revocable, and synchronized consistently with the records they protect. Stale replicas fail safely for sensitive mutations.

## Security impact

Checks occur at command, query, subscription, export, sync, and administrative boundaries. Cache keys include principal, scope, policy version, and relevant relationship state; revocation bounds are explicit.

## Arabic UX impact

The product uses approved Arabic terms for roles and relationships. Denials and delegated-access states are localized and actionable while avoiding sensitive graph disclosure.

## Consequences and tradeoffs

### Positive

- Access rules align with organizational and record relationships.
- Central decisions can be tested, audited, and explained consistently.

### Negative

- Relationship modeling, policy evaluation, caching, and revocation are more complex than flat roles.

### Risks

- Policy graphs can become slow or incomprehensible; mitigate with bounded relations, explicit ownership, decision traces, and representative performance tests.

## Verification

Foundation review confirms ReBAC is the required authorization core. Each production feature must include positive, negative, cross-scope, revocation, and stale-policy tests.

## Follow-up work

- Define the first domain's actors, relations, scopes, permissions, and Arabic terminology.
- Select or implement a policy evaluator only after representative authorization scenarios exist.

## Related decisions and documents

- [ADR-0009](0009-zero-trust-security-model.md)
- [ADR-0007](0007-rust-owned-database.md)
- [Domain glossary](../glossary.md)
