---
name: maintain-project-documentation
description: Create, restructure, audit, or substantially update Eitmad documentation. Use when documentation is the requested deliverable or AGENTS.md identifies a required canonical documentation change.
---

# Maintain Project Documentation

Build documentation that lets an unfamiliar contributor understand the system, locate ownership, reproduce behavior, diagnose an issue, and change code safely. Treat documentation as a tested product surface.

## Read the relevant guidance

Read only what the task needs:

- For new pages, navigation, indexes, taxonomy, or documentation-system changes, read [references/documentation-system.md](references/documentation-system.md).
- For drafting or editing pages, read [references/writing-and-page-patterns.md](references/writing-and-page-patterns.md).
- For bugs, errors, diagnostics, support knowledge, or issue discovery, read [references/issue-search-and-troubleshooting.md](references/issue-search-and-troubleshooting.md).
- For documentation-system review, migration, or release readiness, read [references/quality-gates.md](references/quality-gates.md).
- Read [references/zulip-research.md](references/zulip-research.md) only when the user asks about the Zulip comparison or its research basis.

Treat canonical project documentation as current guidance. Verify behavioral claims against current code, tests, contracts, schemas, configuration, accepted decisions, and verified runtime behavior. Repair confirmed discrepancies instead of ignoring the canonical page.

## Workflow

### 1. Frame the reader task

State the primary audience and the question the page must answer. Choose one page pattern: tutorial, task guide, reference, explanation, troubleshooting, decision, or release/migration note. Split pages when they serve incompatible reader tasks.

Identify the owning product capability. Follow vertical ownership: document a feature as one coherent behavior across Rust authority, IPC, storage, server planes, and thin native adapters. Do not organize feature knowledge into disconnected technical-layer pages.

### 2. Build an evidence map

Inspect the narrowest authoritative sources before writing:

1. repository policy and accepted decisions;
2. owning Rust module or crate and its public boundary;
3. typed commands, queries, subscriptions, errors, versions, and capabilities;
4. migrations, configuration, authorization, audit, sync, and update behavior;
5. tests and runnable verification;
6. native adapter behavior and Arabic UI copy;
7. issue or incident evidence, when supplied.

Record uncertainty as `Needs verification`; never turn inference into a factual claim. Link to canonical definitions instead of copying contracts or schemas. Generated reference remains generated and must identify its source.

### 3. Design discoverability

Place the page in the audience-specific collection defined by `documentation-system.md`. Give it a task-shaped title and a one-sentence search description. Write documentation prose in English. Add exact Arabic UI labels and realistic Arabic search or symptom terms only where they improve product verification or discovery, together with English engineering terms, error identifiers, command names, and configuration keys.

Link the page from the nearest collection index and from relevant overview, subsystem, troubleshooting, and related-task pages. Every page must have a clear next step and a route back to its collection.

### 4. Write for action and understanding

Lead with the outcome or system behavior. Put prerequisites before steps. Use numbered steps only for ordered actions. State expected results, failure signals, rollback or recovery, and verification.

For engineering pages, expose authority, invariants, trust boundaries, scope, lifecycle, failure modes, tests, and safe extension points. Write all documentation, including user help, in English. Preserve exact Arabic UI labels and verify RTL and mixed-direction examples where app behavior requires them. Keep code, identifiers, paths, and keyboard input directionally safe.

Never expose secrets, real customer data, unsafe logs, access tokens, or cross-scope records. Use synthetic Arabic examples.

### 5. Update the knowledge graph

In the same logical change:

- update affected indexes and cross-links;
- update the bilingual glossary for new or changed domain terms;
- add or supersede a decision when a durable choice changes;
- add troubleshooting knowledge when a defect reveals a reusable diagnostic path;
- remove, redirect, or mark superseded pages when behavior disappears;
- update code-adjacent comments only when they explain non-obvious constraints.

### 6. Verify like code

Run product tests only when product behavior changed. For an ordinary documentation change, audit only the changed pages:

```powershell
python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs --files <changed-pages>
```

Run the full documentation audit only when shared navigation, indexes, or the documentation system changed:

```powershell
python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs
```

Build and preview the rendered site only when layout, navigation, diagrams, Arabic presentation, or generated-site behavior changed. Verify only the affected behavior. Run safe commands whose documented behavior changed. Do not claim verification that was not performed.

### 7. Report the documentation change

Summarize the reader outcome, sources checked, pages added or changed, validation run, unresolved gaps, and the trigger for the next review. Keep this shorter than the documentation itself.

## Non-negotiable quality bar

- One canonical source for each fact; links replace duplication.
- Documentation changes with behavior in the same checkpoint.
- Search works by English task and engineering terms plus exact Arabic UI or symptom text where relevant.
- Examples are realistic, synthetic, runnable, and safe.
- Documentation prose is English; Arabic remains a UI/UX subject and exact engineering identifiers remain unchanged.
- Every state-changing flow documents authorization, scope, audit, failure, and recovery.
- Every cross-boundary flow documents versions, compatibility, retry, and partial failure.
- No page exists only to satisfy a checklist; each page must answer a real reader question.
- Documentation for a major capability is incomplete until an unfamiliar engineer can find its authority, tests, failure modes, and extension boundary.
