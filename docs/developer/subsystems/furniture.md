---
title: "Extend the Furniture manager flow safely"
description: "Understand the Arabic-first Furniture list and its transient six-step manager editor."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Windows UI maintainers"
last_verified: "2026-09-02"
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
  - "حفظ ونشر"
  - "YER"
---

# Extend the Furniture manager flow safely

The Windows **الأثاث** page gives managers a compact product table and a complete six-step furniture editor preview. The page is local presentation state. Search, filters, image selection, part quantities, calculated values, variants, colors, handles, prices, margins, draft or published status, duplicate, and archive state are discarded when the shell closes.

## Ownership and current boundary

`shells/windows/Features/Furniture/FurnitureView.xaml` owns the native RTL layout, small vector thumbnails, upload preview surface, custom selectors, tables, option rows and tiles, overlays, keyboard focus targets, and Arabic accessibility names. `FurnitureViewModel.cs` owns transient fixtures, list filtering, the current editor step, selected parts, preview totals, fixed-variant cards, colors, handles, and their visible price adjustments. `FurnitureModels.cs` owns presentation-only row labels, option state, and calculated display values. `MainWindow.xaml` owns the **الأثاث** sidebar destination.

Rust does not yet provide a Furniture capability. The preview has no command, query, subscription, capability, authorization check, scope, audit record, persistence, sync, or authoritative pricing rule. Do not add these responsibilities to the WPF shell. A future Furniture vertical must own the product lifecycle, part relationships, dimension validation, costs, prices, authorization, audit, storage, and synchronization.

## Manager list behavior

The table uses thumbnails only for quick identification. It does not reproduce the large-card receptionist catalog. Search matches furniture name and category with Arabic normalization. Category and status filters compose. Each row isolates the Latin `YER` amount from the RTL labels and provides **تعديل**, **تكرار**, and **أرشفة** in a mouse-point action popup. Archive is non-destructive and keeps the row visible in the all-status view.

The list uses synthetic categories and records. **إضافة منتج** starts a new transient editor. Edit loads a synthetic product projection. Duplicate creates a local **نسخة** and opens the same editor. None of these actions claims that a product was saved or synchronized.

## Six implemented editor steps

The header shows and implements the complete sequence: **المعلومات**, **الأجزاء**, **المقاسات**, **الخيارات**, **التسعير**, and **المراجعة**:

1. **المعلومات** collects **اسم الأثاث**, **الفئة**, **صورة المنتج**, **وصف قصير**, and **ملاحظات داخلية**. Image selection loads one local preview in memory and does not upload or persist the file.
2. **الأجزاء المستخدمة** follows the Parts-to-Raw-Materials interaction pattern. **إضافة جزء** opens a searchable component picker. The manager selects a component, changes its positive quantity, and sees the row total and **تكلفة الأجزاء الحالية** update.
3. **المقاسات الثابتة** shows medium cards that managers can compare on one screen. Each card shows its name, `width × height × depth` in `cm`, a synthetic calculated cost, and **تعديل**, **تكرار**, and **إزالة** actions. These are manager-defined fixed variants, not receptionist customization options.
4. **الخيارات** shows **الألوان المتاحة** as compact rows with a swatch, name, visible price adjustment (**مشمول** or a positive `YER` amount), and active state. **المقابض المتاحة** uses visual tiles with a small vector handle image, name, visible adjustment, and active state. **إضافة لون** and **إضافة مقبض** open transient add dialogs; the resulting options are presentation fixtures. Internal material costing is not shown in this step.
5. **التسعير** keeps one row for every fixed variant on the same page. Cost is supporting information, **سعر البيع** is the prominent editable value, and **هامش الربح** is the calculated absolute difference. A selling price below cost remains visible as **خسارة متوقعة**. Percentage margin is not an input. All values use LTR isolation with `YER`.
6. **المراجعة** is read-only. It shows the image, name, category, description, compact fixed-variant cards with dimensions and selling prices, and active colors and handles with their price adjustments. **السابق** returns to Pricing. **حفظ كمسودة** and **حفظ ونشر** update only the local fixture and return to the Furniture list. The feedback message states that the result exists only in the local preview.

## Failure and recovery

The preview checks that a furniture name and category are present, that at least one part has a positive quantity before Variants, that fixed-variant names and dimensions are positive, and that every variant has a positive selling price before Review. These checks protect the local interaction only and do not define domain validation.

If navigation opens the generic dashboard, inspect `MainWindow.xaml` and `MainWindow.xaml.cs`. If the part picker, row menu, or variant editor is clipped, inspect the explicit popup overlays and physical LTR placement boundary in `FurnitureView.xaml`. Close the preview to discard local edits. Do not edit storage or bypass the shell.

## Verification and safe extension

Run the focused Furniture checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~Furniture"
```

`FurniturePresentationTests` verifies Arabic search normalization, composed state changes, part totals, fixed variants, visible option price adjustments, active-state toggles, selling-price margins, Review entry, and local draft completion. `FurnitureRenderedTests` instantiates the real WPF window, navigates to **الأثاث**, verifies Arabic accessible names and keyboard focus, selects a part, adds a variant, opens Options, adds a color and handle, edits Pricing, opens Review, and publishes back to the Furniture list. A manual rendered check at `1338 × 753` verifies the compact manager table, large image preview, part-cost summary, side-by-side variant cards, option surfaces, pricing rows, and the final read-only summary.

When Rust gains the Furniture vertical, add typed contracts and generated C# bindings before replacing the fixture boundary. Keep thumbnails and selected images as presentation inputs, map all authoritative state and errors from Rust, and preserve the current Arabic labels, fixed-variant distinction, bidirectional isolation, keyboard path, and non-destructive archive behavior.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.
