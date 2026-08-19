---
title: "ADR-0022: Persist tenant identity and fail closed during storage recovery"
description: "Records tenant-rooted identity persistence, offline session rules, migration windows, recovery snapshots, corruption checks, and scoped export."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture, identity, storage, and security maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "identity hierarchy, offline sessions, audit attribution, migration compatibility, recovery, or export policy changes"
keywords:
  - "ADR-0022"
  - "persistent identity"
  - "tenant isolation"
  - "storage recovery"
---

# ADR-0022: Persist tenant identity and fail closed during storage recovery

> - Status: Accepted
> - Date: 2026-08-19
> - Decision owners: Architecture, identity, storage, and security maintainers
> - Supersedes: None
> - Superseded by: None

## Context

Opaque IPC principals and scopes did not provide durable device, account, tenant, or session continuity. Existing migration transactions prevented partial SQL but did not preserve a recovery snapshot before schema change, state a bounded upgrade window, distinguish quick/full corruption checks, or define safe local export.

## Decision

Use tenant as the identity isolation root. Persist stable device, user, account, tenant, organization, workspace, and session IDs in the Rust-owned SQLite authority. Accounts bind users to tenants; organization/workspace and session references use tenant-aware foreign keys. Persist session issue, expiry, last-seen, online/offline state, and closure without storing authentication secrets. Offline sessions remain locally usable only while issued, unexpired, and open.

Audit records preserve principal/scope plus session/device IDs. Session topology supplies deeper account/user/tenant attribution without copying display data into append-only audit rows.

Support direct upgrades from storage versions 2–5 and fresh creation from 0. Reject older and newer histories before modification. Before every pending supported migration, create an owner-private online SQLite backup, run full integrity and compatibility verification, and publish it as a recovery artifact. Run each migration transactionally and fail readiness on any backup, migration, checksum, integrity, or schema-verification failure.

Use quick integrity plus migration/schema verification for normal diagnostics and full integrity for backups/restores and explicit maintenance. Restore only under exclusive stopped-engine authority, preserve the prior database, and expose recovery artifacts without deleting or trusting them.

Define portable local export as exact-tenant JSON. Include identity-directory IDs and organization/workspace configuration; exclude device/session state, audit, idempotency, outbox, credentials, and secrets. Export is not backup.

## Consequences

Local-first identity and audit attribution survive restart and offline periods without granting authentication authority to shells. Composite keys and scoped reads make tenant crossing a storage error. Migration failure has a validated recovery point and cannot silently continue.

Disk space must accommodate migration snapshots. Recovery artifacts and exports remain sensitive and need retention/encryption policy. Version 1 installations require an intermediate upgrade. Production authentication, session issuance, restore/export authorization, UI, scheduling, and remote backup remain unimplemented.

## Alternatives

- Shell-owned device/session files were rejected because they duplicate authority and cannot join audit/storage invariants.
- Globally keyed organizations/workspaces without tenant-aware references were rejected because UUID mistakes could cross isolation boundaries.
- Raw database export was rejected because it exposes sessions, audit, authorization, and operational internals.
- Best-effort migration without a verified snapshot was rejected because transaction rollback cannot protect against every semantic or later-discovered data-loss defect.

## Verification

Tests cover failed migration rollback, out-of-window rejection before history mutation, pre-migration backup validation, WAL-safe backup/restore, corrupt candidate rejection, recovery discovery, identity reopen, offline expiry/refresh/closure, audit session/device attribution, cross-tenant denial, atomic scoped export, and sensitive-state exclusion. See [persistent identity](../developer/subsystems/identity-foundation.md), [local storage](../developer/subsystems/local-storage.md), and [storage v5](../releases/storage-v5-identity-recovery.md).
