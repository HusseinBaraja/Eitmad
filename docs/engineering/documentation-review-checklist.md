# Documentation Review Checklist

> - Category: Reference
> - Status: Active
> - Owner: Engineering maintainers
> - Last verified: 2026-07-11
> - Review trigger: Documentation requirements or review workflow changes

Use this checklist for new or materially changed durable documents. Every applicable item MUST pass. `Not applicable` requires a short reason in the document.

## Reader and structure

- [ ] The document declares one primary Diátaxis category: tutorial, how-to, reference, or explanation.
- [ ] The content serves that category's reader need without mixing incompatible formats.
- [ ] The title, scope, status, owner, last-verified date, and review triggers are clear.
- [ ] The document is linked from the docs index, an appropriate category index, or its owning vertical feature.
- [ ] Terms match the domain glossary; new domain terms include Arabic and English definitions.

## Feature completeness

- [ ] Purpose and user problem identify affected roles, desired outcome, and exclusions.
- [ ] Architecture placement identifies Rust authority, shell adapters, server planes, and external boundaries.
- [ ] Contracts cover commands, queries, subscriptions, errors, versions, capabilities, bindings, and compatibility.
- [ ] Storage impact covers scopes, invariants, migrations, retention, deletion, backup, recovery, and audit.
- [ ] Sync impact covers product mode, offline behavior, idempotency, retry, conflicts, partial failure, and recovery.
- [ ] Security impact covers trust, authentication, relationship authorization, scope, audit, secrets, privacy, and redaction.
- [ ] Arabic UX impact covers terminology, RTL, bidirectional text, search, localization, input, accessibility, and output.
- [ ] Tests name meaningful automated and manual verification, affected platforms, clean build/run evidence, and known gaps.
- [ ] Failure modes state detection, preserved state, user recovery, retry safety, diagnostics, and repair or rollback.
- [ ] Tradeoffs explain the selected design, rejected alternatives, costs, limits, and extension points.

## Accuracy and safety

- [ ] Claims match current implementation, contracts, schemas, configuration, and tests.
- [ ] Commands and procedures are reproducible and safe; prerequisites and rollback are explicit.
- [ ] Links and anchors resolve; diagrams render; examples use synthetic data.
- [ ] No secrets, credentials, real customer data, unsafe logs, or cross-scope information appear.
- [ ] Arabic and mixed-direction examples render and copy correctly where visual output matters.
- [ ] Duplicated specifications are replaced by links to a canonical source.

## Decisions and freshness

- [ ] Durable cross-cutting choices have an ADR; smaller local choices use a decision log when useful.
- [ ] ADR number, status, owners, alternatives, consequences, impacts, and supersession links are complete.
- [ ] `Last verified` reflects an actual evidence-based check, not only a date edit.
- [ ] Review interval and triggers comply with the freshness policy.
- [ ] Deprecated, stale, superseded, or historical content is labeled and links to its replacement.
