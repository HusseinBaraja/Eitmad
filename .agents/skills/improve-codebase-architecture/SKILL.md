---
name: improve-codebase-architecture
description: Review or change Eitmad module boundaries to deepen shallow modules and consolidate tightly coupled code. Use for an explicit architecture or structural refactoring task.
---

# Improve Codebase Architecture

Surface architectural friction and propose **deepening opportunities** — refactors that turn shallow modules into deep ones. The aim is testability and AI-navigability.

## Glossary

Use the vocabulary in [LANGUAGE.md](LANGUAGE.md) when it clarifies the analysis. Preserve established Eitmad terms such as contract, service, adapter, and boundary where they are more precise.

- **Module** — anything with an interface and an implementation (function, class, package, slice).
- **Interface** — everything a caller must know to use the module: types, invariants, error modes, ordering, config. Not just the type signature.
- **Implementation** — the code inside.
- **Depth** — leverage at the interface: a lot of behaviour behind a small interface. **Deep** = high leverage. **Shallow** = interface nearly as complex as the implementation.
- **Seam** — where an interface lives; a place behaviour can be altered without editing in place.
- **Adapter** — a concrete thing satisfying an interface at a seam.
- **Leverage** — what callers get from depth.
- **Locality** — what maintainers get from depth: change, bugs, knowledge concentrated in one place.

Key principles (see [LANGUAGE.md](LANGUAGE.md) for the full list):

- **Deletion test**: imagine deleting the module. If complexity vanishes, it was a pass-through. If complexity reappears across N callers, it was earning its keep.
- **The interface is the test surface.**
- **One adapter = hypothetical seam. Two adapters = real seam.**

This skill is _informed_ by the project's domain model. The domain language gives names to good seams; accepted decision records contain choices the skill should not re-litigate.

## Process

### 1. Explore

Read `docs/glossary.md` and only the decision records applicable to the area being reviewed.

Explore the relevant code directly. Do not follow rigid heuristics; note where understanding and change require unnecessary indirection:

- Where does understanding one concept require bouncing between many small modules?
- Where are modules **shallow** — interface nearly as complex as the implementation?
- Where have pure functions been extracted just for testability, but the real bugs hide in how they're called (no **locality**)?
- Where do tightly-coupled modules leak across their seams?
- Which parts of the codebase are untested, or hard to test through their current interface?

Apply the **deletion test** to anything you suspect is shallow: would deleting it concentrate complexity, or just move it? A "yes, concentrates" is the signal you want.

### 2. Present candidates

Present a numbered list of deepening opportunities. For each candidate:

- **Files** — which files/modules are involved
- **Problem** — why the current architecture is causing friction
- **Solution** — plain English description of what would change
- **Benefits** — explained in terms of locality and leverage, and also in how tests would improve

Use `docs/glossary.md` vocabulary for the domain and [LANGUAGE.md](LANGUAGE.md) where its architecture vocabulary improves precision.

**Decision conflicts**: if a candidate contradicts an accepted decision, surface it only when the friction is real enough to warrant revisiting that decision. Mark the conflict clearly. Do not list every theoretical refactor that a decision excludes.

If the user requested an interactive audit, ask which candidate to explore. Otherwise, prioritize the candidates and continue to the depth or implementation requested.

### 3. Grilling loop

Once the user picks a candidate, drop into a grilling conversation. Walk the design tree with them — constraints, dependencies, the shape of the deepened module, what sits behind the seam, what tests survive.

Side effects happen inline as decisions crystallize:

- **Naming a deepened module after a concept not in `docs/glossary.md`?** Update the glossary only when documentation edits are authorized or required by `AGENTS.md`; otherwise report the proposed term.
- **Sharpening a fuzzy term during the conversation?** Apply the same documentation boundary.
- **User rejects the candidate with a load-bearing reason?** Propose a decision record only when the reason would help a future review avoid repeating the same suggestion. Follow the authorization and documentation-impact rules in `AGENTS.md`; skip ephemeral and self-evident reasons.
- **Want to explore alternative interfaces for the deepened module?** See [INTERFACE-DESIGN.md](INTERFACE-DESIGN.md).
