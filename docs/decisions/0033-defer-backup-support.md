---
title: "ADR-0033: Defer backup support"
description: "Remove backup implementation and installation gates until a separate backup design is accepted."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "product developer"
last_verified: "2026-10-27"
review_triggers:
  - "backup or restore support is proposed again"
keywords:
  - "backup removal"
  - "setup failure logs"
---

# ADR-0033: Defer backup support

> - Status: Accepted
> - Date: 2026-10-27
> - Decision owners: Product developer
> - Supersedes: Backup and restore requirements only in [ADR-0021](0021-checksummed-feature-storage-migrations.md), [ADR-0022](0022-persistent-tenant-identity-and-safe-storage-recovery.md), [ADR-0026](0026-compose-authorized-operational-server-planes.md), [ADR-0030](0030-offline-guided-windows-setup.md), and [ADR-0031](0031-two-self-contained-windows-installers.md)
> - Superseded by: None

## Context

The product developer requested removal of local and external backups, including setup requirements and related project code. Backup support will need a separate design before it is added again. Setup failures also need technical logs that can be copied or saved from the native Arabic screen.

## Decision

Remove backup configuration, capture, copy, scheduling, retention, restore tools, administrative status routes, backup contracts and native management. Remove manual SQLite backup/restore and automatic pre-migration snapshots. Setup and upgrades require no destination or backup generation.

Preserve real local data, historical files, immutable SQL migration bytes, and secret-file interrupted-write recovery. The historical PostgreSQL backup-status table becomes unused; do not rewrite an applied migration or drop existing data. Synchronization snapshots, pending-work recovery, setup journals, integrity checks, and migration transactions keep their existing roles.

Rust reports bounded setup failure categories and native exception details. The native screen explains the failed step and next action and offers clipboard copy and a UTF-8 log export. Diagnostic exports do not contain setup requests or credential input. Recovery availability remains Rust-owned.

## Consequences

There is no supported whole-installation backup or restore path in current source. Existing saved copies are not deleted. Old installed tasks are not silently removed by a source update. An accepted future design must define its data, credentials, storage, authorization, recovery, and verification requirements before implementation.

Read [Windows installation](../operations/install-windows.md), [backup availability](../operations/backup-lan-installation.md), and [storage inspection](../operations/recover-local-storage.md). Return to the [decision index](index.md).
