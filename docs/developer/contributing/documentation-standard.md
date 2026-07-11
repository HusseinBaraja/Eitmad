---
title: "Update documentation with every feature"
description: "Assess product-change impact, update canonical pages, and run documentation checks before completing a feature."
audience: "developer"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "feature completion, documentation structure, language policy, or quality gates change"
keywords:
  - "feature documentation"
  - "maintain-project-documentation"
  - "definition of done"
  - "English documentation"
---

# Update documentation with every feature

A feature is incomplete until a new engineer can find its Rust authority, contracts, invariants, failure modes, tests, and safe extension points. Follow the instructions in `.agents/skills/maintain-project-documentation/SKILL.md` after behavior is complete and before declaring the feature done.

## Documentation language

Write all repository documentation in English, including user help. Arabic is the app's UI/UX language requirement, not the documentation prose language. Preserve exact Arabic UI labels, approved product terms, localized symptoms, search aliases, and representative RTL or bidirectional examples where they are evidence of app behavior.

## Assess change impact

Inspect the narrowest canonical sources: repository policy, the owning Rust module, contracts, migrations, configuration, authorization, audit, synchronization, updates, tests, then the shell adapter and Arabic UI copy. Never use an old document as the sole evidence of behavior.

Update documentation in the same change when any of these change:

- a user workflow, Arabic UI term, or localized label;
- a `Command`, `Query`, `Subscription`, error, version, or capability;
- authorization, scope, audit, or privacy behavior;
- a record lifecycle, schema, migration, backup, or recovery path;
- synchronization, offline behavior, retry, or conflict semantics;
- installation, update, compatibility, diagnostics, or rollback;
- module ownership, an architectural invariant, or an extension point;
- a failure that reveals a reusable diagnostic path.

## Choose the canonical page

| Reader need | Location |
| --- | --- |
| Complete work in the product | `docs/user/`, written in English with exact Arabic UI labels |
| Understand or change a capability | `docs/developer/subsystems/`, organized by product capability |
| Operate or recover the system | `docs/operations/` |
| Depend on an exact contract | `docs/api/` |
| Understand a cross-capability boundary or model | `docs/architecture/` |
| Diagnose a symptom or error identifier | `docs/troubleshooting/` |
| Record a durable choice | `docs/decisions/` |
| Explain release compatibility or migration | `docs/releases/` |

Do not create a collection or page without a real reader task. Update an existing page for a small change, and organize feature knowledge vertically across Rust, IPC, storage, server, and shell boundaries.

## Write what an engineer needs

Cover what applies:

- purpose, outcome, and non-goals;
- Rust authority, boundaries, invariants, and lifecycle;
- contracts, versions, capabilities, compatibility, and partial failure;
- identity, ReBAC, scope, audit, denial, and recovery;
- storage, synchronization, and offline behavior;
- Arabic UI/UX, RTL, bidirectional text, search, and accessibility;
- failures, diagnostics, rollback, or repair;
- tradeoffs, tests, and safe extension points.

Link to the canonical definition instead of copying it. Use synthetic Arabic UI examples when needed, and never expose secrets, customer records, or unredacted logs.

## Update the knowledge graph

In the same logical change:

1. Link the page from the nearest index and related pages.
2. Update the [glossary](../../glossary.md) when terms change.
3. Add or supersede an ADR when a durable decision changes; do not rewrite an accepted decision.
4. Add troubleshooting guidance when a defect yields reusable diagnostic knowledge.
5. Remove stale content or mark it deprecated with a link to its replacement.

## Verify

Run:

```powershell
python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs
```

Then run focused product tests and every safe command in the guide. Build the preview when a renderer exists. Verify links, search, Arabic shaping, RTL/LTR behavior, and copy/paste where Arabic UI examples appear. Record what was actually verified and any remaining gaps.

Use the [documentation review checklist](documentation-review-checklist.md) and [freshness policy](documentation-freshness-policy.md) before handoff.
