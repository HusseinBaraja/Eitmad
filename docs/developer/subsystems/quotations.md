---
title: "Extend the quotation review flow safely"
description: "Understand the Arabic-first quotation list, read-only detail, discount approval preview, tests, and Rust ownership boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Quotation capability maintainers"
last_verified: "2026-09-03"
review_triggers:
  - "Quotation contracts, approval rules, or Windows quotation UI behavior change"
keywords:
  - "QuotationsView"
  - "QuotationsViewModel"
  - "QuotationListItem"
  - "عروض الأسعار"
  - "موافقة الخصم مطلوبة"
  - "موافقة"
  - "رفض"
  - "YER"
---

# Extend the quotation review flow safely

The Windows **عروض الأسعار** page gives a manager a synthetic quotation review list and a read-only detail view. It supports a temporary discount decision for fixtures that require review; it does not create or authorize a quotation.

## Ownership and current boundary

`shells/windows/Features/Quotations/QuotationsView.xaml` owns the native RTL list, filters, detail surface, conditional approval actions, focus target, and Arabic accessibility names. `QuotationsViewModel.cs` owns synthetic rows, Arabic-normalized search, status and relative-date filters, selected detail state, and approval routing. `QuotationModels.cs` owns line totals, quotation totals, status labels, discount percentages, and local approval state. `MainWindow.xaml` owns the **عروض الأسعار** destination.

Rust does not yet provide a quotation capability. No quotation command, query, subscription, capability, authorization check, scope, audit record, durable storage, or synchronization exists in this preview. Keep the shell read-only apart from clearly local fixture decisions.

## Manager workflow

The list shows **رقم عرض السعر**, **العميل**, **التاريخ**, **الإجمالي**, **الخصم**, **الحالة**, and **فتح**. Search matches the quotation number and customer after Arabic normalization. Status filters group **مسودة**, **نشط**, **محوّل**, and **ملغي / منتهي**; date filters cover **اليوم**, **آخر 7 أيام**, and **آخر 30 يوماً**.

Opening a row shows quotation metadata, furniture lines with variant, color, handle, quantity, unit price, and total, followed by subtotal, discount, and final total. Amounts use `YER` with local LTR isolation. The detail is read-only.

For a fixture marked **موافقة الخصم مطلوبة**, the detail shows **موافقة** and **رفض**. These actions update only the local `DiscountApprovalDecision` preview and are hidden for quotations without a pending approval. They do not change quotation status or claim manager authorization.

## Failure and recovery

Closing the detail returns to the list. Closing the shell discards all local quotation state. Do not add a shell-side approval rule or infer authorization from the presence of an approval button. A production approval flow must receive a typed Rust outcome for denial, validation, conflict, retry, and audit status.

## Tests and verification

Run the focused shell checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~Quotations"
```

`QuotationsPresentationTests` covers Arabic search, status and date filter composition, calculated subtotal and final totals, approval gating, and local approve or reject state. `QuotationsRenderedTests` creates the real WPF window at standard and compact sizes and checks list-to-detail focus, visible detail totals, conditional actions, and accessible names.

## Future Rust vertical

When quotations become authoritative, define versioned typed quotation commands, queries, and subscriptions in Rust and generate or validate native bindings. Rust must own quotation lifecycle, discount policy, relationship-based manager authorization, explicit scope, atomic approval and audit, durable storage, idempotency, synchronization, and typed recovery. Preserve the read-only detail shape and Arabic mixed-direction amount handling while keeping WPF as a thin adapter.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.
