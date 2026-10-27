---
title: "Resolve local storage migration, corruption, and restore failures"
description: "Diagnose storage compatibility, corruption, schema drift, and export failures without data loss."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust storage maintainers"
last_verified: "2026-10-24"
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
- full integrity check or tenant export returns `StorageError`;
- the database reports an unsupported storage version, changed migration checksum/history, or schema drift. The current engine opens version `31`, upgrades versions `2–30`, and initializes fresh version `0`; version `1` is unsupported.

No Arabic recovery UI exists. Likely future search text includes `فشل ترحيل قاعدة البيانات`, `تلف قاعدة البيانات`, and `فشل استعادة قاعدة البيانات`.

## Fast checks

1. Stop automatic restart and preserve the private runtime directory.
2. Run the read-only engine diagnostic against the exact runtime directory.
3. Record current engine version, storage version, health check ID, and the diagnostic outcome. Do not open the database in a GUI.
4. Preserve any historical recovery files. Current source has no backup or restore command; see [backup availability](../operations/backup-lan-installation.md).

## Causes and resolutions

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| Version `1` or below, except fresh `0` | Database is outside the supported migration window | Confirm the last engine that successfully owned it | Upgrade first through a release supporting that version; do not force history forward |
| Version above `31` | Engine downgrade or mismatched installation | Compare engine package and database provenance against `CURRENT_STORAGE_VERSION` in `crates/storage/src/lib.rs` | Reinstall a compatible newer engine; do not downgrade the schema |
| Quick/full integrity result is not exactly `ok` | SQLite corruption | Preserve the complete runtime directory | Stop writes and escalate; never repair the only copy |
| Checksum/history/schema mismatch | Edited migration history or schema drift | Compare against the immutable Rust registry and synthetic reproduction | Repair code/release provenance; never edit production rows to match expectations |
| Export destination exists or tenant is unknown | No-clobber or isolation policy denied export | Confirm the existing destination is unchanged, then confirm the exact authorized tenant and a new private path | Select a new destination; never broaden the export query |

## Verify recovery

After the cause is corrected, run diagnostics, a clean engine start to `ready`, a clean stop, and focused tenant identity/configuration reads. Confirm no cross-tenant IDs appear in a synthetic export. Repeated failure means stop and escalate; do not delete or replace retained state.

Safe escalation includes only stable versions, migration IDs, artifact kinds, health/error IDs, timestamps, and sanitized integrity outcomes. Exclude paths, raw SQLite errors, database/export contents, IDs, secrets, and customer data. See [recover and export local storage](../operations/recover-local-storage.md) and [persistent identity](../developer/subsystems/identity-foundation.md).
