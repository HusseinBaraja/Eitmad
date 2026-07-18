---
title: "Upgrade to protocol 1.2 configuration authorization"
description: "Deploy Rust-owned configuration, scoped relationship administration, policy events, and revocation while preserving 1.0 and 1.1."
audience: "support"
page_type: "release"
status: "active"
owner: "Rust contract, configuration, authorization, and platform maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "protocol 1.2 support, configuration contracts, or authorization policy behavior changes"
keywords:
  - "protocol 1.2"
  - "eitmad.capability.authorization-policy-events.v1"
  - "Rust-owned configuration"
---

# Upgrade to protocol 1.2 configuration authorization

Protocol `1.2` keeps all `1.0` and `1.1` identifiers and meanings. It adds direct scoped relationship administration, owner-only relationship listing, authorization-policy events, and the `authorizationRevoked` subscription close reason. The existing configuration query, patch, snapshot, and change-event contracts now route to durable Rust authority.

## Compatibility and capability gates

- Engines negotiate `1.0–1.2`.
- `1.0` remains command/query-only; `1.1` retains bounded subscriptions.
- Relationship grant/revoke/list and authorization-policy subscriptions require `1.2`.
- Policy events additionally require `eitmad.capability.authorization-policy-events.v1`.
- A revoked `1.2` subscription receives `authorizationRevoked`; older connections terminate safely without that unknown variant.
- Generated C# and Swift outputs and the cross-language fixture preserve the full compatibility window.

## Storage migration

On authoritative startup, Rust creates or transactionally migrates `runtime_directory/eitmad.sqlite3` through configuration, relationship, audit/idempotency, and durable publication-outbox schema versions before readiness. The engine drains committed publication rows before accepting IPC traffic. Diagnostics check an existing file read-only. Do not downgrade an engine after it has opened a storage version newer than the target build supports.

## Rollout and verification

1. Back up the user-local runtime directory using an SQLite-safe process while the engine is stopped.
2. Deploy engine and generated shell bindings together where relationship UI is introduced.
3. Run contract drift, workspace, Windows binding, supervision, diagnostic, clean start/stop, and documentation checks from [foundation verification](../operations/index.md).
4. Confirm defaults return `ar-YE`, policy revision and configuration revision begin independently, no-op/replay creates no new event, interrupted publication resumes from the outbox, and a revoked subscription receives no later configuration data.

No native settings UI or import/export IPC operation is included. Production remains fail-closed without trusted identity provisioning; the insecure development authenticator is never a production bootstrap path.

## Recover

If migration or compatibility health fails, stop the engine and preserve the database and stable error metadata. Do not delete, edit, downgrade, or manually patch the SQLite file. Restore the pre-upgrade backup only after confirming no successful newer-engine writes must be retained. For validation, authorization, last-owner, revocation, and storage symptoms, use [configuration and authorization recovery](../troubleshooting/configuration-authorization-failures.md).

See [ADR-0019](../decisions/0019-sqlite-authority-storage.md), [ADR-0020](../decisions/0020-direct-principal-scope-rebac-policy.md), and the [protocol reference](../api/index.md).
