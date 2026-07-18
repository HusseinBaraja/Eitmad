---
title: "ADR-0020: Use direct principal-to-scope ReBAC policy v1"
description: "Records the first compiled ReBAC policy, owner bootstrap, revision, and subscription revocation behavior."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture and authorization maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "policy v1 relations, permissions, principal kinds, bootstrap, or revocation semantics change"
keywords:
  - "ADR-0020"
  - "ReBAC policy v1"
  - "last owner"
---

# ADR-0020: Use direct principal-to-scope ReBAC policy v1

> - Status: Accepted
> - Date: 2026-07-18
> - Decision owners: Architecture and authorization maintainers
> - Supersedes: None
> - Superseded by: None

## Context

Configuration cannot be safely exposed before Rust can authorize exact users, devices, and services in an organization scope. A generic future graph engine would add policy ambiguity before teams, nested scopes, or record relations exist.

## Decision

Compile policy v1 in Rust from direct principal-to-organization relationships: owner, config manager, and member. Members read and subscribe to configuration and query their effective permissions. Config managers also patch, import, and export. Owners also grant, revoke, and list relationships.

Evaluation is deny by default, exact-scope, and uncached. Mutations require expected policy revision and durable idempotency. Preserve at least one persisted owner. Expose a Rust-only audited first-owner bootstrap only for scopes without an owner. In insecure development-auth mode only, treat the authenticated synthetic principal as an ephemeral owner.

After a real policy mutation, publish one scoped policy-change event and reauthorize active subscriptions before later delivery. Protocol `1.2` clients receive `authorizationRevoked`; older clients close without the new enum value.

## Consequences

The first policy is small, testable, and fails closed. It does not support groups, teams, nested scopes, record relations, conditional grants, or stored policy documents. Adding those concepts requires a new policy version and migration rather than overloading current relation meaning.

## Alternatives

- Role claims supplied by shells were rejected because assertions are not authorization evidence.
- A cached evaluator was rejected because current relationship volume does not justify revocation staleness.
- A generic tuple graph and policy language were deferred until a real domain proves their terminology and query needs.

## Verification

Tests cover role decisions, cross-scope isolation, all principal kinds, owner-only listing, revision conflicts, replay, last-owner protection, bootstrap, development ownership, policy events, and active revocation. See the [authorization subsystem](../developer/subsystems/authorization.md) and [protocol 1.2 release](../releases/protocol-1-2-configuration-authorization.md).
