---
title: "Evolve Rust-owned contracts"
description: "Change beta contracts and direct callers together, regenerate bindings, and verify the current negotiated boundary."
audience: "api"
page_type: "task"
status: "active"
owner: "Rust contract maintainers"
last_verified: "2026-08-22"
review_triggers:
  - "the protocol version, supported release window, capability negotiation, generator, or rollout process changes"
keywords:
  - "compatibility window"
  - "backward-compatible contract change"
  - "breaking contract change"
  - "capability negotiation"
---

# Evolve Rust-owned contracts

Change the Rust authority and its direct callers together, then regenerate every derived artifact. The beta has no public production baseline. Breaking changes are allowed; do not add deprecated aliases, parallel bindings, dual writes, or old-protocol branches for unreleased behavior.

## Current desktop boundary

Engine and Windows shell negotiate protocol `1.13` only. Older beta peers fail at handshake. Capability and schema negotiation remain required. A command, query, or subscription envelope must match the negotiated version and authenticated scope.

Keep the protocol and schema identifiers versioned. If a supported release or external consumer is introduced, declare its supported ranges and apply [ADR-0015](../decisions/0015-contract-compatibility-window.md) to that real release boundary. Preserve durable local data and immutable migration history regardless of protocol compatibility.

## Change and verify

1. Edit the owning Rust contract, catalog, implementation, and direct callers.
2. Remove obsolete inputs, fixtures, and generated variants with the implementation. Add a focused boundary test when changed behavior needs protection.
3. Run `npm run contracts:generate --prefix crates/contracts/codegen` and inspect the generated schema, registry, fixture, reference, C#, and Swift diffs.
4. Run `npm run contracts:verify --prefix crates/contracts/codegen` and the affected Rust checks.
5. Run the available native conformance runners and update the affected canonical guide.

## Negotiation and failure

`crates/contracts/src/versioning.rs` selects an overlapping protocol and schema version, intersects capabilities, and rejects missing required capabilities or schemas before normal traffic. Optional capability absence withholds that behavior. Authorization and audit remain Rust-owned for every accepted request.

The drift check compares every generated file against a fresh Rust export. Platform conformance compiles the generated bindings and round-trips the current Rust fixture, including Arabic and mixed text. CI does not regenerate and accept changes automatically.

For exact shapes, use the [contract reference](index.md). For generation failures, follow [resolve generated contract drift](../troubleshooting/contract-binding-drift.md).
