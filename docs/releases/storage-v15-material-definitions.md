---
title: "Upgrade raw materials to storage version 15"
description: "Apply the material definition migration and check protocol 1.10, local records, and rollback limits."
audience: "operations"
page_type: "release"
status: "active"
owner: "material and storage maintainers"
last_verified: "2026-09-29"
review_triggers:
  - "material schema, protocol 1.10, or local migration behavior changes"
keywords:
  - "material.definitions.v1"
  - "storage version 15"
  - "raw materials"
---

# Upgrade raw materials to storage version 15

Storage version `15` adds organization-scoped materials, categories, and units. Protocol `1.10` adds typed material commands, queries, and a change stream. The migration does not transform preview fixtures into records; a Manager creates the first real references and materials after upgrade.

## Verify

1. Preserve the current SQLite file. Start the new engine and confirm it creates its validated `eitmad.pre-migration-vN-to-v15.sqlite3` backup before the migration.
2. Sign in as a Manager. Create a category and a unit with its dimension and rational factor, then create and edit a material with a whole `YER` cost. Restart the engine and confirm the IDs, values, and revision remain.
3. Archive a referenced category or unit. Confirm existing materials still show its name, while new materials cannot select it. Confirm a Receptionist cannot issue material mutations.

## Failure and rollback

An audit, reference, or revision failure leaves the existing record unchanged. Preserve the database and inspect the localized error; never delete reference rows to clear a failure. An older engine cannot open storage version `15`. If no writes occurred after migration, stop the engine and restore the validated pre-migration backup before using an older compatible build. If writes occurred, keep the current file and move forward with a compatible engine.

Material definitions currently persist on one local engine and are not synchronized between devices. See [material capability](../developer/subsystems/raw-materials.md) and [local storage](../developer/subsystems/local-storage.md).
