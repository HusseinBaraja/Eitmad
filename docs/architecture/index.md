---
title: "Understand Eitmad architecture"
description: "Navigate from system shape and authority boundaries to Rust, IPC, security, Arabic UX, and update decisions."
audience: "architecture"
page_type: "explanation"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "an authority, process, trust, storage, sync, update, or platform boundary changes"
keywords:
  - "Eitmad architecture"
  - "Rust authority"
  - "thin native shell"
---

# Understand Eitmad architecture

Eitmad uses thin native desktop applications connected to a separate trusted Rust engine that acts as product authority. Protocol `1.6`, native binding generation, engine lifecycle, SQLite local authority, the modular PostgreSQL server, scoped authorization/audit, shared sync semantics, WAN relay coordination, signed update manifests, least-privilege administration, and Windows process supervision are implemented. No production business domain, native settings UI, MFA/email provider, package CDN, production relay payload router, or backup scheduler exists yet.

## Start here

- [Target architecture](target-architecture.md): system shape, ownership, IPC, storage, synchronization, security, updates, and performance.
- [Base application readiness audit](base-application-readiness-audit.md): evidence, closed critical defects, category findings, and the engineering-versus-production verdict.
- [Known limitations](known-limitations.md): current product, sync, update, platform, recovery, Arabic, and performance limits.
- [Deferred foundation work](deferred-foundation-work.md): ordered production blockers and later platform work.
- [Arabic-first UX requirements](arabic-first-ux.md): RTL, bidirectional text, search, input, documents, and accessibility.
- [Architectural decisions](../decisions/index.md): the historical record of accepted choices.
- [Protocol v1 contracts](../api/index.md): implemented external shapes, compatibility, and generation.
- [Local IPC threat model](local-ipc-threat-model.md): current Windows trust boundary, controls, residual risks, and production blockers.
- [Typed local IPC](../developer/subsystems/local-ipc.md): implemented named-pipe requests, scoped event replay, ordering, backpressure, reconnect, and shutdown.
- [Engine runtime lifecycle](../developer/subsystems/engine-runtime.md): implemented startup, readiness, health, process identity, diagnostics, and shutdown behavior.
- [Rust-owned configuration](../developer/subsystems/configuration.md): implemented defaults, validation, persistence, redaction, import/export, and events.
- [Scoped authorization and audit](../developer/subsystems/authorization.md): implemented tuples, role objects, inheritance, conditions, tenant/workspace isolation, audited boundary gates, owner compatibility, and active revocation.
- [Privacy-preserving observability](../developer/subsystems/privacy-preserving-observability.md): implemented structured metadata-only logs/errors, correlation, redaction, crash projections, and temporary sensitive-debug expiry/audit.
- [Rust-owned secret storage](../developer/subsystems/secret-storage.md): implemented native credential-store lifecycle and an authenticated encrypted fallback with typed references.
- [Persistent tenant identity](../developer/subsystems/identity-foundation.md): implemented device, user/account, session, tenant, organization, workspace, offline, and audit-attribution primitives.
- [Modular server authority](../developer/subsystems/server-authority.md): implemented remote accounts, registered devices, authentication, licensing hooks, update assignment, PostgreSQL isolation, sync history, snapshots, and subscriptions.
- [Windows process supervision](../developer/subsystems/windows-process-supervision.md): implemented Job Object containment, bounded restart, stale-event rejection, and graceful shutdown.
- [Repository layout and ownership](../developer/repository-layout.md): where each current boundary lives.

## Change invariants

- Rust defines domain rules, contracts, storage, authorization, synchronization, and update policy.
- Native shells render state and adapt to the OS; they do not create product truth.
- Every record and operation has an explicit scope, and every state change is authorized and audited.
- Every external boundary is versioned and uses capability negotiation.
- Arabic is the primary UI/UX design and verification case, not a later translation.

Next, review the [initial implementation gates](target-architecture.md#initial-implementation-gates) before the first vertical capability.
