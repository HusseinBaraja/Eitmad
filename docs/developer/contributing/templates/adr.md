---
title: "Template: record an architectural decision"
description: "Draft an ADR with forces, alternatives, consequences, compatibility, security, Arabic UX, and supersession."
audience: "architecture"
page_type: "decision"
status: "draft"
owner: "architecture maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "ADR format or governance changes"
---

# ADR-NNNN: {decision title}

> - Category: Explanation
> - Status: Proposed | Accepted | Superseded | Rejected
> - Date: YYYY-MM-DD
> - Decision owners: {roles or names}
> - Last verified: YYYY-MM-DD
> - Review trigger: {assumption, dependency, boundary, or requirement change}
> - Supersedes: {ADR links or None}
> - Superseded by: {ADR link or None}

## Context and problem

Describe the decision that must be made, the user or engineering problem, existing constraints, and why the decision is durable enough for an ADR.

## Decision drivers

- {force, constraint, or quality attribute}

## Considered options

### {option}

Describe the option and its relevant strengths and weaknesses.

## Decision

State the chosen option and the rules that follow from it.

## Architecture and contract impact

Describe ownership, process boundaries, contracts, versions, capabilities, compatibility, and migration requirements.

## Storage and sync impact

Describe data ownership, migrations, audit/history, product mode, offline behavior, conflicts, and recovery, or state `Not applicable` with a reason.

## Security impact

Describe trust boundaries, authorization, scope isolation, secrets, privacy, audit, and abuse resistance.

## Arabic UX impact

Describe terminology, RTL, bidirectional text, localization, search, accessibility, and generated output impact.

## Consequences and tradeoffs

### Positive

- {consequence}

### Negative

- {cost or limitation}

### Risks

- {risk and mitigation}

## Verification

State tests, prototypes, measurements, reviews, or operational evidence that validate the decision.

## Follow-up work

- {task, owner, and tracking link}

## Related decisions and documents

- {links}
