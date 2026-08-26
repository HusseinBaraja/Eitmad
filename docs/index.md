---
title: "Eitmad documentation"
description: "Choose the relevant task or audience to reach product, engineering, operations, and contract documentation."
audience: "developer"
page_type: "reference"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-08-26"
review_triggers:
  - "a documentation collection or canonical entry point changes"
keywords:
  - "documentation index"
  - "developer guide"
---

# Eitmad documentation

Choose the route that matches your task. Protocol `1.5`, the engine lifecycle, SQLite local authority/recovery, PostgreSQL server authority, persistent identity, scoped authorization/audit, shared sync semantics, WAN relay coordination, signed update manifests, least-privilege administration, Windows process supervision, mandatory CI, and unsigned Windows/server validation packaging are implemented foundations. No production business domain, native settings UI, MFA/email provider, package CDN, production signing service, native desktop updater, production relay payload router, or backup scheduler exists yet.

## Use the product

No user workflow is ready to document. When the first product flow exists, English help for its Arabic-first UI and UX will appear in `user/`.

## Develop the system

- [Start developing Eitmad](developer/index.md): contribution setup, ownership boundaries, tests, and feature documentation.
- [Extend Rust-owned configuration](developer/subsystems/configuration.md): defaults, validation, revisions, redaction, import/export, and events.
- [Extend scoped authorization and audit](developer/subsystems/authorization.md): tuples, roles, inheritance, conditions, isolation, boundary gates, and complete redacted audit.
- [Extend persistent tenant identity](developer/subsystems/identity-foundation.md): device, account, session, scope topology, offline behavior, and audit attribution.
- [Extend synchronization and shared transports](developer/subsystems/synchronization.md): offline edits, one simulation/LAN/WAN interface, authentication/encryption, reconnect/backoff, relay degradation, conflicts, and safe extension.
- [Extend the modular server authority](developer/subsystems/server-authority.md): accounts, tenants, devices, authentication, licensing, update assignment, PostgreSQL isolation, snapshots, and subscriptions.
- [Extend WAN relay coordination](developer/subsystems/wan-relay-coordination.md): sessions, routes, reconnect, health, failures, authorization, and tenant isolation.
- [Publish and evaluate signed updates](developer/subsystems/update-distribution.md): Ed25519 manifests, channels, rollout, compatibility, packages, and key configuration.
- [Extend server administration](developer/subsystems/server-administration.md): diagnostics, backup, migration, audit, visibility, and support workflows.
- [Understand the target architecture](architecture/index.md): Rust authority, trust boundaries, synchronization, security, and updates.
- [Use protocol v1 contracts](api/index.md): exact Rust-owned contracts, compatibility, generation, and native bindings.
- [Review architectural decisions](decisions/index.md): durable choices, reasons, and consequences.
- [Review releases and compatibility](releases/index.md): protocol rollout, rollback, and migration notes.
- [Use approved terminology](glossary.md): English definitions and canonical Arabic UI terms.

## Operate or deploy the system

- [Run foundation checks](operations/index.md): currently safe commands and operational-readiness limits.
- [Run and diagnose the engine](operations/run-engine-runtime.md): headless and supervised modes, readiness JSON, exit codes, and recovery.
- [Recover and export local storage](operations/recover-local-storage.md): migration snapshots, corruption checks, backup/restore, recovery artifacts, and scoped export.
- [Run and recover the modular server](operations/run-server-authority.md): PostgreSQL migration, first-owner bootstrap, TLS, readiness, backup, and rollback.
- [Deploy staging and production servers](operations/deploy-server-environments.md): profiles, reverse proxy, TLS, migrations, restore drills, channels, and rollback.
- [Package Windows desktop](operations/package-windows-desktop.md): repeatable validation bundle, future MSIX signing, update hosting, and recovery.
- [Prepare macOS distribution](operations/prepare-macos-distribution.md): Developer ID signing, notarization, future updater, and rollback expectations.
- [Prepare Linux distribution](operations/prepare-linux-distribution.md): DEB, RPM, AppImage, repository signing, future updater, and rollback expectations.
- [Validate a release candidate](operations/validate-release-candidate.md): mandatory CI, artifact integrity, smoke, security, Arabic/RTL, restore, and promotion evidence.

## Search by symptom or identifier

- [Troubleshoot Eitmad](troubleshooting/index.md): diagnose contract drift and future stable system failures.

## Documentation authority

Current code, tests, contracts, schemas, and configuration are evidence of behavior. Documentation explains these sources; it does not replace them. When they conflict, repair the canonical page in the same change.
