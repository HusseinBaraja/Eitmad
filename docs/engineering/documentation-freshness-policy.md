# Documentation Freshness Policy

> - Category: Reference
> - Status: Active
> - Owner: Engineering maintainers
> - Last verified: 2026-07-11
> - Review trigger: Documentation governance or release process changes

## Policy

Documentation changes with the behavior it describes. A code, contract, configuration, migration, permission, sync, Arabic UX, operational, or architecture change MUST update affected documentation in the same logical change. Reviewers MUST treat missing documentation as incomplete implementation.

Every durable document MUST declare:

- one primary Diátaxis category;
- status;
- an accountable owner role or team;
- `Last verified` as an ISO date;
- concrete review triggers.

Repository policy documents that predate this metadata format SHOULD adopt it when next materially edited.

## Freshness checks

The document owner MUST check a document:

1. whenever a declared review trigger occurs;
2. during review of a change that modifies linked code or contracts;
3. before a release that depends on its procedures or compatibility claims;
4. at least every 180 days for active reference, how-to, and explanation documents;
5. at least every 90 days for security, recovery, update, migration, and operator-critical documents.

Tutorials are re-run before each supported major release and whenever their prerequisites or UI flow changes. Accepted ADRs are not periodically rewritten; their assumptions are checked when dependent architecture changes, and a new ADR supersedes an obsolete decision.

## Check procedure

The owner MUST:

- compare the document with current behavior, tests, contracts, configuration, and UI;
- run commands and procedures where safe and practical;
- verify internal links, anchors, diagrams, and cited file paths;
- check Arabic terminology, RTL examples, mixed-direction data, and localized messages;
- confirm security, storage, sync, failure, recovery, compatibility, and tradeoff claims;
- update `Last verified` only after completing the check;
- record material corrections in the same review change.

A date-only change is valid only when the review produced no required content change. The pull request or commit description SHOULD state what evidence was checked.

## Stale and historical documents

A document is stale when its review interval expires, its owner is missing, a review trigger occurred without review, or known behavior contradicts it.

- Stale operational guidance MUST be marked `Stale` at the top and MUST NOT be used as an authoritative procedure until verified.
- Incorrect security, migration, recovery, or update guidance MUST be corrected or withdrawn immediately.
- Replaced content MUST link to its successor and use `Historical` or `Deprecated` status as appropriate.
- Historical documents remain available only when they preserve useful context; otherwise remove them and repair links.

## Automation and reporting

CI SHOULD enforce required metadata, required feature sections, valid internal links, unique ADR numbers, and allowed statuses. A scheduled check SHOULD report documents beyond their review interval without automatically changing verification dates.
