---
title: "Use Eitmad contracts"
description: "Understand external IPC contract status, authority, and compatibility rules before adding a client or integration."
audience: "api"
page_type: "reference"
status: "active"
owner: "Rust contract maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "a command, query, subscription, error, version, or capability is added"
  - "contract generation or validation tooling changes"
keywords:
  - "IPC contract"
  - "Command Query Subscription"
  - "capability negotiation"
---

# Use Eitmad contracts

No production contracts exist yet. `crates/contracts/` is the reserved Rust authority for commands, queries, subscriptions, errors, versions, and capabilities.

## Current guarantees

- Rust is the canonical source for every external contract.
- Platform bindings must be generated from Rust definitions or mechanically validated.
- Every external boundary is versioned and negotiates versions and capabilities.
- Native shells do not manually duplicate DTOs or validation rules.

These are accepted architecture rules, not callable contracts. See [ADR-0005](../decisions/0005-generated-or-validated-contracts.md) and the [target architecture](../architecture/target-architecture.md#ipc-model).

## Add the first contract

Document:

- the exact name and interaction type: `Command`, `Query`, or `Subscription`;
- version, capabilities, inputs, outputs, errors, and bounds;
- identity, ReBAC permission, scope, and audit behavior;
- deadline, cancellation, retry, compatibility, and partial-failure behavior;
- the generation source or verification command, with one valid and one rejected synthetic example.

## Ownership and verification

Current authority: `crates/contracts/src/lib.rs` and `crates/contracts/OWNERSHIP.md`. The source contains only a foundation boundary, so no callable commands or generated references exist.

Next, [start developing Eitmad](../developer/index.md) before implementing the first vertical product slice.
