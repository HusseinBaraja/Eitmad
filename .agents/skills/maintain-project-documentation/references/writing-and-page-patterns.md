# Writing and page patterns

## Contents

- Universal style
- Tutorial
- Task guide
- Reference
- Explanation and subsystem
- Troubleshooting
- Decision
- User-help specifics

## Universal style

- Write a specific, task-shaped title. Prefer “Restore a backup” over “Backups”.
- Put the outcome and scope in the opening paragraph.
- Use short sections with descriptive headings that contain likely search terms.
- Lead paragraphs with the important fact. Remove introductions that do not change reader action or understanding.
- Use active voice and direct instructions. Name the actor and authoritative component.
- Explain why only when it prevents misuse, supports a decision, or reveals an invariant.
- Use tables for exact mappings and comparisons; use diagrams for boundaries and multi-step flows.
- Put one command per code block when the reader must inspect its result.
- Show expected output only when it is stable and diagnostic.
- Never claim “simply”, “obviously”, “fully synced”, “saved”, or “deleted” without defining the exact state.
- Link the first relevant mention of a canonical concept. Avoid link-heavy paragraphs.

## Tutorial

Use for a guided learning experience with a known successful result.

```markdown
# Build your first local diagnostic session

What the reader will accomplish and learn.

## Before you start
Prerequisites and safe sample data.

## 1. Start the engine in diagnostic mode
Action, expected result, and one sentence of explanation.

## 2. Run a scoped query
Action and expected result.

## What you learned
Concepts demonstrated, cleanup, and next tutorial/task links.
```

Keep side paths out of tutorials. Link to reference and troubleshooting pages.

## Task guide

Use for a reader who already has a goal.

```markdown
# Rotate an update signing key

Outcome, supported scope, and risk warning.

## Prerequisites
Permissions, backups, compatibility, and inputs.

## Steps
1. Action.
2. Check the stated intermediate result.
3. Action.

## Verify
Exact safe checks and expected state.

## Roll back or recover
Stop conditions, preserved state, and recovery.

## Related tasks
Links to prerequisites, concepts, and troubleshooting.
```

## Reference

Use for exact facts. Generate mechanical contract listings where possible.

Include:

- authority and owning path;
- stability/version/capability policy;
- inputs, outputs, errors, and invariants;
- authorization, scope, audit, lifecycle, bounds, and compatibility;
- one minimal valid example and important invalid examples;
- source generator or verification command for generated content.

Do not narrate a workflow in a reference page.

## Explanation and subsystem

Use for mental models, architecture, and vertical capability ownership.

Cover:

1. user or system problem;
2. ownership and non-goals;
3. normal and failure flows;
4. Rust authority, IPC, storage, server, shell, and external boundaries;
5. invariants and trust boundaries;
6. local-first or server-authoritative behavior;
7. Arabic and mixed-direction behavior;
8. tradeoffs and rejected designs;
9. tests and observability;
10. safe extension points.

Prefer a compact boundary or sequence diagram when three or more components interact.

## Troubleshooting

Use the full pattern in `issue-search-and-troubleshooting.md`. The short shape is:

```markdown
# Resolve “Arabic user-facing symptom” (`EXACT_ERROR_ID`)

Meaning, severity, affected scope, and whether work is safe.

## Symptoms
Visible text, error IDs, logs safe to search, and state indicators.

## Fast checks
Ordered, non-destructive checks that split likely causes.

## Causes and resolutions
For each cause: evidence, resolution, verification, and rollback.

## Escalate safely
Sanitized evidence to collect and secrets/data never to include.
```

## Decision

Record status, date, owners, context, forces, decision, alternatives, positive and negative consequences, security, Arabic UX, compatibility, operations, follow-ups, and supersession links. Accepted decisions are historical records; supersede instead of rewriting the original choice.

## User-help specifics

- Write canonical instructions in clear Modern Standard Arabic, using approved local furniture terminology from the glossary.
- Name the user's role when permissions or available actions differ.
- Use the exact Arabic label visible in the UI.
- Describe state precisely: local durable state, queued sync, server accepted, conflict, or failure.
- Put screenshots after the instruction they clarify. Crop to the relevant region, remove personal data, add useful Arabic alt text, and verify the current theme and platform.
- Do not expose architecture unless it changes the user's decision or recovery action.
- Include keyboard alternatives and accessibility behavior where relevant.
