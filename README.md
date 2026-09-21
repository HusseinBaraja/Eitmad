# الاعتماد | Eitmad

Arabic-first operations system for الاعتماد furniture manufacturing.

This repository is a cross-platform native desktop workspace built around a separate, authoritative Rust engine. Native shells stay thin; Rust owns contracts, product behavior, storage, synchronization, authorization, update policy, observability, audit, and external boundaries.

Use English for reasoning and for communication with the user. Keep the application UI Arabic-first, and preserve exact Arabic UI labels and established domain terms when you refer to them.

Start with:

- [Repository policy](AGENTS.md)
- [Target architecture](docs/architecture/target-architecture.md)
- [Repository layout and ownership](docs/developer/repository-layout.md)
- [Documentation index](docs/index.md)

## Run the admin dashboard on Windows

Install the .NET 8 SDK and the stable Rust toolchain. From the repository root, build the debug Rust engine, provision the synthetic development accounts, and start the native Windows dashboard with one command:

```powershell
.\run.ps1
```

Use `test.manager` with `Eitmad-Manager-2026!` or `test.receptionist` with `Eitmad-Reception-2026!` on the sign-in screen. These synthetic accounts are available only after the explicit debug seed command that `run.ps1` runs. Release builds do not contain that command. Closing the window keeps the app in the system tray. Use **إنهاء الاعتماد** in the tray menu to stop the shell and engine. For design and ownership details, see the [Windows operations shell guide](docs/developer/subsystems/windows-native-shell.md).

## Furniture editor flow

The Windows Furniture editor uses six steps: `المعلومات`, `الأجزاء`, `المقاسات`, `الخيارات`, `التسعير`, and `المراجعة`. Managers set selling prices for each fixed variant, then choose `حفظ كمسودة` or `حفظ ونشر` from the read-only review step.

## Ready-made Products manager

The Windows **المنتجات** page manages ready-made items that the company purchases and sells as-is. Its compact table and short **إضافة منتج** / **تعديل المنتج** page support images, shared category interaction, purchase and selling prices, calculated margins, optional supplier variants, notes, duplicate, and confirmed archive. The current workflow uses in-memory preview state only; Rust authorization, audit, storage, and synchronization are not implemented for Products yet.

## Quick pricing preview

The separate Windows **التسعير** page lets a manager search product variants, filter by category, and open **تعديل سعر البيع** without entering the full Furniture editor. The current quick edit changes in-memory preview state only; Rust authorization, audit, storage, and synchronization are not implemented for product pricing yet.

## Manager quotation review

The Windows **عروض الأسعار** page lets a manager search and filter quotations, open a read-only furniture-item detail, and review subtotal, discount, and final total. **موافقة** and **رفض** appear only when the synthetic fixture requires discount approval. These actions update preview state only; Rust quotation authorization, audit, storage, and synchronization are not implemented yet.

## Manager order review

The Windows **الطلبات** page lets a manager search and filter orders, review the five preview statuses, and open a read-only order detail with furniture specifications and totals. It does not expose carpenter actions or change order state. The rows are synthetic presentation fixtures; Rust order authorization, audit, storage, and synchronization are not implemented yet.

## Manager work orders

The Windows **أوامر العمل** page separates furniture that must be manufactured from ready-made Products. One Work order can contain multiple Furniture items, and the list shows both the item summary and total quantity. Managers can search and filter the scan-first list, open **أمر عمل #024**, review the assigned carpenter, due date, every furniture specification, required Parts, and customer notes, then move the local preview through **جديد** → **قيد التنفيذ** → **مكتمل**. It never shows selling price, cost, profit, or margin. The status action is transient; Rust work-order authorization, audit, storage, and synchronization are not implemented yet.

For routine changes, use the [focused check selection table](docs/developer/index.md#choose-the-smallest-normal-proof). Keep the full workspace and platform gates for [release-candidate validation](docs/operations/validate-release-candidate.md).

Production features must be organized by bounded product capability. Do not add generic `utils`, `common`, `shared`, `handlers`, or `services` buckets.
