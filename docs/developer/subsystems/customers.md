---
title: "Maintain customer contact records"
description: "Understand the branch-scoped customer contracts, Arabic and phone search, atomic local-first persistence, authorization, audit, and safe extension boundary."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "customer capability maintainers"
last_verified: "2026-09-28"
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

The customer capability provides durable, branch-scoped contact create, update, get, bounded search, and change notification. Rust owns customer identity, validation, optimistic revisions, ReBAC checks, derived search forms, audit, SQLite state, idempotency replay, event publication, and local-first sync work.

This slice implements contact create and edit plus the Windows projection for customer selection and details. It does not implement archive, restore, merge, extra branch association, or durable quotation attachment. Those operations remain controlled by the [accepted Manager and Receptionist workflow](manager-receptionist-workflows.md).

## Ownership and authority

| Concern | Authority |
| --- | --- |
| Customer orchestration, normalization, audit construction, and sync projection | `crates/customer` |
| Typed values, commands, queries, subscription, event, errors, capability, and schema | `crates/contracts` |
| Manager and Receptionist permission decision in an exact branch scope | `crates/authorization` |
| Storage versions 12 and 14 customer migrations, version 13 local branch binding, and atomic repository | `crates/storage/src/customer.rs` and `crates/storage/src/local_authority.rs` |
| Outbox delivery, incremental download, and shared engine reconciliation | `crates/customer/src/sync.rs` and `crates/sync/src/engine.rs` |
| Customer schema registration, payload validation, branch authorization, and revision conflicts | `server/sync-plane/src/customer.rs` and `server/host` |
| Command, query, subscription authorization, publication, and recovery | `crates/engine-runtime` |
| Generated C# and Swift types | `shells/windows/generated` and `shells/macos/generated` |
| Windows customer selection, detail, and unsaved editor input | `shells/windows/Features/Customers` and `shells/windows/Features/Reception` |

Each customer has a stable UUID and one owning `branch` scope. `CreateCustomer` assigns the UUID in Rust and creates revision `1` with `Active` status. `UpdateCustomer` keeps that identity and status and requires the exact current revision. A local installation currently has one durable engine-created branch. Rust binds active Manager and Receptionist accounts to that branch and issues a second, branch-scoped authorization with the signed-in session. The organization authorization remains in use for organization work. The Windows IPC adapter selects the branch authorization only for customer commands, queries, and subscriptions; WPF never chooses a branch ID. The engine rejects a different branch ID, expired session, inactive account, or missing branch relationship.

## Contracts and bounds

Protocol `1.9` advertises optional capability `eitmad.capability.customer.v1` and schema `eitmad.schema.customer.v1` version `1`. `DesktopSessionState.customerAuthorization` carries the additional engine-issued branch context for customer work; it is absent when the account has no local branch relationship.

| Interaction | Identifier | Result |
| --- | --- | --- |
| Create command | `eitmad.customer.create.v1` | `CustomerCreated(CustomerMutationResult)` |
| Update command | `eitmad.customer.update.v1` | `CustomerUpdated(CustomerMutationResult)` |
| Get query | `eitmad.customer.get.v1` | one exact `Customer` |
| Search query | `eitmad.customer.search.v1` | `CustomerPage` with an optional next customer ID |
| Subscription | `eitmad.customer.changed.subscribe.v1` | resumable discrete stream |
| Event | `eitmad.customer.changed.event.v1` | customer ID, scope, revision, time, and change ID |

Name and one phone are required. Address and notes are optional. Contract values reject surrounding whitespace, unsafe control or bidirectional formatting characters, and values above their declared UTF-8 byte limits. Search pages accept `1..=100` items and use customer UUID order for a stable cursor. An empty term returns a bounded scoped page.

`CustomerMutationResult.potentialDuplicateIds` is advisory. It lists at most ten same-scope records with the same normalized phone. It does not reject creation, establish phone uniqueness, choose a canonical record, or merge identities.

## Windows projection

The Receptionist quotation editor sends bounded name or phone searches through `CustomerClient`. A new keystroke cancels the previous request, and a monotonically increasing request version discards a response that completes after a newer search. Selecting a customer cancels any search in flight; change events refresh that customer by UUID without reopening suggestions. WPF keeps only unsaved form text, the selected customer identity, and the current rendered result set. It does not keep a customer directory or persist contact data.

Creating and editing use the generated `CreateCustomer` and `UpdateCustomer` contracts. `CustomerClient` trims optional address and notes fields and rejects unsafe or oversized optional text and search terms before IPC serialization; Rust remains the validation authority. The editor sends the Rust-returned revision with every update. `eitmad.error.customer-revision-conflict.v1` keeps the unsaved fields open and tells the user in Arabic that no change was saved; the shell does not retry with a newer revision or overwrite the concurrent record. Rust contract-validation fields map to the affected Arabic inputs. Authorization, missing-record, and unavailable failures have separate Arabic messages.

`eitmad.customer.changed.subscribe.v1` refreshes an open customer detail by UUID and refreshes relevant search suggestions. The compact event contains no contact text. Order and quotation fixtures can open the existing detail view only after a scoped Rust lookup identifies one exact name and phone or a temporary quotation carries the selected customer UUID. This preserves the accepted navigation and appearance without adding a Customers destination.

The detail view reads `Customer.syncState` from Rust. It shows `بانتظار المزامنة`, `مؤكد من الخادم`, `رُفضت المزامنة`, or `تعارض يحتاج مراجعة`. Rejected and conflicted records remain visible and cannot be edited from that view until a domain resolution path exists.

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

Each successful contact mutation queues one `ChangeRecord` with `ChangeOperation::Upsert`, the customer UUID as `RecordId`, base and resulting revisions, command idempotency key, and schema-versioned contact payload. The bounded internal batch accepts `1..=50`. `CustomerSyncCycle` stages committed outbox changes in the durable shared `SyncEngine`, submits each through the authenticated direct WAN route, pulls incremental server pages, and acknowledges applied checkpoints. The cycle maps each client's local branch scope to the registered server branch and maps downloaded records back to the local scope. The caller must supply the owner-registered server branch and session and invoke the cycle from a Rust worker thread; this repository does not yet persist desktop enrollment or start a customer sync scheduler during normal engine startup.

The server registers the customer schema and handler. The handler validates the payload identity and revision and resolves the branch through the tenant-scoped `control.branches` registry. A Manager has access through the owning organization relationship; a Receptionist needs the exact branch relationship. PostgreSQL row-level security isolates tenant data. The server preserves the client's change ID and idempotency key, so an exact retry has one effect. A stale revision creates an explicit durable conflict; the server does not overwrite the current record or merge fields automatically. The local contact remains visible with `Conflicted` state and its outbox entry is held for resolution. Domain denial or invalid content becomes `Rejected`. This conservative conflict policy covers same-field concurrent edits and can also flag edits to different fields; no field-aware merge is claimed.

On download, Rust projects each confirmed contact before advancing the shared engine checkpoint. A restart can replay a page safely. Projection does not overwrite a newer pending local revision. After a customer sync cycle, its runtime caller must drain durable publications through `ProductDispatcher::drain_pending_publications`. Version 14 stores rejected and conflicted markers separately from the contact row, so the original contact and queued change survive a restart. The accepted workflow still requires a separate Manager-authorized resolution path before conflicted work can be resubmitted.

## Failure and recovery

| Failure | Result |
| --- | --- |
| Missing read or write permission | Deny before customer state access; mutation denial is audited without contact data |
| Wrong branch scope | No record, count, duplicate suggestion, or event crosses the exact scope |
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
