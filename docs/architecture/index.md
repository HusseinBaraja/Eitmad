---
title: "Understand Eitmad architecture"
description: "Navigate from system shape and authority boundaries to Rust, IPC, security, Arabic UX, and update decisions."
audience: "architecture"
page_type: "explanation"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "an authority, process, trust, storage, sync, update, or platform boundary changes"
keywords:
  - "Eitmad architecture"
  - "Rust authority"
  - "thin native shell"
---

# Understand Eitmad architecture

Eitmad uses thin native desktop applications connected to a separate trusted Rust engine that acts as product authority. Protocol v1 contracts and native binding generation are implemented; no production business capability exists yet.

## Start here

- [Target architecture](target-architecture.md): system shape, ownership, IPC, storage, synchronization, security, updates, and performance.
- [Arabic-first UX requirements](arabic-first-ux.md): RTL, bidirectional text, search, input, documents, and accessibility.
- [Architectural decisions](../decisions/index.md): the historical record of accepted choices.
- [Protocol v1 contracts](../api/index.md): implemented external shapes, compatibility, and generation.
- [Repository layout and ownership](../developer/repository-layout.md): where each current boundary lives.

## Change invariants

- Rust defines domain rules, contracts, storage, authorization, synchronization, and update policy.
- Native shells render state and adapt to the OS; they do not create product truth.
- Every record and operation has an explicit scope, and every state change is authorized and audited.
- Every external boundary is versioned and uses capability negotiation.
- Arabic is the primary UI/UX design and verification case, not a later translation.

Next, review the [initial implementation gates](target-architecture.md#initial-implementation-gates) before the first vertical capability.
