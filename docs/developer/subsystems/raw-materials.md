---
title: "Maintain raw material definitions"
description: "Trace organization-scoped materials, categories, units, costs, Arabic search, audit, and Windows editing."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "material capability maintainers"
last_verified: "2026-10-02"
review_triggers:
  - "material contracts, unit rules, storage, permission, or Windows editor changes"
keywords:
  - "raw material definitions"
  - "المواد الخام"
  - "إدارة الوحدات"
  - "eitmad.material.save.v1"
---

# Maintain raw material definitions

The **المواد الخام** manager screen stores material definitions and current purchase costs in the Rust authority database. It does not track stock balances, reservations, or inventory movements. The accepted [Manager and Receptionist workflow](manager-receptionist-workflows.md#units) supplies the unit and money rules.

## Authority and scope

`crates/material` validates names, whole `YER` costs, unit factors, Arabic search terms, and manager permission. `crates/contracts/src/material.rs` defines versioned shapes. `crates/storage/src/material.rs` owns migration `material.definitions.v1` (storage version `15`) and the atomic repository. `crates/engine-runtime` dispatches commands and publishes change notices. `shells/windows/Features/RawMaterials` renders records and keeps unsaved editor fields; `MaterialClient` uses generated bindings through the Windows IPC bridge.

Every material, category, and unit has a stable UUID and an organization scope. A material stores category and unit IDs, not their names. Renaming or archiving either reference keeps existing material links valid. A new material or changed reference must select an active category and unit. A unit referenced by a material or any saved Part usage cannot change its dimension or exact conversion factor; its display name and symbol may change. There is no hard-delete command. Duplicate creates a new definition only when saved; it never reuses the source ID.

`eitmad.permission.material.read.v1` permits Managers to read names and internal costs. `eitmad.permission.material.write.v1` permits material and category saves. Unit creation and changes require the separate `eitmad.permission.material-unit.manage.v1` permission. Receptionists receive none of these permissions. The Windows unit selector opens **إدارة الوحدات** for administration; it cannot define a unit conversion directly on the material form.

## Contracts and validation

Protocol `1.10` advertises optional `eitmad.capability.material.v1` and `eitmad.schema.material.v1` version `1`. Commands `eitmad.material.save.v1`, `eitmad.material-category.save.v1`, and `eitmad.material-unit.save.v1` carry an optional ID and expected revision. Both are absent on create and present on update. Queries `eitmad.material.list.v1` and `eitmad.material-reference.list.v1` return scoped records; the material list uses a bounded page and UUID cursor. `eitmad.material.changed.subscribe.v1` emits compact ID, kind, scope, revision, and timestamp notices without names or costs.

Names reject empty values, surrounding whitespace, control and bidirectional formatting characters, and excess UTF-8 bytes. Category names are unique within one scope, including archived records; unit names follow the same rule. Unit factors are positive integer numerator and denominator values to one canonical unit of their declared dimension. The dimensions are count, length, area, volume, and mass. No cross-dimension conversion is allowed. `MaterialQuantity` accepts a positive invariant decimal with at most six fractional digits and rejects exponent notation and rounding. The material form stores no quantity. Current cost is a non-negative signed 64-bit count of whole Yemeni rials per selected unit. The shell rejects fractional input before sending the command; Rust validates the canonical integer. A cost change never changes a selling price.

Rust derives Arabic search text separately from the stored display name. It folds alef variants, alef maqsura, ta marbuta, tatweel, combining marks, Persian letter variants, digits, and whitespace. A material search checks its name and the current category and unit names. For example, **اخشاب** finds a material in **أخشاب طبيعية** without changing the stored Arabic.

## Atomic write and recovery

Rust checks manager permission and the exact organization scope before a mutation. One immediate SQLite transaction checks the current revision, active references, and duplicate category or unit name. It writes the definition, redacted mutation audit, idempotency result, and publication outbox together. A stale revision returns `eitmad.error.material-revision-conflict.v1` and preserves the latest record. If audit or another mandatory write fails, no definition or event work commits. Exact retries return the original result and UUID. The dispatcher publishes the compact notice after commit and startup recovery drains pending publication rows.

The screen reloads the scoped list and references on opening, search, and change notices. Rust owns search; the shell filters the returned rows by category and archive state. The shell starts empty and cannot save or archive without the engine. Synthetic records belong to test fixtures. An unavailable engine leaves an explicit Arabic error. An editor keeps unsaved values open after validation, reference, permission, or revision failure. Material definitions are currently local authority records. They are not queued through the customer sync path, and this screen makes no server confirmation claim. Cross-device material synchronization needs a separate scoped schema, server validation, and accepted conflict policy before it can be enabled.

## Verification and extension

Run `cargo test -p eitmad-material`, the focused dispatcher material route test, contract generation verification, the Windows shell build, and `RawMaterialsPresentationTests`, `MaterialClientTests`, and `RawMaterialsRenderedTests`. The rendered baseline is 1920×1080, 1338×753, and 720×560 at 100% display scaling. Use synthetic Arabic records and inspect the list, material editor, and unit dialog.

The [Parts capability](parts.md) now consumes stable Unit IDs with exact six-digit quantities and immutable cost snapshots. Do not add inventory movements or conversion calculations to WPF. See [local storage](local-storage.md), [authorization](authorization.md), and [the Windows native shell](windows-native-shell.md).
