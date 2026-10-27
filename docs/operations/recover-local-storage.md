---
title: "Recover and export local storage"
description: "Validate backups, identify preserved recovery files, restore safely, and export one tenant without exposing sensitive state."
audience: "operations"
page_type: "task"
status: "active"
owner: "Rust storage maintainers"
last_verified: "2026-10-24"
review_triggers:
  - "backup, restore, migration, corruption, recovery artifact, retention, or export policy changes"
keywords:
  - "pre-migration backup"
  - "integrity_check"
  - "eitmad.pre-restore"
  - "eitmad.local-data-export.v1"
  - "استعادة قاعدة البيانات"
---

# Inspect and export local storage

Preserve an unhealthy runtime directory and stop normal writes before investigation. Rust owns storage validation and schema migration. Backup, stopped-engine restore, and automatic pre-migration snapshots are not available; see [backup availability](backup-lan-installation.md) and [ADR-0033](../decisions/0033-defer-backup-support.md).

## Backup and migration protection

Each registered SQLite migration runs in a transaction. A failed migration rolls back its own SQL and history update. Checks reject unsupported versions, changed checksums, and schema drift. Startup creates no backup file. Do not install an older binary over a database that may have migrated. Existing historical artifacts remain untouched.

## Manual SQLite backup and restore

The manual CLI and Rust backup/restore APIs were removed. No current command can restore an old artifact. Preserve the database, WAL, shared-memory files, Windows profile, credentials, and existing artifacts together for an operator investigation. Do not delete or reset the runtime to make startup pass.

## Corruption detection strategy

Readiness diagnostics use `CorruptionCheck::Quick` with read-only history and schema verification. Rust can run `AuthorityStore::verify_integrity(Full)` for explicit maintenance. A non-`ok` result stops use of the database. Diagnostics do not copy the live database or apply pending migrations.

## Export one tenant

`AuthorityStore::export_tenant_data` writes format `eitmad.local-data-export.v1` from one read transaction. It includes tenant identity directory IDs and organization/workspace configuration. It excludes device records, sessions, audit, idempotency, publication outbox, credentials, and secrets. The destination must not exist; Rust writes privately, flushes, and atomically creates the final path without replacement. If another process creates that path first, export fails and preserves the existing file.

Export is not backup and cannot restore the authority database. A future caller must authorize the exact tenant, audit the request, select an approved destination, apply retention/encryption policy, and prevent cross-tenant disclosure.

## Escalate safely

Collect engine/storage versions, integrity outcome, migration ID/version, timestamp, and a synthetic reproduction. Do not attach database files, exports, paths, account/user IDs, authorization graphs, tokens, or customer records to ordinary tickets. Follow [storage startup troubleshooting](../troubleshooting/local-storage-recovery-failures.md) and [local storage ownership](../developer/subsystems/local-storage.md).
