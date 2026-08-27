---
title: "Build the first real product on the Eitmad foundation"
description: "Provides the required sequence for adding one Arabic-first, Rust-owned, authorized, audited, scoped, sync-aware product vertical."
audience: "developer"
page_type: "tutorial"
status: "active"
owner: "product and Rust domain maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "the first-product workflow, authority rules, contracts, sync, Arabic gate, or release gates change"
keywords:
  - "first product vertical"
  - "Rust authority"
  - "Arabic-first feature"
---

# Build the first real product on the Eitmad foundation

Build one complete furniture workflow before you generalize the foundation. A quotation or production-order slice is a useful candidate only after the business owner confirms its terms, states, users, and accounting consequences.

## 1. Define the bounded domain

Write the domain purpose, Arabic UI terms, English code terms, actors, state transitions, invariants, failure modes, and non-goals in `CONTEXT.md` and the canonical feature page. Decide whether history is audit-critical or history-critical before selecting event sourcing.

Name the vertical for product behavior, for example `quotation` or `production_order`. Do not create `services`, `repositories`, `managers`, or `utils` as the owning concept.

## 2. Select authority and scope

Choose one mode per domain:

- use `LocalFirst` for offline productivity where explicit conflicts are acceptable;
- use `ServerAuthoritative` for pricing approval, inventory allocation, accounting, shared order state, and other decisions where the server must confirm truth.

Give every record an explicit tenant and scope. Define whether the scope is organization, workspace, site, workshop, or another reviewed kind. Scope filtering must occur before records, counts, search suggestions, events, or errors can reveal existence.

## 3. Define authorization before behavior

Define versioned actions, objects, relations, conditions, and permissions in Rust. State which roles can create, view, change, approve, cancel, export, or administer the feature. Deny missing rules by default. Test owner, manager, member, absent relation, cross-scope, cross-tenant, revoked, stale-session, and audit-unavailable outcomes.

Do not rely on hidden controls or a shell role enum. Authorize every command, query, subscription, sync delivery, external call, and plugin capability at its Rust boundary.

## 4. Design the contract

Add the smallest typed commands, bounded queries, resumable subscriptions, compact events, stable errors, capability, and domain schema to `eitmad-contracts`. Use idempotency keys for every state-changing command. Carry stable message IDs and structured safe parameters, not English error prose.

Classify the change as additive, coordinated minor, or breaking major. Regenerate the schema, C#, Swift, fixtures, identifier constants, and reference page. Add compatibility tests and a release note when rollout order matters.

## 5. Implement the Rust vertical

Keep the domain service, validation, state transition, authorization call, audit construction, persistence transaction, outbox publication, and focused tests together in the vertical. Expose only narrow product types. The platform shell must never receive a database handle, config path, secret value, permission graph, sync engine, or external provider client.

Every accepted mutation must commit state, complete redacted audit, idempotency result, and durable publication or sync outbox atomically. A failed audit must withhold the mutation. A failed post-commit event publish must leave a bounded durable row for startup recovery.

## 6. Add storage and migration safety

Add the next immutable local or server migration. Include tenant/scope keys, foreign keys, uniqueness, revision checks, bounded indexes, and forced PostgreSQL RLS where applicable. Update `deploy/migrations.sha256` for server migrations.

Test fresh creation, supported upgrade, preserved Arabic and mixed text, migration rollback, changed checksum, schema drift, newer-version rejection, backup creation, restore, and old-binary compatibility. Document the exact rollback or forward-recovery rule.

## 7. Connect synchronization

Register one domain schema and handler. Define payload bounds, authority mode, offline behavior, cache freshness, retry, idempotency window, conflict policy, snapshot behavior, and event projection. Never add a feature-specific LAN protocol or a generic last-write-wins rule.

For local-first work, make queued and conflicted state visible. For server-authoritative work, mark optimistic state as provisional and roll it back on denial. Do not use stale server cache as proof for pricing, permission, inventory, accounting, or approval.

## 8. Complete the Arabic-first gate

Approve Arabic labels, examples, errors, empty states, recovery text, and accessible names before shell work. Define a domain search normalization profile, locale-aware sorting, Arabic and Latin digit behavior, bidi isolation, date/time, units, currency, and document requirements.

Test Arabic-only and mixed values such as `طلب ORD-2048`, `خزانة MDF 120×60 cm`, and `١٢ قطعة / 12 قطعة`. Verify RTL layout, focus order, keyboard switching, cursor and selection behavior, copy/paste, long text, permission denial, offline/conflict state, high contrast, text scaling, and screen-reader output. Render and inspect every Arabic report or PDF; extracted text alone is not sufficient.

## 9. Add the thin native presentation

Use only generated bindings. Offer the feature capability, consume the negotiated intersection, send typed intent, and render Rust snapshots and events. Use subscriptions instead of polling. Page or stream large data. Keep local state ephemeral and reject stale revisions or event order.

Place Windows presentation in the feature folder. Add macOS and Linux presentation only after their native shells exist. Platform adapters own installation, lifecycle, protected OS prompts, and native update mechanics; they do not own product decisions.

## 10. Verify the complete slice

Add focused tests for success, invalid input, denial, tenant isolation, audit failure, idempotent replay, revision conflict, migration, recovery, sync conflict or server denial, compatibility, Arabic/mixed text, RTL, accessibility surfaces, and resource bounds. Run the full [base readiness checklist](../operations/base-application-readiness-checklist.md), the real engine path, and the release-impact checks for the target platform.

Use the [reference-marker vertical](subsystems/reference-marker.md) as a small example of contract, authorization, atomic storage/audit/outbox, event publication, generated bindings, Arabic UI, and tests. Do not copy its local-first mode into a truth-sensitive domain without a domain decision.

## 11. Complete documentation and handoff

Run `.agents/skills/maintain-project-documentation/SKILL.md`. Update the feature page, API reference, indexes, glossary, ADRs, release note, operations, troubleshooting, migration and recovery paths, limitations, and deferred work. An unfamiliar engineer must be able to find the Rust authority, contracts, invariants, security rules, Arabic behavior, tests, failure recovery, and safe extension points.

The feature is complete only when it compiles, runs, has no warning, passes the full test and documentation gates, and has no blocked applicable Arabic or production checklist item for its declared release target.
