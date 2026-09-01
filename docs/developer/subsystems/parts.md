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
  - "Wardrobe Side Panel"
  - "YER"
  - "مستخدم في"
---

# Extend the Parts list safely

The Windows **الأجزاء** page gives managers a compact list of furniture parts. It currently provides search, category and status filters, a cost and product-usage projection, and non-destructive **تعديل**, **تكرار**, and **أرشفة** actions. The page is a local preview: closing the shell discards changes.

## Ownership and design

`shells/windows/Features/Parts/PartsViewModel.cs` owns only transient filtering, editor state, synthetic fixtures, and list projection. `PartListItem` in the same vertical formats the requested example as `9,450 YER` and `3 Products`. `PartsView.xaml` owns the RTL WPF layout, custom selector and popup templates, and accessibility names. `MainWindow.xaml` owns the **القطع** sidebar destination and the **إدارة القطع** dashboard shortcut.

Rust remains the future authority for part identifiers, category membership, cost validation, product relationships, ReBAC authorization, scope, audit, persistence, sync, and idempotency. This preview does not define a command, query, subscription, version, capability, or storage schema. Do not add database, IPC, permission, or domain-validation code to the shell to make the page appear connected.

## Normal behavior

The list starts with four synthetic rows. Search runs on each text change against part name and category and folds Arabic alef variants, `ى`, `ة`, tatweel, and combining marks. Category and status filters compose. Archived rows remain visible in the all-status view with reduced opacity and a neutral status pill. The empty state says **لا توجد أجزاء مطابقة**.

The header and table use the Raw Materials visual system: white cards, walnut/copper accents, rounded inputs, explicit LTR geometry for numeric values, and an RTL text boundary. The row menu uses mouse-point placement so the menu remains inside the window when the action column is at the physical left edge. The cost and usage cells isolate mixed-direction values; the English fixture **Wardrobe Side Panel**, `YER`, and **3 Products** stay readable beside Arabic labels.

The Add, Edit, Duplicate, and Archive paths update only in-memory state. The preview form checks that the name is present and that the cost and usage count are valid non-negative numbers; these are presentation checks against synthetic state, not domain validation. Rust remains authoritative for validation when this vertical is connected. Duplicate creates a local **نسخة** and opens the editor. Archive marks the row inactive and never deletes it. The editor subtitle **بيانات تجريبية غير محفوظة** is intentionally explicit.

## Security, compatibility, and failure recovery

The shell is an untrusted client. No Parts action is authorized, audited, scoped, synchronized, or durable in this preview. A future Rust vertical must negotiate a versioned capability before the shell sends any request and must return typed denial, validation, conflict, and retry outcomes. Until then, treat a closed window as the only rollback for local preview edits.

If the page is missing, opens a generic preview, or shows a clipped menu, inspect `MainWindow.xaml`, `PartsView.xaml`, and `PartsView.xaml.cs`. Follow [Windows shell state recovery](../../troubleshooting/windows-shell-state-recovery.md) for RTL layout, popup placement, stale visual state, and native-window checks. Do not bypass the shell or edit local storage.

## Tests and safe extension points

Run the focused shell suite:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo
```

`PartsSearchAndFiltersUpdateVisibleList` covers English and Arabic search plus combined filters. `PartsActionsRemainNonDestructiveAndEphemeral` covers cost and usage formatting, archive, duplicate, and create. `PartsRenderedTests` instantiates the WPF page, verifies the accessible Arabic search and create controls, drives the Add transition and focus, and opens the row menu to verify mouse-point placement and the **تعديل**, **تكرار**, and **أرشفة** actions.

When the product gains a Rust Parts vertical, keep the WPF page as a thin projection adapter. Add typed contracts and generated bindings first, then replace only the transient fixture and local action boundary. Preserve the Arabic labels, RTL/LTR isolation, numeric formatting, accessibility names, and non-destructive failure behavior while mapping authorization, scope, audit, idempotency, conflict, and sync results from Rust.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout, process, and extension rules.
