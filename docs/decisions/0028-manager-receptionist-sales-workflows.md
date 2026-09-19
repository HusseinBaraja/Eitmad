---
title: "ADR-0028: Define Manager and Receptionist sales workflows"
description: "Records the accepted role, scope, commercial lifecycle, money, numbering, approval, offline, fulfillment, and delivery choices."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "product and architecture maintainers"
last_verified: "2026-09-19"
review_triggers:
  - "Manager or Receptionist authority, commercial lifecycle, currency, offline policy, or branch visibility changes"
keywords:
  - "Manager Receptionist decision"
  - "discount approval"
  - "YER"
  - "server confirmation"
---

# ADR-0028: Define Manager and Receptionist sales workflows

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-09-19
> - Decision owners: Product and architecture maintainers
> - Last verified: 2026-09-19
> - Review trigger: Manager or Receptionist authority, scope, commercial lifecycle, currency, offline policy, or branch visibility changes
> - Supersedes: None
> - Superseded by: None

## Context and problem

Eitmad has Arabic-first Windows previews for manager catalogs, pricing, quotations, orders, work orders, customers, and receptionist sales. Those screens intentionally keep fixture data in memory and do not define production rules. Product implementation needs one accepted workflow before Rust contracts, authorization, storage, synchronization, and shell bindings can be designed.

## Decision drivers

- Keep Rust authoritative and the native shell thin.
- Let reception continue safe draft work during intermittent connectivity.
- Require central confirmation for prices, approvals, commitments, production state, and delivery.
- Isolate branch records while allowing organization-wide management and catalog use.
- Preserve commercial history and make retry outcomes deterministic.
- Use Arabic-first money and unit presentation without changing stable contract identifiers.

## Considered options

### Treat the Windows preview as the product rule

This would be quick but would move transient constants and shell state into business authority. It would not supply scope, audit, durable identity, conflicts, retries, or server confirmation.

### Make every operation server-authoritative

This would simplify central ordering but would prevent useful customer, catalog-draft, and quotation-draft work during normal connectivity loss.

### Separate local drafts from confirmed business truth

This supports offline preparation and keeps irreversible or organization-wide outcomes centrally ordered. It requires explicit promotion boundaries and clear UI states.

## Decision

Adopt the separate-draft model defined by [the normative workflow specification](../developer/subsystems/manager-receptionist-workflows.md).

Managers can see all organization branches and control catalog publication, pricing, discount decisions, quotation validity, order cancellation, and work-order progression. Receptionists operate only in assigned branches and control customer contact work, quotation drafts and issuance within policy, customer acceptance, order conversion, and delivery. Role permissions remain explicit; Manager is not an implicit super-role for Receptionist mutations.

Use `YER` as whole-rial integer money and display **ر.ي** in Arabic UI. The discount threshold is above `5.00%`. Quotation validity defaults to 30 days and is adjustable by the Manager. Official yearly organization-wide sequences use `QT`, `OR`, and `WO`. Customer contact, catalog draft, and quotation draft work is local-first. Publication, prices, approvals, issued commitments, conversion, production transitions, cancellation, and delivery require server confirmation.

## Architecture and contract impact

Rust product verticals must define typed, versioned commands, queries, subscriptions, errors, capabilities, and schemas for the accepted workflow. The shell must not preserve preview calculations or transitions as authority. Server-confirmed operations need idempotency and result lookup after an unknown outcome. Official numbers are server allocated.

The workflow specification is the canonical source for detailed states, calculations, matrices, fingerprints, and acceptance scenarios. This ADR records why those choices are durable and must not become a duplicate contract listing.

## Storage and sync impact

Separate local-first draft domains from server-authoritative identity, publication, commercial-document, fulfillment, and delivery domains. Promotion copies validated snapshots through an idempotent server command. Issued revisions, approvals, numbering, conversion, terminal transitions, and audit are durable history.

Local-first conflicts do not use generic last-write-wins. Server-required operations remain unavailable offline and do not appear complete before confirmation.

## Security impact

Every record has explicit tenant, organization, and branch or organization-wide scope. ReBAC protects every operation. Search, counts, caches, subscriptions, and notifications use the same scope. Approval prohibits the requester from approving their own discount request. All mutations produce complete redacted audit without customer payloads.

## Arabic UX impact

Arabic labels are authoritative presentation terms. Money displays **ر.ي**, dimensions display **سم**, and official Latin identifiers use an LTR isolation boundary. The shell must distinguish local draft, queued, stale, pending approval, server confirmed, denied, and conflict states.

## Consequences and tradeoffs

### Positive

- Implementation agents have one workflow and one acceptance set.
- Reception can prepare work offline without fabricating a commercial commitment.
- Price, approval, order, production, and delivery history stays deterministic.
- Branch isolation applies to all read and notification paths.

### Negative

- Users cannot approve, issue, convert, progress production, cancel committed records, or record delivery during a server outage.
- Promotion between draft and confirmed domains adds contract and storage work.
- Exact snapshots and immutable history consume more storage than mutable rows.

### Risks

- A shell can present stale prices as current. Mitigate with freshness labels and mandatory server validation at issue.
- Duplicate customer records can occur across branches. Mitigate with advisory matching and audited Manager merge.
- A user can be unsure whether a timed-out command committed. Mitigate with stable idempotency keys and result lookup.

## Verification

The decision was reviewed against the domain glossary, target architecture, accepted authorization, storage, sync, and server decisions, and the current Windows Manager and Receptionist preview screens on 2026-09-19. Product implementation must pass the acceptance scenarios in the normative specification.

## Follow-up work

- Sales capability maintainers: define the Rust contract and storage boundaries before changing a preview into production behavior.
- Security maintainers: threat-model customer, approval, commercial-document, and branch-access boundaries.
- Windows shell maintainers: replace fixtures only after generated or validated bindings exist.

## Related decisions and documents

- [Manager and Receptionist workflow specification](../developer/subsystems/manager-receptionist-workflows.md)
- [ADR-0001: Rust core product authority](0001-rust-core-product-authority.md)
- [ADR-0008: Required unified synchronization](0008-required-unified-synchronization.md)
- [ADR-0023: Scoped relationship authorization and audit](0023-scoped-relationship-authorization-and-audit.md)
- [ADR-0025: Modular server authority](0025-modular-server-authority-foundation.md)
