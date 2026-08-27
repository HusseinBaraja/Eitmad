---
title: "Known base application limitations"
description: "Lists current capability, platform, operations, Arabic, recovery, and evidence limits without overstating readiness."
audience: "architecture"
page_type: "reference"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "a listed limitation is implemented, removed, split, or becomes release-critical"
keywords:
  - "known limitations"
  - "not implemented"
  - "production gaps"
---

# Known base application limitations

These limits are deliberate and visible. A caller must not infer unavailable state, advertise an absent capability, or treat a validation artifact as production-ready.

## Product and identity

- No real furniture business domain is implemented or registered on the server.
- Local installation authority trusts one Windows-account boundary. It has no human login, role switching, multi-user session rotation, or remote revocation workflow.
- The reference-marker vertical is a foundation example, not a production workflow.

## Sync and networking

- The sync engine, transport policy, server sync plane, snapshot history, conflict model, and durable reference-marker outbox exist, but the desktop runtime has no production connector or background reconciliation coordinator.
- The engine does not advertise local sync status or record-change capability until that coordinator exists.
- LAN discovery and production WAN/relay payload routing are not implemented. The relay plane coordinates authorized metadata and lifecycle only.
- The base server has an empty `DomainRegistry`, so it correctly advertises no domain schema and rejects domain sync traffic.

## Updates and compatibility

- Signed manifest policy, rollout, revocation, compatibility evaluation, update server routes, and immutable manifest storage exist.
- The desktop has no manifest retrieval coordinator, durable client update state, package download, native installer handoff, interruption recovery, or outcome reconciliation.
- Protocol 1.6 local IPC is a coordinated engine-and-adapter change. A 1.5 shell cannot be mixed with a 1.6 engine.
- An engine older than storage version 9 cannot open a migrated local database. Rollback requires the complete validated pre-v9 artifact.

## Platforms and deployment

- Windows is the only runnable desktop shell.
- macOS has generated Swift binding conformance only. Linux has no native desktop shell.
- The Windows artifact is an unsigned framework-dependent ZIP. MSIX authoring, Authenticode, timestamping, signing-service integration, and native update installation are absent.
- The Linux-hosted server builder creates an unsigned validation tarball. Production container or service definitions, infrastructure as code, SBOM policy, and signed promotion automation are not provided.
- Branch protection and production secret-manager policy are external controls and cannot be proved from the repository alone.

## Server services

- The combined server does not include production billing, MFA, email, package CDN, relay payload router, backup scheduler, diagnostic retention, or operator UI.
- Hermetic tests validate migration SQL and isolation policy shape, but production release still requires a live PostgreSQL role, RLS, migration, backup, restore, and failure test environment.

## Recovery and operations

- SQLite backup, validation, restore, migration snapshots, and scoped export are Rust APIs and runbooks. There is no operator UI, schedule, retention job, remote destination, or automatic recovery coordinator.
- PostgreSQL backup, WAL retention, point-in-time recovery, and restore drills depend on deployment infrastructure.
- The native secret lifecycle test is ignored by default because it writes one synthetic OS credential. Each release platform must run it in an isolated release test account.

## Arabic, search, documents, and accessibility

- The current Windows operations surface has Arabic labels, root RTL, mixed-direction isolation, and automated fixtures.
- No production Arabic localization catalog, missing-translation gate, domain search normalization profile, locale-aware sorting implementation, or searchable product domain exists.
- Quotes, orders, invoices, work instructions, labels, reports, printing, PDF output, and spreadsheet export are not implemented.
- Arabic screen-reader behavior, keyboard traversal, high contrast, 200% text scaling, copy/paste, print, and physical device workflows require manual product-level proof.

## Performance evidence

- Current queues, pages, frames, replay, retries, connections, snapshots, and sync work are bounded. Unsupported capability traffic is suppressed.
- No production domain load profile, benchmark history, long-session soak test, server scale test, or approved CPU, memory, latency, disk, and network budget exists yet.

See [deferred foundation work](deferred-foundation-work.md) for the planned order and [the readiness checklist](../operations/base-application-readiness-checklist.md) for blocking release evidence.
