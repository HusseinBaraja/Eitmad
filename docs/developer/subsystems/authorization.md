---
title: "Extend scoped ReBAC authorization safely"
description: "Understand direct principal-to-scope relationships, policy v1, owner protection, audit, and subscription revocation."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust authorization maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "a principal, relation, permission, policy, bootstrap, or revocation rule changes"
keywords:
  - "eitmad-authorization"
  - "ReBAC"
  - "config-manager"
  - "authorizationRevoked"
---

# Extend scoped ReBAC authorization safely

`eitmad-authorization` owns policy v1 for direct user, device, and service relationships to an exact organization scope. Evaluation is deny by default, has no authorization cache, and never trusts shell UI visibility or asserted permissions.

## Relations and permissions

| Relation | Effective configuration access | Authorization access |
| --- | --- | --- |
| `eitmad.relation.organization.member.v1` | Read and subscribe; query own effective permissions | None |
| `eitmad.relation.organization.config-manager.v1` | Member access plus patch, import, and export | None |
| `eitmad.relation.organization.owner.v1` | Config-manager access | Grant, revoke, and bounded relationship listing |

The policy compiles these relationships into registered permission decisions in stable order. Relationships never cross scopes. `ListScopeRelationships` is owner-only and bounded to 1–500 results.

## Mutations, bootstrap, and audit

Grant and revoke commands require an expected policy revision and durable idempotency key. A real mutation increments policy revision once; an existing grant is a successful no-op. Same-input replay returns `changed: false` and creates no new event, but it resumes any publication left pending by the original committed attempt. Different-input key reuse fails.

Rust refuses to revoke the final persisted owner in a scope. `AuthorizationService::bootstrap_owner` is a Rust-only audited operation that succeeds only when no persisted owner exists. It is not exposed through shell IPC. In explicitly insecure development-auth mode, the authenticated synthetic principal is an ephemeral owner without a persisted relationship. Production remains fail-closed until trusted identity provisioning exists.

Successful mutations commit relationship state, policy revision, idempotency outcome, audit outcome, and a durable publication outbox row together. The dispatcher publishes and removes that row after commit; startup drains rows left by an interruption before accepting IPC traffic. Denied, invalid, conflicting, not-found, and last-owner attempts receive separate audit outcomes. Audit records contain identity, scope, correlation/causation/idempotency IDs, operation, result, revisions, and changed identifiers. A grant records its exact relationship ID and relation plus a versioned SHA-256 subject fingerprint; it does not copy the raw subject principal ID into `changed_identifiers`. Audit records exclude configuration values, secrets, and authorization graphs.

Every public authorization entry point accepts only an `organization` scope. Rust returns the wrong-scope authorization failure before reading relationships, evaluating development ownership, bootstrapping an owner, or applying a mutation. Adding another scope type requires a new documented policy instead of reusing organization relations implicitly.

## Arabic, RTL, and mixed-direction behavior

Authorization policy has no localized branch: the same Rust decision applies to Arabic and non-Arabic sessions. Native shells localize stable message IDs and render the relationship-management workflow RTL, but they must keep relationship IDs, permission IDs, UUIDs, policy revisions, and correlation IDs directionally isolated for reliable reading and copy/paste. This release adds no relationship-management UI; any future Arabic UI must use canonical glossary terms and test Arabic names beside LTR identifiers without inferring access from hidden or disabled controls.

## Design tradeoffs

Policy v1 uses direct principal-to-organization relationships. This keeps evaluation and revocation deterministic but intentionally omits nested teams, inherited record access, and stored policy expressions. Evaluation reads authoritative state without a cache, favoring immediate revocation correctness over lower read latency. The last-owner invariant protects persisted production administration, while explicitly insecure development authentication uses a non-persisted owner to avoid contaminating production relationship state.

## Operations

The engine opens and migrates the authority database before reporting readiness. Production operators must provision a trusted persisted owner before configuration or authorization administration can succeed; insecure development ownership is not a recovery mechanism. Monitor sanitized authorization-unavailable errors and correlation IDs, preserve the runtime directory before repair, and use the non-mutating authority-store diagnostic against that exact directory. For policy conflicts, refresh the relationship page and retry with the returned policy version. For storage, bootstrap, last-owner, or revocation failures, follow the linked recovery guide instead of editing SQLite rows directly.

## Events and revocation

Protocol `1.2` adds `AuthorizationPolicyChanged` and its capability-gated subscription. After commit, the dispatcher publishes the scoped policy revision and signals every active pump to reauthorize. Each pump also reauthorizes immediately before event delivery, so a revoked member receives no later configuration snapshot.

A `1.2` client receives `SubscriptionClosed` with `authorizationRevoked`. A negotiated `1.0` or `1.1` connection is terminated safely instead of receiving an enum value it cannot understand. Each delivery is reauthorized at the writer boundary. If policy changes while a write is blocked, Rust cancels the partial frame and terminates the connection; a close cursor therefore never claims an event that was not fully written. Policy events originate only from real committed mutations. Idempotent replay creates no new event but may finish the original mutation's pending publication.

## Failure and extension rules

Stable failures distinguish denied access, invalid relation, optimistic policy conflict, last-owner protection, and unavailable authority storage. Shells render localized message IDs and may refresh effective permissions after a safe conflict; they must not infer relationships from denials.

Tests cover role mapping, principal kinds, cross-scope isolation, owner-only listing, revision conflict, idempotency, last-owner protection, bootstrap, development ownership, protocol gating, policy events, and active subscription closure. Add nested teams, record-level relations, or stored policy documents only through a new policy version and ADR; do not extend policy v1 by hidden convention. See [ADR-0020](../../decisions/0020-direct-principal-scope-rebac-policy.md) and [recovery guidance](../../troubleshooting/configuration-authorization-failures.md).
