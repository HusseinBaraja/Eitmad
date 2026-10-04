---
title: "Extend server-confirmed Pricing safely"
description: "Use Rust pricing, exact money arithmetic, protected cost projections, durable publication retries, and the native Pricing screen."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Pricing capability maintainers"
last_verified: "2026-10-04"
review_triggers:
  - "Pricing contracts, confirmation policy, authorization, arithmetic, or Windows Pricing UI changes"
keywords:
  - "PricingService"
  - "DirectPriceClient"
  - "pricing-unconfirmed"
  - "التسعير"
  - "تعديل سعر البيع"
  - "هامش الربح"
  - "YER"
---

# Extend server-confirmed Pricing safely

The **التسعير** screen projects Rust-owned prices. Managers review internal costs and margins, stage a whole-rial price, and publish it only after an authenticated server receipt. Receptionists receive confirmed public fields only. The [accepted workflow rules](manager-receptionist-workflows.md#money) remain the authority for money, costing, selling prices, discounts, scope, and confirmation policy.

## Ownership and contracts

`crates/pricing` owns arithmetic, current catalog validation, price review, option selection, publication intents, and permission-aware projections. Ready-made [Products](products.md) retain their fixed supplier variants and purchase costs. Manufactured [Furniture](furniture.md) retains its Part compositions, compatible sizes, colors, and handles. `PriceTarget` references either model; it does not merge their definitions.

Rust contracts declare protocol 1.16, capabilities `eitmad.capability.pricing.v1` and `eitmad.capability.catalog-revisions.v1`, and schemas `eitmad.schema.pricing.v1` and `eitmad.schema.catalog-revisions.v1`. `PublishPrice` returns `PublishedPrice`; `Prices`, `PriceReview`, `SellingPrice`, and `DiscountTotal` queries return separate typed projections. `Prices` subscriptions carry public target/revision invalidations only. Native bindings are generated from these contracts.

`crates/storage/src/pricing.rs` owns SQLite migration 21, immutable confirmed revisions, and durable unresolved intents. `server/sync-plane/src/pricing.rs` owns authenticated server authorization, compare-and-set publication, immutable receipts, and mandatory audit. PostgreSQL migration 7 adds tenant-isolated price tables. Migration 8 adds immutable tenant-isolated catalog revisions in `server/sync-plane/src/catalog_revision.rs`. The server HTTP boundary exposes `/v1/pricing/publish`, `/v1/pricing/status`, and `/v1/pricing/read`; `DirectPriceClient` uses existing native credentials, device proof, TLS trust, and bounded requests.

`PricingClient.cs` sends generated contracts. `PricingViewModel.cs` owns temporary input, category filtering, and editor state. `PricingListItem.cs` formats returned values. WPF does not calculate cost, margin, effective selling price, or discount, and does not write authoritative data.

## Arithmetic and snapshots

Money uses checked signed 64-bit whole-YER integers. Fractional money, negative costs or adjustments, nonpositive published prices, invalid quantities, and overflow are rejected. Draft review permits zero. Product cost comes from the selected current supplier variant. Furniture cost comes from its saved immutable Part references; [Parts](parts.md) perform exact rational material costing and round the total once.

Rust returns absolute margin as selling price minus current cost. `SellingPrice` verifies the current catalog reference and price revision, checks compatible active options, adds color and handle adjustments, and multiplies by a positive integer quantity with checked arithmetic. Its immutable public snapshot contains no cost, margin, composition, or notes. Existing issued snapshots must remain unchanged when later prices change.

`DiscountTotal` sums valid line totals, uses basis points with 128-bit intermediate arithmetic, and rounds the subtotal discount once, half away from zero. It flags approval for rates above 500 basis points. For the accepted midpoint example, subtotal `1,010 YER` at `5.00%` produces discount `51 YER` and total `959 YER`. The calculator does not issue a quotation or grant discount approval; that workflow must consume the returned policy result in Rust.

## Authorization and durable publication

Every query checks organization scope and `eitmad.permission.catalog.read.v1`. Cost review additionally requires `eitmad.permission.pricing.cost.read.v1`. Publication requires both pricing write and cost read. These permissions require explicit organization Manager relationships; owner status alone does not imply them. The server independently checks its authenticated organization relationships, and denies Receptionist publication and intent-status access.

A Manager list includes active draft variants and authorized costs. A Receptionist list includes only prices that match the current active local catalog revision. Unauthorized `costYer` and `marginYer` fields are omitted during Rust serialization, rather than sent as hidden columns or null placeholders. Public server receipts and selections contain neither field. Policy changes close price subscriptions and clear shell projections and editor values before reload.

Publication validates the current catalog reference, expected price revision, positive price, and explicit below-cost confirmation. Rust saves the exact intent and a redacted audit record before network work. It asks for the original command status before retrying the same server idempotency key. A missing receipt allows the same proposal to be submitted; an unavailable status or unknown response retains the intent and cannot report success. A later denial also retains the intent because it does not prove that an earlier unknown request failed to commit.

Before a new publication, Rust transfers the exact Product or Furniture definition and its cost dependencies through `/v1/catalog-revisions/synchronize`. The engine reads these records from its own immutable storage; WPF cannot supply the graph. Transfers are dependency-ordered, use batches of eight records, and share the original publication deadline. The server accepts at most 32 records and 4 MiB per request, with a 512 KiB limit per revision. Only an authenticated organization Manager can write this authority. Each batch and its redacted mutation audits commit atomically; a failed batch retains no accepted prefix. Exact replay creates no duplicate mutation. Partial transfer across batches can remain durable and is safe to retry.

The server keeps Material unit and cost revisions, Part compositions, Product and Furniture categories, and Product/Furniture definitions. It validates scope, identities, positive revisions, bounded collections, and cost dependencies. Part row snapshots must match stored Material and unit revisions. The server recomputes their rational costs with the Part capability's arithmetic, then checks Furniture totals against stored immutable Part references. Product costs come from stored supplier variants. A revision cannot be replaced with different content; an authorized cost change requires a distinct audited catalog revision. A first transfer can bootstrap an existing positive local revision, and later dependency transfer can fill historical gaps without replacing newer records.

The server serializes catalog transfer and price publication under the same tenant lock. New publication resolves the current active definition and category, requires the exact catalog revision and variant, and derives permitted option adjustments from that definition. It enforces below-cost confirmation against the stored cost, then requires the proposal's `cost_yer` and option adjustments to match. Sending `cost_yer = 0` cannot bypass the check. Missing, archived, conflicting, or stale catalog state rejects publication. The server assigns the next immutable price revision and confirmation time, and commits the receipt and audit in one transaction.

The server receipt proves validation against this accepted catalog authority. Managers remain authorized to define purchase costs and material costs; the server does not verify supplier invoices or real-world cost accuracy. Changing those inputs is a separate immutable, audited catalog mutation, rather than an unchecked field on a price proposal. This transfer supports price validation; it does not provide general catalog browsing, automatic cross-device definition download, or background replication of every local catalog edit.

Rust checks the exact receipt and reauthorizes after network work. It commits the receipt, redacted mutation audit, idempotent response, intent resolution, and public event outbox atomically. If a cache refresh already imported that receipt, retry completes without a duplicate revision. An original confirmed receipt remains retrievable after a newer price exists. Audits record an explicit below-cost confirmation marker without storing cost or margin values.

Cost changes keep the previous selling price and immutable snapshots. A changed catalog revision marks the Manager row **بانتظار النشر** and hides it from Receptionist pricing until a new matching price is confirmed. Other engines load current server prices on the first price-list page. Variant pages scan bounded definition batches until the requested page is full or the catalog ends, including gaps from search and permissions. A continuation exists only when another matching variant is available and identifies the last returned variant.

## Configure and recover

Before enabling publication, register the engine organization and store an authenticated server session in Rust native secret storage. Set these engine-owned environment variables:

| Variable | Value |
| --- | --- |
| `EITMAD_PRICING_SERVER` | HTTPS server endpoint |
| `EITMAD_PRICING_TRUST_PEM` | Path to the approved TLS trust certificate |
| `EITMAD_PRICING_CREDENTIAL_ID` | Serialized native `SecretId` reference, never token contents |

The stored session must match the active Rust principal and tenant. The configured organization is the registered local organization. Missing or invalid companion configuration prevents startup; an absent endpoint permits confirmed-cache reads but cannot confirm a publication. Keep credentials and these settings outside WPF. See [direct server connection](server-authority.md) for authentication and trust mechanics.

**تعديل سعر البيع** opens the selected variant editor. Rust asynchronously returns the reviewed margin. A below-cost draft shows **تأكيد نشر سعر أقل من التكلفة** and needs explicit confirmation. Arabic-Indic digits are accepted; fractional published money is rejected. Values remain LTR inside the RTL layout. Saving keeps the editor pending until Rust returns a confirmed receipt. An unknown outcome freezes the proposal and allows exact retry; changing it cannot create another intent. Denial clears restricted data. Conflict keeps the confirmed list and permits explicit reload. Session change clears all cached internal fields.

When the server is unavailable, the screen labels the last confirmed cache. The first-page remote refresh has a two-second cap and ends at least one second before the IPC deadline to reserve time for the local response. Refresh does not invent a successful publication. An unexpected save response keeps the same proposal and retry key pending; a late failure from an ended session cannot restore its editor. A failed reload after a confirmed publication clears the unavailable projection without marking the confirmed command as pending. A catalog reference failure requires resolving the authoritative definition conflict before retry; do not overwrite an existing revision. Restart preserves unresolved intents and confirmed prices. Recover by restoring the authorized server connection and retrying the original proposal. Do not delete intents, reset durable data, or replace idempotency keys to bypass a conflict. Follow [local storage recovery](../../troubleshooting/local-storage-recovery-failures.md) for database failures.

## Tests and verification

Run the smallest focused checks after a pricing change:

```powershell
cargo test --locked -p eitmad-pricing
cargo test --locked -p eitmad-server pricing_http
cargo test --locked -p eitmad-engine-runtime pricing
npm run contracts:verify --prefix crates/contracts/codegen
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter "FullyQualifiedName~Pricing"
```

Rust tests cover forged Product costs, altered Part row and aggregate costs, Furniture cost recomputation, foreign dependencies, immutable revision conflicts, stale references, and approved arithmetic examples, Product/Furniture separation, option compatibility, overflow, durable audit and revisions, stale conflicts, below-cost confirmation, offline rejection, exact retry after refresh, and serialized Receptionist field omission. Dispatcher tests exercise direct backend review/publication denials and public notices. Subscription tests verify policy invalidation while catalog read remains allowed. Shell tests cover returned-margin projection, input, permission loss across pages, unknown retries, native editor focus, accessible warning controls, and Receptionist columns.

The real TLS/PostgreSQL test is `pricing_tls_confirmation_persists_retries_conflicts_and_denies_receptionists` in `crates/server-connection/tests/direct_route.rs`. It requires the same disposable database and trusted-certificate environment as the other direct-route tests. Run it with `cargo test -p eitmad-server-connection --test direct_route pricing_tls_confirmation -- --ignored`. It verifies missing-catalog rejection, forged-cost rejection, immutable catalog replay/conflict, catalog transfer denial, server restart, status receipt recovery, stale CAS, Receptionist denial, public field omission, and immutable history. The PostgreSQL test requires explicit execution; a normal workspace test run leaves it ignored. Do not claim this path passed when those prerequisites are absent.

Rendered Pricing tests request the three repository baseline sizes. On the verification host, Windows scaling was 125%; actual application sizes were approximately `1554 × 882`, `1338 × 753`, and `720 × 560` DIP. The display capped the largest window. Exact full-screen `1920 × 1080` at 100% remains to be verified on a suitable display. Existing fixtures are confined to tests. Review [native UI verification](windows-native-shell.md) before release.

Return to the [repository ownership map](../repository-layout.md) when adding an owner, or the [developer check guide](../index.md#choose-the-smallest-normal-proof) before extending verification.
