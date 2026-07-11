---
title: "Template: document a feature"
description: "Draft an explanation of a vertical product capability, its boundaries, failures, tests, and extension points."
audience: "developer"
page_type: "explanation"
status: "draft"
owner: "engineering maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "feature documentation requirements change"
---

# Feature: {name}

> - Category: Explanation
> - Status: Proposed | Active | Deprecated | Historical
> - Owner: {team or role}
> - Last verified: YYYY-MM-DD
> - Review trigger: {behavior, contract, storage, sync, security, or UX changes}

## Purpose

Explain why the feature exists and the user outcome it enables.

## User problem

Name the affected roles, their current problem, the desired workflow, and what is out of scope.

## Architecture placement

Identify the owning vertical Rust module, native shell adapters, server planes, background jobs, and external boundaries. State which component is authoritative for every decision.

## Contracts

List commands, queries, subscriptions, errors, versions, capabilities, binding generation or validation, and compatibility behavior.

## Storage impact

Describe canonical records, scopes, invariants, indexes, migrations, retention, deletion, backup, recovery, and audit history. State `Not applicable` with a reason when no durable state changes.

## Sync impact

State whether the feature is local-first or server-authoritative. Describe checkpoints, idempotency, retry, ordering, conflict behavior, offline behavior, partial failure, and recovery. State `Not applicable` with a reason when the feature never crosses a sync boundary.

## Security impact

Describe identities, trust boundaries, relationship authorization, scopes, audit records, secrets, privacy, redaction, abuse cases, and permission-denied behavior.

## Arabic UX impact

Record approved Arabic terms, RTL behavior, bidirectional values, Arabic search normalization, localization, input, accessibility, and document or report output. Include representative Arabic and mixed-direction test data.

Link a completed copy or evidence record for the [Arabic-first feature checklist](../arabic-first-feature-checklist.md). Every item must be `Pass`, `Not applicable` with a reason, or `Blocked`; an applicable blocked item prevents feature completion.

## Behavior and flows

Describe the normal flow and important state transitions. Link to tutorials, how-to guides, and exact reference material rather than duplicating them.

## Failure modes

For each credible failure, state detection, preserved state, user-visible Arabic message or recovery action, retry safety, diagnostics, and repair or rollback behavior.

## Tests and verification

List automated tests, manual scenarios, affected platforms, Arabic/RTL cases, bidi interaction, locale formatting, search normalization, generated-document rendering, security cases, sync/offline cases, performance evidence, and clean build/run commands. Record known gaps and their owner.

## Tradeoffs and rejected alternatives

Explain why this design was selected, the alternatives considered, remaining costs, limitations, and any follow-up ADRs.

## Extension points

Name supported seams for future work and invariants that extensions MUST preserve.

## Related documentation

- ADRs: {links or `None`}
- Reference: {links or `None`}
- How-to guides: {links or `None`}
- Tutorials: {links or `None`}
- Decision log: {link or `None`}
