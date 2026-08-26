---
title: "Review Eitmad releases and compatibility"
description: "Find protocol, migration, rollout, and rollback notes for released foundation changes."
audience: "support"
page_type: "reference"
status: "active"
owner: "release maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "a release changes compatibility, migration, rollout, or rollback behavior"
keywords:
  - "release notes"
  - "protocol compatibility"
---

# Review Eitmad releases and compatibility

- [Upgrade local IPC to protocol 1.1](protocol-1-1-local-ipc-subscriptions.md): subscription capability, compatible `1.0` behavior, verification, and rollback limits.
- [Upgrade to protocol 1.2 configuration authorization](protocol-1-2-configuration-authorization.md): SQLite migration, Rust-owned configuration, scoped relationships, policy events, revocation, and compatibility.
- [Upgrade local storage to version 5 identity and recovery](storage-v5-identity-recovery.md): persistent tenant identity, session attribution, compatibility windows, migration snapshots, integrity, restore, and export.
- [Upgrade to protocol 1.3 scoped authorization and audit](protocol-1-3-scoped-authorization-audit.md): tenant/workspace context, relationship graphs, boundary gates, storage version 6, rollout, and rollback.
- [Upgrade local storage to version 7 synchronization state](storage-v7-sync-state.md): scoped durable mode state, offline queues, optimistic rollback, migration compatibility, generated contracts, and rollback limits.
- [Upgrade local storage to version 8 reference markers](storage-v8-reference-marker.md): bounded reference-marker state, atomic audit and publication outboxes, local-first sync, compatibility, and rollback limits.
- [Upgrade to protocol 1.4 modular server authority](protocol-1-4-server-authority.md): PostgreSQL migrations, remote authentication, device proof, sync snapshots, resumable subscriptions, compatibility, and rollback.
- [Upgrade to protocol 1.5 operational server planes](protocol-1-5-operational-server-planes.md): relay coordination, signed manifests, administration migration 3, generated bindings, compatibility, and rollback.
- [Upgrade the server audit envelope to migration 4](server-audit-envelope-v4.md): complete actor, scope, target, correlation, causation, and idempotency evidence with safe rollout and rollback.
- [Validate a release candidate](../operations/validate-release-candidate.md): mandatory CI, repeatable artifacts, signing evidence, staging promotion, restore, and rollback.

Return to the [documentation index](../index.md) or review the [contract evolution process](../api/evolve-contracts-compatibly.md).
