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

# Recover and export local storage

The Rust CLI provides manual `backup` and `restore` commands for an isolated synthetic installation. `ManualRecovery` holds the normal engine authority lock, authenticates local desktop credentials, checks `eitmad.permission.storage.recovery.v1` for every affected tenant root, and uses the storage backup and restore primitives. Organization Owners and Managers receive this permission; Receptionists do not. There is no recovery UI or IPC recovery command. Separate Windows administration tooling supplies [scheduled LAN backup and recovery](backup-lan-installation.md), including retention and off-device storage prerequisites.

## Safety prerequisites

- Stop the engine and confirm its runtime-directory authority is released.
- Preserve the full private runtime directory and available disk space.
- Treat live databases, backups, recovery artifacts, and exports as sensitive.
- Never copy an open SQLite file, delete WAL/SHM companions, edit migration rows, or use SQLite repair commands on the only copy.

## Before real operational use

Install and accept the [scheduled LAN recovery procedure](backup-lan-installation.md) for local SQLite, server PostgreSQL, Windows credentials, and external key dependencies. The procedure defines RPO/RTO, retention, protected off-device storage, and visible failures. These remain readiness requirements; the local hooks below alone do not provide complete installation recovery.

Prove the restore procedure in an isolated environment with synthetic data. Check recovered business records, audit history, and pending synchronization work, and verify that reconnecting does not repeat accepted business effects or silently lose pending work. Record the recovery time and result before using the workflow with real operational data.

Synchronization is not a backup: it can distribute an accidental deletion or an incorrect change. Retain recoverable history independently of the current synchronized state. Apply these requirements when changing storage through the [developer guide](../developer/index.md).

## Backup and migration protection

`AuthorityStore::backup_to` uses SQLite online backup, includes committed WAL data, writes an owner-private temporary file, runs full `PRAGMA integrity_check`, proves migration/schema compatibility on an in-memory copy, then atomically publishes the requested path. Existing destinations are rejected.

Before any pending migration from supported storage versions 2–30, startup creates and validates `eitmad.pre-migration-vN-to-v31.sqlite3`. A retry for the same source and target versions validates and reuses that one artifact. Failure to create or validate it stops startup before migration SQL. Each migration runs in its own transaction; SQL or history failure rolls back that migration and leaves the snapshot discoverable through `AuthorityStore::recovery_artifacts`. Check [the storage migration authority](../developer/subsystems/local-storage.md#migration-history-and-schema-drift) when the engine version changes.

## Manual SQLite backup and restore

Use the same engine build that created the synthetic installation. Stop its shell and engine first. Use explicit private paths; neither command selects the default user runtime directory. Both commands require an existing database at the current storage version and existing desktop credentials. They do not initialize or migrate an installation, or recover a lost/corrupt credential authority. Preserve such a failed runtime directory for a separate authorized recovery procedure.

For enrolled installations, both commands also require live authenticated server confirmation under [ADR-0029](../decisions/0029-require-live-server-authority-for-protected-access.md). A saved verifier does not grant disconnected recovery access. Missing settings, credentials, trust, or server connectivity reject the operation and close its local operator session before any backup or restore. Restore the server connection and use normal desktop online sign-in to repair account credentials before retrying. Only unconnected debug fixtures retain local maintenance authentication.

Supply one JSON object on standard input, with `username` and `password`, then close standard input. The maximum input is 4,096 bytes. Do not put a password in process arguments, shell history, environment variables, or a saved credential file. In PowerShell, collect it interactively:

```powershell
$operator = Get-Credential -Message 'Synthetic Eitmad Manager'
```

The following examples use operator-selected absolute paths in `$runtime`, `$backup`, and `$engine`:

```powershell
@{
    username = $operator.UserName
    password = $operator.GetNetworkCredential().Password
} | ConvertTo-Json -Compress | & $engine backup --runtime-directory $runtime --destination $backup
```

Inspect exit `0` and the JSON `status: succeeded` before continuing. The backup contains the complete SQLite database, including business records, historical revisions, audit rows, durable receipts, publications, checkpoints, and pending work. The backup audit is written to the live database after the backup completes. An existing destination, including one created during publication, is never replaced. The filesystem must support hard links for atomic publication without replacement.

```powershell
@{
    username = $operator.UserName
    password = $operator.GetNetworkCredential().Password
} | ConvertTo-Json -Compress | & $engine restore --runtime-directory $runtime --source $backup
```

Restore authenticates against current credentials and relationships, not the selected backup. It rejects another installation, changed tenant/scope topology, the live database as source, corrupt or incompatible candidates, unsafe database-family paths, and an active engine. Rust verifies identity again on its private staged copy and commits the restore audit there before installation. The original backup is unchanged. The previous database and its companions remain under `eitmad.pre-restore-<uuid>.sqlite3` in the runtime directory.

The staged restore transaction also preserves the operator's persisted session, its referenced identity-directory records, and its unchanged sign-in/sign-out audit evidence. It closes the maintenance session at restore time. This keeps account and user attribution available even when the operator was created after the backup. It does not copy newer desktop credentials, roles, or permission relationships; those remain at the selected backup point.

Backup attempts and failed or denied restore attempts close the maintenance session in the live database with a sign-out audit record. If the operation and session closure both fail, the original operation error is returned. A published backup remains available if the later audit or session closure fails. A live checkpoint or pre-restore move failure removes the staged candidate and its companion files before returning failure; the live database remains in place.

Both commands report only success with elapsed milliseconds or a stable sanitized error code. Exit `1` is failure. `eitmad.error.engine-already-running.v1` requires stopping the engine; `eitmad.error.authorization-denied.v1` requires a current authorized account; `eitmad.error.storage-recovery-unsafe-source.v1` requires a valid matching recovery point; `eitmad.error.storage-recovery-failed.v1` requires checking private permissions, space, and the filesystem. Do not retry by deleting files.

An account-read or session-write failure during sign-in reports `eitmad.error.storage-recovery-failed.v1`. Invalid credentials report `eitmad.error.authorization-denied.v1`. A storage failure does not mean the supplied credential was rejected.

After restore, run diagnostics, then a clean engine start and stop. Keep credentials, certificates, secret-store files, and encryption keys from the same installation: a SQLite backup does not back up OS credentials or files outside SQLite. SQLite backups are owner-private but not encrypted. Use only synthetic contents for this step.

Restore returns to the chosen backup point. Newer writes remain in the preserved previous database; they are not automatically merged or resent. Choose a backup with the required pending work and preserve all newer state before restore. Synchronization cannot restore server receipts that were never backed up, so stop all synthetic writers and take the server and both client recovery points together.

## Validate and restore

1. Call `AuthorityStore::validate_backup` while the engine is stopped. It performs full corruption, compatibility, checksum, pending-migration, and schema-drift checks without changing the candidate.
2. Acquire exclusive engine authority for the target runtime directory.
3. Call `AuthorityStore::restore_from_backup`. Rust privately stages and revalidates the candidate.
4. Rust checkpoints the stopped live database, preserves it as `eitmad.pre-restore-<uuid>.sqlite3`, installs the candidate, and reopens it through normal startup verification.
5. If installation or reopen fails, Rust attempts to reinstall the previous database and preserves the failed candidate as `eitmad.failed-restore-<uuid>.sqlite3` when possible.

After success, run diagnostics and a clean start/stop. Keep the pre-restore file until an approved retention policy confirms the restored state and all required newer writes are retained elsewhere.

## PostgreSQL procedure for the isolated drill

Use the installed loopback PostgreSQL 17 tools and the private test-cluster administrator credential. Stop the test server and both clients before taking the recovery point. The application role remains `NOSUPERUSER NOBYPASSRLS`; the backup operator must be able to read all rows despite forced row-level security. Pass its password only through the child process environment or an approved protected password file, never a connection URL in process arguments.

Run the complete database dump without schema or table filters. In these command templates, `$sourceDatabase` and `$newDatabase` are new synthetic database names and `$dump` is a new path in an owner-private directory:

```powershell
pg_dump --host 127.0.0.1 --port 55432 --username postgres --format=custom --file $dump --dbname $sourceDatabase
```

The archive includes control, sync, audit, administration, migrations, and all other database schemas. Roles and tablespaces are cluster state and are not in `pg_dump`; the drill reuses the existing synthetic `eitmad_test` role. Preserve separately provisioned signing keys, TLS credentials, and immutable manifest files when those features are used. See the [PostgreSQL pg_dump reference](https://www.postgresql.org/docs/17/app-pgdump.html).

```powershell
createdb --host 127.0.0.1 --port 55432 --username postgres --owner eitmad_test $newDatabase
```

`createdb` must succeed for a new name. Never select an existing user database, drop a database, or use `--clean` to prepare this test.

```powershell
pg_restore --host 127.0.0.1 --port 55432 --username postgres --exit-on-error --single-transaction --no-owner --role eitmad_test --dbname $newDatabase $dump
```

An error stops and rolls back this restore transaction. Preserve the failed new database and archive; the source database stays unchanged. Start the test server against the new database only after comparing all recovered table contents and checking ordinary-role authorization. See the [PostgreSQL pg_restore reference](https://www.postgresql.org/docs/17/app-pgrestore.html).

## Exercise and record the recovery path

Install the isolated loopback tools once with `tools/dev-database.ps1` if they are absent. Run from the repository root:

```powershell
./tools/run-manual-recovery-drill.ps1
```

The script creates a new source database, private TLS certificates, and an owner-private run directory under `tests/engine-integration/artifacts/`. It uses Rust 1.85.1 and runs:

```powershell
cargo test --locked -p eitmad-server-connection --test direct_route manual_two_client_recovery -- --ignored --nocapture
```

The drill uses existing synthetic domain fixtures and real TLS routes. It restores two SQLite client databases and a complete PostgreSQL archive into a new database. It supplies trusted Rust authorization contexts directly, without credential sign-in. It compares every SQLite table with the backup, permitting only the new restore audit, and compares all non-system PostgreSQL table row digests. The credential-based CLI regression tests separately check operator session evidence and closure, including an operator created after backup without restoring its credentials or permissions. The drill checks confirmed customers, catalog dependencies and sales data, quotations, orders, audit history, and pending customer, draft, catalog, and order work. An accepted order edit loses its local reply before backup; after restore, replay must return its original result without a second revision. Pending synchronization must reach the server and remain confirmed after another reconnect.

`result.json` records sanitized results and recovery milliseconds from the stopped server through backup, restore, restart, and reconnect verification. Both PostgreSQL databases and private backup artifacts remain available. No existing user database is reset or overwritten. Native credential replacement, encrypted off-device storage, complete device-loss recovery, scheduling, and retention remain outside this synthetic procedure.

### Recorded drill: 2026-10-11

The completed Windows drill used PostgreSQL 17.11 and `rustc 1.85.1`. The installed Clippy wrapper reports `0.1.96`; do not treat its result as a Clippy 1.85.1 run. The application role was checked separately and reported `superuser=false,bypassrls=false`.

| Actual command | Result |
| --- | --- |
| `./tools/run-manual-recovery-drill.ps1` | Passed; two client databases restored; PostgreSQL table rowsets identical before reconnect; source database preserved |
| `rustup run 1.85.1 cargo test --locked -p eitmad-storage -p eitmad-engine-runtime -p eitmad-engine-cli recovery -- --nocapture` | Passed: eight storage cases, three coordinator cases, and two CLI cases, including credential-based audit attribution and storage failure classification |
| `rustup run 1.85.1 cargo test --locked -p eitmad-authorization` | Passed: 13 tests |
| `rustup run 1.85.1 cargo test --locked -p eitmad-engine-cli --test process_modes authority_lock_blocks_concurrent_process_and_allows_replacement -- --nocapture` | Passed: manual backup and restore refused while the engine held authority |
| `rustup run 1.85.1 cargo test --locked -p eitmad-engine-cli --test manual_recovery -- --nocapture` | Passed: credential/role checks, foreign-source and overwrite refusal, preserved state, diagnostics, restored engine `ready`, and clean stop |
| `rustup run 1.85.1 cargo clippy --locked -p eitmad-storage -p eitmad-engine-runtime -p eitmad-engine-cli -p eitmad-authorization -p eitmad-server-connection --all-targets -- -D warnings` | Passed with the installed Clippy wrapper |
| `rustup run 1.85.1 cargo fmt -p eitmad-storage -p eitmad-engine-runtime -p eitmad-engine-cli -p eitmad-authorization -p eitmad-server-connection -- --check` | Passed |

The final drill recorded **3,779 ms** from the stopped server through PostgreSQL backup/restore, both local restores, restart, and reconnect checks. Customers, catalog, quotations, orders, prior audit rows, and pending work survived. The accepted order edit remained at revision `2`, with **zero duplicate order effects**. Pending customer, quotation-draft, and catalog changes became confirmed. Another reconnect left them confirmed and created no second order revision. The complete synthetic scenario took 18.03 seconds, excluding compilation.

## Corruption detection strategy

Readiness diagnostics use `CorruptionCheck::Quick` plus migration and schema verification. Backup creation and restore validation use `CorruptionCheck::Full`. Operators or future background maintenance may call `AuthorityStore::verify_integrity(Full)` when the engine is quiescent. Any non-`ok` result is fail-closed; do not keep writing or attempt destructive repair.

## Export one tenant

`AuthorityStore::export_tenant_data` writes format `eitmad.local-data-export.v1` from one read transaction. It includes tenant identity directory IDs and organization/workspace configuration. It excludes device records, sessions, audit, idempotency, publication outbox, credentials, and secrets. The destination must not exist; Rust writes privately, flushes, and atomically creates the final path without replacement. If another process creates that path first, export fails and preserves the existing file.

Export is not backup and cannot restore the authority database. A future caller must authorize the exact tenant, audit the request, select an approved destination, apply retention/encryption policy, and prevent cross-tenant disclosure.

## Escalate safely

Collect engine/storage versions, artifact kind, integrity outcome, migration ID/version, timestamp, and a synthetic reproduction. Do not attach database files, exports, paths, account/user IDs, authorization graphs, tokens, or customer records to ordinary tickets. Follow [storage startup troubleshooting](../troubleshooting/local-storage-recovery-failures.md) and [local storage ownership](../developer/subsystems/local-storage.md).
