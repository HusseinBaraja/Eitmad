---
title: "Extend the Furniture manager flow safely"
description: "Understand the Arabic-first Furniture list and its transient Information, Parts, Variants, and Options preview steps."
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
  - "YER"
---

# Extend the Furniture manager flow safely

The Windows **الأثاث** page gives managers a compact product table and the first four steps of a six-step furniture editor. The page is a local preview. Search, filters, image selection, part quantities, calculated values, variants, colors, handles, duplicate, and archive state are discarded when the shell closes.

## Ownership and current boundary

`shells/windows/Features/Furniture/FurnitureView.xaml` owns the native RTL layout, small vector thumbnails, upload preview surface, custom selectors, tables, option rows and tiles, overlays, keyboard focus targets, and Arabic accessibility names. `FurnitureViewModel.cs` owns transient fixtures, list filtering, the current editor step, selected parts, preview totals, fixed-variant cards, colors, handles, and their visible price adjustments. `FurnitureModels.cs` owns presentation-only row labels, option state, and calculated display values. `MainWindow.xaml` owns the **الأثاث** sidebar destination.

Rust does not yet provide a Furniture capability. The preview has no command, query, subscription, capability, authorization check, scope, audit record, persistence, sync, or authoritative pricing rule. Do not add these responsibilities to the WPF shell. A future Furniture vertical must own the product lifecycle, part relationships, dimension validation, costs, prices, authorization, audit, storage, and synchronization.

## Manager list behavior

The table uses thumbnails only for quick identification. It does not reproduce the large-card receptionist catalog. Search matches furniture name and category with Arabic normalization. Category and status filters compose. Each row isolates the Latin `YER` amount from the RTL labels and provides **تعديل**, **تكرار**, and **أرشفة** in a mouse-point action popup. Archive is non-destructive and keeps the row visible in the all-status view.

The list uses synthetic categories and records. **إضافة منتج** starts a new transient editor. Edit loads a synthetic product projection. Duplicate creates a local **نسخة** and opens the same editor. None of these actions claims that a product was saved or synchronized.

## Four implemented editor steps

The header shows the complete future sequence: **المعلومات**, **الأجزاء**, **المقاسات**, **الخيارات**, **التسعير**, and **المراجعة**. Only the first four steps are implemented:

1. **المعلومات** collects **اسم الأثاث**, **الفئة**, **صورة المنتج**, **وصف قصير**, and **ملاحظات داخلية**. Image selection loads one local preview in memory and does not upload or persist the file.
2. **الأجزاء المستخدمة** follows the Parts-to-Raw-Materials interaction pattern. **إضافة جزء** opens a searchable component picker. The manager selects a component, changes its positive quantity, and sees the row total and **تكلفة الأجزاء الحالية** update.
3. **المقاسات الثابتة** shows medium cards that managers can compare on one screen. Each card shows its name, `width × height × depth` in `cm`, a synthetic calculated cost, and **تعديل**, **تكرار**, and **إزالة** actions. These are manager-defined fixed variants, not receptionist customization options.
4. **الخيارات** shows **الألوان المتاحة** as compact rows with a swatch, name, visible price adjustment (`Included` or a positive `YER` amount), and active state. **المقابض المتاحة** uses visual tiles with a small vector handle image, name, visible adjustment, and active state. **إضافة لون** and **إضافة مقبض** open transient add dialogs; the resulting options are presentation fixtures. Internal material costing is not shown in this step.

The **التالي** action on step 4 stays on the current screen and reports that **التسعير** is not implemented. This stop is deliberate. Do not add Pricing, Review, or a final save command as part of this preview.

## Failure and recovery

The preview checks that a furniture name and category are present, that at least one part has a positive quantity before Variants, and that fixed-variant names and dimensions are positive. These checks protect the local interaction only and do not define domain validation.

If navigation opens the generic dashboard, inspect `MainWindow.xaml` and `MainWindow.xaml.cs`. If the part picker, row menu, or variant editor is clipped, inspect the explicit popup overlays and physical LTR placement boundary in `FurnitureView.xaml`. Close the preview to discard local edits. Do not edit storage or bypass the shell.

## Verification and safe extension

Run the focused Furniture checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~Furniture"
```

`FurniturePresentationTests` verifies Arabic search normalization, composed state changes, part totals, fixed variants, visible option price adjustments, active-state toggles, and the deliberate stop at step 4. `FurnitureRenderedTests` instantiates the real WPF window, navigates to **الأثاث**, verifies Arabic accessible names and keyboard focus, selects a part, adds a variant, opens Options, adds a color and handle, and checks the deliberate pricing stop. A manual rendered check at `1338 × 753` verifies the compact manager table, large image preview, part-cost summary, side-by-side variant cards, color rows, and handle tiles.

When Rust gains the Furniture vertical, add typed contracts and generated C# bindings before replacing the fixture boundary. Keep thumbnails and selected images as presentation inputs, map all authoritative state and errors from Rust, and preserve the current Arabic labels, fixed-variant distinction, bidirectional isolation, keyboard path, and non-destructive archive behavior.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.
