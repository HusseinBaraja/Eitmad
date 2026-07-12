---
title: "ADR-0017: Use bounded JSON frames over Windows named pipes"
description: "Records the first typed local IPC transport, framing, development authentication, deadlines, and shutdown semantics."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "local IPC transport, framing, payload bound, authentication, or platform scope changes"
keywords:
  - "ADR-0017"
  - "Windows named pipe"
  - "length-prefixed JSON"
---

# ADR-0017: Use bounded JSON frames over Windows named pipes

> - Status: Accepted
> - Date: 2026-07-12
> - Decision owners: Architecture, Rust engine, and Windows platform maintainers
> - Supersedes: None
> - Superseded by: None

## Context

The separate Rust engine needs typed command/query traffic without overloading the unauthenticated lifecycle stdout stream or moving product authority into C#. Windows is the only implemented native adapter; macOS and Linux shell technologies remain undecided.

## Decision

Windows local IPC uses an engine-created named pipe with four-byte little-endian length-prefixed UTF-8 JSON frames and an 8 MiB maximum. Every connection negotiates Rust-owned `PeerHello` contracts, receives an engine-issued session, correlates concurrent calls by request ID, applies deadlines, and uses structured failures. Large domain results use pagination or a future negotiated streaming capability rather than unbounded frames or temporary files.

The temporary development handshake uses a random bearer token and asserted synthetic identity only behind an explicit insecure-development flag. It is forbidden for production. Typed shutdown is acknowledged before runtime drain; inherited stdin and Job Object containment remain abandonment and forced-recovery controls.

## Alternatives

- Multiplexing product traffic on stdout would couple lifecycle observation, payload volume, and authentication.
- Shared temporary files increase cleanup, integrity, scope, and substitution risks.
- Unbounded JSON frames are simple but violate low-resource and hostile-input requirements.
- A new binary encoding would conflict with the accepted Rust-derived JSON contract decision without evidence that JSON is the bottleneck.

## Consequences

Rust and generated bindings remain authoritative, unavailable-engine handling stays local to the adapter, and independent calls can complete out of order. JSON serialization and one-frame buffering impose bounded memory cost. Other platforms must preserve the protocol semantics but may choose their native local transport through a later decision.

This capability has no UI or Arabic terminology. Unicode payloads are preserved unchanged; future shells localize structured message IDs.

## Verification and follow-up

Rust tests cover framing, handshake, dispatch, deadlines, Unicode, and bounds. Windows tests cover unavailable endpoints and a real engine session/shutdown. Production peer authentication, real ReBAC-backed handlers, subscriptions, and Unix-domain transports remain follow-up work.

See the [local IPC threat model](../architecture/local-ipc-threat-model.md) and [subsystem guide](../developer/subsystems/local-ipc.md).
