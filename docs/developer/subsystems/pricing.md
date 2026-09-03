---
title: "Extend the quick Pricing manager flow safely"
description: "Understand the Arabic-first Pricing list, selling-price editor, margin state, tests, and Rust ownership boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Pricing capability maintainers"
last_verified: "2026-09-03"
review_triggers:
  - "Pricing contracts, price validation, or Windows Pricing UI behavior change"
keywords:
  - "PricingView"
  - "PricingViewModel"
  - "PricingListItem"
  - "التسعير"
  - "تعديل سعر البيع"
  - "سعر البيع"
  - "هامش الربح"
  - "YER"
---

# Extend the quick Pricing manager flow safely

The Windows **التسعير** page lets a manager review synthetic product variants and change a selling-price preview. It is a focused pricing surface, not the Furniture editor: it does not show raw materials, parts, or construction details.

## Ownership and current boundary

`shells/windows/Features/Pricing/PricingView.xaml` owns the native RTL list, filters, modal editor, focus target, and Arabic accessibility names. `PricingViewModel.cs` owns transient fixtures, Arabic-normalized search, category filtering, editor state, numeric input normalization, and margin calculation. `PricingListItem.cs` owns presentation labels and the selling-price, margin, and status projections. `MainWindow.xaml` owns the **التسعير** destination.

Rust does not yet provide a Pricing capability. The preview has no price command, query, subscription, capability, authorization check, scope, audit record, durable storage, or synchronization. Do not add those responsibilities to WPF.

## Manager workflow

The list shows **المنتج**, **الخيار / المقاس**, **الفئة**, **تكلفة الشراء**, **سعر البيع**, **الهامش**, **الحالة**, and **تعديل سعر البيع**. Search matches product and variant after Arabic normalization, and the category filter composes with search. `YER` amounts use an explicit local LTR boundary inside the RTL layout.

**تعديل سعر البيع** opens a small editor with the product, fixed variant, cost, selling-price input, and calculated absolute **هامش الربح**. Arabic-Indic digits and separators are accepted. Invalid input keeps the editor open with **أدخل سعر بيع صالحاً يساوي صفراً أو أكثر.** A negative margin remains visible in the preview so the manager can identify an unprofitable price.

Saving updates only the selected in-memory fixture and reports **حُدث سعر البيع في المعاينة المحلية فقط.** Canceling closes the editor without changing the item. When a different variant is selected, the editor margin is recalculated even if its formatted selling price is unchanged from the previous edit.

## Failure and recovery

The preview has no durable save or rollback operation. Close the shell to discard local pricing changes. If the editor shows a margin from the previous variant after canceling and reopening, verify the selected item and rerun the focused Pricing tests; `BeginEdit` must refresh the margin for every selection.

Do not treat a local feedback message as an authorized or synchronized price change. A production failure must come from the Rust command result and preserve the typed denial, validation, conflict, or retry state.

## Tests and verification

Run the focused shell checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~Pricing"
```

`PricingPresentationTests` covers Arabic search normalization, category filtering, Arabic-numeral price input, validation, local save behavior, and the cancel-then-select regression for equal formatted selling prices. `PricingRenderedTests` creates the real WPF window at standard and compact sizes and checks list rendering, focus, validation, save, and cancel behavior.

## Future Rust vertical

When Pricing becomes authoritative, define its versioned typed commands, queries, and subscriptions in Rust and generate or validate the native bindings. Rust must own price and currency rules, relationship-based manager authorization, explicit record scope, atomic mutation and audit, durable storage, idempotency, synchronization, and typed failure recovery. Keep the WPF page a thin projection and preserve the Arabic labels, LTR amount isolation, keyboard path, and explicit local-preview state.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.
