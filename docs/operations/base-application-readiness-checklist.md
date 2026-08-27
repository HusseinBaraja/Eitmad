---
title: "Base application readiness checklist"
description: "Separates passed foundation gates from blocking production and cross-platform release evidence."
audience: "operations"
page_type: "reference"
status: "active"
owner: "release and architecture maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "a readiness gate, production blocker, audit result, supported platform, or release artifact changes"
keywords:
  - "readiness checklist"
  - "production blockers"
  - "first product gate"
---

# Base application readiness checklist

Use `Pass` only with current evidence. `Blocked` prevents the named release target. `Deferred` is allowed only when the capability is not advertised and the current target does not need it.

## First-product engineering foundation

- **Pass:** work is on a feature branch and the audit checkpoints are committed.
- **Pass:** Rust owns contracts, identity, authorization, config, storage, sync semantics, update policy, audit, and server authority.
- **Pass:** native shells have no direct database, config-file, secret, external API, or domain-validation authority.
- **Pass:** local IPC uses protocol 1.6, a private process bootstrap token, Rust-owned identity, exact session context, bounded frames, bounded concurrency, and capability negotiation.
- **Pass:** implemented commands, queries, subscriptions, server routes, and sync operations authorize in Rust and fail closed when audit persistence fails.
- **Pass:** every implemented record family has explicit tenant/scope ownership; SQLite checks scope and PostgreSQL migrations force tenant RLS.
- **Pass:** local storage version 9 has immutable checksummed migrations, pre-migration backups, drift checks, integrity checks, restore rollback, and recovery documentation.
- **Pass:** Rust, generated contracts, C# conformance, Windows shell, real engine, repository policy, release scripts, and documentation audits pass.
- **Pass:** npm, NuGet, and RustSec checks report no current vulnerability or warning.
- **Pass:** the current Windows surface is Arabic-first, root RTL, mixed-direction aware, and tested with Arabic fixtures.
- **Pass:** unsupported sync and update runtime capabilities are not advertised or queried.

Result: the foundation can accept the first real Rust-owned vertical.

## Production Windows desktop

- **Blocked:** define and implement the real business vertical and its user authentication model.
- **Blocked:** connect the selected product domains to the sync engine and a production LAN/WAN route where required.
- **Blocked:** implement Rust-owned client update state, preflight, signed manifest retrieval, package verification policy, and crash reconciliation.
- **Blocked:** implement the native Windows download/install/relaunch/result adapter.
- **Blocked:** author, sign, timestamp, install, repair, upgrade, and uninstall a production MSIX.
- **Blocked:** prove native credential-store lifecycle on supported Windows versions.
- **Blocked:** automate backup, retention, monitoring, and restore drills for the chosen product mode.
- **Blocked:** verify Arabic keyboard use, screen reader, high contrast, 200% text scaling, search behavior, documents, and printing for every applicable product flow.
- **Blocked:** record production load, latency, memory, disk, startup, sync-backlog, and long-session evidence against approved budgets.
- **Blocked:** produce and verify signed release evidence, SBOM policy, immutable package hosting, channel promotion, interrupted update, and recovery.

Result: the Windows desktop is not production-eligible.

## Production server product

- **Blocked:** register at least one domain handler; the base registry is intentionally empty.
- **Blocked:** deploy staging PostgreSQL with separate migration/runtime roles and prove forced RLS with real role credentials.
- **Blocked:** configure TLS, secret management, token-key rotation, update public keys, immutable manifest storage, and least-privilege network access.
- **Blocked:** provide required MFA, email, billing, relay routing, administration UI, or other product services.
- **Blocked:** execute and record migration, authentication, denial, cross-tenant isolation, sync resume, snapshot fallback, update, backup, point-in-time restore, and rollback drills.
- **Blocked:** run scale, connection-budget, rate-limit, storage-growth, audit-retention, and failure-injection tests.

Result: the combined server is an engineering foundation, not a production product deployment.

## Cross-platform desktop

- **Pass:** generated Swift contracts and macOS CI conformance are defined.
- **Blocked:** no runnable macOS native shell, supervision adapter, protected-secret integration proof, package, notarization, or updater exists.
- **Blocked:** no runnable Linux native shell, supervision adapter, keyring proof, managed package, repository signing, or updater exists.

Result: cross-platform desktop readiness is blocked.

## Reverification commands

Run the complete command set in [Operate and release Eitmad](index.md), then apply the environment and platform checks in [Validate a release candidate](validate-release-candidate.md). A local pass cannot replace signing, native platform, live database, backup, restore, or staging evidence.
