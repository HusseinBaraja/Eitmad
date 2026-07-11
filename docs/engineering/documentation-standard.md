# Engineer Documentation Standard

## Purpose

Documentation is part of the product, not post-development cleanup. Every completed feature must leave enough concise, current information for another engineer to understand its purpose, authority boundary, operation, risks, verification, and safe extension.

## Required documentation for a feature

The feature document MUST cover:

1. **Purpose and user outcome** — who needs it, which problem it solves, and what is explicitly out of scope.
2. **Design and ownership** — the involved Rust modules, shell adapters, server planes, storage, and external boundaries; identify the authority for each decision.
3. **Contracts and compatibility** — commands, queries, subscriptions, errors, versions, capabilities, generated bindings, and compatibility behavior.
4. **Data and lifecycle** — canonical records, explicit scopes, invariants, migrations, retention, deletion, backup, and recovery.
5. **Security and privacy** — threat assumptions, authentication, relationship authorization, audit behavior, secrets, redaction, and sensitive-data handling.
6. **Sync and offline behavior** — product mode, idempotency, retry, checkpoints, conflicts, partial failure, and recovery where applicable.
7. **Arabic-first behavior** — approved terminology, RTL and bidirectional concerns, search normalization, localization, accessibility, and document output.
8. **Operational behavior** — startup/shutdown interaction, background jobs, observability, support diagnostics, failure modes, and rollback or repair.
9. **Performance and efficiency** — expected load, bounds, budgets, IPC shape, cancellation, streaming/paging, and measured results where material.
10. **Tradeoffs and rejected alternatives** — why this design was chosen and which costs or limitations remain.
11. **Tests and verification** — meaningful automated tests, manual scenarios, affected platforms, clean build/run evidence, and known gaps.
12. **Extension points** — the intended seams for later behavior and the invariants future work must preserve.

Small changes MAY use a compact document or a focused update to an existing document, but MUST address every applicable topic. Major features require a dedicated design document and any necessary architectural decision records.

## Architectural decision records

Create an ADR when a change establishes or alters a durable cross-cutting choice, including process boundaries, authority, contract technology, persistence strategy, sync semantics, security model, update compatibility, or a substantial dependency.

ADRs live in `docs/decisions/` and use zero-padded sequence names such as `0001-local-ipc-transport.md`. Each ADR contains:

- title, date, status, and decision owners;
- context and forces;
- decision;
- considered alternatives;
- positive and negative consequences;
- security, Arabic UX, compatibility, and operational implications where applicable;
- follow-up work and superseded decisions.

ADR statuses are `proposed`, `accepted`, `superseded`, or `rejected`. Accepted decisions are not silently rewritten; a new ADR supersedes them.

## Writing rules

- Write for an engineer who did not participate in the change.
- Lead with purpose and behavior, then implementation detail.
- Use explicit normative language for requirements and plain language elsewhere.
- Prefer small diagrams and tables only when they clarify boundaries, sequences, ownership, or mappings.
- Link to canonical definitions instead of copying them. Duplicated specifications drift.
- Use the [domain glossary](../domain-glossary.md) consistently and add new domain terms in Arabic and English.
- Record assumptions, bounds, failure behavior, and deliberate omissions.
- Never include secrets, real customer data, access tokens, internal credentials, or unsafe diagnostic payloads.
- Keep examples synthetic and representative, especially for Arabic and bidirectional behavior.
- Use repository-relative Markdown links so documentation works locally and in code review.

## Location and discoverability

- `docs/README.md` is the documentation index and MUST link to every normative or durable document.
- Cross-cutting architecture belongs under `docs/architecture/`.
- Product and UX requirements belong under `docs/product/`.
- Engineering practices belong under `docs/engineering/`.
- Architectural decisions belong under `docs/decisions/`.
- Feature documentation SHOULD live near the owning vertical feature when a production structure exists, with a durable link from the docs index or relevant domain index.

Documentation structure SHOULD follow product boundaries rather than technical-layer mirrors. A feature should not require readers to reconcile separate shell, engine, database, and API documents to understand one behavior.

## Definition of documented

Documentation is complete only when:

- links and anchors resolve;
- diagrams render and terminology matches the glossary;
- commands, errors, config, and examples match the implementation;
- tests named in the document exist and verification claims are reproducible;
- affected Arabic, security, sync, update, and performance requirements are addressed;
- obsolete content is removed or clearly marked as historical;
- the docs index exposes the new or changed document;
- reviewers can identify authority, failure behavior, and extension points without reading all source files.

CI SHOULD progressively enforce link validity, formatting, generated contract drift, migration documentation, unsafe example data, and required feature-doc sections.

## Review and maintenance

Documentation changes are reviewed with the code they describe. Reviewers MUST reject a feature whose documentation contradicts its implementation or omits a material boundary or risk.

When behavior changes, update the canonical document in the same commit or logical change. When a feature is removed, remove or archive its active documentation and repair incoming links. Periodic audits SHOULD verify that diagrams, compatibility ranges, runbooks, and extension points still reflect the product.

## Initial foundation exception

This initial documentation set precedes production code. Its verification therefore covers document structure, internal links, completeness against the requested foundation topics, and consistency with repository policy. Build, runtime, migration, and platform verification become mandatory with the first production implementation.
