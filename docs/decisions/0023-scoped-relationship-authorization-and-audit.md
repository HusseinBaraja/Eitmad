---
title: "ADR-0023: Use scoped relationship graphs and one audited boundary gate"
description: "Records generic relationship tuples, role objects, inherited permissions, conditional grants, strict isolation, and mandatory boundary audit."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture, authorization, security, and audit maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "tuple semantics, inheritance, conditions, isolation, boundary enforcement, or audit completeness changes"
keywords:
  - "ADR-0023"
  - "relationship graph"
  - "can(actor, action, object)"
  - "audit completeness"
---

# ADR-0023: Use scoped relationship graphs and one audited boundary gate

> - Status: Accepted
> - Date: 2026-08-19
> - Decision owners: Architecture, authorization, security, and audit maintainers
> - Supersedes: [ADR-0020](0020-direct-principal-scope-rebac-policy.md) as the target authorization model
> - Superseded by: None

## Context

Direct principal-to-organization roles secured configuration but could not represent product records, role objects, teams, inherited access, conditional grants, synchronization peers, providers, or plugins. Tenant and workspace existed in persistent identity but were not mandatory in authorization context or audit. Read-only and non-domain boundaries also lacked one reusable fail-closed enforcement and result-audit seam.

## Decision

Use immutable, Rust-owned relationship policy snapshots. Every object embeds tenant and optional workspace identity. Tuples relate a principal or scoped object to another scoped object. Roles are objects connected through `eitmad.relation.role.member.v1`, not trusted claims. Permission rules map an action/object-kind pair to grant relations and explicit inheritance relations.

Evaluate `can(actor, action, object)` deny-by-default. Reject cross-tenant and cross-workspace object edges during policy construction. Deny request scope mismatch, missing rules/tuples, failed optional attribute conditions, cycles, and traversal beyond the fixed depth bound.

Use one `AuthorizationGate` shape for commands, queries, sync, external adapters, and plugin capabilities. Denied actions never execute. Domain mutations keep audit and state in one transaction; the convenience gate is restricted to reads, reject-only work, or callers whose transaction includes audit.

Require a complete audit envelope with actor, explicit device attribution, session, tenant/workspace, scope, operation, target, result, timestamp, correlation, and redacted error metadata. Persist new fields through storage version 6. Reserve typed extension markers for approvals, ledgers, conflicts, security events, and undo-critical work without treating the marker as implementation.

Protocol `1.3` carries mandatory tenant/workspace authorization context. Local IPC requires `eitmad.capability.authorization-scopes.v1`, and generated bindings deploy with the engine.

Keep direct organization policy v1 as the persisted compatibility path for configuration administration until product verticals define policy-v2 tuple lifecycle and persistence. Do not overload its table with generic graph semantics.

## Consequences

Authorization vocabulary now matches product objects and organizational delegation. Scope is inseparable from object identity, and all covered boundaries share one rejection/audit behavior. Immutable snapshots make one decision coherent and immediate revocation can replace the snapshot.

Policy provisioning and persistence remain future work for each product vertical. String equality conditions are intentionally small; complex time, network, or mutable external attributes require a later decision. Bounded graph traversal may deny very deep legitimate graphs, so product modeling must remain shallow and explainable. Protocol `1.3` requires coordinated generated bindings.

## Alternatives

- Flat roles were rejected because they cause role explosion and cannot express record relationships safely.
- Client-supplied role or attribute claims were rejected because boundary assertions are not authorization evidence.
- An unscoped tuple store was rejected because filtering after evaluation risks cross-tenant disclosure.
- Unbounded recursive traversal was rejected because cycles and adversarial graphs can exhaust resources.
- Best-effort audit after an irreversible mutation was rejected because it can report failure after state escaped without its mandatory record.
- A general policy language was deferred until concrete domains require operators beyond equality, `all`, and `any`.

## Security, Arabic, compatibility, and operations

Audit stores stable error identifiers and classes, not raw messages or data. Target identifiers require owning-vertical redaction. Plugin/provider wrappers do not grant authority by being installed. Arabic and non-Arabic actors receive identical policy decisions; future RTL UI localizes outcomes and directionally isolates machine identifiers.

Storage version 6 is additive for supported histories. Protocol `1.3` is capability-required because an unscoped authenticated context is unsafe. Rollback below storage 6 must use a validated pre-migration artifact; do not point an older engine at the migrated database.

## Verification

Tests prove direct and denied access, role membership, inherited permission, conditional access, tenant/workspace isolation, cross-scope edge rejection, unauthorized reads/writes, sync and plugin rejection, audit completeness/redaction, append-only persistence, migration/recovery, contract drift, and C# round-trip. See [authorization and audit](../developer/subsystems/authorization.md) and the [protocol 1.3 release](../releases/protocol-1-3-scoped-authorization-audit.md).
