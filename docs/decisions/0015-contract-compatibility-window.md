---
title: "ADR-0015: Support a rolling protocol compatibility window"
description: "Records the minor-version support window, capability-gated evolution, and major-version transition policy."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture and Rust contract maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "a release cannot support the current and immediately preceding protocol minor"
---

# ADR-0015: Support a rolling protocol compatibility window

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-12
> - Decision owners: Architecture and Rust contract maintainers
> - Last verified: 2026-07-12
> - Review trigger: A release cannot support the current and immediately preceding protocol minor
> - Supersedes: None
> - Superseded by: None

## Context and problem

Engine, shell, server, and diagnostic releases may not update atomically. “Protocol v1” alone does not define how long old peers remain supported or when optional behavior is safe to send.

## Decision

Within one protocol major, released peers support the current minor and immediately preceding minor for at least one coordinated release cycle. Optional behavior is sent only after capability negotiation. Required capabilities and schemas fail closed before normal traffic. The active foundation window is `1.0` only; `1.0` becomes a retained compatibility fixture before `1.1` ships.

Breaking representation or meaning changes use a new major, parallel versioned identifiers and bindings, and an explicit bridge or coordinated upgrade plan. A prior version is removed only after its documented window expires.

## Consequences

- Rolling shell/engine updates have a bounded, testable overlap.
- Each new minor adds retained fixture and bidirectional conformance cost.
- Major transitions require rollout, rollback, sync, storage, and update-order planning rather than only regenerated DTOs.

## Verification

`versioning::negotiate` tests protocol, required-capability, and schema outcomes. Contract CI compares every derived output with a fresh Rust export, verifies C#/Swift identifier parity, blocks platform jobs on drift, and compiles and round-trips native bindings.

## Related decisions and documents

- [ADR-0014](0014-rust-derived-json-contracts.md)
- [Contract evolution process](../api/evolve-contracts-compatibly.md)
- [Contract subsystem](../developer/subsystems/contract-layer.md)
