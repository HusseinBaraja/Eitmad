---
title: "Extend Rust-owned local storage safely"
description: "Understand SQLite setup, feature migrations, transactions, schema drift, scoped queries, backup, and restore boundaries."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust storage maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "database setup, migration history, transaction, query, backup, restore, or schema verification changes"
keywords:
  - "eitmad-storage"
  - "schema_migrations"
  - "schema drift"
  - "backup restore"
  - "eitmad.sqlite3"
---

# Extend Rust-owned local storage safely

`eitmad-storage` is the only product-database access boundary. It opens the bundled `SQLite` database, applies and verifies migrations, provides scoped repositories and transaction boundaries, and exposes Rust-only recovery hooks. Native shells and platform adapters cannot open `eitmad.sqlite3` or reference a SQLite driver.

## Ownership and startup

The engine acquires authority for the runtime directory before `AuthorityStore::open` creates or opens `eitmad.sqlite3`. Every connection enables foreign keys, WAL mode, and a five-second busy timeout. The runtime directory and database receive owner-only OS permissions; failure is fatal before readiness.

Configuration and authorization persistence remain vertical modules inside the storage crate. Each module declares its migrations beside its repository. Shared migration history, connection policy, audit/idempotency tables, the publication outbox, and recovery mechanics remain storage infrastructure. No raw `Connection` or `Transaction` crosses the crate boundary.

## Migration history and schema drift

The ordered registry assigns every migration a numeric order, stable ID, owning feature, SQL body, and SHA-256 checksum. `schema_migrations` persists those values. Existing numeric version 1–4 history is transactionally rebuilt and backfilled from the known registry before later checks or migrations run.

Startup requires applied history to be an exact registry prefix. It rejects gaps, reordered or unknown migrations, changed checksums, and databases newer than the engine. After migration, Rust builds the expected schema in memory from the same registry and compares it with the authoritative schema, including tables, indexes, and triggers. Diagnostics perform integrity, history, pending-migration, and schema-drift checks against an in-memory backup; they never mutate the live database.

Never edit an applied migration. Add the next ordered migration to the owning feature, preserve upgrade behavior from supported history, and add rollback and drift tests.

## Transactions and permission-filtered queries

Repositories use crate-private deferred read transactions and immediate write transactions. The helper commits only when its closure succeeds; an error rolls back all feature state, audit, idempotency, and publication rows written within that boundary. Snapshot reads use one read transaction so related revision and page data cannot come from different database states.

The supported read path is authenticated IPC, Rust dispatcher, ReBAC authorization, exact-scope service, scope-filtered repository, then SQLite. Configuration and relationship SQL includes `scope_kind` and `scope_id`; authorization is denied by default before product data is returned. The cross-repository boundary test scans native shell and adapter source for the database filename and known SQLite drivers.

## Backup and stopped-engine restore hooks

`AuthorityStore::backup_to` uses SQLite's online backup API, so committed WAL state is included without copying an open file. The destination must not exist. Rust reapplies private permissions and validates integrity, migration compatibility, checksums, and schema before publishing the backup path.

`AuthorityStore::validate_backup` is read-only. `AuthorityStore::restore_from_backup` requires the caller to hold exclusive engine authority. It validates and stages the candidate first, checkpoints the stopped live database, preserves the previous database under a unique `eitmad.pre-restore-*.sqlite3` name, installs the candidate, and reopens it through normal migration and drift checks. Failed installation attempts restore the previous database when possible and preserve the failed candidate for investigation.

These are Rust library hooks, not IPC, shell, scheduling, retention, or production operator workflows. A future coordinator must define permission, audit, retention, disk-space, encryption, and update-preflight policy before exposing them.

## Security, Arabic data, and failure handling

Storage errors are sanitized as unavailable authority state; raw SQL, paths, customer values, relationship graphs, and backup contents do not enter routine logs. Backups have the same sensitivity and scope coverage as the live database. SQLite remains OS-permission protected, not encrypted, so production sensitive plaintext is still prohibited.

Storage preserves UTF-8 Arabic and mixed-direction values without localization branches. Permission and scope behavior is identical for Arabic and non-Arabic sessions. User-visible recovery UI does not exist; future shells must localize stable Rust errors and must not infer database state or bypass authorization.

## Tests and safe extension

Focused tests cover fresh creation, legacy history upgrade with preserved Arabic locale data, ordered history, migration rollback, incomplete or changed history, schema drift, transaction rollback, snapshot reads, atomic audit/idempotency/outbox writes, WAL-safe backup, valid restore, corrupt restore rejection, scope isolation, permission denial, and shell database-access prohibition.

Run `cargo test -p eitmad-storage`, strict workspace Clippy, all workspace tests, and the real engine diagnostic/start/stop path after storage changes. For startup symptoms, follow [engine authority recovery](../../troubleshooting/engine-startup-failures.md). Review [ADR-0019](../../decisions/0019-sqlite-authority-storage.md) for the SQLite choice and [ADR-0021](../../decisions/0021-checksummed-feature-storage-migrations.md) for migration and recovery policy.
