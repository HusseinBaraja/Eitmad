# Interface Design

When the user wants to explore alternative interfaces for a chosen deepening candidate, develop distinct designs before choosing one. This follows the "Design It Twice" principle without delegating repository work.

Uses the vocabulary in [LANGUAGE.md](LANGUAGE.md) — **module**, **interface**, **seam**, **adapter**, **leverage**.

## Process

### 1. Frame the problem space

Write a concise explanation of the problem space for the chosen candidate:

- The constraints any new interface would need to satisfy
- The dependencies it would rely on, and which category they fall into (see [DEEPENING.md](DEEPENING.md))
- A rough illustrative code sketch to ground the constraints — not a proposal, just a way to make the constraints concrete

Continue directly to the alternative designs unless a missing product decision would materially change them.

### 2. Develop alternatives

Develop two or three materially different interfaces for the deepened module. Do this in the primary agent as required by `AGENTS.md`.

Evaluate alternatives with different design constraints when they are relevant:

- Minimize the interface and maximize leverage per entry point.
- Optimize for the most common caller and make the default case simple.
- Support justified extension points without speculative abstractions.
- When applicable, place a port at a real external seam.

Use [LANGUAGE.md](LANGUAGE.md) where it improves precision and `docs/glossary.md` for Eitmad domain language.

For each alternative, provide:

1. Interface (types, methods, params — plus invariants, ordering, error modes)
2. Usage example showing how callers use it
3. What the implementation hides behind the seam
4. Dependency strategy and adapters (see [DEEPENING.md](DEEPENING.md))
5. Trade-offs — where leverage is high, where it's thin

### 3. Present and compare

Present designs sequentially so the user can absorb each one, then compare them in prose. Contrast by **depth** (leverage at the interface), **locality** (where change concentrates), and **seam placement**.

After comparing, give your own recommendation: which design you think is strongest and why. If elements from different designs would combine well, propose a hybrid. Be opinionated — the user wants a strong read, not a menu.
