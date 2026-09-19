---
title: "Implement Manager and Receptionist workflows"
description: "Use the accepted permissions, lifecycle, pricing, offline, approval, fulfillment, and delivery rules for Eitmad sales operations."
audience: "developer"
page_type: "reference"
status: "active"
owner: "sales and fulfillment capability maintainers"
last_verified: "2026-09-19"
review_triggers:
  - "Manager or Receptionist permissions, record visibility, money, catalog, quotation, order, work-order, delivery, or offline behavior changes"
keywords:
  - "Manager workflow"
  - "Receptionist workflow"
  - "action permission matrix"
  - "discount approval"
  - "price snapshot"
  - "quotation conversion"
  - "work-order transition"
  - "ر.ي"
  - "موظف الاستقبال"
  - "بانتظار الموافقة"
---

# Implement Manager and Receptionist workflows

This page is the normative implementation specification for the Manager and Receptionist sales-to-delivery workflows. Rust owns every rule, permission, record, calculation, transition, audit result, and synchronization outcome defined here. The Windows shell projects Rust state and keeps only temporary input and navigation state.

The current Windows screens are design evidence. Their fixture values, preview roles, status transitions, `5%` constant, `QT-PREVIEW` numbers, calculated totals, and in-memory handoffs are not implemented product behavior. Where a preview differs from this page, this page controls future implementation.

The accepted product choices are recorded in [ADR-0028](../../decisions/0028-manager-receptionist-sales-workflows.md). The architecture boundaries remain controlled by the [target architecture](../../architecture/target-architecture.md), [scoped authorization](authorization.md), and [synchronization model](synchronization.md).

## Scope and non-goals

This specification covers:

- customer records used by sales;
- manager catalog and pricing changes;
- receptionist quotation drafting and customer acceptance;
- manager discount approval and quotation-validity policy;
- quotation issuance and order conversion;
- manager work-order progression;
- receptionist delivery recording;
- role visibility, offline behavior, audit, numbering, money, units, and price snapshots.

It does not define inventory reservation, procurement, accounting, tax, payment collection, deposits, invoices, production tasks, inspections, rework, installation, returns, or physical printer configuration. Those capabilities must not be inferred from these workflows.

## Authority and invariant summary

- Every record has a tenant, organization, and explicit branch or organization-wide scope.
- A role name in the UI is not authority. Rust evaluates an authenticated principal's relationships for every command and query.
- A Manager and a Receptionist are separate permission bundles. A Manager does not implicitly inherit Receptionist mutations.
- Every mutation writes its audit result atomically with authoritative state. Denial and validation outcomes use redacted stable error identifiers.
- Draft contact, catalog, and quotation work can be durable offline. Publishing or committing business truth requires the server where this page says **Server confirmation**.
- A server-confirmed operation never appears complete while queued, optimistic, or awaiting an unknown result.
- Official numbers come only from the server and are never reused.
- Issued commercial documents keep immutable price and customer snapshots. Later catalog or customer edits do not rewrite history.
- Cancellation and archive preserve history. They are not deletion.

## Roles, scopes, and record visibility

### Scope model

Catalog definitions and published prices are organization-wide. Customers, quotations, orders, work orders, and deliveries have one owning branch inside the organization. A customer can have explicit associations with more than one branch.

The authorization implementation must use scoped ReBAC objects. It must not trust a branch identifier supplied by the shell. The authenticated session, protected object, relationship graph, and command payload must resolve to the same tenant and organization. A branch-scoped object must also resolve to one of the actor's authorized branches.

### Visibility matrix

| Record or projection | Manager | Receptionist |
| --- | --- | --- |
| Organization catalog and published prices | Read all; maintain through authorized commands | Read active published entries and current prices |
| Catalog drafts | Read and edit all organization drafts | No access |
| Customers | Read all organization branches | Read customers associated with an assigned branch |
| Quotations and approval history | Read all organization branches | Read quotations owned by an assigned branch; cannot see internal approval notes outside those records |
| Orders and delivery history | Read all organization branches | Read orders owned by an assigned branch |
| Work orders | Read all organization branches, including production detail | Read status summaries needed to explain readiness; no parts, costs, assignments, or internal production notes |
| Costs and margins | Read as allowed by catalog and pricing permissions | No access |
| Audit and authorization data | Only through separate diagnostic permissions | No access |

Search and subscriptions enforce the same scope as direct reads. Counts, suggestions, customer matching, exports, notifications, and offline caches must not reveal records outside the authorized result set.

## Action and permission matrix

Permission identifiers below are the required semantic actions. The Rust contract catalog must register versioned identifiers such as `eitmad.permission.customer.read.v1` when implementation starts.

| Operation | Required permission | Manager | Receptionist | Authority and confirmation |
| --- | --- | --- | --- | --- |
| Read active catalog and prices | `catalog.read` | Organization | Organization | Authorized Rust query; confirmed cache can be shown offline with freshness |
| Create or edit catalog draft | `catalog.draft.write` | Organization | Denied | Local-first draft; no Receptionist visibility |
| Publish or archive catalog entry | `catalog.publish` | Organization | Denied | Server confirmation |
| Change a published selling price | `pricing.write` | Organization | Denied | Server confirmation |
| Read customer | `customer.read` | All branches | Assigned branches | Authorized Rust query; scoped offline cache allowed |
| Create or edit customer contact | `customer.write` | All branches | Assigned branches | Local-first contact change |
| Add a customer branch association | `customer.branch.assign` | All branches | Denied | Server confirmation |
| Merge, archive, or restore customer | `customer.manage` | All branches | Denied | Server confirmation |
| Create or edit quotation draft | `quotation.draft.write` | Denied | Assigned branches | Local-first draft |
| Request discount approval | `quotation.approval.request` | Denied | Assigned branches | Server confirmation |
| Approve or reject discount | `quotation.approval.decide` | All branches | Denied | Server confirmation; approver differs from requester |
| Manage default or per-quotation validity | `quotation.validity.manage` | All branches | Denied | Server confirmation |
| Issue quotation; approval is required above `5.00%` | `quotation.issue` | Denied | Assigned branches | Server confirmation |
| Record customer acceptance | `quotation.accept` | Denied | Assigned branches | Server confirmation |
| Convert accepted quotation to order | `quotation.convert` | Denied | Assigned branches | Server confirmation |
| Cancel draft quotation | `quotation.draft.write` | Denied | Assigned branches | Local-first while still a draft |
| Cancel issued or accepted quotation | `quotation.cancel` | All branches | Denied | Server confirmation |
| Read order | `order.read` | All branches | Assigned branches | Authorized Rust query; confirmed cache can be shown offline |
| Cancel order before delivery | `order.cancel` | All branches | Denied | Server confirmation |
| Read full work order | `work-order.read` | All branches | Denied | Authorized Rust query |
| Read readiness summary | `order.read` | All branches | Assigned branches | Authorized Rust order projection |
| Start or complete work order | `work-order.transition` | All branches | Denied | Server confirmation |
| Record delivery | `delivery.record` | Denied | Assigned branches | Server confirmation |
| Print issued quotation or confirmed order | Same read permission as source record | Allowed | Allowed in scope | Local OS action from a confirmed immutable snapshot; printing is not a state transition |

If one principal has both role relationships, Rust grants the union of their explicit permissions. The approval rule still prohibits that principal from approving a request they submitted.

## Customer lifecycle

### States

`Active` is available for new quotations. `Archived` remains visible in history but cannot be attached to a new quotation. `Merged` redirects future reads to one canonical customer and cannot be restored as an independent record.

Create assigns a stable UUID and the current branch association. Name and one phone number are required. Address and notes are optional. A customer can have more contact fields in a later contract version, but a shell must not fabricate them.

Phone input keeps the user's original display value. Rust also creates a search form by mapping Arabic and Persian digits to ASCII, removing spaces, hyphens, and parentheses, and preserving one leading `+`. The search form is not a unique key. Shared household or business phone numbers are allowed.

Potential duplicate matches are advisory. They do not block creation and never auto-merge. Only a Manager can merge. Merge must:

1. select one canonical customer;
2. preserve both source UUIDs and audit history;
3. move branch associations and future navigation to the canonical customer;
4. preserve the customer snapshot already stored on issued quotations, orders, and deliveries;
5. reject a cross-tenant or cross-organization merge.

Concurrent contact edits do not use last-write-wins. The local-first customer domain keeps an explicit conflict when the same field changes on different devices. Non-overlapping field changes can merge only through a domain-owned deterministic rule.

## Catalog editing and pricing

### Catalog lifecycles

Furniture and ready-made Products remain distinct catalog types.

- **Furniture** can reference Parts, fixed variants, dimensions, colors, handles, and non-negative option price adjustments.
- **Product** is purchased and sold as-is. It can have supplier-defined variants and purchase cost, but it has no Furniture Parts, BOM, or construction detail.

Each catalog entry follows `Draft → Published → Archived`. A Manager can create a new draft from an existing entry, but duplication creates a new UUID and does not copy history. Publication creates a new immutable catalog revision. Editing a published entry creates a new draft revision; it does not mutate the published revision in place.

Archive removes an entry or option from new selection. It does not change existing quotation, order, or work-order snapshots. A referenced record is never hard-deleted.

Publication requires a name, category, at least one active saleable variant, valid units, and a positive selling price for every active variant. Furniture also requires valid fixed dimensions and its required production definition. Ready-made Product variants require a supplier variant name. Image failure cannot block authoritative text and price data unless the future media policy makes an image mandatory.

### Price model

Only a Manager can publish a selling price. A published price is one whole-YER amount for one catalog revision and variant. Furniture option adjustments are non-negative whole-YER amounts. The effective unit price is:

`variant selling price + selected color adjustment + selected handle adjustment`

The effective price must be positive and must not overflow the Money range. Purchase cost and calculated Furniture cost are internal values. Margin is `selling price - current cost` and is never shown to a Receptionist or customer. A Manager can publish a price below cost after an explicit warning and audited confirmation because the cost model is advisory in this workflow. Draft prices can be zero; published prices cannot.

Changing cost does not silently change a selling price. Changing a published price creates a new price revision and does not reprice an existing quotation line.

## Money, percentages, rounding, and units

### Money

All commercial records in this specification use Yemeni rial, ISO currency `YER`. Arabic UI and customer documents display the unit as **ر.ي**. Contracts and storage use `YER`; they do not persist the localized label.

`Money` is a signed 64-bit integer count of whole rials plus currency. Floating-point values are prohibited. Inputs with a fractional rial are invalid; the shell must not silently round a price input. Negative selling prices, line totals, discounts, quotation totals, and order totals are invalid.

Line total is exact integer multiplication of unit price by whole item quantity. Sum checked line totals to get subtotal. The quotation has one percentage discount represented as basis points from `0` through `10,000`, so `5.00%` is `500` basis points. Calculate the discount once:

`discount = round_half_away_from_zero(subtotal × discount_basis_points ÷ 10,000)`

Do not round each line's share. Final total is `subtotal - discount`. Tax, deposits, payments, and currency conversion are outside this specification.

### Units

- Sale quantity is a positive whole count. Fractional Furniture or Product sale quantities are invalid.
- Physical dimensions are stored as positive integer millimetres. Arabic UI displays and accepts centimetres with at most one decimal place and uses **سم**. Conversion is exact: `1 سم = 10 mm`.
- A quotation, order, and work-order snapshot stores normalized millimetres and the display value used when the document was issued.
- A BOM quantity is an exact decimal with at most six fractional digits and an explicit Unit ID. Binary floating point and free-form unit strings are prohibited.
- Unit conversion requires two Unit records in the same measurement dimension and a versioned exact rational conversion factor. No automatic conversion is allowed between count, length, area, volume, and mass.
- Unit administration is a separate Manager capability. Catalog editing can select active Units but cannot define an unreviewed conversion inline.

## Quotation lifecycle

### States and transitions

| State | Arabic UI | Allowed next state | Rule |
| --- | --- | --- | --- |
| `Draft` | **مسودة** | `PendingApproval`, `Issued`, `Cancelled` | Receptionist can edit locally |
| `PendingApproval` | **بانتظار الموافقة** | `Draft`, `Issued`, `Cancelled` | Commercial content is frozen to the approval fingerprint |
| `Issued` | **صادر** | `Accepted`, `Expired`, `Cancelled` | Official number and immutable issued revision exist |
| `Accepted` | **مقبول** | `Converted`, `Cancelled` | Receptionist recorded customer acceptance |
| `Converted` | **محوّل** | None | Exactly one Order exists |
| `Expired` | **منتهي** | None | Server time passed `valid_until`; create a new revision to continue |
| `Cancelled` | **ملغي** | None | History remains readable |

A draft requires one active customer, one or more valid lines, one owning branch, and one validity value before issue. Browsing and adding lines do not require customer data, but saving a durable draft requires it.

The Manager controls an organization default validity of 30 calendar days. A Manager can change the default and can set a per-quotation validity while the quotation is Draft or Pending Approval. A policy change does not alter existing issued quotations. The Receptionist can use the current default but cannot extend it.

Validity uses the organization time zone, initially `Asia/Aden`. On issue, the server stores `issued_at`, the applied validity days, and an exact `valid_until` instant at the end of the resulting local calendar date. Server time controls expiry. An expired quotation cannot be accepted or converted. To extend an issued but unaccepted quotation, a Manager creates a new quotation revision. The new revision keeps the official quotation number, increments its revision, invalidates prior approval, and requires issuance again.

### Price snapshots and stale drafts

Adding a line to a draft records:

- catalog entry, revision, variant, and selected option IDs;
- normalized dimensions and quantity;
- base price and each option adjustment;
- effective unit price and currency;
- the display names needed to preserve the customer document.

Draft lines do not silently reprice when the catalog changes. At issuance, the server compares every line to the active published catalog revision. If any price, option, availability, or currency differs, issuance fails with a typed stale-price conflict. The Receptionist must explicitly refresh all affected lines before a new approval request or issuance. Version 1 has no stale-price override.

After issue, the snapshot is immutable. Catalog price changes, archive, customer edits, or customer merge do not change the quotation. Order conversion copies the issued snapshot exactly.

## Discount approval and invalidation

A Receptionist can issue a discount from `0.00%` through `5.00%` without approval. A discount above `5.00%` requires one Manager approval. The Manager can approve or reject the requested percentage but cannot replace it with another percentage inside the decision command. Rejection requires a short reason. Approval and rejection include actor, server time, quotation revision, request revision, and audit correlation.

An approval request contains an immutable fingerprint over:

- quotation UUID and revision;
- tenant, organization, and branch;
- customer UUID;
- every line identity, catalog revision, selections, dimensions, quantity, and price snapshot;
- currency, subtotal, discount basis points, discount amount, and final total;
- validity days and proposed `valid_until`.

Changing any fingerprint field invalidates a pending or approved decision. Rust returns the quotation to Draft and preserves the old request and decision as history. Changing customer display contact fields, address, or internal notes without changing the customer UUID does not invalidate approval. Printing, reopening, or changing shell-only view state never invalidates it.

An approval is single-purpose. It authorizes issuance of the exact fingerprint once. It cannot authorize another quotation, revision, issue retry with different content, or validity extension. Exact idempotent retry of the same issue request returns the stored result.

## Acceptance and order conversion

The Receptionist records customer acceptance only for an Issued, unexpired quotation. The acceptance record includes the quotation revision, acceptance method, optional note, actor, and server timestamp. Supported version-1 methods are `InPerson`, `Phone`, and `Written`. The customer does not become an authenticated system actor through this operation.

Conversion is one server transaction. It must:

1. authorize the Receptionist against the exact branch-scoped quotation;
2. verify state is Accepted and not expired or cancelled;
3. verify no Order already references the quotation revision;
4. allocate the official Order number;
5. copy customer, branch, currency, lines, price snapshots, discount, totals, and source quotation identity;
6. create one initial Work Order containing all Furniture lines, or no Work Order for a ready-made-only Order;
7. set the quotation to Converted;
8. write mutation audit and publish committed events atomically.

An idempotency key is mandatory. Exact retry returns the same Order and number. A different request using the key fails. A second conversion intent returns the already-created Order and does not create another record.

## Numbering

Official numbers use organization-wide, server-issued, non-reusable sequences per record type and organization-local calendar year:

- quotation: `QT-YYYY-NNNNN`;
- order: `OR-YYYY-NNNNN`;
- work order: `WO-YYYY-NNNNN`.

The numeric part starts at `00001` for each type and year. Sequence allocation is atomic. A rolled-back transaction can leave a gap; numbers are never recycled or reassigned. Cancellation, archive, merge, and expiry do not free a number. IDs and numbers are different: UUID is identity, while the number is a human reference.

Offline drafts use a localized **غير مرقم** label backed by UUID. They must not invent a temporary number that resembles an official number. Official numbers render in an LTR isolation boundary inside Arabic UI and documents.

## Order, work-order, and delivery lifecycles

### Order state

Order state is derived from committed fulfillment state except for cancellation and delivery:

| State | Arabic UI | Rule |
| --- | --- | --- |
| `Confirmed` | **مؤكد** | Order exists and at least one Work Order has not started |
| `InProduction` | **قيد الإنتاج** | At least one Work Order is In Progress and not all are complete |
| `Ready` | **جاهز** | All Work Orders are complete, or the Order contains only ready-made Products |
| `Delivered` | **تم التسليم** | Receptionist recorded one confirmed delivery |
| `Cancelled` | **ملغي** | Manager cancelled before delivery |

There is no direct command to set `Confirmed`, `InProduction`, or `Ready`. Rust derives them. A Manager can cancel an undelivered Order. Cancellation cancels its Planned or In Progress Work Order, preserves completed Work Orders, and blocks delivery. A Delivered Order cannot be cancelled in version 1.

### Work-order state

Version 1 creates at most one Work Order during conversion. It contains all Furniture lines and excludes ready-made Products. The model must permit later `1..n` Work Orders without changing Order derivation.

Allowed transitions are:

`Planned → InProgress → Completed`

`Planned → Cancelled` and `InProgress → Cancelled` occur only as part of authorized Order cancellation. Completed and Cancelled are terminal. Version 1 does not reopen completion and does not model rework. A Manager supplies a due date and assignment before starting. Completion requires all included Furniture specifications to remain present; inspection is not implied.

### Delivery

Only a Receptionist can record delivery. The Order must be Ready, belong to an assigned branch, and have no existing delivery. The command requires recipient name and acceptance method; delivery notes are optional. Rust uses server time for `delivered_at` and moves the Order to Delivered in the same transaction. Payment status is not read or changed.

Exact idempotent retry returns the original Delivery. A second different delivery intent is rejected. An offline device cannot show Delivered until server confirmation succeeds.

## Offline and server-confirmation behavior

The implementation uses separate synchronization domains so one domain does not mix local-first and server-authoritative semantics:

| Domain or operation | Sync mode | Offline behavior | Confirmation rule |
| --- | --- | --- | --- |
| Customer contact create/edit | Local-first | Commit to scoped local authority, label queued, reconcile later | Local Rust commit is durable; sync is not yet confirmed |
| Customer identity merge/archive/branch association | Server-authoritative | Read cached state only; mutation unavailable | Server result required |
| Catalog draft | Local-first | Create and edit durable private draft | Local Rust commit is durable; not published |
| Catalog publish/archive and published price | Server-authoritative | Mutation unavailable | Server result required |
| Quotation draft | Local-first | Create and edit durable branch-scoped draft | Local Rust commit is durable; not issued |
| Approval request or decision | Server-authoritative | Unavailable | Server result required |
| Quotation issue, revised issue, acceptance, cancellation | Server-authoritative | Unavailable | Server result required |
| Order conversion or cancellation | Server-authoritative | Unavailable | Server result required |
| Work-order start, completion, or cancellation | Server-authoritative | Unavailable | Server result required |
| Delivery | Server-authoritative | Unavailable | Server result required |

The shell can browse an authorized confirmed cache offline. It must show freshness and connectivity. Stale catalog prices can support browsing and draft composition, but they cannot prove issuance eligibility. Immutable issued documents already confirmed by the server can be viewed and printed offline from scoped durable cache. The shell must label any local draft, queued customer change, stale projection, or unresolved conflict accurately.

For a server-confirmed command with an unknown network outcome, keep the same idempotency key and query command status before retry. Do not create a new intent, roll forward local state, or infer success from a closed dialog.

## Required contract and storage shape

Implementation can split crates by vertical capability, but it must preserve one transaction boundary for each aggregate mutation. At minimum, Rust-owned contracts need:

- commands for every mutation in the action matrix;
- scoped queries for catalogs, customer matches, quotations, approval inbox, orders, work orders, and delivery;
- subscriptions for catalog publication, approval decisions, quotation state, order readiness, work-order state, and delivery;
- stable validation, authorization, stale-price, approval-invalidated, state-conflict, duplicate-conversion, offline-required-server, and idempotency errors;
- capability and schema identifiers for each registered sync domain;
- optimistic concurrency revisions on every mutable aggregate.

Storage must preserve immutable issued revisions, price snapshots, approval requests and decisions, official number allocation, conversion identity, terminal transitions, merge aliases, and complete audit. Audit metadata must not contain customer names, phone numbers, addresses, notes, line descriptions, or raw payloads.

## Acceptance scenarios

These scenarios are the minimum behavior proof. Contract, domain, authorization, storage, sync, and shell tests can divide them by boundary, but no shell-only test proves them.

### Visibility and customers

1. Given a Receptionist assigned to Branch A, when they search customers, quotations, or orders, then Branch B data, counts, suggestions, and notifications are absent.
2. Given a Manager in the organization, when they query operational records, then authorized records from all branches are available without exposing another organization.
3. Given two customers with the same normalized phone, when a Receptionist creates the second, then Rust preserves both and returns a duplicate advisory without merging them.
4. Given two organization customers, when a Manager merges them, then history resolves to the canonical customer, issued document snapshots remain unchanged, and an offline or cross-organization merge is denied.
5. Given concurrent edits to the same customer phone, when synchronization reconciles them, then an explicit conflict remains; neither value silently wins.

### Catalog, money, and price snapshots

6. Given a Manager catalog draft, when the device is offline, then the draft saves durably but no Receptionist can see it as published.
7. Given a price of `12,500 YER`, when the Manager publishes online, then Receptionist UI and customer documents display `12,500 ر.ي`, while contracts retain `YER` and integer `12500`.
8. Given a published item below current cost, when the Manager confirms the warning, then publication succeeds with an audit marker; cost and margin remain hidden from Receptionists.
9. Given subtotal `1,010 YER` and discount `5.00%`, when Rust calculates the quotation, then it rounds the midpoint `50.5` away from zero to `51` and produces final total `959 YER`.
10. Given a draft line at price revision 3 and an active catalog price at revision 4, when the Receptionist issues the quotation, then issuance fails with a stale-price conflict until the line is explicitly refreshed.
11. Given an issued quotation, when the Manager changes or archives its catalog entry, then the issued price, names, dimensions, and totals do not change.

### Approval and quotation lifecycle

12. Given a valid draft at exactly `5.00%`, when the Receptionist issues it online, then no Manager approval is required.
13. Given a valid draft above `5.00%`, when the Receptionist requests approval, then the exact fingerprint enters Pending Approval and the quotation cannot issue before approval.
14. Given a pending request, when the requesting principal also has Manager permissions, then Rust denies self-approval.
15. Given an approved request, when quantity, option, price, customer UUID, branch, discount, or validity changes, then approval becomes historical, the quotation returns to Draft, and issuance requires a new request.
16. Given an approved request, when only the customer's display phone or the quotation's internal note changes, then the approval remains valid because the customer UUID and commercial fingerprint are unchanged.
17. Given an organization default of 30 days, when a Manager changes it to 45 days, then new drafts use 45 and already-issued quotations keep their stored validity.
18. Given an issued unaccepted quotation, when a Manager extends it, then Rust creates a new revision under the same number, invalidates prior approval, and requires reissuance.
19. Given an expired quotation, when a Receptionist records acceptance or converts it, then Rust denies the command using server time.

### Conversion, production, and delivery

20. Given an accepted mixed quotation, when the Receptionist converts it, then one transaction creates one Order, one Work Order containing only Furniture lines, copied immutable prices, official numbers, audit, and Converted quotation state.
21. Given an accepted ready-made-only quotation, when it converts, then no Work Order is created and the Order is Ready.
22. Given a successful conversion, when the same idempotency key is retried, then Rust returns the same Order; a second intent cannot create another Order.
23. Given a Planned Work Order, when a Receptionist tries to start it, then Rust denies the action even if the shell exposes a stale control.
24. Given a Manager offline, when they try to start or complete work, then the UI keeps authoritative state unchanged and explains that server confirmation is required.
25. Given the only Work Order changes from Planned to In Progress to Completed, then its Order derives Confirmed, In Production, and Ready without a direct Order-status command.
26. Given a Ready Order in Branch A, when an assigned Receptionist records delivery online, then one Delivery and audit record commit atomically and the Order becomes Delivered.
27. Given an Order that is not Ready, is cancelled, is in another branch, or already has a Delivery, when delivery is attempted, then Rust denies it without changing state.
28. Given an unknown response after conversion or delivery submission, when the client reconnects, then it queries the retained idempotency result before retrying and never invents another number or record.

## Preview mapping and implementation boundary

The accepted workflow deliberately changes some preview concepts:

- Preview `Active` quotation becomes accepted `Issued`, and accepted behavior adds `Accepted` before conversion.
- Preview `New` Order becomes accepted `Confirmed` and is derived from fulfillment.
- Preview `New` Work Order becomes accepted `Planned`.
- Preview `QT-PREVIEW` numbers are replaced by **غير مرقم** until the server allocates an official number.
- Preview `YER` display text becomes Arabic **ر.ي** while contracts remain `YER`.
- Preview whole-number calculations happen to resemble the accepted money model, but only Rust calculations are authoritative.
- Preview customer matching, five-percent approval, one Work Order, and local handoff remain fixtures until the relevant Rust capabilities implement this specification.

Do not migrate preview state. Replace it with typed Rust projections and keep fixtures clearly isolated for design testing until removal.

## Verification and safe extension

Before production implementation, complete a threat model and define the exact command, query, subscription, error, capability, schema, storage migration, and ReBAC relation catalog. Tests must cover every acceptance scenario plus atomic audit failure, tenant and branch isolation, command replay after the idempotency window, integer overflow, invalid unit conversion, cache staleness, and subscription revocation.

Use the [Arabic-first feature checklist](../contributing/arabic-first-feature-checklist.md) for rendered shell verification. Return here when a role, threshold, visibility boundary, status, unit, number, price snapshot, approval fingerprint, validity rule, or offline confirmation rule changes.
