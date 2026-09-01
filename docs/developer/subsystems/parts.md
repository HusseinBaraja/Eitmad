---
title: "Extend the Parts list safely"
description: "Understand the Arabic-first Parts list, its transient WPF projection, filters, actions, and path to Rust ownership."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Windows UI maintainers"
last_verified: "2026-09-01"
review_triggers:
  - "Parts domain contracts, Rust projections, or Windows Parts UI behavior change"
keywords:
  - "PartsView"
  - "PartsViewModel"
  - "PartListItem"
  - "الأجزاء"
  - "إضافة جزء"
  - "معلومات الجزء"
  - "المواد الخام المستخدمة"
  - "مراجعة الجزء"
  - "حفظ الجزء"
  - "Wardrobe Side Panel"
  - "YER"
  - "مستخدم في"
---

# Extend the Parts list safely

The Windows **الأجزاء** page gives managers a compact list of furniture parts and a guided three-step creation flow. The list provides search, category and status filters, a cost and product-usage projection, and non-destructive **تعديل**, **تكرار**, and **أرشفة** actions. The page is a local preview: closing the shell discards changes.

## Ownership and design

`shells/windows/Features/Parts/PartsViewModel.cs` owns only transient filtering, wizard state, synthetic fixtures, material selection, calculated cost, and list projection. `PartMaterialUsage.cs` owns the selected amount and row-cost projection. `PartListItem` in the same vertical formats the requested example as `9,450 YER` and `3 Products`. `PartsView.xaml` owns the RTL WPF layout, custom selector and popup templates, and accessibility names. `MainWindow.xaml` owns the **القطع** sidebar destination and the **إدارة القطع** dashboard shortcut.

Rust remains the future authority for part identifiers, category membership, cost validation, product relationships, ReBAC authorization, scope, audit, persistence, sync, and idempotency. This preview does not define a command, query, subscription, version, capability, or storage schema. Do not add database, IPC, permission, or domain-validation code to the shell to make the page appear connected.

## Normal behavior

The list starts with four synthetic rows. Search runs on each text change against part name and category and folds Arabic alef variants, `ى`, `ة`, tatweel, and combining marks. Category and status filters compose. Archived rows remain visible in the all-status view with reduced opacity and a neutral status pill. The empty state says **لا توجد أجزاء مطابقة**.

The header and table use the Raw Materials visual system: white cards, walnut/copper accents, rounded inputs, explicit LTR geometry for numeric values, and an RTL text boundary. The row menu uses mouse-point placement so the menu remains inside the window when the action column is at the physical left edge. The cost and usage cells isolate mixed-direction values; the English fixture **Wardrobe Side Panel**, `YER`, and **3 Products** stay readable beside Arabic labels.

The Add, Edit, Duplicate, and Archive paths update only in-memory state. **إضافة جزء** opens a three-step RTL wizard:

1. **معلومات الجزء** collects the part name, category, and optional description or notes.
2. **المواد الخام المستخدمة** opens a searchable local material picker, adds selected materials to an editable usage table, and calculates each row and **تكلفة الجزء** from positive quantities.
3. **مراجعة الجزء** shows the part identity, a read-only material table, and the prominent **إجمالي تكلفة الجزء**. It does not allow inline editing; **السابق** returns to the owning step, and **حفظ الجزء** returns to the list.

The preview checks that the name is present, that a new part has at least one material, and that each amount is greater than zero. These are presentation checks against synthetic state, not domain validation. Rust remains authoritative for validation when this vertical is connected. The sample material costs are selected so `1.2 m²` of `MDF 18mm` plus `3 m` of `Edge Band` calculates to `9,450 YER`; no hidden surcharge exists. Duplicate creates a local **نسخة** and opens the same wizard. Archive marks the row inactive and never deletes it.

## Security, compatibility, and failure recovery

The shell is an untrusted client. No Parts action is authorized, audited, scoped, synchronized, or durable in this preview. A future Rust vertical must negotiate a versioned capability before the shell sends any request and must return typed denial, validation, conflict, and retry outcomes. Until then, treat a closed window as the only rollback for local preview edits.

If the page is missing, opens a generic preview, or shows a clipped menu, inspect `MainWindow.xaml`, `PartsView.xaml`, and `PartsView.xaml.cs`. Follow [Windows shell state recovery](../../troubleshooting/windows-shell-state-recovery.md) for RTL layout, popup placement, stale visual state, and native-window checks. Do not bypass the shell or edit local storage.

## Tests and safe extension points

Run the focused shell suite:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo
```

`PartsSearchAndFiltersUpdateVisibleList` covers English and Arabic search plus combined filters. `PartsActionsRemainNonDestructiveAndEphemeral` covers cost and usage formatting, archive, duplicate, the three wizard steps, material quantities, and the calculated `9,450 YER` total. `GuidedCreationValidatesStepsAndFiltersMaterialPicker` covers step guards, material search, selection, and removal. `PartsRenderedTests` instantiates the WPF page, verifies the accessible Arabic controls and focus path, drives create through all three steps and back to the list, and opens the row menu to verify mouse-point placement and the **تعديل**, **تكرار**, and **أرشفة** actions.

When the product gains a Rust Parts vertical, keep the WPF page as a thin projection adapter. Add typed contracts and generated bindings first, then replace only the transient fixture and local action boundary. Preserve the Arabic labels, RTL/LTR isolation, numeric formatting, accessibility names, and non-destructive failure behavior while mapping authorization, scope, audit, idempotency, conflict, and sync results from Rust.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout, process, and extension rules.
