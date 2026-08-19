---
title: "Resolve local storage migration, corruption, and restore failures"
description: "Diagnose storage compatibility, pre-migration backup, corruption, schema drift, restore, and export failures without data loss."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust storage maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "storage error contracts, compatibility window, integrity, backup, restore, or export behavior changes"
keywords:
  - "authoritative storage is unavailable"
  - "schema drift"
  - "migration checksum"
  - "eitmad.pre-migration"
  - "فشل استعادة قاعدة البيانات"
---

# Resolve local storage migration, corruption, and restore failures

`authoritative storage is unavailable` is a sanitized Rust boundary error. Work must stop when readiness rejects storage; the live database is not proven safe for authoritative reads or writes.

## Symptoms

- engine startup fails during `componentStartup`, or diagnostics mark `eitmad.health.authority-store.v1` unhealthy;
- a pre-migration, pre-restore, or failed-restore artifact appears;
- backup validation, restore, full integrity check, or tenant export returns `StorageError`;
- the database reports a storage version outside 2–5, changed migration checksum/history, or schema drift.

No Arabic recovery UI exists. Likely future search text includes `فشل ترحيل قاعدة البيانات`, `تلف قاعدة البيانات`, and `فشل استعادة قاعدة البيانات`.

## Fast checks

1. Stop automatic restart and preserve the private runtime directory.
2. Run the read-only engine diagnostic against the exact runtime directory.
3. Record current engine version, storage version, health check ID, and whether a recovery artifact exists. Do not open the database in a GUI.
4. Use `AuthorityStore::recovery_artifacts` only to classify preserved files; validate a selected candidate separately.

## Causes and resolutions

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| Version `1` or below, except fresh `0` | Database is outside the supported migration window | Confirm the last engine that successfully owned it | Upgrade first through a release supporting that version; do not force history forward |
| Version above `5` | Engine downgrade or mismatched installation | Compare engine package and database provenance | Reinstall a compatible newer engine; do not downgrade the schema |
| `eitmad.pre-migration-*` exists, validates, and startup failed | Snapshot succeeded; migration or post-migration verification failed | Preserve the snapshot and failed live family | Escalate the failing migration; restore only through the stopped-engine coordinator after deciding which writes must survive |
| `eitmad.pre-migration-*` exists but fails validation | The reusable snapshot is corrupt, incompatible, or drifted | Preserve the complete runtime directory and validate an independent known backup | Stop retrying startup and escalate; do not delete or replace the only recovery artifact |
| No pre-migration artifact for a pending supported migration | Snapshot creation, privacy permission, disk space, or validation failed | Check free space and owner-only path access without exposing paths | Correct the environment and retry; no migration SQL should have run |
| Quick/full integrity result is not exactly `ok` | SQLite corruption | Validate an independent known backup | Restore a validated backup; never repair the only copy |
| Checksum/history/schema mismatch | Edited migration history or schema drift | Compare against the immutable Rust registry and synthetic reproduction | Repair code/release provenance; never edit production rows to match expectations |
| `eitmad.failed-restore-*` exists | Candidate installed but normal reopen failed | Confirm a pre-restore artifact and current live-file integrity | Keep all artifacts; use the validated pre-restore database only through the coordinator |
| Export destination exists or tenant is unknown | No-clobber or isolation policy denied export | Confirm the existing destination is unchanged, then confirm the exact authorized tenant and a new private path | Select a new destination; never broaden the export query |

## Verify recovery

Run full backup validation, diagnostics, a clean engine start to `ready`, a clean stop, and focused tenant identity/configuration reads. Confirm no cross-tenant IDs appear in a synthetic export. Repeated failure means stop and escalate; do not cycle restore candidates.

Safe escalation includes only stable versions, migration IDs, artifact kinds, health/error IDs, timestamps, and sanitized integrity outcomes. Exclude paths, raw SQLite errors, database/export contents, IDs, secrets, and customer data. See [recover and export local storage](../operations/recover-local-storage.md) and [persistent identity](../developer/subsystems/identity-foundation.md).
