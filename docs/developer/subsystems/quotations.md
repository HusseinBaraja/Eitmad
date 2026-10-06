---
title: "Extend the quotation review flow safely"
description: "Understand the Arabic-first quotation list, read-only detail, discount approval preview, tests, and Rust ownership boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Quotation capability maintainers"
last_verified: "2026-10-06"
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

Production lifecycle, `5.00%` threshold, approval fingerprint, price snapshot, validity, numbering, permission, and offline behavior is accepted in the [Manager and Receptionist workflow specification](manager-receptionist-workflows.md). This page describes the current preview only; preview `Active`, approval, conversion, and `QT-PREVIEW` behavior does not override the accepted specification.

## Ownership and current boundary

`shells/windows/Features/Quotations/QuotationsView.xaml` owns the native RTL list, filters, detail surface, conditional approval actions, focus target, and Arabic accessibility names. `QuotationsViewModel.cs` owns synthetic rows, Arabic-normalized search, status and relative-date filters, selected detail state, and approval routing. `QuotationModels.cs` owns line totals, quotation totals, status labels, discount percentages, and local approval state. `MainWindow.xaml` owns the **عروض الأسعار** destination.

Rust provides quotation evaluation through `crates/pricing/src/quotation.rs` and durable drafts through `crates/pricing/src/drafts.rs`. Draft persistence and server transfer are implemented without shell connections. Issuance, approval decisions, conversion, and numbering remain unimplemented. The manager list and handoffs remain fixtures.

## Authoritative quotation evaluation

`eitmad.quotation.evaluate.v1` is a typed Query with capability `eitmad.capability.quotation-evaluation.v1` and schema `eitmad.schema.quotation-evaluation.v1`, version 1. Native bindings come from `crates/contracts/src/quotation.rs`. The Windows adapter uses the authenticated branch context; Rust checks customer-read access there and catalog-read access in the organization derived from the authenticated tenant. Foreign catalog scopes fail the entire query. No client price, total, description, or approval flag is accepted in the quotation intent.

The input contains an optional saved customer UUID and expected revision, stable line UUIDs, exact catalog/variant references and definition revisions, expected price revisions, selected options, normalized dimensions, integer quantities, and discount basis points. Rust reads the customer and public catalog entries in one SQLite read transaction. It returns the validated customer snapshot, public line descriptions and selected option names, immutable public price snapshots, whole-YER amounts, and typed errors associated with a field and optional line UUID. Internal cost, margin, compositions, and customer notes are excluded.

Evaluation reuses the catalog configuration validator and [Pricing arithmetic](pricing.md#arithmetic-and-snapshots). The line count is bounded at 1,000. Missing or changed customers, duplicate line identities, stale definition or price revisions, withdrawn items, invalid options or dimensions, invalid quantities or discounts, and checked arithmetic overflow withhold aggregate totals. Valid evaluation returns subtotal, discount, total, and the approval requirement. It does not grant approval, allocate a number, save a draft, or issue a quotation. Query outcomes are audited without retaining customer or line payloads.

The dispatcher attempts a bounded authorized catalog refresh when replication is configured. Otherwise it evaluates the last confirmed public projection and reports `server_available = false`. Cache evaluation does not prove that future server issuance can succeed. Stale lines require explicit catalog refresh and user review; evaluation does not silently replace their revisions.

For a live catalog session, `QuotationEvaluationState.cs` sends intent after customer, line, discount, or catalog changes. It clears old totals immediately, cancels replaced requests, and discards replies with an obsolete version or session. WPF displays returned totals and approval requirements. Local fixture approval cannot authorize a live evaluation. Discounts accept at most two decimal percentage places for exact conversion to basis points. Standalone synthetic previews retain their temporary calculations.

Focused Rust tests in `crates/pricing/src/tests/quotation.rs` cover accepted rounding, threshold boundaries, public furniture adjustments, customer and catalog changes, scope denial, invalid intent, and overflow. Contract tests reject forged authority fields and fractional numeric inputs. `QuotationEvaluationTests` checks returned totals, validation errors, obsolete replies, session end, and fixture isolation. Run `cargo test -p eitmad-pricing quotation`, `cargo test -p eitmad-contracts quotation`, and the shell test filter `FullyQualifiedName~QuotationEvaluationTests`.

## Durable quotation drafts

Protocol `1.19` registers `eitmad.capability.quotation-draft.v1` and `eitmad.schema.quotation-draft.v1`, schema version 1. `CreateQuotationDraft` and `UpdateQuotationDraft` accept only evaluator intent. `GetQuotationDraft`, `ListQuotationDrafts`, and `QuotationDraftChanges` use the authenticated branch. Lists use a stable UUID cursor with limits from 1 through 100. Notifications contain identity, scope, revision, time, and change identity; subscribers must query the authorized snapshot.

`QuotationDraftService` owns this boundary next to the evaluator. It assigns a UUID and revision 1, then increments revisions with compare-and-swap. Drafts have no official number, approval grant, or issuance state. Receptionists need the branch-scoped `eitmad.permission.quotation.draft.write.v1`. Managers and assigned Receptionists can read under `eitmad.permission.quotation.draft.read.v1`; Manager permission does not grant draft mutation. Lists, direct reads, and subscriptions use the same exact branch scope.

SQLite migration `quotation.drafts.v1` stores the configured intent and evaluator result, including the customer reference and public price snapshots. Evaluation and save share one SQLite writer transaction. The transaction also stores the successful audit, principal-bound request fingerprint and replay result, subscription publication, and bounded sync outbox. Failed mandatory writes roll back all of them. An exact authorized retry returns the original identity and result before new evaluation. A changed request under the same key fails closed.

Reads preserve saved prices after catalog changes. An update that refers to a changed customer, catalog definition, or price returns field errors without replacing the draft. Refresh must be explicit. A save from confirmed cached catalog data remains a local draft and cannot prove issuance eligibility. Draft content and pending publication survive engine restart.

`QuotationDraftSyncCycle` uses the shared local-first protocol and a schema-specific durable engine checkpoint. It stages at most 50 changes, sends them through the real server connection, projects pages before advancing the checkpoint, and acknowledges each page. Authenticated enrollment maps local branch and catalog scopes to their server identities; the shell cannot supply this mapping. Transport failure leaves outbox work available for retry. PostgreSQL migration `0011_quotation_drafts.sql` retains immutable draft revisions under tenant RLS and exact branch authorization. The domain handler verifies the customer snapshot against its scoped retained contact revision, and verifies public descriptions and evaluated amounts against retained catalog and price revisions. A stale published revision can transfer as draft content; transfer does not issue it or silently replace its price. Customer delivery must arrive first; an absent server customer revision leaves the draft retryable.

Concurrent edits create a server conflict instead of applying a generic merge. The local draft remains visible with `conflicted` state, server conflict identity, and the remote input. Later queued revisions of that draft cannot bypass the first conflict. Rejected and conflicted work remains durable but stops automatic resubmission. Updates are blocked until a future explicit resolution workflow; no resolution or issuance UI is connected in this task.

Focused tests in `crates/pricing/src/tests/quotation.rs` cover restart, atomic rollback, retry, stale input, authorization, and competing projection. The ignored live test `quotation_drafts_restart_transfer_replay_and_conflict_through_real_server` in `crates/server-connection/tests/direct_route/quotation_drafts.rs` uses two isolated SQLite authorities, the TLS host, and a disposable PostgreSQL role without superuser or `BYPASSRLS` rights. It covers forged price and customer snapshots, interrupted acknowledgement, replay, second-client delivery, two queued competing edits, immutable revisions, and tenant isolation. Use the certificate and database setup in [server operations](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test), then run:

```powershell
cargo test -p eitmad-server-connection --test direct_route quotation_drafts_restart_transfer_replay_and_conflict_through_real_server -- --ignored
```

## Manager workflow

The list shows **رقم عرض السعر**, **العميل**, **التاريخ**, **الإجمالي**, **الخصم**, **الحالة**, **المتابعة**, and **فتح**. Search matches the quotation number and customer after Arabic normalization. Status filters include **بانتظار الموافقة** and group **مسودة**, **نشط**, **محوّل**, and **ملغي / منتهي**; date filters cover **اليوم**, **آخر 7 أيام**, and **آخر 30 يوماً**.

Opening a row shows quotation metadata, furniture lines with variant, color, handle, quantity, unit price, and total, followed by subtotal, discount, and final total. Screen amounts and customer documents display **ر.ي** with local LTR isolation, following the [currency display rule](manager-receptionist-workflows.md#money). The detail is read-only.

For a fixture marked **موافقة الخصم مطلوبة**, the detail shows **موافقة** and **رفض**. These actions update only the local `DiscountApprovalDecision` preview and are hidden for quotations without a pending approval. The status label shows **الخصم مقبول** or **الخصم مرفوض** after the decision. This does not claim manager authorization.

## Receptionist list and existing quotation detail

The receptionist home card and sidebar open a separate `QuotationsView` configured through `ConfigureReceptionist`. It reuses the manager table, filters, metadata, and totals without exposing manager approval controls. The list has number, customer, date, total, status, and open columns. Search also matches synthetic phone numbers, including Arabic digits. Status and relative-date filters compose with search; **بانتظار الموافقة** is a separate receptionist status.

**+ عرض سعر جديد** opens an empty sales catalog session in a native window and preserves the main window's current quotation. Browsing and adding items require no customer details. The window reuses `PageHeader` for an explicit quotation action, available from the catalog and selection screens. New and edit windows can return to review after browsing. Opening review moves keyboard focus to **متابعة اختيار المنتجات**; continuing restores focus to the selection or catalog. Editable rows offer **تعديل**. Printing and conversion also require any requested discount to be approved. Edit creates a detached fixture projection through `QuotationPreviewProjection` and reuses the existing catalog and Current Quotation controls. Unsaved edits are discarded when that window closes. Draft save and quotation completion publish a temporary snapshot to both role lists for the current main-window session. Printing reuses the customer document and native print preview with the quotation number and original date. Conversion opens the shared `DialogHost` with customer, line count, final total, **سيتم إنشاء طلب جديد من عرض السعر الحالي.**, **تأكيد وإنشاء الطلب**, and **إلغاء**. Cancel receives initial focus; cancel or Escape closes the dialog and restores focus. Confirmation opens the existing Order Detail in a native preview window. It copies the synthetic customer, lines, variants, color and handle options, quantities, prices, and discount without opening the editor. Both the confirmation notice and order window identify the result as an unsaved preview. No order is persisted and quotation status remains unchanged. Production creation, approved-price validation, and duplicate-conversion protection still require Rust authority.

Waiting-approval rows are read-only and explain that printing is available after approval. A decision appears on the same receptionist detail. Approved rows can reopen the editor and complete the preview with the approved discount. Rejected rows can reopen for correction or another request, but cannot print or convert with that discount. Converted rows show **تم التحويل إلى طلب**, hide edit, print, and conversion, and offer **فتح الطلب** for a synthetic related order detail. Cancelled and expired rows have no modification actions. These are presentation rules for fixtures, not authorization. Production status, linked-order identity, conversion, immutable converted records, and audit must come from Rust.

`ReceptionQuotationsRenderedTests` checks receptionist navigation, phone search, status-based actions, and isolation from manager approval controls. It also checks conversion cancellation, initial and return focus, inherited order data, and unchanged quotation status. It also checks that new quotation windows start in the catalog, keep additions there, and return to review only by explicit action. Synthetic captures cover list, active, waiting, converted, catalog selection, and conversion-confirmation states.

## Receptionist current quotation preview

`Features/Reception/CurrentQuotationView.xaml` presents a full quotation page without a side cart. `CurrentQuotation.cs` holds temporary line snapshots and synthetic customer form state alongside `SalesCatalogViewModel`. Rows show the catalog image or shared illustration, selected options, furniture dimensions, quantity, unit price, and total. Edit reuses the furniture or ready-made selection screen with preselected options; save replaces the original line, and cancel preserves it. Duplicate creates a separate line identity. Remove updates totals and the empty state.

The bottom summary reuses `FormField`, `AmountDisplay`, and `StatusBadge` for one percentage input, whole-YER discount value, and total. Live sessions receive amounts and approval requirements from Rust evaluation. In standalone fixtures, `QuotationDiscountPreview.cs` uses a synthetic 5% limit. Arabic and Persian digits are accepted; invalid input blocks both save actions. Above the fixture limit, **يتطلب موافقة المدير** offers **طلب موافقة**. Requesting changes the temporary state to **بانتظار موافقة المدير**, keeps review and **حفظ كمسودة** available, and blocks quotation finalization. The total is labeled as the requested discount total. Changing the percentage, quotation lines, or customer fields detaches the editor from the prior decision. The prior request remains an immutable snapshot until a new draft or request replaces it. No price override or manager pricing rules are exposed. Requests require one Rust-selected or newly created customer when a branch-scoped session is available. `ReceptionHandoffPreview` shares quotation snapshots between the two role lists in one main window and keeps the selected customer UUID for detail navigation. The dashboard **الموافقات** action opens the pending-discount inbox with reset filters. The same filter is available on the quotation list. **المتابعة** distinguishes independent samples, new reception snapshots, edited snapshots, and discount requests. New snapshots receive a `QT-PREVIEW` number. Customer contact create and edit can be durable through the Rust Customer capability; the quotation, approval, conversion, and order handoffs remain temporary preview state and are not persisted. Close the editor before switching accounts with Alt+K, review the request as manager, then return to the receptionist list and reopen the quotation. The main catalog editor also observes decisions while it remains open.

Customer name and phone are checked when creating a customer, attempting quotation or draft save, or opening the customer preview. Address and notes are optional. The branch-scoped `CustomerClient` searches and creates customer contacts through the Rust customer capability. **+ عميل جديد** reuses the inline fields; cancel restores the previous fields, while save retains the selected customer UUID in temporary quotation state. Rust owns customer validation and persistence. Quotation save reports that durable quotation saving is unavailable and does not claim a durable save. Temporary quotation state is discarded when the shell closes.

The final actions remain on this review page. Save attempts show all missing item, customer-name, and phone errors through the existing `FormField`; focus moves to the first missing input or the item-selection action. Errors clear when corrected. **طباعة** opens a customer-only document in the reusable `Controls/PrintPreview` native page viewer. It is unavailable for empty quotations, invalid discounts, or discounts requiring approval; incomplete customer fields receive the same inline errors. **رجوع** closes the preview, restores focus, and preserves the editor.

`QuotationCustomerDocument` explicitly selects the brand wordmark, synthetic company information, quotation number (or unassigned label), preview date, customer name, phone, optional address, item names, selected options, quantities, prices, and totals. It never binds the editor into the printable document. Cost, margin, raw materials, parts, and internal notes are excluded. The A4 document paginates and carries a visible unsaved-fixture label. No production logo or company identity source exists yet; the wordmark and contact details are placeholders. The native print dialog submits the document only after user confirmation, reports printer failures, and restores preview page dimensions after printing. **تحويل إلى طلب** is not exposed because no saved quotation or Rust-authorized conversion exists.

Run `QuotationFinalActionsRenderedTests` for inline-error focus, preview and return focus, customer-only content, discount gating, and multi-page pagination. Physical printer output requires a separate manual check.

Run `SalesCatalogPresentationTests` and `SalesCatalogRenderedTests` in the Windows test project for snapshot editing, cancellation, duplicate identity, totals, customer selection, and the rendered selection-to-quotation path. Set `EITMAD_UI_CAPTURE_DIR` to capture the affected synthetic screens.

## Failure and recovery

Closing the detail returns to the list. Closing the shell discards all local quotation state. Do not add a shell-side approval rule or infer authorization from the presence of an approval button. A production approval flow must receive a typed Rust outcome for denial, validation, conflict, retry, and audit status.

## Tests and verification

`PreviewHandoffRenderedTests` covers the temporary request, inbox, approved and rejected outcomes, reopening an approved editor, draft gating, replacement without duplicate rows, preservation of ready-made item type and customer fields, and rejection of an old decision after item edits. It captures normal and compact layouts when `EITMAD_UI_CAPTURE_DIR` is set.

Run the focused shell checks:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --filter "FullyQualifiedName~Quotations"
```

`QuotationsPresentationTests` covers Arabic search, status and date filter composition, calculated subtotal and final totals, approval gating, and local approve or reject state. `QuotationsRenderedTests` creates the real WPF window at standard and compact sizes and checks list-to-detail focus, visible detail totals, conditional actions, and accessible names.

## Future Rust vertical

For quotation issuance, extend the durable draft and evaluation boundaries with the accepted workflow specification, versioned typed commands and subscriptions in Rust, and generated native bindings. Rust must own quotation lifecycle, discount policy, relationship-based authorization, explicit scope, atomic approval and audit, durable storage, idempotency, synchronization, and typed recovery. Preserve the read-only detail shape and Arabic mixed-direction amount handling while keeping WPF as a thin adapter.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.
