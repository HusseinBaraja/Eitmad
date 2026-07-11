# وثائق الاعتماد | Eitmad Documentation

This directory is the entry point for the engineering and product documentation of **الاعتماد**, an Arabic-first operations system for a local furniture manufacturer.

The documents in this initial foundation are normative unless they explicitly say otherwise. The words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** describe requirement strength.

## Start here

| Document | Purpose | Status |
| --- | --- | --- |
| [Repository policy](../AGENTS.md) | Defines mandatory project mission, architecture, delivery, testing, Git, and collaboration rules. | Foundation / normative |
| [Target architecture](architecture/target-architecture.md) | Defines system boundaries, ownership, processes, IPC, sync, security, updates, performance principles, and prohibited designs. | Foundation / normative |
| [Arabic-first UX](product/arabic-first-ux.md) | Defines RTL, bidirectional text, localization, search, input, reporting, and accessibility requirements. | Foundation / normative |
| [Documentation standard](engineering/documentation-standard.md) | Defines the documentation required for every completed feature and architectural decision. | Foundation / normative |
| [Domain glossary](domain-glossary.md) | Establishes shared Arabic and English product, furniture, operational, and technical terminology. | Foundation / evolving |

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
