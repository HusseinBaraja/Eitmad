---
title: "Upgrade to protocol 1.4 modular server authority"
description: "Roll out server identity, device proof, sync snapshots, resumable subscriptions, generated bindings, and PostgreSQL migrations safely."
audience: "support"
page_type: "reference"
status: "active"
owner: "release maintainers"
last_verified: "2026-08-22"
review_triggers:
  - "protocol 1.4, server capabilities, PostgreSQL migrations, or rollout order changes"
keywords:
  - "protocol 1.4"
  - "server connection"
  - "snapshot chunks"
  - "PostgreSQL migration"
---

# Upgrade to protocol 1.4 modular server authority

Protocol `1.4` adds the remote server contract without removing protocol `1.0–1.3` local IPC behavior. It adds account, tenant, organization, registered-device, authentication, session, license, update-assignment, server-stream, snapshot-manifest, snapshot-chunk, snapshot-completion, and snapshot-required shapes plus stable capabilities and errors.

## Compatibility

The encoded protocol window is `1.0–1.4`. Existing local IPC peers retain their earlier capability gates. A remote server connection requires protocol `1.4` and all of these capabilities: sync, server connection, device proof, snapshot chunks, and subscription resume. A client that lacks any required behavior is rejected before normal traffic with `eitmad.error.server-client-incompatible.v1`.

Generated JSON schema, protocol catalog, C# binding, Swift binding, cross-language fixture, and generated reference must ship from the same Rust contract revision. Do not hand-copy the new DTOs or identifiers.

## Database rollout

The server adds two PostgreSQL migrations: control foundation first, sync foundation second. Both create tenant-scoped forced-RLS data. Back up PostgreSQL before deployment. Use an application role that cannot bypass RLS.

Roll out in this order:

1. Back up and verify restore readiness.
2. Deploy regenerated client bindings without enabling server traffic.
3. Run `eitmad-server migrate` with controlled migration credentials.
4. Run synthetic cross-tenant and authentication checks with the normal application role.
5. Start one server replica and verify `/livez`, `/readyz`, protocol negotiation, token rotation, snapshot fallback, and subscription resume.
6. Enable compatible protocol `1.4` clients gradually.
7. Add replicas only after shared PostgreSQL ordering and idempotency checks pass.

## Rollback

Client rollback is safe while it has not committed data that only a newer domain schema understands. A protocol `1.3` client cannot use the remote server boundary and must remain offline or use its supported local behavior.

Server binary rollback is safe only when the older binary understands the applied PostgreSQL migrations and emitted data. Otherwise stop traffic and restore the full pre-migration database backup. Never remove columns, RLS policies, audit rows, history, conflicts, or snapshots by hand. Token-key rollback must preserve the same secret or explicitly revoke all affected sessions.

## Verification status

Generated contract verification, C# and Rust contract tests, workspace tests, strict Clippy, Rust `1.85.1` compilation, and synthetic server configuration validation pass. Live PostgreSQL migration and network-flow verification are still required in deployment CI because the development workstation had no PostgreSQL, Docker, or Podman runtime.

See [server operations](../operations/run-server-authority.md), [server architecture](../developer/subsystems/server-authority.md), and [contract evolution](../api/evolve-contracts-compatibly.md).
