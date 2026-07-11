# ADR-0008: Require one versioned synchronization protocol

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: A product domain proves that the unified protocol cannot preserve its consistency or availability requirements
> - Supersedes: None
> - Superseded by: None

## Context and problem

The product must support local-first and server-authoritative domains across intermittent LAN and WAN connectivity. Independent sync implementations would produce different identities, conflicts, retries, authorization, and audit results.

## Decision drivers

- Required offline and multi-device operation
- Consistent semantics across transports and product modes
- Resumable, auditable, authorization-aware exchange

## Considered options

- **One versioned protocol:** shared semantics with domain-specific policies.
- **Transport-specific sync:** easier local optimization, but causes behavioral drift.
- **Server-only operation:** simpler consistency, but fails local resilience needs.
- **No sync:** incompatible with the product mission.

## Decision

Synchronization is a required core capability. Local-first and server-authoritative domains MUST use one Rust-owned, versioned protocol across LAN and WAN transports, aligned with IPC identity and causation concepts. Domain policy MAY vary, but transport MUST NOT change business semantics.

The protocol MUST define identity, scope, actor, device, causation, capability and schema negotiation, incremental checkpoints, resumption, idempotency, deduplication, ordering, deletion, conflict handling, authorization, audit correlation, bounded batches, backpressure, and partial-failure recovery.

## Architecture and contract impact

Control, sync, relay, update, and admin server planes have separable contracts, privileges, and failure domains even if initially deployed together.

## Storage and sync impact

Each domain declares local-first or server-authoritative mode. Automatic conflict resolution is allowed only when it cannot invent business truth; otherwise explicit user or supervisor resolution is required.

## Security impact

Every replicated read and mutation is authenticated, authorized, scope-bound, validated, and auditable. Transport encryption does not replace record-level policy.

## Arabic UX impact

Sync preserves canonical Unicode data. Shells present localized offline, queued, syncing, conflict, failed, and current states with safe next actions and correct RTL behavior.

## Consequences and tradeoffs

### Positive

- LAN and WAN behavior cannot silently diverge.
- Offline work and central authority share traceable outcomes.

### Negative

- Protocol design and compatibility testing are substantial upfront work.

### Risks

- A universal protocol could become over-generalized; mitigate with a small shared envelope and domain-owned conflict semantics.

## Verification

Foundation review covers all required protocol properties. Implementation requires interruption, retry, duplicate, stale-client, clock-skew, conflict, authorization, migration, and scope-leakage tests.

## Follow-up work

- Define the first domain's product mode and sync invariants.
- Threat-model sync and separate server-plane privileges.

## Related decisions and documents

- [Target architecture: Synchronization requirement](../architecture/target-architecture.md#synchronization-requirement)
- [ADR-0005](0005-generated-or-validated-contracts.md)
- [ADR-0007](0007-rust-owned-database.md)
- [ADR-0009](0009-zero-trust-security-model.md)
