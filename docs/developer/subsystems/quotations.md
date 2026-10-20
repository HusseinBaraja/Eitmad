---
title: "Extend the quotation review flow safely"
description: "Create and reopen Rust-owned quotation drafts, follow subscription updates, and recover from stale prices, save failures, and conflicts."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Quotation capability maintainers"
last_verified: "2026-10-18"
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

### Automatic recovery of offline saves

The configured engine starts `ProductDispatcher::start_quotation_draft_recovery` for supervised and headless operation. A saved offline draft transfers after connectivity returns or the engine restarts, without another save, approval request, or issuance action. `refresh_approval_draft()` uses the same recovery path during foreground actions.

`QuotationDraftService::recover_pending` reads the existing draft outbox and recovers each revision's author from the successful create, update, or cancel audit committed with that entry. It uses that author's account credential and current local and server authority. It does not use the active desktop account, restore a signed-out session, or submit another author's work under a Manager credential. If an earlier revision's author has lost local write authority, that revision and later revisions stay pending; other drafts can continue.

Each pass reads at most 50 entries, selects only the earliest queued revision of each draft, and allows at most ten seconds for transfer. A rotating in-memory cursor prevents blocked dependencies or denied authors from stopping later drafts. Restart resets the cursor and retains durable work. Shared service clones serialize background and action-triggered transfer. Idle or failed passes wait two seconds; successful passes can continue immediately. Shutdown cancels further passes and lets the current bounded pass finish.

The customer worker delivers customer revisions first. Recovery checks the exact referenced customer revision against the local outbox and retained customer failures before sending a draft. A missing dependency or a rejected/conflicted customer leaves the draft pending. Newer pending customer edits do not block an older delivered revision. The server independently checks the retained customer revision. Draft recovery does not change the saved customer reference or substitute a newer contact snapshot.

Transfer sends `DiscountApprovalAction::Refresh` with the exact saved snapshot and original idempotency key. It never reevaluates prices, changes commercial terms, requests discount approval, assigns a quotation number, or issues a document. Server confirmation projects that exact revision with audit and durable publication. An interrupted local acknowledgement replays the same server receipt. An older acknowledgement cannot replace a newer local saved revision.

Temporary transport or server unavailability leaves the outbox retryable. A server conflict marks the draft `conflicted`; an explicit invalid or denied response marks it `rejected`. Both states retain the input and all outbox revisions, stop automatic resubmission, and require an explicit resolution workflow. They are not converted into network retry states. Local authority loss prevents submission and does not manufacture a server rejection.

`crates/pricing/src/tests/quotation/recovery.rs` covers author changes, local revocation, customer delivery and retained failure, bounded batches, fair progress, exact saved revisions after restart and price changes, lost acknowledgement, duplicate prevention, and terminal versus temporary failures. The live approval test below also runs the actual startup worker through a simulated outage and a fresh dispatcher, then reads each unchanged Draft from a separate Manager client over authenticated TLS. It does not require another save or a desktop sign-in.

Concurrent edits create a server conflict instead of applying a generic merge. The local draft remains visible with `conflicted` state, server conflict identity, and the remote input. Later queued revisions of that draft cannot bypass the first conflict. Rejected and conflicted work remains durable and stops automatic resubmission until [explicit Manager review](#explicit-draft-conflict-resolution).

If a draft projection commits but engine reconciliation fails, a later local edit can refer to a revision absent from the engine. The next cycle replays the outstanding server page before staging and submitting that edit. It reads the outbox again after replay so newly confirmed or conflicted work is not submitted from an obsolete batch. For a rejected or conflicted WAN submission, the cycle removes all queued revisions from the shared sync engine before saving its terminal state; the domain outbox remains durable. If that status write fails, the next cycle retries the server result and status write without dequeuing already held changes. Local content and its durable outbox remain intact throughout recovery.

Failed draft saves record the authorization-denied code for denials, the quotation-draft-conflict code for stale revisions and unresolved conflicts, the contract-invalid code for changed requests under a retained idempotency key, and the quotation-draft-invalid code for other audited errors. Unavailable storage does not create a separate failure audit. Regression tests in `crates/pricing/src/tests/quotation/draft_sync.rs` cover both recovery paths through restart and verify the recorded audit outcomes and codes.

Focused tests in `crates/pricing/src/tests/quotation.rs` cover restart, atomic rollback, retry, stale input, authorization, and competing projection. The ignored live test `quotation_drafts_restart_transfer_replay_and_conflict_through_real_server` in `crates/server-connection/tests/direct_route/quotation_drafts.rs` uses two isolated SQLite authorities, the TLS host, and a disposable PostgreSQL role without superuser or `BYPASSRLS` rights. It covers forged price and customer snapshots, interrupted acknowledgement, replay, second-client delivery, two queued competing edits, immutable revisions, and tenant isolation. Use the certificate and database setup in [server operations](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test), then run:

```powershell
cargo test -p eitmad-server-connection --test direct_route quotation_drafts_restart_transfer_replay_and_conflict_through_real_server -- --ignored
```

### Explicit draft conflict resolution

The accepted policy assigns `eitmad.permission.quotation.draft.resolve.v1` only to Managers. General draft editing remains Receptionist-only. Protocol `1.28` adds `eitmad.capability.quotation-draft-resolution.v1`, the typed `GetQuotationDraftReview` query, and `ResolveQuotationDraft` command. Rust owns both the decision and authorization; native role visibility is only presentation. The server verifies current tenant, organization membership, and branch containment for every read, decision, and retry.

In **عروض الأسعار**, the Manager opens **مراجعة تعارض المسودة**. The screen shows the current server snapshot, the preserved device input, and the server snapshot at conflict detection. Each includes contact revision, public line details, quantities, options, dimensions, saved prices, and totals. The Manager explicitly selects **الاحتفاظ بنسخة الخادم الحالية** or **استخدام التعديل المحفوظ من الجهاز**, then confirms. There is no default decision. The query pages one open conflict at a time and includes up to 50 locally retained revisions within a 2 MiB display budget. Each completed review exposes the next held input. A Manager can review server conflicts from another client; held device work must be reviewed on the device that retains it.

The command binds the draft, branch, source identity or exact held change, expected current server revision, choice, principal, and idempotency key. The server serializes against draft and lifecycle changes, checks current Manager authority, then compare-and-swaps the server revision. It validates the selected full snapshot against retained customer, catalog, option, price, and calculation evidence. Historical valid draft prices remain unchanged after current prices change. Existing issuance checks still require current prices. The decision invalidates pending or approved grants bound to the old draft revision and does not request, approve, or issue anything. Issued, accepted, converted, expired, or cancelled documents reject new draft decisions.

The server commits the new draft revision, both original conflict inputs, merge provenance, exact receipt, audit, and notifications in one transaction. A mandatory audit failure rolls back the decision. Changed retries fail closed; exact authorized retries return the original receipt even after later state changes. SQLite migration `quotation.draft-resolution.v1` (version 30) freezes uncertain requests before network I/O. An unavailable response keeps the original choice and key across restart. **إعادة إرسال القرار السابق** resends that request. A stale or invalid decision requires a refreshed review and a new explicit choice.

Projection atomically moves all queued work descended from superseded state into immutable held inputs. It marks only the explicitly reviewed input complete. Revision numbers above the decision do not bypass review. Repeated decision delivery cannot hold new edits based on the confirmed result. The background worker reads confirmed resolution provenance for blocked drafts under ordinary draft-read authority; it never gains decision permission. Shared sync checkpoint replay and retained server receipts let interrupted clients converge after restart. Held revisions remain available for separate decisions instead of automatic replay or rebasing.

`crates/pricing/src/tests/quotation/resolution.rs` covers Manager-only authority, denied audit, uncertain restart, exact input preservation, long queues, and duplicate projection. The live test `quotation_drafts_restart_transfer_replay_and_conflict_through_real_server` also covers resolution between two conflicting clients, stale decisions, changed retries, immutable history, and later explicit review. The live approval and order scenarios also test audit rollback, forged dependencies, changed active prices, invalidated approvals, and unchanged issued and accepted snapshots. `QuotationDraftResolutionRenderedTests` checks the Arabic screen, explicit choice, keyboard focus, accessible names, unknown outcomes, stale review, and late-response fencing. Use the direct-server prerequisites above, then the corresponding focused test filters.

A different current Manager can recover an uncertain decision on the shared PC by opening draft review. Rust calls `POST /v1/quotation-drafts/reconcile-resolution` with the original author and exact frozen request. Under the resolution tenant and schema locks, the server returns the original receipt or permanently closes its unused key. Late submission cannot apply a closed request. Recovery never resubmits the original decision under the current Manager's identity.

SQLite migration 31 archives the original principal, request, and any receipt before releasing the intent. Recovery has its own audit attributed to the current Manager; the original server decision audit remains unchanged. Server unavailability or a failed local archive/audit retains the pending intent. A new decision needs current review and a new request key. Do not remove uncertain intents by hand.

## Receptionist editing and reopening

`CurrentQuotationView` keeps items, customer fields, and financial details in one vertical scroll area. Details must not consume the item viewport at the 720 × 560 minimum window. Below 1050 DIP of item content width, each line places its identity above labeled quantity and amounts, then its edit, duplicate, and remove actions. Wide layouts retain the aligned item columns. `SalesCatalogRenderedTests.QuotationItemsRemainReadableAndEditableAboveScrollableDetails` checks item visibility, name width, action bounds, focus, and return to the existing editor at all three baseline sizes. If items disappear at a narrow size, check the shared scroll viewport and row bounds before changing field visibility.

**+ عرض سعر جديد** opens an independent catalog editor. The main catalog editor keeps its current local choices. Product variants and Furniture sizes, colors, handles, dimensions, quantities, and stable line identities form the generated evaluator intent. Customer selection and creation still use the [Customer capability](customers.md); an existing contact's address and notes are read-only here and can be edited through Customer Detail. Draft contracts preserve the public customer address and omit private notes.

**حفظ كمسودة** sends `CreateQuotationDraft` or `UpdateQuotationDraft` with the retained expected revision. Rust evaluates and saves in one transaction. WPF accepts amounts, customer snapshots, identity, and revision only from the successful reply. It reports locally saved/pending sync separately from server-confirmed sync. Changes made during a save remain local and are labeled unsaved.

Closing a live quotation editor with unsaved changes or an unconfirmed save opens **تعديلات غير محفوظة**. **حفظ** uses the same draft command and closes only after confirmed success with no newer edits. Validation failures, conflicts, and unavailable replies keep the editor and its input open. Close requests during a draft operation cannot dispose the editor. **تجاهل التعديلات** closes without another save and leaves the stored draft unchanged. **متابعة التعديل** and Escape dismiss the question and retain the fields and lines. An unchanged saved draft closes directly. Session or authority invalidation still closes protected editors immediately. `QuotationEditorClosingTests` exercises these close paths with synthetic engine replies; it does not prove real storage or synchronization.

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

Manager reads span authorized organization branches; Receptionist reads remain branch-scoped. The manager panel groups the requested percentage and amount above the decision area; quotation details remain available in the same scroll view. **موافقة على الخصم** submits approval directly. Expanding **رفض الخصم** shows the labeled multiline **سبب الرفض** field, its requirement, and **تأكيد الرفض**. Both decisions submit the exact request ID, request revision, quotation revision, and opaque commercial fingerprint. Rejection requires a trimmed reason of at most 240 characters. Local and server ReBAC both deny Receptionist decisions, and the server denies self-decision even when one person has both roles. A stale or competing decision returns a conflict. A reused idempotency key returns its exact prior receipt only for the same authenticated principal and input; changed input is invalid.

Migration `0012_quotation_approvals.sql` retains immutable approval history and receipts under forced tenant RLS. The server serializes transitions with draft transfer and commits approval state, redacted audit, retry receipt, and branch/organization events in the same transaction. Audit failure rolls back the transition. PostgreSQL notification wakes live subscriptions; durable scoped event pages remain the source of truth. One host listener is shared by clients. Each page checks current authority, and the desktop reconnects with bounded backoff. Subscription events trigger authorized reads in both shells. Neither shell polls or makes local approval decisions.

Unknown replies retain the original command and key for retry. The shell keeps the previous confirmed decision while the command is pending. A confirmed subscription can resolve an uncertain result. Session changes clear content and fence late responses. Unsaved commercial edits show that the displayed approval cannot authorize the edited input; saved changes obtain server invalidation.

Configure the enrolled Rust engine through these settings:

| Setting | Value |
| --- | --- |
| `EITMAD_QUOTATION_SERVER` | HTTPS server endpoint |
| `EITMAD_QUOTATION_TRUST_PEM` | Approved TLS trust certificate path |
| `EITMAD_QUOTATION_CREDENTIAL_ID` | Native `SecretId` route reference used by Rust `store_account_session`; requests and subscriptions select credentials for the exact actor and device, never token contents |
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

Editor, list, and subscription requests through `QuotationDraftClient` can overlap. Rust owns deadline-bounded credential admission and uses an independent HTTP socket per request; the shell does not queue commands behind unrelated reads. An unavailable result remains a failure and does not infer approval or trigger an automatic command retry. `DiscountApprovalTests.PendingApprovalReadDoesNotBlockDecisionSubmission` holds a read open and checks that a decision can complete. Connected handler tests must await actual submission and the current list refresh before checking the selected approval; awaiting a previous completed handler task does not prove a decision.

`crates/contracts/src/quotation_lifecycle.rs` owns the generated lifecycle contract. Protocol minor 22 is required for `eitmad.capability.quotation-lifecycle.v1` and `eitmad.schema.quotation-lifecycle.v1`. Rust exposes typed issue, validity, revision, cancellation, and acceptance commands, a scoped quotation query, and quotation-change subscriptions. Each reply supplies the actions permitted for the authenticated actor. WPF renders these actions and does not derive authorization from role flags.

`server/sync-plane/src/quotation_lifecycle.rs` serializes transitions with draft, approval, and catalog writes. Issuance checks aggregate and draft revisions, current public catalog eligibility, active customer identity, and the exact approval fingerprint when required. Approval includes the applied validity and issue-day expiry; a request from an earlier calendar date must be requested and approved again. Contact-only changes retain the grant, and issuance freezes its original approved commercial revision. Manager-created revisions invalidate the previous grant.

The server assigns the final `QT` number during first issuance. After an authorized eligibility check, its organization/year counter commits a reservation before the tenant write lock is acquired. The locked issuance checks eligibility and revisions again. A failed or competing issue can leave a reserved-number gap. A number is retained through revision, expiry, and cancellation. Immutable PostgreSQL history, command receipts, redacted audit, and branch/organization events commit together. A retry with the same actor, key, and intent returns the retained result. A changed intent or competing revision fails without replacing a commercial record.

The applied validity starts at 30 calendar days. Manager per-quotation changes require server confirmation and affect only open drafts. Extending an issued or expired quotation creates an editable document revision under the same official number and requires issuance again. Organization default-policy editing is not implemented by these commands. Server time controls expiry at the end of the resulting `Asia/Aden` calendar date. Reads and transitions commit due expiry for the requested records. Active Rust subscriptions use an indexed due-record query, shared per-scope checks at most once per minute, and the same audited expiry transition. Notifications deliver retained events without starting an expiry scan. Initialized pages need no tenant write lock unless an expiry or initialization requires a mutation. An inactive server without reads or subscriptions materializes due expiry on the next authorized access.

Receptionists cancel unnumbered drafts through an audited local-first terminal revision; synchronization preserves the commercial record. Managers cancel issued quotations through the server with a required reason. Cancelled records remain readable and cannot be edited or reissued. There is no delete operation.

The engine retains confirmed scoped history in SQLite. An unavailable server returns a cache page with `server_available = false` and no server-authoritative actions. Local Rust can still permit branch draft edits and cancellation. Confirmed immutable issued snapshots can still be printed. The shell labels the cache as offline. A server denial cannot fall back to cached content. Confirmed Manager revisions project into Receptionist drafts through the existing conflict-preserving projection. Unknown command responses retain the original intent and key; session changes fence late replies.

The Windows role lists use **صادر**, **منتهي**, and **ملغي**, display the official number in an LTR boundary, and show the applied validity and document revision. The Receptionist editor uses **إصدار عرض السعر** only when Rust permits issue. Printed customer documents use the retained customer, line, price, discount, and validity snapshot. Accepted quotations can be converted into confirmed orders through the workflow below.

## Saved customer documents

Protocol `1.24` adds `eitmad.capability.customer-documents.v1`. `eitmad.quotation.customer-document.v1` reads the exact saved draft or retained issued record under `quotation.read` and the record scope. Rust projects only public customer details, saved line descriptions and options, dimensions, quantities, prices, discounts, totals, document identity, dates and status. It never reevaluates the current catalog. Unsaved editor changes do not enter this result. Incomplete saved evaluations cannot produce a document. Approval identities, decisions, fingerprints, purchase costs and internal notes are excluded from the document contract.

Saved drafts show **مسودة محفوظة**, have no official number and cannot print or export. Issued records use Rust's print permission. Both roles can print in scope, including from the confirmed offline cache. Server denial does not fall back to retained content. The shell closes protected previews on session cleanup or denial, discards obsolete replies, and reads Rust again before opening the native print dialog and before submitting its output. Virtual printers use the same gate. There is no separate file-export command.

`QuotationCustomerDocument.CreateSaved` renders the public read model. `CustomerDocumentPages` supplies the same vector pagination to the native preview and print adapter. The preview retains searchable glyphs in an in-memory XPS package, which closes with the preview; it does not create a customer file. Documents use portrait A4 at 96 DIP per inch, 48-DIP side and bottom margins, and a 100-DIP top content inset for the repeated document heading and continuation column headings. Pages advance in reading order and show the current page and page count. WPF wraps table content, and rows can continue across a page boundary. Continuation pages repeat the document number and column headings. The totals block stays together. There is no signature field in these document types. Printer output uses the selected device's printable area without scaling the complete document. Arabic is `ar-YE`, and technical values have separate LTR runs. Output remains legible in monochrome.

`CustomerDocumentLayout` uses the existing brand accent for document identity, places customer details beside document dates and terms, and aligns the item table and financial totals to the printable width. Saved quotations, orders, and drafts share this presentation. The paginator selects the marked item table for continuation headers; identity, details, and totals tables must not supply those headers. Synthetic previews use the same layout and retain their preview warning. No company address, contact detail, or legal copy is added.

Before pagination, `CustomerDocumentLayout.FitTables` fits every table's absolute columns proportionally to the current page width minus its horizontal padding and table margins. Continuation headings use those same fitted widths. Native printing applies the selected printable area before pagination and restores both page size and table widths afterward. Text keeps its native size and wraps within the cells. If print output loses leading digits while the A4 preview is complete, compare the table width with this content width; do not compensate with preview zoom or complete-document scaling. `CustomerDocumentsRenderedTests.PrintableWidthPreservesCompleteValuesAndContinuationColumns` checks 793.7, 745.7, and 697.7 DIP page widths through native XPS, including complete monetary values and document numbers, glyph and clip bounds, unscaled output, row grouping, continuation headings, and totals.

The shared `PrintPreview` starts with one complete page visible after layout, using native `FitToMaxPagesAcross(1)` to fit both viewport dimensions. It refits when its viewport changes until the user selects a native zoom or page-layout command. Fit-width and manual zoom remain available; resize and authorization refresh preserve the user's choice. Track user commands rather than changes to `DocumentViewer.Zoom`: native pagination can adjust that property without user input. Preview zoom does not change the saved document or printer output.

`CustomerDocumentsRenderedTests` covers both document types and draft restrictions at the three baseline window sizes, initial whole-page fitting, resize behavior, and refresh after native percentage, incremental, width, and height zoom commands. It rejects the viewer's native print command through the Rust gate and checks a multi-page preview and native XPS output. Inspect the page images as well as their text. `QuotationLifecycleTests` checks denial and late-reply fencing; Pricing's `customer_document` regression checks saved values and totals after catalog edits. The adapter scenario below checks capability negotiation and missing-document failures through real IPC. A physical printer and PDF printer are separate device checks; native XPS evidence does not establish PDF conformance.

The local display is 1920 × 1080 at 125% scaling. Actual windows measured 1550.4 × 830.4 DIP maximized, 1338.4 × 752.8 DIP default, and 720 × 560 DIP minimum. The 100% scaling baseline remains unverified.

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
