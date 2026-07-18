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

Grant and revoke commands require an expected policy revision and durable idempotency key. A real mutation increments policy revision once; an existing grant is a successful no-op. Same-input replay returns `changed: false`, so it cannot emit a duplicate event. Different-input key reuse fails.

Rust refuses to revoke the final persisted owner in a scope. `AuthorizationService::bootstrap_owner` is a Rust-only audited operation that succeeds only when no persisted owner exists. It is not exposed through shell IPC. In explicitly insecure development-auth mode, the authenticated synthetic principal is an ephemeral owner without a persisted relationship. Production remains fail closed until trusted identity provisioning exists.

Successful mutations commit relationship state, policy revision, idempotency outcome, and audit outcome together. Denied, invalid, conflicting, not-found, and last-owner attempts receive separate audit outcomes. Audit records contain identity, scope, correlation/causation/idempotency IDs, operation, result, revisions, and changed identifiers; they exclude configuration values, secrets, and authorization graphs.

## Events and revocation

Protocol `1.2` adds `AuthorizationPolicyChanged` and its capability-gated subscription. After commit, the dispatcher publishes the scoped policy revision and signals every active pump to reauthorize. Each pump also reauthorizes immediately before event delivery, so a revoked member receives no later configuration snapshot.

A `1.2` client receives `SubscriptionClosed` with `authorizationRevoked`. A negotiated `1.0` or `1.1` connection is terminated safely instead of receiving an enum value it cannot understand. Replay and policy events are published only for real mutations, never failed operations or idempotent replay.

## Failure and extension rules

Stable failures distinguish denied access, invalid relation, optimistic policy conflict, last-owner protection, and unavailable authority storage. Shells render localized message IDs and may refresh effective permissions after a safe conflict; they must not infer relationships from denials.

Tests cover role mapping, principal kinds, cross-scope isolation, owner-only listing, revision conflict, idempotency, last-owner protection, bootstrap, development ownership, protocol gating, policy events, and active subscription closure. Add nested teams, record-level relations, or stored policy documents only through a new policy version and ADR; do not extend policy v1 by hidden convention. See [ADR-0020](../../decisions/0020-direct-principal-scope-rebac-policy.md) and [recovery guidance](../../troubleshooting/configuration-authorization-failures.md).
