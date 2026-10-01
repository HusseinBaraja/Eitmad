---
title: "Extend the ready-made Products manager flow safely"
description: "Maintain Rust-owned ready-made Products, fixed supplier variants, purchase-cost access, and immutable history from the Windows UI."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Products capability maintainers"
last_verified: "2026-10-01"
review_triggers:
  - "Product contracts, category ownership, pricing rules, or Windows Products UI behavior change"
keywords:
  - "ProductsView"
  - "ProductsViewModel"
  - "ready-made product"
  - "المنتجات"
  - "إضافة منتج"
  - "هل لهذا المنتج مقاسات أو أنواع مختلفة؟"
---

# Maintain durable ready-made Products

The Windows **المنتجات** page stores Manager-created ready-made Product definitions in the Rust authority database. Products are purchased and sold as-is. They have no Furniture Parts, BOM, construction details, or manufacturing cost. See the [glossary](../../glossary.md) and [accepted sales workflow](manager-receptionist-workflows.md).

## Authority and scope

`crates/product/` owns validation, scoped authorization, archive selection, immutable history, and mutation audit orchestration. `crates/contracts/src/product.rs` owns the typed contracts. `crates/storage/src/product.rs` owns migration 17 and atomic persistence. The engine dispatcher composes these boundaries; WPF uses generated C# bindings through `ProductClient`.

Every record is organization-scoped. Rust checks authenticated identity and relationships on each operation. Managers receive `product.read`, `product.write`, and `product.cost.read` permissions. Receptionists receive definition read permission only. These names have the `eitmad.permission.` prefix and `.v1` suffix. An organization Owner must also have an explicit Manager relationship to edit Products, as with Parts and raw materials.

All current and historical read projections omit purchase-cost fields and internal notes without cost permission. Mutation results containing costs require that permission too. Change events contain only scope, record ID, revision, category flag, and time. Audit never contains names, costs, descriptions, or notes. The Windows projection uses Rust-supplied management and cost-access flags, clears internal editor values on loss of permission, and clears all account state on session end.

On an organization policy change, Rust reauthorizes the Products subscription. If definition read access remains, it closes the stream with `projectionInvalidated`; loss of read access closes it with `authorizationRevoked`. The Windows adapter maps both to a session-change failure. `ProductClient` clears cached records, editor costs, notes, and restricted retry payloads before it replaces the subscription and queries again. Pending refreshes and save completions cannot restore the old projection. A failed replacement query leaves the page unavailable with no cached costs. Ordinary transport recovery retains exact unknown-outcome retry keys.

## Definitions and history

Categories have separate stable IDs, unique Arabic-normalized names, revisions, and archive state. A Product has a name, category ID, category name snapshot, description, internal notes, active/archive state, and between 1 and 100 retained supplier variants. Costs are non-negative whole-YER `i64` values. A Product without optional supplier choices uses one **قياسي** variant.

Creating a category selects it in the open Product editor. Renaming or archiving another category preserves the selected category ID. Product list, category list, and revision queries use deferred SQLite snapshots so they can read committed definitions while a writer holds an immediate transaction. Saves retain immediate transactions for atomic validation and persistence.

Updates require the exact expected Product or category revision. A Product update creates an immutable revision containing its names, category snapshot, costs, and supplier options. Variant IDs remain stable across edits. Duplication creates new Product and variant IDs. Reusing another Product's variant ID is rejected. Removing a saved option archives it; archive never deletes history or silently reactivates an option. An active Product must retain at least one active variant.

Historical references contain scope, Product ID, variant ID, revision, and schema version. `product-revision.get` resolves that exact immutable revision. For new work, its `forNewWork` flag additionally requires the current revision, an active Product, active category, and active referenced variant. `product.list` with `selectableOnly` returns only active definitions and active variants. A retained archived category can remain on its existing Product, but cannot be assigned to a new Product. Existing quotation and order snapshots remain readable through their owning capability.

These are local durable definitions, not published catalog revisions. Catalog publication, selling-price policy, quotation issue, and server-confirmed catalog archive remain owned by the accepted sales workflow. The Products editor does not save selling prices or calculate margins. The Receptionist catalog does not offer unpriced Product definitions for new sales. Historical quotation previews resolve their stored snapshots independently of current definitions.

## Contracts, storage, and recovery

The threat boundary is the authenticated IPC request. Rust checks organization scope, relationships, bounded input, revision, and variant ownership; the shell cannot grant permission or make an option selectable. Cost redaction covers current and historical reads, paged shell projections, and permission loss. Redacted events and audits prevent indirect cost disclosure. Atomic audit failure and exact retry tests cover partial commits and duplicate changes; future new-work consumers must validate references in their commit transaction to close the selection-to-save race.

Protocol `1.12` advertises `eitmad.capability.product.v1` and `eitmad.schema.product.v1`. Commands are `product.save` and `product-category.save`. Queries are `product.list`, `product-category.list`, and `product-revision.get`. The discrete `product.changed` subscription is scope-bound and reauthorizes before event delivery. Operation identifiers have the `eitmad.` prefix and `.v1` suffix; subscriptions and events add `.subscribe` and `.event` before `.v1`.

List queries use UUID cursors and limits from 1 to 100. Search terms are bounded to 256 UTF-8 bytes; Rust normalizes Arabic only for matching. Names preserve supplied text and reject unsafe direction controls. Descriptions and internal notes are bounded to 4096 bytes. Native text and numbers remain in RTL layouts with LTR money input and display.

One immediate SQLite transaction writes current state, immutable Product history, stable option ownership, audit, exact retry response, and a compact publication outbox event. A mandatory write failure rolls back every write. Retry hashes bind actor, scope, operation, and input; a changed request cannot reuse a saved retry key. The runtime publishes committed events and recovers the outbox after restart. Multi-device reconciliation for these definitions is not implemented in this change; local durable storage and outbox events do not imply synchronization.

The shell uses asynchronous IPC, subscriptions, and cancellable refreshes. It retains the revision opened in the editor across refreshes. A conflict or validation error keeps the editor open. An unknown outcome keeps the request and retry key for an exact retry. Reconnect refreshes authoritative state. If data is unavailable, the list states that failure without claiming that records were deleted. Media attachment persistence is outside this definition contract; the transient image picker has been removed.

## Verify and extend

Run focused Rust behavior tests:

```powershell
cargo test -p eitmad-product
```

Verify generated contracts after regeneration:

```powershell
npm run contracts:verify --prefix crates/contracts/codegen
```

Build and run the Products shell checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter "FullyQualifiedName~ProductClientTests|FullyQualifiedName~ProductsPresentationTests|FullyQualifiedName~ProductsRenderedTests|FullyQualifiedName~SalesCatalogPresentationTests"
```

The real engine restart path is in `platform-adapters/windows/tests/Program.cs`; it verifies a Product save, exact retry, event delivery, restart, and retained supplier cost. Rendered tests use synthetic Arabic definitions at all three [baseline sizes](https://github.com/HusseinBaraja/Eitmad/blob/main/AGENTS.md#focused-ui-verification). Inspect their captures for native RTL layout, popup placement, cost values, archive confirmation, and editor focus.

Extend Product behavior through this Rust owner. Integrate new-work consumers through `forNewWork` reference validation at their commit boundary; a list result alone is not authorization or a guarantee that a later selection remains active. Keep selling-price publication in Pricing and follow the [authorization guide](authorization.md) for new permissions. Return to the [repository ownership map](../repository-layout.md).
