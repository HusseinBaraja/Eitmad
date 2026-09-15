---
title: "Extend the quotation review flow safely"
description: "Understand the Arabic-first quotation list, read-only detail, discount approval preview, tests, and Rust ownership boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Quotation capability maintainers"
last_verified: "2026-09-15"
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

Rust does not yet provide a quotation capability. No quotation command, query, subscription, capability, authorization check, scope, audit record, durable storage, or synchronization exists in this preview. Keep shell changes limited to clearly local fixture decisions and temporary receptionist selections.

## Manager workflow

The list shows **رقم عرض السعر**, **العميل**, **التاريخ**, **الإجمالي**, **الخصم**, **الحالة**, and **فتح**. Search matches the quotation number and customer after Arabic normalization. Status filters group **مسودة**, **نشط**, **محوّل**, and **ملغي / منتهي**; date filters cover **اليوم**, **آخر 7 أيام**, and **آخر 30 يوماً**.

Opening a row shows quotation metadata, furniture lines with variant, color, handle, quantity, unit price, and total, followed by subtotal, discount, and final total. Amounts use `YER` with local LTR isolation. The detail is read-only.

For a fixture marked **موافقة الخصم مطلوبة**, the detail shows **موافقة** and **رفض**. These actions update only the local `DiscountApprovalDecision` preview and are hidden for quotations without a pending approval. They do not change quotation status or claim manager authorization.

## Receptionist list and existing quotation detail

The receptionist home card and sidebar open a separate `QuotationsView` configured through `ConfigureReceptionist`. It reuses the manager table, filters, metadata, and totals without exposing manager approval controls. The list has number, customer, date, total, status, and open columns. Search also matches synthetic phone numbers, including Arabic digits. Status and relative-date filters compose with search; **بانتظار الموافقة** is a separate receptionist status.

**+ عرض سعر جديد** opens an empty Current Quotation preview in a native window and preserves the current catalog selection. Draft and active rows offer **تعديل**, **طباعة**, and **تحويل إلى طلب**. Edit creates a detached fixture projection through `QuotationPreviewProjection` and reuses the existing catalog and Current Quotation controls. Edits are temporary and are discarded when that window closes. Printing reuses the customer document and native print preview with the quotation number and original date. Conversion reports that it is unavailable and does not create an order or change status.

Waiting-approval rows are read-only and explain that printing is available after approval. Converted rows show **تم التحويل إلى طلب**, hide edit, print, and conversion, and offer **فتح الطلب** for a synthetic related order detail. Cancelled and expired rows have no modification actions. These are presentation rules for fixtures, not authorization. Production status, linked-order identity, conversion, immutable converted records, and audit must come from Rust.

`ReceptionQuotationsRenderedTests` checks receptionist navigation, phone search, status-based actions, and isolation from manager approval controls. Synthetic captures cover list, active, waiting, and converted states.

## Receptionist current quotation preview

`Features/Reception/CurrentQuotationView.xaml` presents a full quotation page without a side cart. `CurrentQuotation.cs` holds temporary line snapshots and synthetic customer form state alongside `SalesCatalogViewModel`. Rows show the catalog image or shared illustration, selected options, furniture dimensions, quantity, unit price, and total. Edit reuses the furniture or ready-made selection screen with preselected options; save replaces the original line, and cancel preserves it. Duplicate creates a separate line identity. Remove updates totals and the empty state.

The bottom summary reuses `FormField`, `AmountDisplay`, and `StatusBadge` for one percentage input, immediate whole-YER discount value, and total. `QuotationDiscountPreview.cs` uses a synthetic 5% limit, not a manager-configured production policy. Arabic and Persian digits are accepted; invalid input blocks both save actions. Above the fixture limit, **يتطلب موافقة المدير** offers **طلب موافقة**. Requesting changes the temporary state to **بانتظار موافقة المدير**, keeps review and **حفظ كمسودة** available, and blocks quotation finalization. The total is labeled as the requested discount total. Changing the percentage or quotation lines invalidates the pending request. No price override or manager pricing rules are exposed. Requests and draft saves explicitly report that nothing was sent or persisted; the manager preview is not connected to this state. The heading supports an existing number, but new previews have no assigned number.

Customer name and phone are checked when saving a new customer preview, attempting quotation save, or opening the customer preview. Address and notes are optional. Name or phone searches offer synthetic customer matches inline. **+ عميل جديد** reuses the inline fields; cancel restores the previous fields, while save attaches the temporary customer. These checks illustrate the proposed workflow and are not authoritative domain validation. Quotation save reports that durable saving is unavailable and does not assign a number or claim success. All state is discarded when the shell closes.

The final actions remain on this review page. Save attempts show all missing item, customer-name, and phone errors through the existing `FormField`; focus moves to the first missing input or the item-selection action. Errors clear when corrected. **طباعة** opens a customer-only document in the reusable `Controls/PrintPreview` native page viewer. It is unavailable for empty quotations, invalid discounts, or discounts requiring approval; incomplete customer fields receive the same inline errors. **رجوع** closes the preview, restores focus, and preserves the editor.

`QuotationCustomerDocument` explicitly selects the brand wordmark, synthetic company information, quotation number (or unassigned label), preview date, customer name, phone, optional address, item names, selected options, quantities, prices, and totals. It never binds the editor into the printable document. Cost, margin, raw materials, parts, and internal notes are excluded. The A4 document paginates and carries a visible unsaved-fixture label. No production logo or company identity source exists yet; the wordmark and contact details are placeholders. The native print dialog submits the document only after user confirmation, reports printer failures, and restores preview page dimensions after printing. **تحويل إلى طلب** is not exposed because no saved quotation or Rust-authorized conversion exists.

Run `QuotationFinalActionsRenderedTests` for inline-error focus, preview and return focus, customer-only content, discount gating, and multi-page pagination. Physical printer output requires a separate manual check.

Run `SalesCatalogPresentationTests` and `SalesCatalogRenderedTests` in the Windows test project for snapshot editing, cancellation, duplicate identity, totals, customer selection, and the rendered selection-to-quotation path. Set `EITMAD_CATALOG_CAPTURE_DIR` to capture the affected synthetic screens.

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
