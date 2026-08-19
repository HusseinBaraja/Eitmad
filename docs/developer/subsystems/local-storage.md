---
title: "Extend Rust-owned local storage safely"
description: "Understand SQLite setup, migration snapshots and windows, corruption checks, transactions, recovery, and export boundaries."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust storage maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "database setup, migration history/window, transaction, integrity, backup, restore, export, or schema verification changes"
keywords:
  - "eitmad-storage"
  - "schema_migrations"
  - "schema drift"
  - "backup restore"
  - "eitmad.sqlite3"
---

# Extend Rust-owned local storage safely

`eitmad-storage` is the only product-database access boundary. It opens bundled SQLite, applies and verifies migrations, provides scoped repositories and transaction boundaries, and exposes Rust-only recovery/export hooks. Native shells and platform adapters cannot open `eitmad.sqlite3` or reference a SQLite driver.

## Ownership and startup

The engine acquires authority for the runtime directory before `AuthorityStore::open` creates or opens `eitmad.sqlite3`. Every connection enables foreign keys, WAL mode, and a five-second busy timeout. The runtime directory and database receive owner-only OS permissions; failure is fatal before readiness.

Configuration, direct authorization, identity, and audit persistence remain focused modules inside the storage crate. Each module declares its migrations beside its repository. Shared migration history, connection policy, idempotency, the publication outbox, and recovery mechanics remain storage infrastructure. No raw `Connection` or `Transaction` crosses the crate boundary.

Persistent identity is a sibling vertical. Its tenant-rooted tables and public behavior are documented in [persistent identity](identity-foundation.md).

## Migration history and schema drift

The ordered registry assigns every migration a numeric order, stable ID, owning feature, SQL body, and SHA-256 checksum. `schema_migrations` persists those values. Existing numeric version 2–5 history is transactionally rebuilt and backfilled from the known registry only when its rows are the exact contiguous sequence `1..=N`.

Startup requires applied history to be an exact registry prefix. Storage version 6 accepts fresh version 0 and upgrades versions 2–5; version 1, gaps, reordered/unknown migrations, changed checksums, and databases newer than the engine are rejected before history modification. Before pending supported migrations, Rust creates a validated `eitmad.pre-migration-vN-to-v6.sqlite3` online backup. Migration `audit.authorization-envelope.v2` adds tenant, workspace, target, redacted-error, and extension-point columns. A retry for the same version pair validates and reuses its artifact. Snapshot failure prevents migration SQL. After migration, Rust builds the expected schema in memory from the same registry and compares tables, indexes, and triggers. Diagnostics perform quick integrity, history, pending-migration, and schema-drift checks against an in-memory backup without mutating live state.

Never edit an applied migration. Add the next ordered migration to the owning feature, preserve upgrade behavior from supported history, and add rollback and drift tests.

## Transactions and permission-filtered queries

Repositories use crate-private deferred read transactions and immediate write transactions. The helper commits only when its closure succeeds; an error rolls back all feature state, audit, idempotency, and publication rows written within that boundary. Snapshot reads use one read transaction so related revision and page data cannot come from different database states.

The supported read path is authenticated IPC, Rust dispatcher, ReBAC authorization, exact-scope service, scope-filtered repository, then SQLite. Configuration and relationship SQL includes `scope_kind` and `scope_id`; authorization is denied by default before product data is returned. The cross-repository boundary test scans native shell and adapter source for the database filename and known SQLite drivers.

## Backup and stopped-engine restore hooks

`AuthorityStore::backup_to` uses SQLite's online backup API, so committed WAL state is included without copying an open file. The destination must not exist. Rust creates the temporary destination with owner-only permissions before writing content, runs full integrity, migration compatibility, checksum, and schema verification, then publishes the backup path.

`AuthorityStore::validate_backup` is read-only. `AuthorityStore::restore_from_backup` requires the caller to hold exclusive engine authority. It validates and privately stages the candidate first, checkpoints the stopped live database, preserves the previous database under a unique `eitmad.pre-restore-*.sqlite3` name, installs the candidate, and reopens it through normal migration and drift checks. Database-family moves preflight the main, WAL, and shared-memory paths and roll back completed renames after a companion failure. Failed installation attempts restore the previous database when possible and preserve a failed candidate for investigation.

These are Rust library hooks, not IPC, shell, scheduling, retention, or production operator workflows. A future coordinator must define permission, audit, retention, disk-space, encryption, and update-preflight policy before exposing them.

`AuthorityStore::recovery_artifacts` classifies preserved pre-migration, pre-restore, and failed-restore files without opening or deleting them. `CorruptionCheck::Quick` supports readiness diagnostics; `Full` is required for backup/restore validation and explicit maintenance. Follow [recover and export local storage](../../operations/recover-local-storage.md).

`export_tenant_data` writes private, atomic `eitmad.local-data-export.v1` JSON from one tenant-scoped read transaction. Final publication creates the destination without replacement, so a path created concurrently remains unchanged and the export fails. It includes identity directory IDs and organization/workspace configuration but excludes devices, sessions, audit, idempotency, outbox, credentials, and secrets. `LocalDataExportPolicy` describes this fixed contract for inspection; callers cannot configure it. Export cannot restore the database.

## Security, Arabic data, and failure handling

Storage errors are sanitized as unavailable authority state; raw SQL, paths, customer values, relationship graphs, and backup contents do not enter routine logs. Backups have the same sensitivity and scope coverage as the live database. SQLite remains OS-permission protected, not encrypted, so production sensitive plaintext is still prohibited.

Storage preserves UTF-8 Arabic and mixed-direction values without localization branches. Permission and scope behavior is identical for Arabic and non-Arabic sessions. User-visible recovery UI does not exist; future shells must localize stable Rust errors and must not infer database state or bypass authorization.

## Tests and safe extension

Focused tests cover fresh creation through storage version 6, supported legacy upgrade with preserved Arabic locale data, audit-envelope persistence/completeness, out-of-window rejection before mutation, history gaps, migration rollback, schema drift, bounded pre-migration snapshots, quick/full integrity, WAL-safe backup/restore, recovery discovery, no-clobber scoped export, identity mapping conflicts, device timestamp monotonicity, session attribution, tenant isolation, transaction rollback, and prohibited shell database access.

Run `cargo test -p eitmad-storage`, strict workspace Clippy, all workspace tests, and the real engine diagnostic/start/stop path after storage changes. For symptoms, follow [storage recovery failures](../../troubleshooting/local-storage-recovery-failures.md). Review [ADR-0019](../../decisions/0019-sqlite-authority-storage.md), [ADR-0021](../../decisions/0021-checksummed-feature-storage-migrations.md), [ADR-0022](../../decisions/0022-persistent-tenant-identity-and-safe-storage-recovery.md), and [ADR-0023](../../decisions/0023-scoped-relationship-authorization-and-audit.md).
