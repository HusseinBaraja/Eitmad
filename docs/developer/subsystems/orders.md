---
title: "Extend the order review flow safely"
description: "Understand the Arabic-first order list, server-confirmed conversion, fulfillment, delivery, recovery, tests, and Rust ownership boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Order capability maintainers"
last_verified: "2026-10-10"
review_triggers:
  - "Order contracts, lifecycle rules, or Windows order UI behavior change"
keywords:
  - "OrdersView"
  - "OrdersViewModel"
  - "OrderListItem"
  - "الطلبات"
  - "فتح الطلب"
  - "قيد الإنتاج"
  - "الإجمالي النهائي"
  - "YER"
---

# Extend the order workflow safely

The Manager and Receptionist **الطلبات** screens read the same Rust-owned, server-confirmed orders. Receptionists record quotation acceptance and convert an eligible accepted quotation. Managers can edit fulfillment notes, cancel an undelivered order, and progress its work. Receptionists record delivery of a Ready order. Source quotation navigation uses the retained accepted document.

The [accepted lifecycle and scope rules](manager-receptionist-workflows.md#order-work-order-and-delivery-lifecycles) remain the domain authority. Commercial content cannot be edited after conversion. Fulfillment-note editing is the only order edit permission.

## Ownership and contracts

`crates/orders` owns transition validation, derived status and action permissions. `crates/contracts/src/order.rs` owns typed requests and results. Protocol `1.22` registers `eitmad.capability.orders.v1` and `eitmad.schema.order.v1`. Commands accept intent, expected revision and the IPC idempotency key. Lists use UUID cursors with a limit from 1 through 100; details and subscriptions use the same scoped authorization.

`server/sync-plane/src/orders.rs` owns central transactions. `crates/server-connection` supplies authenticated TLS HTTP and the shared WebSocket subscription transport. `crates/engine-runtime/src/orders.rs` authorizes local commands and queries, stages uncertain requests, caches confirmed results and publishes durable notifications. WPF projects Rust results and permitted actions; it does not assign numbers, calculate commercial totals or advance order state.

## Conversion and fulfillment

Acceptance records the issued document revision, method, optional note, authenticated actor and server time. Conversion checks Accepted state, expected revision and validity on the server. The server reserves organization/year `OR` and optional `WO` numbers. One tenant transaction then serializes against quotation changes, copies the accepted snapshot, retains the order and Furniture-only work, and marks the quotation Converted. A Products-only order has no work and is Ready.

A unique `(tenant_id, draft_id)` constraint prevents a second order. Competing valid conversion requests return the winner. Principal-bound receipt hashes reject changed intent under a reused key. Exact retries return their retained result even after subsequent transitions. Official number counters commit in an independent short reservation before conversion, so rollback can leave gaps but cannot recycle a number. Order history, receipts, audit and shared sync publications commit together.

Managers start Planned work with assignment and due date, then complete In Progress work. Rust derives Confirmed, In Production and Ready. Cancellation requires a reason, cancels unfinished work and retains completed work. Delivered and Cancelled are terminal. Delivery requires a recipient and acceptance method, permits a note, and uses server time. A unique delivery constraint and revision check prevent another delivery. Fulfillment notes cannot change customer, items, quantities, prices, discount or acceptance.

## Scope, projection and durability

Every server read and transition checks current relationships under tenant RLS. Receptionists read assigned branches; Managers read their organization. Receptionist work projections omit assignments. The shared domain handler rejects local writes and filters history and snapshot pages before transport. Order reads contain public commercial data and readiness summaries, without catalog costs or Parts.

PostgreSQL migration `0014_orders.sql` adds immutable order, work, delivery and receipt history, with forced tenant RLS and transactional number counters. Earlier migration files remain unchanged. SQLite migration 27, `orders.confirmed-cache.v1`, retains exact tenant/scope confirmed history and principal-bound pending intent. Cached actions are empty and work assignments are removed. Offline Managers therefore see readiness summaries until the server supplies full work detail again. Cache history, redacted audit and publication commit atomically. Replaying an identical revision does not publish another event.

Before transport, Rust persists the exact request. A lost reply remains pending through restart. The shell retries the original command and key. Quotation conversion retains that pair per quotation until a definitive result or session cleanup; changing selection or refreshing revisions does not replace an uncertain request. After restart, use the Orders pending-operation retry. A definitive rejection is retained and shown separately from confirmed order state. Pending reads return at most 100 requests for the authenticated tenant, principal and scope. Unresolved requests come before rejected requests, and each group is ordered by request key. This keeps a full rejected-history page from hiding retryable intent. The shell shows **بانتظار تأكيد الخادم** for uncertain results and **غير متصل** for cached reads. It never presents an unconfirmed delivery or cancellation as success. The shared subscription reconnects and replays durable server events, then the shell reloads authorized results.

## Native workflow

Both role screens use the existing RTL list, Arabic search, date/status filters and detail. Metadata and the source quotation appear before state-changing actions. Managers see **حفظ ملاحظات التنفيذ**, **إلغاء الطلب**, and permitted work actions. Receptionists see **تسجيل التسليم** only when Rust returns delivery permission. An order confirmed from a quotation opens its exact detail. **فتح أمر العمل** opens the [confirmed manufacturing screen](work-orders.md); its related Order link returns to the Manager detail. Both screens use the same confirmed Order aggregate and shared change feed.

**عرض السعر الأصلي** opens the retained accepted quotation. Printing uses the customer-only document. Sign-out clears protected rows, selected detail and pending presentation, and fences late replies. Loading, denial, conflict and unavailability have explicit Arabic states. Preview fixtures remain available only when no engine client is attached.

## Saved customer documents

Protocol `1.24` adds `eitmad.order.customer-document.v1` and `eitmad.order.quotation-document.v1`, under `eitmad.capability.customer-documents.v1`. Both require `order.read` for the confirmed source order. Rust returns a customer-only projection of the retained accepted commercial snapshot. The order document uses its saved order number, revision, status and dates. **عرض السعر الأصلي** preserves the accepted quotation's own number, revision, validity, status and dates. Customer contacts, configured lines, dimensions, prices, discounts and totals come from that retained snapshot, even after catalog or customer edits.

Both roles can print an authorized confirmed order or its retained source quotation. Offline confirmed reads remain available; a server denial never falls back to cache. Drafts and unconfirmed orders cannot enter this path. The document contract excludes fulfillment notes, work assignments, delivery evidence, approval data and purchase costs. Session changes close previews and fence delayed reads. The native print dialog and virtual-printer output require fresh Rust authorization. See [saved quotation documents](quotations.md#saved-customer-documents) for the shared Arabic layout, pagination, print/export policy and rendering evidence.

`OrderAuthorityTests` checks Rust document replies, denial and late-session fencing. Runtime document tests verify confirmed cached order and source quotation totals, cross-scope denial and server-denial behavior. `CustomerDocumentsRenderedTests` inspects synthetic mixed Arabic/Latin contacts, configured Furniture dimensions and saved totals through the native preview and XPS output.

## Focused verification

Use the [disposable database and TLS setup](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test). Run the live scenario on a fresh database:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route orders_cross_client -- --ignored
```

Domain tests cover derived readiness, immutable commercial terms and invalid transitions. Storage tests cover migration from version 26, immutable history, exact retry, tenant/principal/scope isolation, restart and publication rollback. `OrderAuthorityTests` covers lost replies, durable pending projection, denied reads, missing work items and sign-out fencing. `QuotationConversionTests` checks exact retry across selection and revision refresh, definitive rejection and session cleanup. `OrderAuthorityRenderedTests` renders both roles at requested full-screen, default and minimum sizes and checks new input focus and delivery method popup access.

The local display is 1920 × 1080 at 125% scaling. The maximized window measured 1550.4 × 830.4 DIP; the default window measured 1338.4 × 752.8 DIP and the minimum measured 720 × 560 DIP. Rendered captures use the WPF DIP raster. The required 100% scaling baseline remains unverified. OS high contrast and physical printer output are not verified by these checks.

Return to [the Windows shell guide](windows-native-shell.md) for shared native layout rules.
