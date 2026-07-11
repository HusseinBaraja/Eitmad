# وثائق الاعتماد | Eitmad Documentation

> - Category: Reference
> - Status: Active
> - Owner: Engineering maintainers
> - Last verified: 2026-07-11
> - Review trigger: Documentation structure, governance, or durable document changes

This directory is the entry point for the engineering and product documentation of **الاعتماد**, an Arabic-first operations system for a local furniture manufacturer.

The documents in this initial foundation are normative unless they explicitly say otherwise. The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** describe requirement strength.

## Start here

| Document | Purpose | Status |
| --- | --- | --- |
| [Repository policy](../AGENTS.md) | Defines mandatory project mission, architecture, delivery, testing, Git, and collaboration rules. | Foundation / normative |
| [Target architecture](architecture/target-architecture.md) | Defines system boundaries, ownership, processes, IPC, sync, security, updates, performance principles, and prohibited designs. | Foundation / normative |
| [Arabic-first UX](product/arabic-first-ux.md) | Defines RTL, bidirectional text, localization, search, input, reporting, and accessibility requirements. | Foundation / normative |
| [Documentation standard](engineering/documentation-standard.md) | Defines the documentation required for every completed feature and architectural decision. | Foundation / normative |
| [Documentation freshness policy](engineering/documentation-freshness-policy.md) | Defines ownership, review triggers, review cadence, and stale-document handling. | Foundation / normative |
| [Documentation review checklist](engineering/documentation-review-checklist.md) | Provides the required review gate for feature, module, and decision documentation. | Foundation / normative |
| [Architectural decisions](decisions/README.md) | Indexes ADRs and defines their lifecycle and numbering. | Foundation / normative |
| [Documentation templates](templates/README.md) | Provides feature, module, ADR, and decision-log templates. | Foundation / normative |
| [Domain glossary](domain-glossary.md) | Establishes shared Arabic and English product, furniture, operational, and technical terminology. | Foundation / evolving |

## Diátaxis categories

Every durable document MUST declare one primary reader need. A document MAY link to other categories, but SHOULD NOT mix their purposes.

| Category | Reader need | Repository location | Typical content |
| --- | --- | --- | --- |
| Tutorial | Learn through a guided, successful path | [`docs/tutorials/`](tutorials/README.md) | First-run walkthroughs and training exercises |
| How-to | Complete a specific task | [`docs/how-to/`](how-to/README.md) | Operational procedures, maintenance, recovery, and migration steps |
| Reference | Look up exact facts or contracts | [`docs/reference/`](reference/README.md) | Contracts, schemas, configuration, errors, capabilities, and module interfaces |
| Explanation | Understand reasons, boundaries, and tradeoffs | [`docs/explanation/`](explanation/README.md) and [`docs/decisions/`](decisions/README.md) | Architecture, feature design, concepts, ADRs, and tradeoffs |

Current foundation documents retain their domain-oriented locations. New documents SHOULD use the Diátaxis directories unless a feature-local document is more discoverable beside its owning vertical module.

## Product intent

الاعتماد centralizes the daily work of managers, receptionists, carpenters, installers, and workshop supervisors. It is intended to reduce manual pricing errors, improve coordination, preserve trustworthy operational history, and work reliably across supported desktop platforms.

## Architectural decision summary

- Rust is the product authority and runs as a separate engine process.
- Native desktop shells are thin, replaceable platform adapters.
- Typed, versioned local IPC is the only application boundary between a shell and the engine.
- Commands, queries, and subscriptions are explicit and authorized by the engine.
- Local-first and server-authoritative product modes share one sync protocol.
- Arabic and RTL behavior are foundational requirements, not a later translation pass.
- Security assumes no shell, device, peer, server, user, or plugin is trusted implicitly.
- The Rust engine owns update policy; native adapters own platform installation mechanics.
- Correctness and reliability take priority, followed by responsiveness and resource efficiency.

## Documentation lifecycle

Changes to architecture, contracts, domain language, or cross-cutting requirements MUST update the relevant document in the same change. A feature is not complete until its engineer documentation satisfies the [documentation standard](engineering/documentation-standard.md).

This foundation deliberately contains no production implementation.
