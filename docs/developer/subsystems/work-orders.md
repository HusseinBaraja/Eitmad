---
title: "Extend the Work Orders review flow safely"
description: "Understand the Arabic-first manufacturing work-order list, multi-furniture detail, local status preview, tests, and Rust ownership boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Work Orders capability maintainers"
last_verified: "2026-09-03"
review_triggers:
  - "Work Order contracts, manufacturing lifecycle rules, or Windows Work Orders UI behavior change"
keywords:
  - "WorkOrdersView"
  - "WorkOrdersViewModel"
  - "WorkOrderListItem"
  - "أوامر العمل"
  - "ملاحظات الطلب"
  - "قيد التنفيذ"
  - "الأجزاء المطلوبة"
  - "WO-024"
---

# Extend the Work Orders review flow safely

The Windows **أوامر العمل** page gives a manager a synthetic manufacturing review list and a detail view for production staff. One Work Order can contain multiple Furniture items. The page does not show selling price, cost, profit, or margin.

## Ownership and current boundary

`shells/windows/Features/WorkOrders/WorkOrdersView.xaml` owns the native RTL list, filters, specification cards, detail surface, status action, focus target, and Arabic accessibility names. `WorkOrdersViewModel.cs` owns synthetic rows, Arabic-normalized search, status and due-date filters, selected detail state, and local feedback. `WorkOrderModels.cs` owns Furniture specifications, required Parts, status transitions, and display formatting. `MainWindow.xaml` owns the **أوامر العمل** destination.

Rust does not yet provide a Work Orders capability. The preview has no work-order command, query, subscription, capability, authorization check, scope, audit record, durable storage, or synchronization. Ready-made Products never enter this list.

## Manager workflow

The scan-first list shows **رقم أمر العمل**, **العميل / الطلب**, a Furniture summary, total **الكمية**, **مسند إلى**, **موعد التسليم**, **الحالة**, and **فتح**. Search matches the work-order number, related order, customer, Furniture, and assigned carpenter after Arabic normalization. Status filters expose **جديد**, **قيد التنفيذ**, **مكتمل**, and **ملغي**. Due-date filters cover **متأخر**, **اليوم**, and **خلال 7 أيام**.

Opening a row shows the related order, customer, carpenter, due date, and one illustrated specification card for every Furniture item. Each card includes fixed dimensions, color, handle, and item quantity. The detail then shows the required **الأجزاء المطلوبة** and prominent **ملاحظات الطلب**.

The one status action advances **جديد** → **قيد التنفيذ** → **مكتمل**. It reports **المعاينة المحلية فقط** and does not authorize, audit, persist, or synchronize a production change. Completed and cancelled rows cannot advance.

## Failure and recovery

Use the detail back action to return to the list. Closing the shell discards local status state. If cost or selling-price fields appear, or a ready-made Product enters the list, inspect the Work Orders view and model: those are ownership-boundary regressions. A production status failure must preserve the typed Rust denial, validation, conflict, retry, and audit outcome.

## Tests and verification

Run the focused shell checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~WorkOrders"
```

`WorkOrdersPresentationTests` covers Arabic search, status and due-date filter composition, multi-Furniture detail projection, required Parts, status progression, and terminal-state gating. `WorkOrdersRenderedTests` creates the real WPF window at standard and compact sizes and checks manufacturing content, Arabic accessible names, scrollable wide tables, detail focus, and the absence of costing fields.

## Future Rust vertical

When Work Orders become authoritative, define versioned typed commands, queries, and subscriptions in Rust and generate or validate native bindings. Rust must own the order-to-work-order relationship, Furniture and Parts projections, relationship-based manager and carpenter authorization, explicit record scope, atomic status mutation and audit, durable storage, idempotency, synchronization, and typed recovery. Preserve multi-Furniture cards, the separation from Products and Orders, and the Arabic mixed-direction layout in the shell adapter.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.
