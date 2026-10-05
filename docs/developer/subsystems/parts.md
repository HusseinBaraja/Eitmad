---
title: "Maintain durable part compositions"
description: "Trace Rust-owned parts, category references, exact costing, immutable compositions, and the Arabic manager editor."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Parts capability maintainers"
last_verified: "2026-10-05"
review_triggers:
  - "Parts domain contracts, Rust projections, or Windows Parts UI behavior change"
keywords:
  - "PartsView"
  - "PartsViewModel"
  - "PartListItem"
  - "الأجزاء"
  - "إضافة جزء"
  - "معلومات الجزء"
  - "المواد الخام المستخدمة"
  - "مراجعة الجزء"
  - "حفظ الجزء"
  - "Wardrobe Side Panel"
  - "YER"
  - "مستخدم في"
---

# Maintain durable part compositions

The manager **الأجزاء** screen saves organization-scoped part definitions in Rust. A saved part can be reopened after engine restart. Each save creates an immutable composition revision. The [accepted composition rules](manager-receptionist-workflows.md#part-composition) define quantity precision, exact conversion, and cost rounding.

## Authority and storage

`crates/part` owns validation, costing, manager authorization, and composition revisions. Rust contracts live in `crates/contracts/src/part.rs`. `crates/storage/src/part.rs` owns migration `part.compositions.v1`, storage version `16`, and its immediate SQLite transactions. `crates/engine-runtime` dispatches requests and publishes committed notices. `Features/Parts` in the Windows shell keeps unsaved fields and renders typed Rust responses; it has no cost formula or local save path.

Parts and part categories have distinct stable UUIDs and explicit organization scopes. Part categories are separate from material categories. A part stores its category ID, description, archived state, and material usages with exact quantity text and Unit IDs. One part can contain 1–100 different materials. A quantity has at most six fractional digits and at most 32 characters. Duplicate material rows, non-positive quantities, exponent notation, excess precision, invalid names, incompatible units, and missing or foreign-scope references are rejected.

New references must be active. Existing unchanged references can remain after archive. Archive creates a revision and never deletes a part or its history. A unit used by a material or any saved part usage cannot change its dimension or conversion factor. Renames preserve identity.

## Current costs and historical references

The bounded part list returns `PartProjection`: `part` is the saved definition and cost snapshot, while `currentCost` is an advisory calculation against current material costs. A material change updates that projection on reload. It does not change the saved part revision, composition, or selling price.

A `CompositionReference` contains the organization scope, Part ID, revision, and schema version `1`. Resolving it returns the immutable saved part, quantities, material definitions and revisions, consumption and cost units and revisions, and original calculated cost. SQLite rejects updates or deletion of a composition row. Commercial records must retain this exact reference and their issued display snapshot; they must never substitute the current-cost projection. If a commercial workflow needs a new cost snapshot, the Manager saves a new part revision first. [Pricing](pricing.md) confirms Furniture publication separately; commercial issuance remains a separate capability.

The approved example is `1.2 m² × 7,250 YER` plus `3 m × 250 YER`, giving `9,450 YER`. Rust uses arbitrary-precision rational arithmetic and rounds the aggregate once, half away from zero. Individual row amounts are rounded for display and can sum differently from the exact rounded aggregate. The final amount must fit signed 64-bit whole-YER money.

## Typed IPC, permission, and recovery

Protocol `1.11` negotiates optional `eitmad.capability.part.v1` and `eitmad.schema.part.v1`, version `1`. Part and category save commands carry no ID or expected revision on create, and both on update. Part and category list queries use UUID cursors and pages of 1–100 records. Cost review is a bounded read query. The composition query resolves one exact reference. The compact part change stream carries scope, ID, record kind, revision, and timestamp without names or costs. Material change notices refresh the current-cost projection.

`eitmad.permission.part.read.v1` and `eitmad.permission.part.write.v1` require an organization Manager relationship. Receptionists cannot read internal part costs or mutate Parts. Rust checks each command, query, and subscription. Part and material reads use the same exact organization scope.

One immediate transaction checks the part revision, category, material and unit references, and reviewed material/unit revisions. It calculates the cost and writes the part, immutable composition, usage rows, redacted audit, retry result, and publication outbox together. A mandatory audit or storage failure rolls back all of these writes. Changed part or reviewed material revisions return `eitmad.error.part-revision-conflict.v1`. Exact retries use the original key and return the original result. Reusing a key for different input or another principal is rejected.

The engine publishes after commit and recovers pending publication through the existing outbox. Part categories and immutable compositions use the existing sync protocol through the [catalog cycle](synchronization.md#catalog-replication). Dependency ordering preserves exact Material and unit revisions. Private local save does not claim catalog publication. Receptionists cannot read Part payloads. Follow [local storage recovery](local-storage.md) for migration backups and rollback limits.

## Windows manager flow

Opening the screen loads Rust parts, separate categories, and durable material/unit references. Part and material-picker search query Rust's Arabic-normalized indexes; category and status filters apply to the returned scoped projection. **إضافة جزء** opens the existing three-step wizard. The information step selects a category or saves a new separate category. The materials step selects multiple materials and explicit units, preserving typed quantity text. **التالي** requests Rust cost review; the review step displays the calculated rows and total in **ر.ي**.

Save, edit, duplicate, and archive use typed Rust commands. Duplicate remains an unsaved editor until the Manager saves it. Validation, denial, reference, and revision failures keep unsaved fields and show Arabic errors. A lost save response keeps the reviewed data frozen for **إعادة محاولة الحفظ** with the original key. Category and part saves retain separate retry payloads and keys, so an unknown category outcome cannot block a part save. Retry a category with its original name to resolve an unknown outcome before changing that category request. Cost-reference refresh retains the selected saved unit when the active picker omits it; a matching refreshed unit supplies its current revision. Rust still decides whether the retained reference is permitted. Account switching clears part data and editor state. Loading, unavailable, denied, and empty results have explicit presentation states.

## Focused verification

Run `cargo test -p eitmad-part --offline` for multi-material restart, exact retry, approved costs, conversion and rounding, invalid quantities/references, historical cost protection, concurrent edits, and audit rollback. The focused dispatcher test `routes_material_and_part_mutations_and_denies_receptionist_write` covers typed saves, queries, notices, and Receptionist denial. Regenerate and verify contract bindings with the commands in the [developer guide](../index.md#choose-the-smallest-normal-proof).

Run `PartsPresentationTests` and `PartsRenderedTests` in `shells/windows/tests/Eitmad.WindowsShell.Tests.csproj`. The rendered suite drives the actual WPF wizard through multi-material save and reopen with fixed synthetic Rust responses, including accessible controls, unit popup, keyboard focus, and row actions. The Windows adapter's existing real-engine integration path also saves, retries, subscribes, and reopens a multi-material part after restart.

The 2026-09-30 rendered check used Windows display scaling of 125%, with actual windows of 1553.6×881.6, 1338.4×752.8, and 720×560 DIPs. The first is the nearest supported size to the requested 1920×1080 window on this display. Exact baseline rendering at 100% scaling and native high-contrast mode were not verified. Synthetic captures were inspected for the information, material, review, and list surfaces.

Return to the [Windows shell guide](windows-native-shell.md) for native layout and lifecycle rules.
