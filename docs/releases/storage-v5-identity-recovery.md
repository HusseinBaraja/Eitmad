---
title: "Upgrade local storage to version 5 identity and recovery"
description: "Roll out persistent tenant identity, audit attribution, migration snapshots, integrity strategy, restore recovery, and scoped export."
audience: "support"
page_type: "release"
status: "active"
owner: "Rust storage, identity, and release maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "storage version 5 identity behavior, recovery, or export policy changes"
keywords:
  - "storage version 5"
  - "identity.foundation.v1"
  - "migration compatibility window"
  - "pre-migration backup"
---

# Upgrade local storage to version 5 identity and recovery

Storage version 5 added persistent tenant identity topology and sessions, session/device audit attribution, automatic validated pre-migration snapshots, explicit quick/full corruption checks, recovery-artifact discovery, and tenant-scoped portable export. Storage version 6 now supersedes it for new engines; see the [protocol 1.3 and audit release](protocol-1-3-scoped-authorization-audit.md).

## Compatibility window

- Fresh storage version `0` creates version 5 directly.
- Versions `2`, `3`, and `4` upgrade to `5` after a validated pre-migration backup.
- Version `1` is rejected before migration-history rewrite; use an intermediate compatible engine.
- Version `6` requires the protocol `1.3` engine and cannot be opened by the version 5 engine.

Migration `identity.foundation.v1` is additive for supported databases. It creates identity tables and adds nullable session/device columns to existing audit rows. New configuration and authorization mutations populate those columns; older audit rows remain valid with no fabricated attribution.

## Rollout

1. Stop the old engine cleanly and confirm private free space for a database-sized snapshot.
2. Deploy the new Rust engine. Do not change shell authentication claims or expose identity APIs as production login.
3. Start once. Confirm one `eitmad.pre-migration-vN-to-v5.sqlite3` artifact for an upgraded database and `ready: true`. A retry for the same version pair must validate and reuse it.
4. Run storage tests, full workspace tests, strict Clippy, diagnostics, clean start/stop, and documentation audit.
5. Retain the pre-migration artifact until application verification and an approved retention decision complete.

## Rollback and recovery

Do not run an older engine against version 5. Stop the new engine, determine whether it accepted writes, and use the stopped-engine restore coordinator with the validated pre-migration artifact only when losing newer writes is explicitly acceptable. Follow [recover local storage](../operations/recover-local-storage.md) and [storage troubleshooting](../troubleshooting/local-storage-recovery-failures.md).

No production session issuer, operator restore UI, export IPC, backup scheduler, encryption, or retention service is included. Those remain release blockers before exposing these hooks to users.
