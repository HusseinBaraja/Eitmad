---
title: "Maintain customer contact records"
description: "Understand the scoped customer contracts, Arabic and phone search, atomic local-first persistence, authorization, audit, and safe extension boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "customer capability maintainers"
last_verified: "2026-10-18"
review_triggers:
  - "customer contracts, search normalization, permissions, storage, sync, or lifecycle rules change"
keywords:
  - "customer capability"
  - "العميل"
  - "بحث العملاء"
  - "eitmad.customer.create.v1"
  - "customer.initial.v1"
---

# Maintain customer contact records

The customer capability provides durable branch-scoped contact mutations, authorized branch and organization reads, bounded search, and change notification. Rust owns customer identity, validation, optimistic revisions, ReBAC checks, derived search forms, audit, SQLite state, idempotency replay, event publication, and local-first sync work.

This slice implements contact create and edit plus the Windows projection for customer selection and details. Durable quotation drafts reference the confirmed customer UUID and revision. Managers can search and inspect contacts across their organization branches. Archive, restore, merge, and extra branch association are not implemented. Those operations remain controlled by the [accepted Manager and Receptionist workflow](manager-receptionist-workflows.md).

## Ownership and authority

| Concern | Authority |
| --- | --- |
| Customer orchestration, normalization, audit construction, and sync projection | `crates/customer` |
| Typed values, commands, queries, subscription, event, errors, capability, and schema | `crates/contracts` |
| Manager organization reads and Receptionist assigned-branch authorization | `crates/authorization` |
| Storage versions 12, 14, and 29 customer migrations, version 13 local branch binding, and atomic repository | `crates/storage/src/customer.rs` and `crates/storage/src/local_authority.rs` |
| Outbox delivery, incremental download, and shared engine reconciliation | `crates/customer/src/sync.rs` and `crates/sync/src/engine.rs` |
| Customer schema, scoped directory reads, subscriptions, and revision conflicts | `server/sync-plane/src/customer.rs`, `server/sync-plane/src/customer_directory.rs`, and `server/host` |
| Command, query, subscription authorization, publication, and recovery | `crates/engine-runtime` |
| Generated C# and Swift types | `shells/windows/generated` and `shells/macos/generated` |
| Windows customer selection, detail, and unsaved editor input | `shells/windows/Features/Customers` and `shells/windows/Features/Reception` |

Each customer has a stable UUID and one owning `branch` scope. `CreateCustomer` assigns the UUID in Rust and creates revision `1` with `Active` status. `UpdateCustomer` keeps that identity and status and requires the exact current revision. A local installation currently has one durable engine-created branch. Rust binds active Manager and Receptionist accounts to that branch and issues a second, branch-scoped authorization with the signed-in session. The organization authorization remains in use for organization work. The Windows IPC adapter selects organization authorization for Manager reads and subscriptions, and assigned-branch authorization for Receptionist reads and contact mutations. A Manager detail query includes the Rust-returned owning branch ID; Rust checks its organization membership. The engine rejects a different branch ID, expired session, inactive account, or missing branch relationship.

## Contracts and bounds

Protocol `1.27` advertises optional capability `eitmad.capability.customer.v1` and schema `eitmad.schema.customer.v1` version `1`. `DesktopSessionState.customerAuthorization` carries the additional engine-issued branch context for customer work; it is absent when the account has no local branch relationship.

| Interaction | Identifier | Result |
| --- | --- | --- |
| Create command | `eitmad.customer.create.v1` | `CustomerCreated(CustomerMutationResult)` |
| Update command | `eitmad.customer.update.v1` | `CustomerUpdated(CustomerMutationResult)` |
| Resolve command | `eitmad.customer.resolve-conflict.v1` | authoritative `CustomerConflictResolved(ChangeRecord)` |
| Conflict query | `eitmad.customer.conflicts.v1` | current contact, up to 20 preserved conflicts, continuation, and up to 50 local pending reviews |
| Get query | `eitmad.customer.get.v1` | one exact `Customer` |
| Search query | `eitmad.customer.search.v1` | `CustomerPage` with an optional `(customer_id, branch_id)` cursor |
| History query | `eitmad.customer.history.v1` | authorized quotation/order pages for the exact customer and branch |
| Subscription | `eitmad.customer.changed.subscribe.v1` | resumable discrete stream |
| Event | `eitmad.customer.changed.event.v1` | customer ID, scope, revision, time, and change ID |

Name and one phone are required. Address and notes are optional. Contract values reject surrounding whitespace, unsafe control or bidirectional formatting characters, and values above their declared UTF-8 byte limits. Search pages accept `1..=100` items and use customer UUID, then branch UUID order for a stable cursor. Equal customer UUIDs in different branches remain distinct. An empty term returns a bounded scoped page.

`CustomerMutationResult.potentialDuplicateIds` is advisory. It lists at most ten same-scope records with the same normalized phone. It does not reject creation, establish phone uniqueness, choose a canonical record, or merge identities.

## Organization directory boundary

`POST /v1/customers/read` negotiates protocol 1.27 and the customer capability. The authenticated Rust server authorizes the requested scope before reading tenant-isolated records. Managers can read their organization branches; Receptionists can read only a branch with their explicit relationship. Organization reads do not grant organization writes. Conflict resolution has a separate Manager permission. Existing branch contact writes keep their separate permission.

Each request examines at most 1,000 authorized candidates and returns at most 100 matches. The continuation records the last examined customer and branch, including when no candidate matches. Continue until `next` is absent. Search uses the same Arabic and phone normalization as local customer authority. Direct organization reads require the owning branch ID. Counts derive only from the returned authorized matches; Home shows a lower bound when more pages exist.

Organization contacts are read from the server without a durable organization contact cache. Offline or failed reads are unavailable and clear protected UI data. Receptionist branch reads retain their authorized local-first cache. Customer text and search terms never enter audit metadata.

## Windows projection

The Receptionist quotation editor sends bounded name or phone searches through `CustomerClient`. A new keystroke cancels the previous request, and a monotonically increasing request version discards a response that completes after a newer search. Selecting a customer cancels any search in flight; change events refresh that customer by UUID without reopening suggestions. WPF keeps only unsaved form text, the selected customer identity, and the current rendered result set. It does not retain a complete customer directory or persist contact data.

Creating and editing use the generated `CreateCustomer` and `UpdateCustomer` contracts. `CustomerClient` trims optional address and notes fields and rejects unsafe or oversized optional text and search terms before IPC serialization; Rust remains the validation authority. The editor sends the Rust-returned revision with every update. `eitmad.error.customer-revision-conflict.v1` keeps the unsaved fields open and tells the user in Arabic that no change was saved; the shell does not retry with a newer revision or overwrite the concurrent record. Rust contract-validation fields map to the affected Arabic inputs. Authorization, missing-record, and unavailable failures have separate Arabic messages.

`eitmad.customer.changed.subscribe.v1` refreshes an open customer detail by UUID and refreshes relevant search suggestions.

Managers open **العملاء** from the sidebar or customer matches on Home. The generated contracts carry branch context for each match and detail. The directory retains one page, replaces it on continuation, and offers read-only contact details and paginated sales history. It does not grant quotation drafting, issuance, or acceptance permissions to Managers. A failed protected read clears its data; sign-out clears contacts, search text, history, cursors, and pending reads. Late responses from the previous session cannot repopulate the view.

Rust authorizes the contact first, reads at most 100 quotation and 100 order candidates per history request, then matches both customer UUID and owning branch. Each source has its own continuation, including an empty matching page. Name or phone equality never attaches another customer's history. History uses the existing commercial read permissions and scoped confirmed caches. An unavailable history source does not fabricate a successful empty history.
The compact event contains no contact text. Organization customer subscriptions cover only branches registered to that organization. The server rechecks permission on each read and stream page. Compact customer events invalidate the active page or detail; quotation and order events invalidate history.

The detail view reads `Customer.syncState` from Rust. It shows `بانتظار المزامنة`, `مؤكد من الخادم`, `رُفضت المزامنة`, or `تعارض يحتاج مراجعة`. Rejected and conflicted records remain visible and ordinary editing stays blocked in Rust and WPF. Managers can open the comparison and resolution action from customer details. Receptionists see the restriction and can continue to read authorized contact and sales history.

## Explicit conflict resolution

The [accepted field selection policy](manager-receptionist-workflows.md#customer-conflict-resolution) requires a separate explicit Manager decision. `eitmad.permission.customer.resolve.v1` grants no Receptionist quotation, issuance, acceptance, or delivery mutation. Rust rechecks current Manager authority before and after remote work. The server checks tenant, organization ownership, and the exact branch before loading contact inputs.

`POST /v1/customers/conflicts` and `POST /v1/customers/resolve` require protocol 1.27 and the customer capability. The comparison includes both original contacts and the current server contact. The Manager selects each final field or enters a corrected value. `ResolveCustomerConflict` names the preserved conflict or an exact retained pending input, the owning branch, expected server revision, and final typed contact. A decision is not an ordinary `UpdateCustomer` retry.

The server serializes the decision with normal submissions. It checks the current revision under the same scope lock. One PostgreSQL transaction writes the next authoritative change, preserved conflict inputs, resolved status, merge provenance, idempotent result, and redacted audit. Exact retries return the original result, even when the server revision has since advanced. A changed request with the same key is rejected. A stale review cannot overwrite current server state; the Manager must reload and select fields again. Customer values never enter audit or diagnostics.

SQLite migration 29 freezes the exact request and key before network I/O. An uncertain result survives engine restart and is retried with identical bytes. A definitive failure clears the intent with audit. Successful projection atomically moves the held source and all later entries for that customer out of the delivery outbox. Original bytes remain in `customer_pending_reviews`; only the selected source is marked reviewed. Later edits require separate Manager review and are never automatically rebased or replayed. Repeated projection is safe. A late delivery outcome checks the exact old change ID and cannot block a newer edit.

The Arabic dialog shows local input, server input at conflict, current server input, and the selected final value per field. It requires an explicit choice for every field. Unknown outcomes freeze the form and offer the same-decision retry, including after reopening. Stale outcomes clear selections and require a fresh comparison. Account changes clear rendered inputs and discard late reads. Existing issued documents keep their original customer snapshots.


## Arabic-name and phone search

Storage keeps the entered name, phone, address, and notes unchanged. Rust derives separate indexed search forms during the mutation transaction.

The customer-name profile:

- decomposes Unicode and removes combining marks and tatweel;
- treats alef with madda, hamza, and wasla as alef;
- treats alef maqsura and Persian yeh as Arabic yeh;
- treats ta marbuta as heh and Persian kaf as Arabic kaf;
- maps Arabic and Persian digits to ASCII;
- removes zero-width join controls, collapses whitespace, and lowercases text.

For example, the stored value `إعـتماد القيسي` matches search `اِعتماد` without rewriting the stored name.

The phone profile maps Arabic and Persian digits to ASCII, removes spaces, hyphens, and parentheses, and preserves one leading `+`. The normalized phone is a search value, not a unique key. `+٩٦٧ (٧٧٧) ١٢٣-٤٥٦` remains the displayed value and matches `+967777123456`.

## Atomic mutation and retry flow

1. Rust verifies `eitmad.permission.customer.write.v1` in the exact branch scope.
2. Create assigns a UUID. Update loads the same scoped UUID and calculates `expectedRevision + 1`.
3. One immediate SQLite transaction compares the revision and derives same-phone advisory matches.
4. The transaction writes `customers`, `customer_sync_outbox`, redacted `mutation_audit`, `idempotency_records`, and `publication_outbox`.
5. The dispatcher publishes the compact event only after commit and then removes its publication row.

An exact retry with the same idempotency key and command bytes returns the first stored `CustomerMutationResult`, including the original UUID and advisory list. Reusing the key for different bytes fails. A stale update writes a conflict audit and returns `eitmad.error.customer-revision-conflict.v1`; it does not overwrite the newer customer. If mandatory audit or another transaction write fails, no customer, sync work, idempotency result, or event work commits.

Audit contains stable operation, scope, actor, customer UUID, changed field identifiers, revisions, correlation, causation, and idempotency metadata. It does not contain the name, phone, address, notes, normalized forms, or raw payload.

## Local-first synchronization

Each successful contact mutation queues one `ChangeRecord` with `ChangeOperation::Upsert`, the customer UUID as `RecordId`, base and resulting revisions, command idempotency key, and schema-versioned contact payload. The bounded internal batch accepts `1..=50`. `CustomerSyncCycle::run` stages committed outbox changes in the durable shared `SyncEngine`, submits each through the authenticated direct WAN route, pulls incremental server pages, and acknowledges applied checkpoints. Uploads retain the original author's tenant, principal kind, principal ID, and device. Current write authority and that author's account credential are required.

`crates/engine-cli/src/customer_sync.rs` also runs `DirectCustomerSyncClient::receive_incoming` independently of uploads. Each worker pass selects active imported desktop accounts with an exact local branch assignment. The first account that completes a download serves that branch; denied or unavailable accounts do not prevent attempts with another account. `CustomerSyncCycle::download` sends no local submissions and requires `eitmad.permission.customer.read.v1`, not customer-write permission. The authenticated route also enforces server read authority. Local read authority is checked before connecting and again for every received page, including an empty page.

Incoming sync starts with the ordinary supervised or headless runtime and uses the configured installation route. It does not depend on a customer query, subscription, local edit, or nonempty outbox. The existing worker checks for new history every two seconds when uploads make no progress. A successful upload batch can start the next pass immediately. Customer IPC subscriptions receive the resulting durable publications; they do not open a separate server watcher. Desktop sign-out or account switching does not transfer another author's uploads. Background downloads use current imported account grants and matching device credentials; account or branch revocation stops that account's access without deleting saved work. See [connected workflow setup](../../operations/run-workflow-test-environment.md) for route configuration and [desktop enrollment](../../operations/run-server-authority.md) for installation setup.

The server registers the customer schema and handler. The handler validates the payload identity and revision and resolves the branch through the tenant-scoped `control.branches` registry. A Manager has access through the owning organization relationship; a Receptionist needs the exact branch relationship. PostgreSQL row-level security isolates tenant data. The server preserves the client's change ID and idempotency key, so an exact retry has one effect. A stale revision creates an explicit durable conflict; the server does not overwrite the current record or merge fields automatically. The local contact remains visible with `Conflicted` state and its outbox entry is held for resolution. Domain denial or invalid content becomes `Rejected`. This conservative conflict policy covers same-field concurrent edits and can also flag edits to different fields; no field-aware merge is claimed.

On download, Rust projects each confirmed contact before advancing the shared engine checkpoint. A restart can replay a page safely. Projection preserves any pending local edit and its rejected or conflicted marker. If incoming history advances before an offline edit enters the shared queue, staging retains the edit's original base revision so the server can return its conflict; it still rejects out-of-order local queue entries. Incoming and outgoing cycles share one serialized worker and durable checkpoint per branch. Reconnect and restart resume after that checkpoint without resetting pending work. After a customer sync cycle, its runtime caller must drain durable publications through `ProductDispatcher::drain_pending_publications`. Version 14 stores rejected and conflicted markers separately from the contact row, so the original contact and queued change survive a restart. The explicit Manager resolution path retires the held input; it never resubmits that input as an ordinary edit.

## Failure and recovery

| Failure | Result |
| --- | --- |
| Missing read or write permission | Deny before customer state access; mutation denial is audited without contact data |
| Unauthorized branch or organization | No record, count, duplicate suggestion, history, or event crosses the authorized scope |
| Customer missing | Return `eitmad.error.customer-not-found.v1`; an update attempt writes a redacted failure audit |
| Revision mismatch | Preserve current state and return the actual revision |
| Event publication failure | Keep the committed publication row for bounded startup recovery |
| Delivery or acknowledgement failure | Keep the customer outbox and last durable engine checkpoint; retry the same change identity |
| Server rejection | Keep the local contact, mark `Rejected`, and hold its outbox entry |
| Concurrent remote revision | Keep both inputs in server/shared-engine conflict state and mark the local contact `Conflicted` |
| Audit or storage failure | Roll back the complete mutation and return `eitmad.error.customer-unavailable.v1` |
| Unknown command outcome | Retry the same bytes with the same idempotency key |

Do not inspect or repair contact rows manually. Follow [local storage recovery](../../operations/recover-local-storage.md) for database failure.

## Verification and extension

`customer_resolution_converges_after_restart_and_lost_reply_without_replaying_dependents` in `crates/server-connection/tests/direct_route/customer_resolution.rs` uses two SQLite clients, the application dispatcher, authenticated TLS HTTP, and PostgreSQL. It covers conflict/restart, stale decisions, lost success replies, exact and changed retries, separate dependent review, scope and Receptionist denial, audit rollback, and convergence. Use the [direct-route prerequisites](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test) with a fresh disposable database, then run:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route customer_resolution_converges -- --ignored
```

Customer unit tests cover atomic projection/hold rollback and delayed rejection after a newer edit. `CustomerResolutionRenderedTests` checks explicit selection, frozen retries, recovered intent, stale reload, and session clearing in the native Arabic dialog.

`manager_customer_visibility_is_bounded_scoped_and_live` in the ignored `direct_route` suite exercises PostgreSQL tenant isolation, authenticated HTTP, duplicate UUID pagination across two branches, Arabic search, direct reads, Receptionist denial, and live customer updates. Use the same disposable database and TLS prerequisites as the direct-route tests below. Windows `CustomerDirectoryTests` and `ManagerCustomersRenderedTests` check page replacement, stale completion after deactivation, read-only details, keyboard focus, and rendered layouts.


The customer `incoming_` tests cover multi-page catch-up with an empty outbox, checkpoint recovery after reopen, incoming updates, exact branch rejection, revoked readers, and protection of unstaged local edits through a later server conflict. `downloaded_revision_does_not_block_staging_a_committed_offline_edit` in the shared engine tests protects stale-base staging and local queue ordering.

The ignored direct-route test `clean_installation_receives_customers_in_the_ordinary_runtime` enrolls two separate installations and starts two real supervised engine processes. It verifies customer creation, a later update, and catch-up after restarting the receiver while its customer outbox remains empty. Use a fresh disposable PostgreSQL database and TLS certificates from the [direct-route prerequisites](../../operations/run-server-authority.md#run-the-direct-desktop-connection-test), build the current engine, set `EITMAD_CUSTOMER_SYNC_ENGINE` to its absolute executable path, then run:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route clean_installation_receives_customers_in_the_ordinary_runtime -- --ignored
```

Run:

```powershell
cargo test -p eitmad-customer -p eitmad-authorization -p eitmad-storage -p eitmad-engine-runtime
```

```powershell
npm run contracts:verify --prefix crates/contracts/codegen
```

The focused customer tests cover restart persistence, exact text preservation, Arabic-name and phone search, stable paging, permission denial, same-database branch isolation, exact retry identity, duplicate advisory without merge, audit rollback, rejected-state recovery, and concurrent-edit conflict. The ignored live `two_isolated_customer_engines_recover_and_preserve_conflicts` test uses two independent engine data directories with distinct local tenant and branch IDs, two registered server devices, TLS, and a disposable PostgreSQL database; it verifies mapped-scope download, replay, restart recovery, conflict preservation, and cross-tenant row-level security.

Add lifecycle management only through separate Manager-authorized server-confirmed commands. Preserve existing customer UUIDs, source history, immutable commercial-document snapshots, and cross-organization rejection. Do not make phone unique and do not add automatic merge behavior.

Related pages: [accepted workflow](manager-receptionist-workflows.md), [authorization](authorization.md), [local storage](local-storage.md), [synchronization](synchronization.md), [contract layer](contract-layer.md), [customer storage](customers.md).
