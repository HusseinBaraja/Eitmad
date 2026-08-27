---
title: "Upgrade to protocol 1.6 local installation authority"
description: "Roll out the Rust-owned local identity handshake, storage version 9, regenerated bindings, recovery, and rollback safely."
audience: "operations"
page_type: "reference"
status: "active"
owner: "Rust engine, storage, and Windows platform maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "protocol 1.6 handshake, storage version 9, local identity, or Windows bootstrap changes"
keywords:
  - "protocol 1.6"
  - "storage version 9"
  - "local installation authority"
---

# Upgrade to protocol 1.6 local installation authority

Protocol `1.6` removes `DevelopmentIdentityAssertion`, `developmentBearerToken`, and every client-supplied authorization field from `HandshakeRequest`. The Windows launcher sends a random process-lifetime bootstrap token through inherited standard input. Rust returns the authorization context after it loads or creates storage version 9's stable installation identity and durable owner relationship.

## Compatibility and rollout

The handshake shape is not backward compatible. Deploy the protocol 1.6 engine, generated bindings, Windows adapter, and shell as one package. Do not combine a 1.5 shell with a 1.6 engine. Remote server traffic remains additive: the server accepts protocol `1.4–1.6`.

Storage migration `identity.local-authority.v1` is additive. Before upgrade, Rust creates and validates the normal `eitmad.pre-migration-vN-to-v9.sqlite3` artifact. The authority row is created only when supervised IPC first needs it. Identity rows, authorization scope, owner relationship, and singleton locator commit in one transaction.

Verify these conditions before promotion:

1. Generated Rust, C#, and Swift contracts contain `bootstrapToken` and no development identity assertion.
2. Process arguments and environment contain no bootstrap token.
3. A wrong token is rejected, and a correct token returns the Rust-owned tenant, scope, principal, and session.
4. Restart keeps the identity, tenant, and scope stable while it creates a new session ID.
5. The real Windows supervisor can read configuration, write an Arabic locale value, and stop the engine cleanly.
6. Storage integrity, recovery artifact, contract conformance, strict Clippy, and full test suites pass.

## Recovery and rollback

If startup fails before migration, preserve the database and inspect the sanitized error. If migration 9 fails, restore the validated pre-migration artifact through the stopped-engine recovery flow. An engine older than storage version 9 cannot open the migrated database. To roll back the package, stop the engine and restore the complete pre-v9 artifact. Do not delete the singleton row, owner relationship, or migration history manually.

This installation authority supports one Windows-account trust boundary. A shared-machine or multi-user product must add reviewed user authentication, session rotation, and revocation before release.

Related pages: [local IPC](../developer/subsystems/local-ipc.md), [local storage](../developer/subsystems/local-storage.md), [Windows supervision](../developer/subsystems/windows-process-supervision.md), [threat model](../architecture/local-ipc-threat-model.md), and [engine operations](../operations/run-engine-runtime.md).
