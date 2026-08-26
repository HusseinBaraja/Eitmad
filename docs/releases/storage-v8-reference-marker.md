---
title: "Upgrade local storage to version 8 reference marker state"
description: "Roll out the checksummed reference marker tables, local-first outbox, generated contracts, and compatible Windows shell."
audience: "support"
page_type: "release"
status: "active"
owner: "Rust storage, reference marker, contract, and release maintainers"
last_verified: "2026-08-26"
review_triggers:
  - "storage version 8, reference marker schema, compatibility, or rollback behavior changes"
keywords:
  - "storage version 8"
  - "reference-marker.initial.v1"
  - "reference_markers"
  - "eitmad.schema.reference-marker.v1"
---

# Upgrade local storage to version 8 reference marker state

Storage version `8` adds checksummed migration `reference-marker.initial.v1`, the scoped `reference_markers` table, and the bounded `reference_marker_sync_outbox`. The same release adds the generated reference marker command, paged query, compact subscription event, permissions, capability, schema, errors, C#/Swift bindings, and Arabic Windows view.

## Compatibility and migration

- Fresh storage version `0` creates version `8` directly.
- Supported versions `2` through `7` upgrade after a validated `eitmad.pre-migration-vN-to-v8.sqlite3` backup.
- Version `1`, incomplete or changed history, schema drift, and versions newer than `8` fail closed.
- Older engines cannot open version `8`. Do not use an in-place binary rollback after version `8` accepts writes.
- Peers advertise `eitmad.capability.reference-marker.v1` and optional `eitmad.schema.reference-marker.v1` version `1` before using the feature.

## Rollout and verification

1. Stop the old engine cleanly and preserve enough private disk space for a database-sized backup.
2. Deploy the engine with the generated schema, identifier catalog, C#/Swift bindings, and Windows adapter from the same build.
3. Start once. Confirm storage version `8`, a valid migration checksum, and a pre-migration artifact when upgrading existing storage.
4. Use only a synthetic organization scope to create `مرجع REF-١٢`. Confirm revision `1`, a succeeded audit row, a compact event, and one pending sync change.
5. Run workspace tests, strict Clippy, contract verification, engine diagnostics, the Windows scenario suites, a clean app start/stop, and the documentation audit.

## Rollback and recovery

If version `8` accepted no writes, stop the engine and restore the validated pre-migration artifact before deploying the older compatible engine. If it accepted marker or other writes, restoring the artifact loses those writes. Preserve evidence and plan an explicit compatible migration.

Never delete marker tables, clear outboxes, or edit migration rows by hand. Follow [recover local storage](../operations/recover-local-storage.md), [storage troubleshooting](../troubleshooting/local-storage-recovery-failures.md), and the [reference marker subsystem](../developer/subsystems/reference-marker.md).
