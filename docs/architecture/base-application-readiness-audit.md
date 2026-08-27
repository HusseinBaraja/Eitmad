---
title: "Base application architecture and readiness audit"
description: "Records the 2026-08-27 authority, security, compatibility, Arabic, operations, recovery, resource, and release readiness verdict."
audience: "architecture"
page_type: "reference"
status: "active"
owner: "architecture and security maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "a critical foundation control, production blocker, supported platform, protocol, storage version, or release gate changes"
keywords:
  - "architecture audit"
  - "base application readiness"
  - "critical issues closed"
---

# Base application architecture and readiness audit

## Verdict

The base is **ready for first-product engineering on the Windows foundation**. All critical defects found in implemented boundaries during this audit are closed. No implemented engine or server capability is known to bypass Rust-owned identity, authorization, tenant scope, audit, migration checks, or generated contracts.

The base is **not ready for a production desktop release** and is **not a complete cross-platform desktop product**. Production signing and installation, the native Windows updater, live product sync wiring, macOS and Linux shells, automated backup execution, and production restore evidence remain blocking work. This distinction is mandatory: engineering readiness does not grant production eligibility.

| Readiness target | Result | Meaning |
| --- | --- | --- |
| First Rust-owned product vertical | Pass | The authority, contract, storage, authorization, audit, Arabic, test, and documentation seams are usable |
| Windows engineering validation app | Pass | The real shell, adapter, engine, SQLite store, and typed IPC run together |
| Production Windows desktop | Blocked | MSIX, signing, native update installation, protected-secret live evidence, accessibility proof, and restore drills are incomplete |
| Production server product | Blocked | No domain is registered; live PostgreSQL, backup, restore, scale, and provider evidence is external to hermetic CI |
| macOS or Linux desktop | Blocked | Generated contract conformance exists, but no runnable native shell or updater exists |

## Audit method and evidence

The audit traced Rust contracts to native bindings, shell requests to dispatcher authority, SQLite and PostgreSQL writes to scope and audit controls, server routes to authentication and capability negotiation, migrations to immutable history and recovery artifacts, and release claims to executable tests. It also searched shell and adapter source for database, config, secret, external API, and domain-validation ownership violations.

The verified local evidence includes:

- `cargo fmt --all -- --check`;
- strict Clippy for changed crates and the full workspace release gate;
- `cargo test --workspace --all-targets`, including authorization, isolation, audit, migration, sync, update, secret, IPC, and server tests;
- real Windows engine supervision, protocol 1.6 negotiation, configuration query, configuration subscription, Arabic configuration write, and clean stop;
- generated Rust, C#, Swift, fixture, and reference drift checks;
- C# contract conformance and Windows Arabic/RTL shell tests;
- repository ownership, unsafe logging, migration checksum, Arabic root direction, and documentation-impact policy;
- focused and full documentation audits;
- npm high-severity audit, NuGet vulnerable-package checks, and RustSec audit with no remaining advisory or warning;
- deterministic artifact and repository-policy script tests.

Swift compilation, Linux server artifact creation, live PostgreSQL isolation, native credential-store lifecycle, signing, installation, backup, and restore remain platform or environment gates. They are not claimed as local evidence.

## Findings by requested area

| Area | Result | Evidence and disposition |
| --- | --- | --- |
| Rust ownership leaks | Closed | The shell-supplied development identity and ephemeral-owner bypass were removed. Rust now creates and verifies the stable local installation identity and owner relation in storage version 9. |
| Duplicated contracts | Pass | Rust owns protocol types and identifiers. C# and Swift bindings, fixtures, and reference pages are generated and checked for drift. |
| Direct UI database, config, or secret access | Pass | Shell policy and tests reject database clients, config file ownership, secret APIs, external API clients, and generic command submission. The Windows bridge selects only process path and runtime location. |
| Missing authorization | Closed | Implemented commands, queries, subscriptions, sync traffic, server routes, relay, update publication, and administration execute through Rust checks. Unsupported work is not advertised as a capability. |
| Tenant isolation | Pass for implemented storage; live proof required for release | SQLite records use explicit scope and transaction checks. PostgreSQL migrations force row-level security on tenant tables and tests check policy presence and cross-tenant denial. A staging database must still prove deployment roles and RLS. |
| Unsafe audit records | Closed | Local mutations and reads use complete redacted envelopes. Server migration 4 supplies one canonical actor, scope, target, correlation, causation, idempotency, outcome, and error envelope. Denied, invalid, and pre-handler sync outcomes are mandatory audit events. |
| Sync gaps | Foundation present; product wiring deferred | One bounded local-first/server-authoritative engine and one transport contract exist. The base executable does not advertise local sync status because no production connector or product domain is wired. Reference-marker changes remain in a bounded durable outbox. |
| Compatibility gaps | Closed for current boundaries | Protocol 1.6 removes client authorization assertions. The server accepts 1.4–1.6. The Windows shell consumes negotiated capability intersections and does not send unsupported sync or update traffic. The 1.6 local handshake requires a coordinated package rollout. |
| Update and migration risk | Safe foundation; production updater blocked | Signed manifest, rollout, compatibility, revocation, immutable migrations, checksums, pre-migration SQLite backup, restore, and server migration order exist. Native installation, interruption reconciliation, and production signing are not implemented. |
| Arabic and RTL | Pass for the current Windows foundation; product coverage required | The WPF root is Arabic and RTL, mixed identifiers use LTR isolation, bidi controls are rejected at identity boundaries, and Arabic/mixed contract fixtures pass. Arabic search profiles, reports, documents, and full accessibility proof need the first product domain. |
| Documentation | Pass | Canonical subsystem, ADR, operations, release, troubleshooting, glossary, and generated contract pages are indexed and pass the complete documentation audit. |
| Tests | Pass for hermetic foundation | Full workspace, policy, contract, C#, Windows adapter, shell, release-script, migration, recovery, authorization, audit, sync, update, and tenant tests pass. Live service and platform gates remain explicit. |
| Deployment | Validation only | Repeatable unsigned Windows ZIP and Linux-server tar builders exist. No production desktop installer, signing service, CDN, macOS/Linux shell, or native updater exists. |
| Recovery | Safe primitives; automation deferred | SQLite backup, migration artifacts, validation, restore rollback, export, and server runbooks exist. Scheduled retention, remote backup, operator UI, and recorded live PostgreSQL restore drills do not. |
| Resource efficiency | Pass for current bounds; load proof deferred | IPC frames, requests, subscriptions, event replay, queues, pages, server connections, sync work, snapshot chunks, and retries are bounded. Subscriptions replace polling, unsupported capability traffic is suppressed, and release builds use thin LTO. Production load, memory, and latency budgets need domain workloads. |

## Critical defects closed in this audit

| Commit | Closed defect |
| --- | --- |
| `50d3ea1` | Removed high/critical contract-tool dependency advisories and added the npm audit gate |
| `3894bc4` | Stopped the empty base server from advertising domain schemas or successful unimplemented support actions |
| `bf21666` | Made server sync authorization session-aware and required complete audit records for denied, invalid, acknowledged, and handler outcomes |
| `507bb29` | Removed shell-controlled local identity and the development owner bypass; added Rust-owned installation authority and storage migration 9 |
| `5b50943` | Repaired the full workspace tests to use durable authority instead of the removed bypass |
| `3f24633` | Corrected Windows capability offers and stopped unsupported sync/update request traffic |
| `4f8de59` | Removed the RustSec `rand 0.9.2` unsoundness warning by updating to 0.9.3 |

`07c8783` also made repository policy robust when a tracked file is deleted, so CI reports the intended policy result instead of crashing.

## Readiness rule

Do not change this verdict to production-ready until every blocking item in the [base readiness checklist](../operations/base-application-readiness-checklist.md) has current evidence. Do not hide a missing platform, provider, restore drill, or product vertical behind the foundation pass. Review the [known limitations](known-limitations.md), [deferred work](deferred-foundation-work.md), and [first-product guide](../developer/build-first-product.md) before implementation.
