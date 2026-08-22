---
title: "ADR-0025: Deploy modular server authority as one process first"
description: "Records the combined Rust server deployment, PostgreSQL isolation, authentication, and shared sync coordination boundaries."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-08-22"
review_triggers:
  - "server planes separate, database tenancy changes, authentication changes, or sync authority moves"
---

# ADR-0025: Deploy modular server authority as one process first

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-08-22
> - Decision owners: Architecture and server platform maintainers
> - Last verified: 2026-08-22
> - Review trigger: Server process, trust, database, authentication, compatibility, or sync ownership changes
> - Supersedes: None
> - Superseded by: None

## Context and problem

Eitmad needs a server foundation for identity, tenants, devices, authentication, licensing, update assignment, and both local-first and server-authoritative synchronization. The first deployment must be practical for a small team, but its boundaries must permit later scale and independent plane deployment. Every remote boundary remains zero-trust, tenant-scoped, versioned, and audited.

## Decision drivers

- Keep Rust authoritative and native shells thin.
- Prevent cross-tenant and unauthenticated access by default.
- Preserve exact retry, conflict, and history semantics under concurrent replicas.
- Support offline-first productivity and server-authoritative business domains with one sync protocol.
- Permit later control, sync, relay, update, and admin service separation without an early distributed system.
- Keep client upgrades explicit through capability and schema negotiation.

## Considered options

### One undivided server module

This is easy to start but mixes identity, sync, update, and operations. It makes privilege separation, testing, and later extraction unsafe.

### Separate services immediately

This gives strong deployment isolation but adds network calls, distributed transactions, deployment sequencing, and operational cost before product domains exist.

### Modular process with explicit planes

This keeps one deployment while placing control, sync, relay, update, admin, host, and contract concerns behind explicit crate boundaries.

## Decision

Use one Rust `eitmad-server` process initially. Keep control and sync as separate library crates and PostgreSQL migration owners. Keep relay, update, and admin as separate deployment boundaries even while they are placeholders.

Use PostgreSQL transactions, row locks, explicit Rust tenant checks, forced row-level security, append-only audit, durable idempotency results, durable sync history, and snapshot-backed compaction. Design all writes to be safe with multiple replicas even though the first deployment uses one replica.

Use native credentials with Argon2 password hashes, opaque access and rotating refresh tokens stored as keyed hashes, Ed25519 device proof with nonce replay prevention, and explicit session expiry. Use ReBAC relationships for server permissions. Provide MFA, invitation delivery, license provider, and update artifact integration only as named hooks until provider decisions exist.

Use HTTPS JSON for bounded request/response operations and one authenticated WebSocket for negotiated sync, snapshots, acknowledgements, and resumable subscriptions. Protocol `1.4` is the first server-boundary minor.

## Architecture and contract impact

`server/host` composes `server/control-plane` and `server/sync-plane`. The external contract authority remains `crates/contracts`; generated JSON, C#, Swift, fixtures, and references move together. Clients must negotiate protocol, required capabilities, and registered domain schemas before normal WebSocket traffic. Plane extraction must preserve these external contracts and add authenticated internal contracts; it must not expose database tables as an API.

## Storage and sync impact

Tenant identity, sessions, token families, relationships, license state, update assignments, audit, outbox, sync operations, conflicts, projections, snapshots, checkpoints, and subscription events are durable PostgreSQL data. Tenant-scoped tables force RLS. Migrations are append-only.

Each registered domain fixes its local-first or server-authoritative mode and supplies authorization plus validation or command execution. Exact idempotency retries return the stored result. Conflicting key reuse fails closed. Operation history is retained for at least 90 days and can be compacted only behind a complete snapshot and safe client checkpoints.

## Security impact

Transport authentication does not grant authorization. Every protected route validates the token, device proof, tenant, scope, and relationship before state access. Database roles must not bypass RLS. Secret values and domain payloads do not enter logs or audit. Refresh-token reuse revokes its family. Cross-tenant access is a critical incident.

## Arabic UX impact

Tenant display names, organization display names, usernames, and opaque domain payloads support UTF-8 Arabic and mixed-direction content. Tenant codes remain lowercase ASCII routing identifiers. Usernames reject bidi control characters. No server UI is added. Future shells localize stable message IDs and use isolated LTR display for identifiers inside RTL layouts.

## Consequences and tradeoffs

### Positive

- One deployable process keeps initial operations small.
- Crate and storage ownership makes later service extraction reviewable.
- PostgreSQL transactions, row locking, and RLS support replica-safe tenant isolation.
- The same sync semantics serve local-first and server-authoritative domains.

### Negative

- The combined process has a larger fault and privilege boundary than separated services.
- PostgreSQL is required even for a small deployment.
- Provider-backed email, MFA, billing, artifacts, and conflict UI remain incomplete.

### Risks

- A privileged database role can bypass RLS. Deployment policy and synthetic isolation tests mitigate this.
- Compaction can strand old clients. Snapshot completeness and checkpoint checks must gate deletion.
- Token-key loss invalidates verification; token-key disclosure requires credential rotation and session revocation.
- An empty domain registry means the foundation cannot carry production records until a reviewed domain is registered.

## Verification

Rust unit and static migration tests cover authentication boundaries, identity persistence, forced RLS, Arabic usernames, tenant isolation structure, session policy, unauthorized changes, idempotency, conflicts, snapshots, and incompatible clients. Workspace tests, strict Clippy, Rust `1.85.1` compilation, contract generation/verification, and a synthetic `check-config` run pass. Live PostgreSQL migration and end-to-end network verification remain required before production because no compatible database runtime was present on the development workstation.

## Follow-up work

- Server platform maintainers: add PostgreSQL-backed integration tests in CI.
- First domain owner: register one domain with permissions, mode, conflict policy, Arabic UX, and compatibility tests.
- Security maintainers: select production MFA and invitation delivery providers.
- Release maintainers: define signed update artifact hosting and native installer rollout.
- Operations maintainers: define tested PostgreSQL backup, restore, key rotation, and multi-replica runbooks.

## Related decisions and documents

- [ADR-0007: Rust-owned database](0007-rust-owned-database.md)
- [ADR-0008: Required unified synchronization](0008-required-unified-synchronization.md)
- [ADR-0009: Zero-trust security](0009-zero-trust-security-model.md)
- [ADR-0010: ReBAC authorization](0010-rebac-authorization-foundation.md)
- [Server authority](../developer/subsystems/server-authority.md)
- [Protocol 1.4 rollout](../releases/protocol-1-4-server-authority.md)
