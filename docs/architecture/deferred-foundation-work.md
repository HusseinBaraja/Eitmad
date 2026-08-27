---
title: "Deferred foundation work"
description: "Prioritizes production blockers and later platform work that remain after the base architecture audit."
audience: "architecture"
page_type: "reference"
status: "active"
owner: "architecture and product maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "a deferred item starts, changes priority, becomes unnecessary, or is completed"
keywords:
  - "deferred work"
  - "production blockers"
  - "foundation roadmap"
---

# Deferred foundation work

Priority describes the first release that needs the item. It does not authorize speculative framework work. Build the smallest real vertical that proves each extension point.

## P0: before the first production Windows product

1. Define the first product domain, user roles, human authentication boundary, scope topology, Arabic terms, and product mode.
2. Connect that vertical to the shared sync engine and implement the required secure LAN or WAN connector, background reconciliation, status events, conflict UX, and recovery.
3. Implement Rust-owned client update coordination: manifest retrieval, eligibility, preflight, durable state, package digest policy, installation handoff, restart verification, audit, and crash reconciliation.
4. Implement the native Windows update adapter and production MSIX signing, timestamping, install, repair, upgrade, uninstall, interruption, and recovery paths.
5. Automate SQLite and PostgreSQL backup schedules, retention, encryption, monitoring, isolated restore drills, and evidence recording for the selected product mode.
6. Complete live PostgreSQL deployment tests with migration and runtime roles, forced RLS, authentication, denial, cross-tenant probes, sync resume, snapshot fallback, update selection, administration, backup, restore, and rollback.
7. Complete Arabic search profiles, localization completeness, keyboard and mixed-direction input, accessibility, documents, reports, print, and export proof for the real product flows.
8. Define measurable performance budgets and run startup, long-session, queue, sync, database, server, and failure-injection tests.
9. Establish release signing, secret-manager access, immutable hosting, SBOM policy, channel promotion, monitoring, incident response, and branch-protection evidence.

## P1: after the first vertical proves the base

1. Add a production administration UI and bounded support-bundle retention.
2. Add the product-required email, MFA, billing, CDN, and relay routing providers behind the Rust external-service boundary.
3. Add safe configuration import/export and local recovery operator workflows through authorized IPC, if operations need them.
4. Add domain-specific metrics and capacity alerts without recording customer payloads or authorization graphs.
5. Split control, sync, relay, update, or administration planes only when scale, privilege, or failure evidence requires it.

## P2: platform expansion

1. Build the macOS native shell, engine supervision, Keychain proof, Arabic/RTL and VoiceOver coverage, notarized package, and updater.
2. Build the Linux native shell, supervision, keyring proof, Arabic/RTL and accessibility coverage, signed managed package, and updater.
3. Add platform-specific release, installation, upgrade, interruption, rollback, and recovery matrices.

## Explicitly not scheduled as generic work

- a generic repository or service layer outside a product vertical;
- a second sync protocol for LAN or a specific feature;
- shell-owned validation, permissions, config, database, secrets, or update policy;
- event sourcing for domains without history-critical requirements;
- last-write-wins for pricing, orders, inventory, permissions, accounting, or other truth-sensitive data;
- microservice extraction without measured privilege, scale, or failure need.

When an item starts, create its vertical documentation, tests, release impact, and troubleshooting route in the same logical change. Remove the item from this page only after the [readiness checklist](../operations/base-application-readiness-checklist.md) has replacement evidence.
