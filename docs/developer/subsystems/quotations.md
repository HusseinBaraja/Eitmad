---
title: "Extend the quotation review flow safely"
description: "Create and reopen Rust-owned quotation drafts, follow subscription updates, and recover from stale prices, save failures, and conflicts."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Quotation capability maintainers"
last_verified: "2026-10-08"
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

The Windows **عروض الأسعار** pages show authorized branch drafts for both roles. Receptionists create and reopen mixed Product/Furniture drafts through Rust; Managers review the same synchronized draft from their own client. Receptionists request discount approval from the server. Managers approve or reject from their own client. Receptionists issue eligible quotations. Managers change draft validity, create revisions of issued or expired quotations, and cancel issued quotations. Both roles can print authorized confirmed snapshots. Receptionists record customer acceptance and convert eligible accepted quotations into server-confirmed orders.

Production lifecycle, `5.00%` threshold, approval fingerprint, price snapshot, validity, numbering, permission, and offline behavior is accepted in the [Manager and Receptionist workflow specification](manager-receptionist-workflows.md). Draft persistence, discount approval, issuance, expiry, revision, and cancellation implement the current part of that lifecycle. Isolated fixture `Active`, conversion, and `QT-PREVIEW` behavior does not override the accepted specification.

## Ownership and current boundary

`shells/windows/Features/Quotations/QuotationDraftClient.cs` consumes generated commands, queries, and subscriptions. `QuotationDraftList.cs` projects Rust snapshots into the existing native lists and read-only details. `Features/Reception/QuotationDraftState.cs` stages local edits, sends draft intent, retains retry identity, and reopens saved snapshots. `MainWindow` activates the appropriate role list after sign-in and clears both projections and editors at session end. The Windows process adapter negotiates the draft capability/schema and routes draft commands, queries, and subscriptions through the engine-issued branch authorization used by customers.

Rust owns evaluation in `crates/pricing/src/quotation.rs` and durable draft behavior in `crates/pricing/src/drafts.rs`. The shell has no database or server access. The existing customer selection, catalog configuration, and navigation remain native presentation concerns.

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

If a draft projection commits but engine reconciliation fails, a later local edit can refer to a revision absent from the engine. The next cycle replays the outstanding server page before staging and submitting that edit. It reads the outbox again after replay so newly confirmed or conflicted work is not submitted from an obsolete batch. For a rejected or conflicted submission, the cycle removes all queued revisions of that draft before saving its terminal state. If that status write fails, the next cycle retries the server result and status write without dequeuing already held changes. Local content and its durable outbox remain intact throughout recovery.

Failed draft saves record the authorization-denied code for denials, the quotation-draft-conflict code for stale revisions and unresolved conflicts, the contract-invalid code for changed requests under a retained idempotency key, and the quotation-draft-invalid code for other audited errors. Unavailable storage does not create a separate failure audit. Regression tests in `crates/pricing/src/tests/quotation/draft_sync.rs` cover both recovery paths through restart and verify the recorded audit outcomes and codes.

Focused tests in `crates/pricing/src/tests/quotation.rs` cover restart, atomic rollback, retry, stale input, authorization, and competing projection. The ignored live test `quotation_drafts_restart_transfer_replay_and_conflict_through_real_server` in `crates/server-connection/tests/direct_route/quotation_drafts.rs` uses two isolated SQLite authorities, the TLS host, and a disposable PostgreSQL role without superuser or `BYPASSRLS` rights. It covers forged price and customer snapshots, interrupted acknowledgement, replay, second-client delivery, two queued competing edits, immutable revisions, and tenant isolation. Use the certificate and database setup in [server operations](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test), then run:

```powershell
cargo test -p eitmad-server-connection --test direct_route quotation_drafts_restart_transfer_replay_and_conflict_through_real_server -- --ignored
```

## Receptionist editing and reopening

**+ عرض سعر جديد** opens an independent catalog editor. The main catalog editor keeps its current local choices. Product variants and Furniture sizes, colors, handles, dimensions, quantities, and stable line identities form the generated evaluator intent. Customer selection and creation still use the [Customer capability](customers.md); an existing contact's address and notes are read-only here and can be edited through Customer Detail. Draft contracts preserve the public customer address and omit private notes.

**حفظ كمسودة** sends `CreateQuotationDraft` or `UpdateQuotationDraft` with the retained expected revision. Rust evaluates and saves in one transaction. WPF accepts amounts, customer snapshots, identity, and revision only from the successful reply. It reports locally saved/pending sync separately from server-confirmed sync. Changes made during a save remain local and are labeled unsaved. Closing a separate editor discards local edits; it does not change the stored draft.

Reopening queries `GetQuotationDraft` and retains the saved prices, exact references, discount basis points, options, quantities, and dimensions. It does not silently substitute current catalog prices. **تعديل** loads current public variants and preserves compatible selected options and dimensions. The original quotation line stays unchanged until the user accepts the checked selection. The user must then save the draft. Changed or withdrawn definitions and stale price revisions produce explicit Rust field failures. Refresh/review is required; a price failure does not overwrite the saved draft.

A failed save preserves local input and shows validation, denial, unavailable, or conflict guidance. An uncertain response retains the exact command and idempotency key. Retry first confirms that command, even if newer local edits exist; it does not create another draft. A successful retry labels any newer input unsaved. Revision conflicts block further saves. **إعادة فتح النسخة المحفوظة** asks before replacing local edits and reads the current draft. Rejected/conflicted sync state remains visible and disables saving; the future conflict-resolution workflow is unavailable.

## Both role lists and subscriptions

Both lists query Rust branch drafts with bounded pages and the returned UUID cursor. The existing customer/phone search, status/date filters, row/detail navigation, customer link, Arabic labels, and LTR amount isolation remain. Drafts show **بدون رقم رسمي**, their Rust totals, and a separate local-pending, confirmed, rejected, or conflicted sync label. Repeated subscription notifications replace rows by current snapshot; no editor publishes a list row or approval outcome locally.

`QuotationDraftChanges` notices and resync/reconnect signals trigger authorized list reads. WPF does not poll. Subscription-driven refresh preserves the selected detail by draft identity. Denial or unavailable reads remove retained list content and show explicit feedback instead of samples. Editor notices refresh sync state without replacing unsaved lines; a newer revision requires explicit reopen. Session changes cancel reads, clear customer/draft projections, and close separate editors. Delayed results cannot restore the prior session.

Managers receive read-only commercial detail and server decision controls for pending requests. A draft whose evaluation requires approval has no grant until an authorized Receptionist requests approval and a different authorized Manager decides. The server-confirmed lifecycle below supplies issuance, official numbering, validity management, and printing. Acceptance and order conversion use the confirmed workflow described below. `ReceptionHandoffPreview` and `QuotationPreviewProjection` are isolated fixture helpers; production draft persistence and role handoff do not use them.

## Server-confirmed discount approval

Protocol 1.20 adds the generated request, decision, list, and change-subscription contracts from `crates/contracts/src/quotation_approval.rs`, capability `eitmad.capability.quotation-approval.v1`, and schema `eitmad.schema.quotation-approval.v1`. Rust Pricing owns the commercial fingerprint and decision checks in `approval.rs`. The server sync plane owns atomic transitions in `quotation_approval.rs`; the desktop runtime composes authenticated transport without allowing WPF to supply a price or approval state. Use the [accepted approval policy](manager-receptionist-workflows.md#discount-approval-and-invalidation) for threshold, roles, and invalidation rules.

The persisted request freezes the saved quotation revision, tenant, organization, branch, customer UUID, line configuration and price snapshots, integer totals, discount basis points, applied quotation validity (initially 30 days), and proposed end-of-day expiry in Asia/Aden. Contact and display-only changes retain this original commercial revision. A commercial or validity change invalidates pending and approved requests; every previous request and decision remains immutable. Approval alone cannot produce an issued document; the Receptionist must confirm issuance separately.

**طلب موافقة** saves pending editor input, transfers saved draft revisions in order, and requests server confirmation. Historical customer and catalog revisions must already exist on the server. The server independently validates every promoted snapshot and checks the current public selling prices before it creates a request. Missing dependencies or unavailable transport leave the local draft pending. Conflicting revisions fail safely and require reload or the future conflict-resolution workflow.

Manager reads span authorized organization branches; Receptionist reads remain branch-scoped. **موافقة** and **رفض** submit the exact request ID, request revision, quotation revision, and opaque commercial fingerprint. Rejection requires a trimmed reason of at most 240 characters. Local and server ReBAC both deny Receptionist decisions, and the server denies self-decision even when one person has both roles. A stale or competing decision returns a conflict. A reused idempotency key returns its exact prior receipt only for the same authenticated principal and input; changed input is invalid.

Migration `0012_quotation_approvals.sql` retains immutable approval history and receipts under forced tenant RLS. The server serializes transitions with draft transfer and commits approval state, redacted audit, retry receipt, and branch/organization events in the same transaction. Audit failure rolls back the transition. PostgreSQL notification wakes live subscriptions; durable scoped event pages remain the source of truth. One host listener is shared by clients. Each page checks current authority, and the desktop reconnects with bounded backoff. Subscription events trigger authorized reads in both shells. Neither shell polls or makes local approval decisions.

Unknown replies retain the original command and key for retry. The shell keeps the previous confirmed decision while the command is pending. A confirmed subscription can resolve an uncertain result. Session changes clear content and fence late responses. Unsaved commercial edits show that the displayed approval cannot authorize the edited input; saved changes obtain server invalidation.

Configure the enrolled Rust engine through these settings:

| Setting | Value |
| --- | --- |
| `EITMAD_QUOTATION_SERVER` | HTTPS server endpoint |
| `EITMAD_QUOTATION_TRUST_PEM` | Approved TLS trust certificate path |
| `EITMAD_QUOTATION_CREDENTIAL_ID` | Native `SecretId` reference for the authenticated account and device; never token contents |
| `EITMAD_QUOTATION_ORGANIZATION_ID` | Enrolled server organization UUID |
| `EITMAD_QUOTATION_BRANCH_ID` | Enrolled server branch UUID |

The stored server credential must match the local signed-in principal and tenant. The enrollment mapping is configured in Rust and cannot be changed by a shell command. If the server settings are absent, approval reads and transitions return unavailable; local drafting remains available. Use the [server connection setup](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test) for disposable integration tests.

```powershell
cargo test -p eitmad-pricing discount_
cargo test -p eitmad-engine-runtime discount_decision_direct
cargo test -p eitmad-server-connection --test direct_route discount_approval_cross_client_live_replay_invalidation_and_rejection -- --ignored
```

The live test uses distinct authenticated principals and separate clients, real TLS, and a disposable PostgreSQL role without superuser or `BYPASSRLS`. Lifecycle checks also cover exact issuance retry, competing issue and cancellation/revision requests, frozen snapshots, required approval, expiry on both clients, and stale catalog prices. It checks live request and decision delivery, approval, rejection, exact retry, duplicate and stale decisions, commercial invalidation, self-decision, Receptionist denial, immutable history, audit, and tenant isolation. `DiscountApprovalTests` verifies shell confirmation, subscription return, retry identity, and session fencing. `DiscountApprovalRenderedTests` checks the changed native controls at the three baseline sizes.

## Server-confirmed lifecycle

`crates/contracts/src/quotation_lifecycle.rs` owns the generated lifecycle contract. Protocol minor 22 is required for `eitmad.capability.quotation-lifecycle.v1` and `eitmad.schema.quotation-lifecycle.v1`. Rust exposes typed issue, validity, revision, cancellation, and acceptance commands, a scoped quotation query, and quotation-change subscriptions. Each reply supplies the actions permitted for the authenticated actor. WPF renders these actions and does not derive authorization from role flags.

`server/sync-plane/src/quotation_lifecycle.rs` serializes transitions with draft, approval, and catalog writes. Issuance checks aggregate and draft revisions, current public catalog eligibility, active customer identity, and the exact approval fingerprint when required. Approval includes the applied validity and issue-day expiry; a request from an earlier calendar date must be requested and approved again. Contact-only changes retain the grant, and issuance freezes its original approved commercial revision. Manager-created revisions invalidate the previous grant.

The server assigns the final `QT` number during first issuance. After an authorized eligibility check, its organization/year counter commits a reservation before the tenant write lock is acquired. The locked issuance checks eligibility and revisions again. A failed or competing issue can leave a reserved-number gap. A number is retained through revision, expiry, and cancellation. Immutable PostgreSQL history, command receipts, redacted audit, and branch/organization events commit together. A retry with the same actor, key, and intent returns the retained result. A changed intent or competing revision fails without replacing a commercial record.

The applied validity starts at 30 calendar days. Manager per-quotation changes require server confirmation and affect only open drafts. Extending an issued or expired quotation creates an editable document revision under the same official number and requires issuance again. Organization default-policy editing is not implemented by these commands. Server time controls expiry at the end of the resulting `Asia/Aden` calendar date. Reads and transitions commit due expiry for the requested records. Active Rust subscriptions use an indexed due-record query, shared per-scope checks at most once per minute, and the same audited expiry transition. Notifications deliver retained events without starting an expiry scan. Initialized pages need no tenant write lock unless an expiry or initialization requires a mutation. An inactive server without reads or subscriptions materializes due expiry on the next authorized access.

Receptionists cancel unnumbered drafts through an audited local-first terminal revision; synchronization preserves the commercial record. Managers cancel issued quotations through the server with a required reason. Cancelled records remain readable and cannot be edited or reissued. There is no delete operation.

The engine retains confirmed scoped history in SQLite. An unavailable server returns a cache page with `server_available = false` and no server-authoritative actions. Local Rust can still permit branch draft edits and cancellation. Confirmed immutable issued snapshots can still be printed. The shell labels the cache as offline. A server denial cannot fall back to cached content. Confirmed Manager revisions project into Receptionist drafts through the existing conflict-preserving projection. Unknown command responses retain the original intent and key; session changes fence late replies.

The Windows role lists use **صادر**, **منتهي**, and **ملغي**, display the official number in an LTR boundary, and show the applied validity and document revision. The Receptionist editor uses **إصدار عرض السعر** only when Rust permits issue. Printed customer documents use the retained customer, line, price, discount, and validity snapshot. Accepted quotations can be converted into confirmed orders through the workflow below.

## Verification

Run the focused shell proof:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter "FullyQualifiedName~QuotationLifecycle|FullyQualifiedName~DiscountApproval|FullyQualifiedName~QuotationDraft|FullyQualifiedName~QuotationEvaluation|FullyQualifiedName~SalesCatalogAuthority"
```

`QuotationDraftTests` covers mixed intent, immutable reopen, manager subscription refresh, typed price failures, explicit review, lost-reply idempotency, unsaved edits, revision conflict, and session fencing. `QuotationDraftRenderedTests` exercises the existing save and edit controls, failed save, reopened Furniture configuration, Manager list/detail, focus, and Arabic accessible names. Baseline cases request 1920 × 1080 fullscreen, 1338 × 753, and 720 × 560; report actual dimensions and scaling when the display cannot provide the exact baseline. Set `EITMAD_UI_CAPTURE_DIR` for synthetic captures. Rendering uses typed synthetic replies; real durability and server transfer require the Rust and adapter proofs below.

`cargo test --locked -p eitmad-pricing quotation` covers evaluator policy, persistence restart, mandatory-write rollback, stale input, retry, authorization, and competing projection. The real-server test documented above now creates a mixed Product/Furniture draft, reopens the SQLite authority, delivers it to a second client with Manager read authority, rejects Manager mutation, and then enables competing-edit checks. Run it against a fresh disposable PostgreSQL database with the documented TLS setup. The Windows adapter real-engine scenario verifies capability negotiation, branch-scoped Manager reads/subscription, Manager write denial, and typed Receptionist save validation through real IPC:

```powershell
dotnet run --project platform-adapters/windows/tests/Eitmad.Platform.Windows.Tests.csproj --configuration Release -- --engine <absolute-path-to-target/debug/eitmad-engine-cli.exe>
```

`QuotationLifecycleTests` covers Rust-supplied actions, unknown-response retry, and session fencing. `QuotationLifecycleRenderedTests` checks both role action screens, focus, and Arabic accessible names at the baseline sizes.

Standalone fixture tests still cover print preview and conversion presentation. Fixtures cannot request or decide approval. Those tests do not establish production authority.

Return to the [Windows shell subsystem guide](windows-native-shell.md) for shared layout and trust-boundary rules.

## Acceptance and order conversion

Protocol `1.22` adds server-confirmed acceptance of a valid Issued document. Receptionists record the acceptance method and optional note; Rust retains the actor, server time and document revision. Accepted quotations expose conversion to their assigned Receptionist. Successful conversion marks the quotation Converted and opens the confirmed order. The retained accepted commercial snapshot remains available from the order. See [orders](orders.md) for numbering, competing conversion, permissions, fulfillment and durable retry.
