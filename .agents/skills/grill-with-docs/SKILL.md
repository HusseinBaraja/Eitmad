---
name: grill-with-docs
description: Stress-test a plan against Eitmad's domain language and accepted decisions. Use when the user asks for an interactive, documentation-aware design interview.
---

<what-to-do>

Interview me relentlessly about every aspect of this plan until we reach a shared understanding. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.

Ask the questions one at a time, waiting for feedback on each question before continuing.

If a question can be answered by exploring the codebase, explore the codebase instead.

</what-to-do>

<supporting-info>

## Domain awareness

During codebase exploration, also look for existing documentation:

Use `docs/glossary.md` for canonical domain and Arabic terms. Use `docs/decisions/` for accepted durable decisions. Do not create alternative glossary, context, or ADR locations.

## During the session

### Challenge against the glossary

When the user uses a term that conflicts with `docs/glossary.md`, call it out immediately. "The glossary defines 'cancellation' as X, but you seem to mean Y — which is it?"

### Sharpen fuzzy language

When the user uses vague or overloaded terms, propose a precise canonical term. "You're saying 'account' — do you mean the Customer or the User? Those are different things."

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific scenarios. Invent scenarios that probe edge cases and force the user to be precise about the boundaries between concepts.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you find a contradiction, surface it: "Your code cancels entire Orders, but you just said partial cancellation is possible — which is right?"

### Record documentation impact

When a term is resolved, update `docs/glossary.md` only when the user authorized documentation edits or the task includes a required documentation change under `AGENTS.md`. Otherwise, report the proposed glossary change without editing. Include only terms that are meaningful to domain experts.

### Offer decision records sparingly

Only offer to create a decision record when all three are true:

1. **Hard to reverse** — the cost of changing your mind later is meaningful
2. **Surprising without context** — a future reader will wonder "why did they do it this way?"
3. **The result of a real trade-off** — there were genuine alternatives and you picked one for specific reasons

If any of the three is missing, skip the decision record. Create or update a record in `docs/decisions/` only when the user authorized documentation edits or the task includes a required documentation change under `AGENTS.md`.

</supporting-info>
