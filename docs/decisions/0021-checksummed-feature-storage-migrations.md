---
title: "ADR-0021: Use checksummed feature-owned storage migrations"
description: "Records immutable feature migration history, schema-drift verification, shared transactions, and stopped-engine recovery hooks."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture, storage, and security maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "migration ownership, history checksums, schema verification, transaction, backup, or restore policy changes"
keywords:
  - "ADR-0021"
  - "feature-owned migrations"
  - "schema drift"
  - "backup restore"
---

# ADR-0021: Use checksummed feature-owned storage migrations

> - Status: Accepted
> - Date: 2026-07-18
> - Decision owners: Architecture, storage, and security maintainers
> - Supersedes: None
> - Superseded by: None

## Context

ADR-0019 established one Rust-owned SQLite authority and ordered transactional migrations. A central anonymous SQL list could not identify feature ownership or detect edited history and unexpected schema changes. Copying an open WAL database also could not provide safe recovery hooks.

## Decision

Each storage feature declares migrations beside its repository. The storage infrastructure assembles one contiguous registry and records order, stable migration ID, feature, and SHA-256 checksum. Existing numeric history is backfilled from the known registry. Startup and read-only diagnostics reject history gaps, unknown or changed migrations, newer versions, corruption, and schema that differs from a fresh application of the registry.

Repositories use shared crate-private read and write transaction helpers; raw SQLite handles remain private. All product query paths authorize in Rust and bind repository reads to an exact scope. A repository boundary test rejects SQLite access markers in native shells and platform adapters.

Backup uses SQLite's online backup API and validates the result. Restore is a Rust-only stopped-engine hook: validate and stage first, preserve the prior database, install the candidate under exclusive authority, and reopen through normal migration checks. Scheduling, retention, operator UI, and IPC exposure are deferred.

## Consequences

Applied migrations become immutable and attributable. Manual schema edits, damaged history, and shell database access fail before release or readiness. Feature repositories share consistent rollback and snapshot behavior. Recovery can include committed WAL state and preserve the replaced database, but production restore orchestration still needs explicit authorization, audit, disk-space, retention, encryption, and update policies.

The design keeps one cross-feature transaction boundary. It does not move raw connections into product crates or create one database per feature.

## Verification

Migration tests cover legacy upgrade, order, checksum, gaps, rollback, newer versions, and schema drift. Transaction and recovery tests cover atomic rollback, snapshot consistency, WAL-safe backup, valid restore, and corrupt candidate rejection. Cross-boundary tests cover permission denial, scope isolation, and prohibited shell database dependencies. See [local storage](../developer/subsystems/local-storage.md) for extension rules.
