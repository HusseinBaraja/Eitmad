---
title: "Extend Rust-owned local storage safely"
description: "Understand SQLite setup, migration snapshots and windows, corruption checks, transactions, recovery, and export boundaries."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust storage maintainers"
last_verified: "2026-10-05"
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

Configuration, direct authorization, identity, local installation authority, synchronization state, and audit persistence remain focused modules inside the storage crate. Each module declares its migrations beside its repository. Shared migration history, connection policy, idempotency, the publication outbox, and recovery mechanics remain storage infrastructure. No raw `Connection` or `Transaction` crosses the crate boundary.

Persistent identity is a sibling vertical. Its tenant-rooted tables and public behavior are documented in [persistent identity](identity-foundation.md).

## Migration history and schema drift

The ordered registry assigns every migration a numeric order, stable ID, owning feature, SQL body, and SHA-256 checksum. `schema_migrations` persists those values. Existing numeric version 2–5 history is transactionally rebuilt and backfilled from the known registry only when its rows are the exact contiguous sequence `1..=N`.

Startup requires applied history to be an exact registry prefix. Storage version 25 accepts fresh version 0 and upgrades versions 2–24; version 1, gaps, reordered/unknown migrations, changed checksums, and databases newer than the engine are rejected before history modification. Before pending supported migrations, Rust creates a validated `eitmad.pre-migration-vN-to-v25.sqlite3` online backup. Migration `sync.scoped-state.v1` adds the mode-bound `sync_scopes` table after `audit.authorization-envelope.v2`. Migration `reference-marker.initial.v1` remains immutable schema history for the retired demo. Its rows are preserved; its API and runtime dispatch are removed. Retired publication events are excluded from active recovery without deleting them. Migration `identity.local-authority.v1` adds the singleton installation authority locator. Migration `identity.desktop-credentials.v1` adds tenant-scoped desktop accounts and Argon2 password verifiers. Migration `identity.desktop-account-management.v1` adds display names and optimistic account revisions to that existing store. Migration `customer.initial.v1` adds branch-scoped customer contact state, derived search indexes, and a bounded customer sync outbox. Migration `identity.local-desktop-branch.v1` binds the singleton local branch to the installation tenant. Migration `customer.sync-exceptions.v1` keeps rejected and conflicted customer markers separate from retained local contacts and outbox changes. Migration `material.definitions.v1` adds organization-scoped categories, units, and materials with stable foreign keys. Migration `part.compositions.v1` adds separate part categories, scoped parts, immutable composition revisions, and explicit material usage rows. Migration `product.definitions.v1` (17) adds ready-made definitions and immutable revisions. Migration `furniture.definitions.v1` (18) adds Furniture definitions and immutable Part references. Migration `catalog.images.v1` (19) adds immutable scoped image blobs and durable upload work. Migration `catalog.image-upload-deferral.v1` (20) adds an audited retry schedule without changing existing assets; see [catalog images](catalog-images.md). Migration `pricing.confirmed-prices.v1` (21) adds immutable confirmed prices and durable publication intents; see [Pricing](pricing.md) for atomic audit and retry behavior. Migration `catalog.synchronization.v1` (22) retains immutable transfer work, role-specific checkpoints, registered refresh clients, and complete confirmed sales projections; see [synchronization](synchronization.md#catalog-replication). Migration `catalog.sync-exceptions.v1` (23) retains terminal server dispositions separately from immutable catalog inputs and pending dependency work. Same-revision imports compare typed catalog values, so omitted defaults do not create false conflicts. An older engine cannot open this version-25 store; rollback requires a validated pre-migration backup and loses writes made after that backup. On first supervised IPC start, Rust atomically creates its tenant-rooted identity rows, organization scope, durable owner relationship, and corresponding append-only audit record in one transaction. Later starts verify the complete join before returning an authorization context. A retry for the same version pair validates and reuses its artifact. Snapshot failure prevents migration SQL. After migration, Rust builds the expected schema in memory from the same registry and compares tables, indexes, and triggers. Standalone diagnostics perform quick integrity, history, pending-migration, and schema-drift checks against an in-memory backup without mutating live state. Normal startup readiness reuses the already-open authority store and does not create a second in-memory database copy.

Never edit an applied migration. Add the next ordered migration to the owning feature, preserve upgrade behavior from supported history, and add rollback and drift tests.

## Transactions, synchronization, and permission-filtered queries

Repositories use crate-private deferred read transactions and immediate write transactions. The helper commits only when its closure succeeds; an error rolls back all feature state, audit, idempotency, and publication rows written within that boundary. Snapshot reads use one read transaction so related revision and page data cannot come from different database states.

`sync_scopes` stores opaque serialized `SyncEngine` state under exact `scope_kind`/`scope_id`/`schema_id`, immutable application mode, required `state_version`, and a compare-and-swap storage revision. `commit_sync_state` and `commit_sync_domain_state` require and atomically write the successful mutation audit; callers cannot commit state without one. The synchronization crate owns the versioned state schema and behavior, validates the row and payload version before decoding, and must add an explicit migration for a future version. Storage never interprets records or merges. Read [dual-mode synchronization](synchronization.md) before changing this repository.

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

Focused tests cover fresh creation through storage version 25, stable installation identity bootstrap, durable owner authorization, supported legacy upgrade with preserved Arabic locale data, account revision and last-Manager protection, session revocation, retired demo publication preservation, customer restart persistence, material, Part, and Furniture restart persistence, immutable cost snapshots, and stable references, normalized search, scope isolation, retry safety, audit rollback and revision conflict, sync scope/mode/version isolation, mandatory audit revision attribution, invalid-mode constraints, audit-envelope persistence/completeness, out-of-window rejection before mutation, history gaps, migration rollback, schema drift, bounded pre-migration snapshots, quick/full integrity, WAL-safe backup/restore, recovery discovery, no-clobber scoped export, identity mapping conflicts, device timestamp monotonicity, session attribution, tenant isolation, transaction rollback, and prohibited shell database access.

Run `cargo test -p eitmad-storage`, strict workspace Clippy, all workspace tests, and the real engine diagnostic/start/stop path after storage changes. For symptoms, follow [storage recovery failures](../../troubleshooting/local-storage-recovery-failures.md). Review [ADR-0019](../../decisions/0019-sqlite-authority-storage.md), [ADR-0021](../../decisions/0021-checksummed-feature-storage-migrations.md), [ADR-0022](../../decisions/0022-persistent-tenant-identity-and-safe-storage-recovery.md), and [ADR-0023](../../decisions/0023-scoped-relationship-authorization-and-audit.md).

Furniture definitions use migration `furniture.definitions.v1` at version `18`; see [durable Furniture definitions](furniture.md) for atomic save, immutable history, and scoped Part reference rules.

## Durable quotation storage

Migration 24, `quotation.drafts.v1`, stores branch-scoped evaluated quotation snapshots and their bounded local-first outbox. Customer intent, configured lines, public price snapshots, audit, idempotency response, and subscription publication commit together. See [durable quotation drafts](quotations.md#durable-quotation-drafts) for authority and conflict recovery.

Migration 25, `sync.domain-state.v1`, adds schema identity to the `sync_scopes` primary key. It copies every previous row into the empty-schema namespace before replacing the old table. Existing customer state is preserved. `SyncEngine::open_domain` isolates quotation checkpoints and replay state from customer state in the same authorized branch; `SyncEngine::open` continues to address the existing stream. Both stores retain compare-and-swap revisions and atomic audit.
