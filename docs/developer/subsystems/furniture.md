---
title: "Maintain durable Furniture definitions"
description: "Trace Rust-owned Furniture compositions, immutable revisions, and the existing native six-step editor."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Furniture capability maintainers"
last_verified: "2026-10-05"
review_triggers:
  - "Furniture contracts, Rust projections, pricing rules, or Windows Furniture UI behavior change"
keywords:
  - "FurnitureView"
  - "FurnitureViewModel"
  - "الأثاث"
  - "إضافة منتج"
  - "الأجزاء المستخدمة"
  - "المقاسات الثابتة"
  - "إضافة مقاس"
  - "الألوان المتاحة"
  - "المقابض المتاحة"
  - "إضافة لون"
  - "إضافة مقبض"
  - "سعر البيع"
  - "هامش الربح"
  - "حفظ كمسودة"
  - "حفظ التعريف"
  - "YER"
---

# Maintain durable Furniture definitions

The manager **الأثاث** list and its six-step editor save organization-scoped Furniture definitions through Rust. A saved definition can be reopened after engine restart. Each save preserves an immutable definition revision and exact Part composition references for future quotation and production snapshots. The [accepted Furniture and sales specification](manager-receptionist-workflows.md#catalog-editing-and-pricing) remains the authority for commercial publication.

## Authority and definition lifecycle

`crates/furniture` owns validation, costs, selections, permissions, and revision behavior. Contracts live in `crates/contracts/src/furniture.rs`; `crates/storage/src/furniture.rs` owns migration `furniture.definitions.v1`, storage version `18`. The engine dispatcher supplies typed IPC and committed change notices. `shells/windows/Features/Furniture` stages unsaved fields and renders Rust results.

Definitions use stable scoped UUIDs, separate Furniture category IDs, descriptions, internal notes, Parts, fixed variants, colors, handles, and `Draft`, `Active`, or `Archived` state. Active means a complete private manager definition. It does not mean a published catalog entry or a confirmed selling price. **حفظ كمسودة** and **حفظ التعريف** save local definitions; archive removes a definition from new definition selection. [Pricing](pricing.md) confirms catalog publication and published-price changes. Server acceptance of a changed or archived private definition withdraws its previous sales projection; a new active revision needs a matching publication. Quotation issuance remains separate. The Receptionist catalog receives an explicit public sales projection, never these private definitions. Its remaining preview tests use explicit synthetic projections.

The shell keeps Rust records for list loading and creates editable copies only when an editor opens. List rows are immutable display projections.

One optional [durable catalog image](catalog-images.md) is retained per revision. The information step imports through Rust and stages replacement or removal until save. Local paths are never stored as references. Asset transfer remains separate. The [catalog cycle](synchronization.md#catalog-replication) transfers the reference with the immutable definition, after its exact Part dependencies.

## Composition, dimensions, and option rules

A definition requires 1–100 distinct Parts. Each usage has a positive whole count of at most 1,000,000 and an exact organization-scoped `CompositionReference` with schema version `1`. Rust validates current active references for new relationships. An existing unchanged reference can remain when its Part is archived or advances to a new revision. Save never silently substitutes the current composition. The [Parts capability](parts.md#current-costs-and-historical-references) owns material and unit snapshots behind that reference.

Rust computes definition cost as the checked integer sum of each immutable Part cost times its whole count. Fixed sizes share that specified composition cost; there is no invented volume multiplier. Cost does not change a proposed selling price. Rust review supplies row totals and margins to the editor. Later material changes do not rewrite the Furniture revision or its saved cost.

A definition has 1–100 fixed variants. Width, height, and depth are integer millimetres in `1..=100000`. The native editor accepts centimetres with at most one decimal place, displays **سم**, and converts without rounding. Each variant can retain optional minimum and maximum dimensions. The fixed size must be within every bound. Without customization bounds, a prospective selection must exactly match the fixed size.

Colors and handles each have at most 100 stable options, names, supported visual values, non-negative whole-YER adjustments, and active/archive state. A variant can name compatible color and handle IDs; an empty compatibility list allows all options of that kind. Missing, duplicated, foreign, incompatible, or unavailable option references are rejected. Removed options and variants are retained as archived, and archived option IDs cannot be reactivated or moved to another definition or option kind. Color visuals are six-digit RGB hex values; handles use the existing native `Standard`, `BlackMetal`, and `Brass` illustrations.

Draft proposed prices can be zero. Active definitions require a positive proposed price for each active variant and a checked effective price after compatible option adjustments. A below-cost proposal requires explicit confirmation, recorded as redacted audit metadata. This is internal definition review; it does not publish a commercial price. Selection review checks the current active definition, category, variant, permitted dimensions, compatible active options, whole item quantity, and checked price and total. Future commercial consumers must revalidate these references in their own transaction and retain their issued display snapshots.

## Atomic save, permissions, and failures

Protocol `1.13` negotiates optional `eitmad.capability.furniture.v1` and `eitmad.schema.furniture.v1`. Commands save a Furniture definition or category. Bounded UUID-cursor queries list definitions and categories, review staged fields, resolve an immutable revision, or check a prospective selection. `furniture.changed` carries scope, record ID, kind, revision, and time without names, notes, or costs.

`eitmad.permission.furniture.read.v1` and `eitmad.permission.furniture.write.v1` require an organization Manager relationship. Receptionists cannot read internal definitions or write them. Every command, query, and subscription is checked in Rust. The native shell cannot create authority through a control state.

One immediate transaction checks the expected definition revision, category, Part revisions, option identities, and bounds. It writes current state, immutable history, scoped Part relationships, option identities, redacted audit, exact retry result, and the publication outbox together. Any mandatory write failure rolls back all state. SQLite prevents update or deletion of historical definitions.

Updates carry the ID and expected revision together; creates carry neither. A stale edit returns `eitmad.error.furniture-revision-conflict.v1`. The editor retains its opened revision across list refreshes. Validation and conflict results preserve unsaved fields. An unknown save outcome freezes the exact request until retry with the original key resolves it. Furniture and category retries remain separate. Account changes and authorization revocation clear retained definitions and editor fields. Furniture search waits for 250 ms without further input before querying only the definition list; matching runs in Rust. Change notices coalesce Furniture list refreshes. Part picker data loads when an editor opens and refreshes after Part changes. Immutable compositions are cached by Part ID and revision within the session and resolved only for the opened definition. Session changes and authorization revocation discard those caches. Loading, unavailable, denial, failure, and empty results do not invent durable success.

## Existing native editor

The existing **المعلومات**, **الأجزاء**, **المقاسات**, **الخيارات**, **التسعير**, and **المراجعة** steps remain. The information step can save a separate Furniture category. A typed category name must be saved or replaced with an existing category before the Manager continues or saves the definition. The variant dialog uses native controls for permitted bounds and named compatible options. The options step retains color rows and handle tiles. Pricing renders Rust costs and margins, using **ر.ي** while contracts retain whole-YER integers. The final step offers a draft save or a complete definition save. Duplicate opens an unsaved definition with new Furniture option and variant identities; it does not copy history or insert a list row before confirmation.

## Focused verification and extension

Run `cargo test -p eitmad-furniture --offline` for restart, retry identity, historical protection, stale edits, scopes, quantities, dimensions, options, money, Manager permission, and audit rollback. The storage recovery test also includes the new migration. Run affected-crate formatting and strict Clippy as described in the [developer guide](../index.md#choose-the-smallest-normal-proof).

Regenerate and verify contracts using the developer guide. Run `FurniturePresentationTests` and `FurnitureRenderedTests` in `shells/windows/tests/Eitmad.WindowsShell.Tests.csproj`. The complete rendered workflow uses fixed synthetic Rust responses, saves and reopens all definition fields, checks keyboard focus, and preserves conflict and unknown-outcome input. The existing Windows adapter real-engine path saves, retries, receives committed events, rejects invalid and stale requests, denies a Receptionist, and reopens the definition after engine restart.

Rendered checks used 125% Windows display scaling: a maximum window of 1553.6 × 881.6 DIPs, a default window of 1338.4 × 752.8 DIPs, and the minimum 720 × 560 DIPs. Geometry is recorded with the synthetic captures. The workflow also renders with high-contrast resource overrides; this does not verify native OS high contrast or text scaling. Exact 100% scaling requires the corresponding Windows display environment. Before adding commercial behavior, implement server-confirmed publication and Pricing through their owning capabilities; do not make this manager projection a sales catalog.

Return to the [Windows shell guide](windows-native-shell.md) for native layout and lifecycle rules, or the [local storage guide](local-storage.md) for recovery.
