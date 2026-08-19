---
title: "Review Eitmad releases and compatibility"
description: "Find protocol, migration, rollout, and rollback notes for released foundation changes."
audience: "support"
page_type: "reference"
status: "active"
owner: "release maintainers"
last_verified: "2026-08-19"
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

Return to the [documentation index](../index.md) or review the [contract evolution process](../api/evolve-contracts-compatibly.md).
