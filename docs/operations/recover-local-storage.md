---
title: "Recover and export local storage"
description: "Validate backups, identify preserved recovery files, restore safely, and export one tenant without exposing sensitive state."
audience: "operations"
page_type: "task"
status: "active"
owner: "Rust storage maintainers"
last_verified: "2026-08-26"
review_triggers:
  - "backup, restore, migration, corruption, recovery artifact, retention, or export policy changes"
keywords:
  - "pre-migration backup"
  - "integrity_check"
  - "eitmad.pre-restore"
  - "eitmad.local-data-export.v1"
  - "استعادة قاعدة البيانات"
---

# Recover and export local storage

Rust provides stopped-engine recovery hooks and tenant-scoped export primitives. No production operator UI, IPC command, scheduler, retention job, or remote destination is implemented, so only a trusted Rust coordinator holding exclusive engine authority may invoke restore.

## Safety prerequisites

- Stop the engine and confirm its runtime-directory authority is released.
- Preserve the full private runtime directory and available disk space.
- Treat live databases, backups, recovery artifacts, and exports as sensitive.
- Never copy an open SQLite file, delete WAL/SHM companions, edit migration rows, or use SQLite repair commands on the only copy.

## Backup and migration protection

`AuthorityStore::backup_to` uses SQLite online backup, includes committed WAL data, writes an owner-private temporary file, runs full `PRAGMA integrity_check`, proves migration/schema compatibility on an in-memory copy, then atomically publishes the requested path. Existing destinations are rejected.

Before any pending migration from supported storage versions 2–7, startup creates and validates `eitmad.pre-migration-vN-to-v8.sqlite3`. A retry for the same source and target versions validates and reuses that one artifact. Failure to create or validate it stops startup before migration SQL. Each migration runs in its own transaction; SQL or history failure rolls back that migration and leaves the snapshot discoverable through `AuthorityStore::recovery_artifacts`.

## Validate and restore

1. Call `AuthorityStore::validate_backup` while the engine is stopped. It performs full corruption, compatibility, checksum, pending-migration, and schema-drift checks without changing the candidate.
2. Acquire exclusive engine authority for the target runtime directory.
3. Call `AuthorityStore::restore_from_backup`. Rust privately stages and revalidates the candidate.
4. Rust checkpoints the stopped live database, preserves it as `eitmad.pre-restore-<uuid>.sqlite3`, installs the candidate, and reopens it through normal startup verification.
5. If installation or reopen fails, Rust attempts to reinstall the previous database and preserves the failed candidate as `eitmad.failed-restore-<uuid>.sqlite3` when possible.

After success, run diagnostics and a clean start/stop. Keep the pre-restore file until an approved retention policy confirms the restored state and all required newer writes are retained elsewhere.

## Corruption detection strategy

Readiness diagnostics use `CorruptionCheck::Quick` plus migration and schema verification. Backup creation and restore validation use `CorruptionCheck::Full`. Operators or future background maintenance may call `AuthorityStore::verify_integrity(Full)` when the engine is quiescent. Any non-`ok` result is fail-closed; do not keep writing or attempt destructive repair.

## Export one tenant

`AuthorityStore::export_tenant_data` writes format `eitmad.local-data-export.v1` from one read transaction. It includes tenant identity directory IDs and organization/workspace configuration. It excludes device records, sessions, audit, idempotency, publication outbox, credentials, and secrets. The destination must not exist; Rust writes privately, flushes, and atomically creates the final path without replacement. If another process creates that path first, export fails and preserves the existing file.

Export is not backup and cannot restore the authority database. A future caller must authorize the exact tenant, audit the request, select an approved destination, apply retention/encryption policy, and prevent cross-tenant disclosure.

## Escalate safely

Collect engine/storage versions, artifact kind, integrity outcome, migration ID/version, timestamp, and a synthetic reproduction. Do not attach database files, exports, paths, account/user IDs, authorization graphs, tokens, or customer records to ordinary tickets. Follow [storage startup troubleshooting](../troubleshooting/local-storage-recovery-failures.md) and [local storage ownership](../developer/subsystems/local-storage.md).
