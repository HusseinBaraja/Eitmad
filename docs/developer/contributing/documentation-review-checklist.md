---
title: "Review documentation quality"
description: "Check evidence, structure, search, security, English prose, Arabic UI evidence, and rendering before accepting feature documentation."
audience: "developer"
page_type: "reference"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "documentation requirements, language policy, audit automation, or review workflow changes"
keywords:
  - "documentation review checklist"
  - "documentation quality gate"
  - "English documentation"
---

# Review documentation quality

Accept a change only when each page answers a real reader task and matches current evidence.

## Evidence and structure

- [ ] Every behavior claim is supported by code, a contract, schema, configuration, test, or verified runtime result.
- [ ] The page has one audience and pattern, with matching title, description, and metadata.
- [ ] The nearest index links the page, and no canonical fact is duplicated.
- [ ] Links, anchors, examples, and commands are correct; unverified claims are marked `Needs verification`.

## Language, search, and action

- [ ] Documentation prose is English in every collection, including user help.
- [ ] Exact Arabic UI labels, localized symptoms, and search aliases appear only where relevant to app UI/UX behavior.
- [ ] Relevant English engineering terms and exact identifiers are searchable.
- [ ] Prerequisites, expected results, failure signals, verification, and recovery are stated.
- [ ] A symptom or error page leads to the owning capability and a safe recovery path.

## Authority and security

- [ ] Rust is named as authority for domain rules, contracts, storage, authorization, synchronization, and updates.
- [ ] The native shell remains a thin adapter and does not duplicate schemas, validation, or database access.
- [ ] State-changing flows cover identity, ReBAC, scope, audit, denial, retry, partial failure, and recovery.
- [ ] Examples are synthetic and scoped, with no secrets, customer data, or unredacted logs.

## Arabic UI/UX and rendering

- [ ] The [Arabic-first feature checklist](arabic-first-feature-checklist.md) is complete; each item is `Pass`, reasoned `Not applicable`, or an explicit blocker that prevents acceptance.
- [ ] Exact Arabic labels and terms match the [glossary](../../glossary.md) and current shell where applicable.
- [ ] RTL, bidirectional text, numerals, identifiers, and copy/paste were rendered and checked where applicable.
- [ ] Locale-sensitive dates, numbers, units, currencies, search results, and generated documents have evidence where applicable.
- [ ] Images that show Arabic UI have useful English alt text and no personal data.
- [ ] The documentation renderer and search index were built when available.

## Verification and handoff

- [ ] `audit_docs.py`, focused product tests, and the workspace check suite pass.
- [ ] Reviewed sources, commands, platforms, and remaining gaps are recorded.
- [ ] The change states the next review trigger.

Return to the [feature documentation standard](documentation-standard.md) when a check fails.
