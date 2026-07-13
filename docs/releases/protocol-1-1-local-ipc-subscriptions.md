---
title: "Upgrade local IPC to protocol 1.1"
description: "Deploy capability-gated engine-to-shell events while preserving protocol 1.0 command and query compatibility."
audience: "support"
page_type: "release"
status: "active"
owner: "Rust contract and Windows platform maintainers"
last_verified: "2026-07-13"
review_triggers:
  - "protocol 1.0 support or local IPC subscription behavior changes"
keywords:
  - "protocol 1.1"
  - "eitmad.capability.local-ipc-subscriptions.v1"
  - "subscription upgrade"
---

# Upgrade local IPC to protocol 1.1

Protocol `1.1` adds bounded engine-to-shell subscriptions. It does not change storage, sync schemas, or product records, so no data migration is required.

## Compatibility

- New engines and the Windows supervisor advertise `1.0–1.1`.
- A `1.0` peer continues to use commands and queries but cannot subscribe.
- Subscription traffic requires negotiated `1.1` and `eitmad.capability.local-ipc-subscriptions.v1`.
- Generated C# and Swift bindings include the new closed-union variants. Old peers never receive them because capability and minor-version gates run first.

## Verify

Run contract drift and cross-language conformance, then run the Windows real-engine scenario. Confirm the handshake selects `1.1`, subscriptions use processed cursors, and shutdown remains clean. Exact commands are in [run foundation checks](../operations/index.md).

## Roll back or recover

Rolling the shell or engine back to a compatible `1.0` build preserves command/query operation but disables subscriptions. Do not emulate missing subscriptions with UI polling. If an old cursor is rejected after replacement, open a fresh subscription and query the authoritative projection as described in [local IPC recovery](../troubleshooting/local-ipc-failures.md).

See [ADR-0018](../decisions/0018-bounded-resumable-local-ipc-events.md) and the [protocol reference](../api/index.md).
