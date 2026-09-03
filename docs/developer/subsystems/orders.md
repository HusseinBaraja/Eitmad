---
title: "Extend the order review flow safely"
description: "Understand the Arabic-first order list, read-only detail, status and date filters, tests, and Rust ownership boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Order capability maintainers"
last_verified: "2026-09-03"
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

# Extend the order review flow safely

The Windows **الطلبات** page gives a manager a synthetic order list and a read-only detail view. It is a review surface only: it has no order status action and no carpenter workflow.

## Ownership and current boundary

`shells/windows/Features/Orders/OrdersView.xaml` owns the native RTL list, filters, detail surface, focus target, and Arabic accessibility names. `OrdersViewModel.cs` owns synthetic rows, Arabic-normalized order-number and customer search, status and relative-date filters, and selected detail state. `OrderModels.cs` owns line totals, subtotal, discount, final total, status labels, and display formatting. `MainWindow.xaml` owns the **الطلبات** destination.

Rust does not yet provide an order capability. The preview has no order command, query, subscription, capability, authorization check, scope, audit record, durable storage, or synchronization. Do not add these responsibilities to WPF.

## Manager workflow

The list shows **رقم الطلب**, **العميل**, **التاريخ**, **الإجمالي**, **الحالة**, and **فتح**. Search matches the order number and customer after Arabic normalization. Status filters expose **جديد**, **قيد الإنتاج**, **جاهز**, **تم التسليم**, and **ملغي**. Date filters cover **اليوم**, **آخر 7 أيام**, and **آخر 30 يوماً**.

Opening an order shows its metadata and each product line with variant, dimensions, color, handle, quantity, and selling price. The read-only detail then shows subtotal, discount, and final total. `YER` values stay inside explicit LTR boundaries within the Arabic layout.

There is deliberately no **تغيير حالة الطلب** action and no **فتح سير عمل النجّار** action. Manufacturing progression belongs to the separate Work Orders capability.

## Failure and recovery

Use the detail back action to return to the list. Closing the shell discards the synthetic state because no order data is durable. If an order page displays an action that changes status or starts carpenter work, treat it as a boundary regression and inspect `OrdersView.xaml` and `OrdersViewModel.cs`.

A production order capability must return typed Rust-owned denial, validation, conflict, retry, and audit outcomes for every state-changing operation; the shell must not infer them from local state.

## Tests and verification

Run the focused shell checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~Orders"
```

`OrdersPresentationTests` covers Arabic search, status and date filter composition, all visible status labels, calculated totals, and list/detail transitions. `OrdersRenderedTests` creates the real WPF window at standard and compact sizes and checks the list, read-only detail, focus, required fields, scrolling, and absence of status or carpenter actions.

## Future Rust vertical

When orders become authoritative, define versioned typed order commands, queries, and subscriptions in Rust and generate or validate native bindings. Rust must own order lifecycle, relationships to customers and furniture, relationship-based manager authorization, explicit scope, atomic mutation and audit, durable storage, idempotency, synchronization, and typed recovery. Keep carpenter operations in the Work Orders vertical and preserve the read-only review boundary here.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.
