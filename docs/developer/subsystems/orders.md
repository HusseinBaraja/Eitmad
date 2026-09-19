---
title: "Extend the order review flow safely"
description: "Understand the Arabic-first order list, read-only detail, status and date filters, tests, and Rust ownership boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Order capability maintainers"
last_verified: "2026-09-19"
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

The Windows **الطلبات** page gives managers and receptionists a synthetic order list and a read-only detail view. It includes navigation to the source quotation and manufacturing preview, without a direct order status action.

Production conversion, derived status, cancellation, numbering, visibility, permission, delivery, and offline behavior is accepted in the [Manager and Receptionist workflow specification](manager-receptionist-workflows.md). This page describes the current preview only.

## Ownership and current boundary

`shells/windows/Features/Orders/OrdersView.xaml` owns the native RTL list, filters, detail surface, focus target, and Arabic accessibility names. `OrdersViewModel.cs` owns synthetic rows, Arabic-normalized order-number and customer search, status and relative-date filters, and selected detail state. `OrderModels.cs` owns line totals, subtotal, discount, final total, status labels, and display formatting. `MainWindow.xaml` owns the **الطلبات** destination.

Rust does not yet provide an order capability. The preview has no order command, query, subscription, capability, authorization check, scope, audit record, durable storage, or synchronization. Do not add these responsibilities to WPF.

## Manager workflow

The list shows **رقم الطلب**, **العميل**, **التاريخ**, **الإجمالي**, **الحالة**, and **فتح**. Search matches the order number and customer after Arabic normalization. Status filters expose **جديد**, **قيد الإنتاج**, **جاهز**, **تم التسليم**, and **ملغي**. Date filters cover **اليوم**, **آخر 7 أيام**, and **آخر 30 يوماً**.

Opening an order shows its metadata and each product line with variant, dimensions, color, handle, quantity, and selling price. The read-only detail then shows subtotal, discount, and final total. `YER` values stay inside explicit LTR boundaries within the Arabic layout.

**عرض السعر الأصلي** opens the source quotation document and is disabled when no source exists. The **أوامر العمل** section shows the linked work-order number and status. **فتح أمر العمل** opens that detail and moves keyboard focus to its back action. When no work order exists for eligible furniture, **بدء أمر عمل تجريبي** creates a temporary fixture. Ready-made Products are excluded. Manufacturing progression remains on the Work Orders page. The composed main window shares synthetic order snapshots between both role views.

## Receptionist workflow

The receptionist home card and sidebar open a separate `OrdersView` configured with `ConfigureReceptionist`. This reuses the shared page header, filters, operations table, status badges, feedback notice, catalog illustrations, amount display, and print preview. Both configurations reuse the same source-quotation action.

Search also matches synthetic phone numbers with Arabic or Latin digits. All five status filters compose with date and search. The customer detail shows furniture and ready-made Products with images or synthetic catalog illustrations, variants, quantities, and selling prices. Only furniture shows dimensions, color, and handle. Subtotal, discount, and final total follow the items. Production costing, raw materials, and Parts are absent.

A Ready order shows **الطلب جاهز** above the actions and metadata. **طباعة** opens the shared native print preview using `OrderCustomerDocument`, an explicit customer-only projection. **عرض السعر الأصلي** opens the linked quotation fixture with its own number, date, items, and prices. The action is disabled if no original quotation is supplied. The preview does not provide a customer-contact action. Closing a document returns focus to its action; returning to the list preserves filters and focuses search.

The Home **طلبات جاهزة حديثاً** section lists unreviewed notices from work completed in this preview session. Opening a notice selects the exact order. The detail identifies the completed work order and offers **تمت مراجعة التنبيه**. Acknowledgement removes the notice but preserves Ready status. The Home ready count reads the current sample orders; repeat completion cannot add a duplicate notice.

The quotation conversion preview opens this customer detail, and the existing converted quotation opens the matching Ready order fixture. All records remain synthetic; these links are not a production relationship or authorization implementation.

## Failure and recovery

Use the detail back action to return to the list. Closing the shell discards the synthetic state because no order data is durable. Production status changes in the composed preview come from the linked work-order fixture. They are not evidence of manufacturing completion outside this session.

A production order capability must return typed Rust-owned denial, validation, conflict, retry, and audit outcomes for every state-changing operation; the shell must not infer them from local state.

## Tests and verification

Run the focused shell checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~Orders"
```

`OrdersPresentationTests` covers Arabic search, status and date filter composition, all visible status labels, calculated totals, and list/detail transitions. `OrdersRenderedTests` creates the real WPF window at standard and compact sizes and checks the list, read-only detail, focus, required fields, scrolling, and absence of direct order status actions. `PreviewHandoffRenderedTests` verifies order-to-work-order navigation, return focus, exclusion of ready-made Products, completion notices, and acknowledgement without losing Ready status.

`ReceptionOrdersRenderedTests` covers receptionist navigation, Arabic-digit phone search with status and date filters, Ready visibility, mixed item types, document opening and return focus, and a compact detail. It captures synthetic screens under the temporary `eitmad-reception-orders` directory. Physical printer output and OS high-contrast mode are not verified by these checks.

## Future Rust vertical

When orders become authoritative, implement the accepted workflow specification, define versioned typed order commands, queries, and subscriptions in Rust, and generate or validate native bindings. Rust must own order lifecycle, relationships to customers and furniture, relationship-based authorization, explicit scope, atomic mutation and audit, durable storage, idempotency, synchronization, and typed recovery. Keep production operations in the Work Orders vertical and preserve the read-only review boundary here.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.
