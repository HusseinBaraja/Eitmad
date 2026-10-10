---
title: "Extend confirmed Work Orders safely"
description: "Understand Furniture manufacturing snapshots, production transitions, readiness, scoped synchronization, native screens, and recovery checks."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Work Orders capability maintainers"
last_verified: "2026-10-10"
review_triggers:
  - "Work Order contracts, manufacturing lifecycle rules, or Windows Work Orders UI behavior change"
keywords:
  - "WorkOrdersView"
  - "WorkOrdersState"
  - "work_order_history"
  - "أوامر العمل"
  - "مخطط"
  - "قيد التنفيذ"
  - "الأجزاء المطلوبة"
---

# Extend confirmed Work Orders safely

The Manager **أوامر العمل** screen reads Rust-owned manufacturing work and sends server-confirmed transitions. Receptionists see authorized progress in the related Order. The [accepted workflow specification](manager-receptionist-workflows.md#order-work-order-and-delivery-lifecycles) defines the production rules.

## Ownership and snapshots

`crates/orders` owns transition validation, exact Furniture-line coverage, derived Order readiness, and the authenticated confirmation interface. `crates/contracts/src/work_order.rs` owns the manufacturing read projection. Protocol `1.23` registers `eitmad.capability.work-orders.v1`, `eitmad.schema.work-order.v1`, and `eitmad.work-order.list.v1`. Work start and completion use the existing typed Order commands and aggregate revision. Lists use UUID cursors with limits from 1 through 100.

`server/sync-plane/src/work_orders.rs` owns full manufacturing reads, snapshots, and its registered shared-sync handler. Conversion creates one initial Work Order containing all Furniture lines. Products never enter manufacturing. The snapshot retains accepted quantities, dimensions, color, handle, Furniture and variant references, and names. Parts come from the exact accepted Furniture revision and exact referenced Part revisions. Required Part quantities multiply usage by accepted Furniture quantity in Rust. Later transitions reuse the retained composition; current catalog edits cannot rewrite it. No selling price, cost, profit, or margin appears in the manufacturing projection.

Full snapshots are persisted in the Order transaction. Existing confirmed Order records can reconstruct their first manufacturing snapshot from immutable catalog revisions; missing historical authority fails the read or transition. It does not invent a specification.

## Transitions and readiness

A Manager starts `Planned` work with an assignment and due date, then completes `InProgress` work. Completed and Cancelled work cannot advance. There is no independent work cancellation command: cancellation of an undelivered Order cancels Planned and In Progress work and preserves Completed work. The retained accepted commercial document remains unchanged.

Rust requires each accepted Furniture line to belong to exactly one nonempty Work Order, with matching accepted specifications. Readiness considers all applicable work: all Completed means Ready; any In Progress means In Production; otherwise the Order remains Confirmed. Products-only Orders are Ready at conversion. The model supports multiple Work Orders without changing these rules, although version 1 conversion creates at most one.

## Scope, durability, and recovery

Every full read and transition checks current Manager relationships under tenant RLS. Managers can read branches in their organization. Receptionists cannot read the full manufacturing projection or advance production; their Order summary omits assignments and Parts. The sync handler authorizes reads, checks projected organization and branch, and rejects client writes.

PostgreSQL migration `0015_work_orders.sql` adds immutable manufacturing history with forced tenant RLS and Order/branch links. SQLite migration 28, `work-orders.confirmed-cache.v1`, adds immutable tenant/scope confirmation history. Earlier migrations are unchanged. SQLite cache, redacted audit, and durable publication commit together. Cached actions are disabled and scoped reads cannot cross a tenant or branch.

Order revision checks serialize competing production actions. Principal-bound idempotency receipts retain the exact confirmed result. Order state, manufacturing history, audit, receipt, and shared sync publications commit in one server transaction. Shared Order notices wake both role screens; reconnect replays durable changes and reloads current authorized state. A notification can be repeated without repeating the production mutation.

Before sending, Rust stores the exact pending Order intent and key. A lost reply remains retryable after restart. The shell retries that pair and never advances a local confirmed row. An exact receipt replay after a later transition does not rewind the latest stored revision. Definitive denial, invalid input, and competing revision rejection remain distinct from uncertainty. Sign-out clears protected presentation and fences late replies.

## Native workflow

The existing RTL list provides Arabic-normalized search, status/date filters, customer and Order links, assignment, due date, and multi-Furniture details. **مخطط**, **قيد التنفيذ**, **مكتمل**, and **ملغي** reflect Rust state. Planned work exposes assignment and date inputs before **تغيير حالة أمر العمل**. The related Order link opens the exact Manager Order; **فتح أمر العمل** returns to production.

Loading, empty, denied, offline, conflict, and pending-confirmation states use explicit Arabic messages. Retry controls appear for pending operations. Offline rows show the last confirmed state with disabled transitions. Synthetic preview state is available only without an attached engine client and remains clearly labeled.

## Focused verification

Use the [disposable PostgreSQL and TLS setup](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test), then run:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-orders
rustup run 1.85.1 cargo test --locked -p eitmad-storage work_orders
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route orders_cross_client -- --ignored
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --filter "FullyQualifiedName~WorkOrderAuthority|FullyQualifiedName~WorkOrders|FullyQualifiedName~OrderAuthority"
```

Domain checks cover partial completion, exact Furniture coverage, retained terms, and cancellation. Storage checks cover migration from version 27 with preserved Order/pending bytes, immutable history, scope isolation, restart, and publication rollback. The live scenario covers mixed Furniture/Product conversion, Manager progression and Receptionist reads/subscriptions on separate clients, competing revisions, exact receipt replay, cancellation, server restart, shared sync authorization, and forced tenant RLS. Native tests cover durable retry, denial, offline actions, session fencing, Arabic controls, focus, and date popup access. The Windows adapter test with `--engine target/debug/eitmad-engine-cli.exe` verifies Work Orders capability negotiation and Manager/Receptionist read boundaries through real IPC.

Rendered checks request 1920 × 1080, 1338 × 753, and 720 × 560 windows. The available physical display is 1920 × 1080 at 125% scaling; the maximized WPF window is 1550.4 × 830.4 DIP. The required 100% scaling baseline and OS high contrast remain unverified. Use synthetic Arabic data for future captures.

Return to the [Order guide](orders.md) for conversion and delivery, or the [Windows shell guide](windows-native-shell.md) for shared native layout rules.
