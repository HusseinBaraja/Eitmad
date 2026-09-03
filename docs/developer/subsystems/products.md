---
title: "Extend the ready-made Products manager flow safely"
description: "Understand the Arabic-first Products list and its transient single-page add and edit workflow."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Products capability maintainers"
last_verified: "2026-09-03"
review_triggers:
  - "Product contracts, category ownership, pricing rules, or Windows Products UI behavior change"
keywords:
  - "ProductsView"
  - "ProductsViewModel"
  - "ready-made product"
  - "المنتجات"
  - "إضافة منتج"
  - "هل لهذا المنتج مقاسات أو أنواع مختلفة؟"
---

# Extend the ready-made Products manager flow safely

The Windows **المنتجات** page lets a manager maintain ready-made items that the company purchases and sells as-is. It is separate from **الأثاث**: a Product does not have raw materials, furniture parts, construction details, calculated manufacturing cost, or the six-step Furniture editor.

The current page is a local presentation preview. Search, filters, categories, image previews, direct prices, supplier variants, notes, duplicate state, and archive state are discarded when the shell closes.

## Ownership and boundary

`shells/windows/Features/Products/ProductsView.xaml` owns the native RTL list, product thumbnails, short single-page form, image preview, category selector, pricing hierarchy, optional variants table, confirmation overlay, keyboard focus targets, and Arabic accessibility names. `ProductsViewModel.cs` owns transient fixtures, Arabic-normalized search, filters, editor state, calculated margins, duplicate behavior, category interaction, and confirmed archive state. `ProductModels.cs` owns presentation-only labels and calculations. `MainWindow.xaml` owns the **المنتجات** sidebar destination.

Rust does not yet provide a Products capability. The preview has no Product command, query, subscription, capability, authorization check, scope, audit record, storage, synchronization, or authoritative pricing rule. Do not add these responsibilities to WPF. A future Rust vertical must own the Product lifecycle, category authority, pricing validation, authorization, audit, durable storage, and synchronization.

## Manager list

The manager opens **المنتجات** to see a compact table with image, name, category, supplier variant or size, purchase cost, selling price, status, and actions. Search matches the Arabic product name, category, and variant while normalizing Arabic diacritics and common Alef, Ya, and Ta Marbuta forms. Category and status filters compose with search.

The row menu reuses the compact **تعديل**, **تكرار**, and **أرشفة** actions. Archive is non-destructive and requires the common centered confirmation style. An archived row remains available through the **مؤرشف** status filter.

## Short add and edit workflow

**إضافة منتج** opens one scrolling page. It does not use steps.

1. **المعلومات الأساسية** collects **اسم المنتج**, **الفئة**, **صورة المنتج**, and **وصف قصير**. Image selection loads a preview into memory and does not upload or persist the file. The category selector reuses the established inline add and manage interaction.
2. **هل لهذا المنتج مقاسات أو أنواع مختلفة؟** defaults to **لا**. Without variants, the manager enters **تكلفة الشراء** and the emphasized **سعر البيع**, then reads the calculated **الهامش**.
3. With **نعم**, one supplier-defined row replaces the single-price surface. Each row contains name, purchase cost, selling price, calculated margin, and remove action. These rows describe fixed ready-made options, not customizable dimensions.
4. **ملاحظات** remains optional and secondary.
5. **حفظ المنتج** updates the transient fixture and returns to the list. **إلغاء** returns without applying the editor. The edit state uses the same page under **تعديل المنتج** and adds **أرشفة المنتج**.

The current preview validates a non-empty product name, an active category, non-negative price inputs, and at least one named variant when variants are enabled. Negative margins remain visible because the manager might need to identify an unprofitable supplier price; Rust must define any future rejection policy.

## Failure and recovery

Validation text appears in the form footer and keeps the editor open. Invalid numeric text is held by WPF binding validation until the manager corrects it. If navigation opens the dashboard instead of Products, inspect `MainWindow.xaml` and `MainWindow.xaml.cs`. If a category popup, row action menu, or confirmation surface is clipped, inspect the explicit popup and overlay boundaries in `ProductsView.xaml`.

Close the preview to discard all local Product changes. Do not edit storage or imply that a local preview action was authorized, audited, saved, or synchronized.

## Verify

Build the shell and run the focused Product checks:

```powershell
dotnet build shells/windows/Eitmad.WindowsShell.csproj
```

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~Products"
```

`ProductsPresentationTests` verifies Arabic search normalization, composed filters, culture-independent YER labels, direct and variant pricing, calculated margins, duplicate, confirmed archive, and the inline category interaction. `ProductsRenderedTests` instantiates the real WPF window, opens **المنتجات**, verifies the manager list and compact action menu, opens **إضافة منتج**, checks keyboard focus and accessible names, and exposes the archive confirmation.

Perform one rendered check at `1338 × 753` after a visible UI change. Verify the compact table, mixed-direction YER values, short form, pricing emphasis, variants switch, popup placement, and archive confirmation. Add a compact-width pass only when responsive behavior changes.

## Extend the capability

When Rust gains the Products vertical, define the typed contracts and generated C# bindings before replacing the fixture boundary. Preserve the Product/Furniture distinction, direct supplier cost, optional fixed variants, Arabic labels, mixed-direction value isolation, keyboard path, and non-destructive archive behavior.

Related concepts: [Furniture manager flow](furniture.md), [Arabic-first feature checklist](../contributing/arabic-first-feature-checklist.md), and [approved terminology](../../glossary.md).
