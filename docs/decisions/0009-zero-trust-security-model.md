---
title: "ADR-0009: Apply zero trust across every boundary"
description: "Records why shells, engines, devices, peers, servers, administrators, and extensions are not implicitly trusted."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture and security maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "a new trust boundary, identity type, transport, or privileged workflow appears"
---

# ADR-0009: Apply zero trust across every boundary

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: The threat model, deployment topology, regulated data, or supported extension surface changes
> - Supersedes: None
> - Superseded by: None

## Context and problem

The system spans UI processes, engines, devices, peers, servers, administrators, and future plugins. Location, localhost, device ownership, or an administrative UI does not prove identity or permission. Implicit trust would expose business records and mutation paths when any component is compromised.

## Decision drivers

- Explicit identity and least privilege at every boundary
- Tenant and operational scope isolation
- Auditable state changes and revocable access

## Considered options

- **Zero trust:** authenticate and authorize each boundary operation.
- **Trusted local network/device:** simpler, but vulnerable to local and lateral compromise.
- **Perimeter-only security:** insufficient for offline devices, plugins, and internal misuse.

## Decision

The product MUST use zero trust across shells, engines, devices, peers, servers, administrators, and plugins. Every command and query is authenticated, authorized in Rust, scope-bound, and validated. Access is least-privilege, deny-by-default, capability-limited, and revocable where feasible. Every state-changing command produces an immutable or tamper-evident audit outcome.

Threat models MUST precede identity, permissions, sync, updates, plugins, remote access, or sensitive exports.

## Architecture and contract impact

Contracts carry authenticated principal, device, scope, capability, correlation, and causation context without trusting client assertions. Boundaries fail closed when required security capabilities are absent.

## Storage and sync impact

Every record, cache, search result, audit item, and sync message is explicitly scoped. Encryption, backup, retention, and deletion follow data sensitivity and deployment threats.

## Security impact

This decision establishes the security baseline: peer authentication, input validation, least privilege, secret isolation, encrypted transport, signed updates, audit, and secure failure behavior.

## Arabic UX impact

Arabic permission, authentication, denial, and recovery messages must be clear and actionable without exposing secrets, identifiers, policy graphs, or internal causes.

## Consequences and tradeoffs

### Positive

- Compromise of one component does not imply unrestricted product access.
- Security behavior is explicit and testable.

### Negative

- Identity, credential, capability, revocation, and audit infrastructure add latency and complexity.

### Risks

- Excessive friction could encourage bypasses; mitigate with coarse-grained authenticated sessions while authorizing every operation.

## Verification

Foundation review confirmed no trusted-local or UI-only authorization exception. Production verification requires threat models, negative authorization tests, cross-scope isolation tests, and audit completeness checks.

## Follow-up work

- Threat-model authenticated local IPC before implementation.
- Define identity, device enrollment, credential rotation, and audit retention.

## Related decisions and documents

- [Target architecture: Security model](../architecture/target-architecture.md#security-model)
- [ADR-0010](0010-rebac-authorization-foundation.md)
- [ADR-0012](0012-privacy-preserving-observability.md)
