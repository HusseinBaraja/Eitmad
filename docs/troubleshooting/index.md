---
title: "Troubleshoot Eitmad"
description: "Find safe diagnostic paths for stable errors, generated contract drift, and recoverable system failures."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-08-24"
review_triggers:
  - "a stable diagnosable error or recovery path is added"
keywords:
  - "troubleshooting"
  - "contract drift"
  - "schema drift"
---

# Troubleshoot Eitmad

Use symptom and identifier pages to reach the authoritative owner and a non-destructive recovery path.

## Contract failures

- [Resolve generated contract drift](contract-binding-drift.md): repair missing or stale schemas, registries, C#/Swift bindings, fixtures, and generated references.

## Engine process failures

- [Resolve local IPC connection, request, and subscription failures](local-ipc-failures.md): diagnose unavailable engines, rejected sessions, resync, backpressure, version mismatch, deadlines, and payload bounds.
- [Resolve engine startup and authority failures](engine-startup-failures.md): diagnose failed startup, storage migration or schema drift, invalid supervision, readiness checks, shutdown failures, and duplicate engine authorities.
- [Resolve Windows engine supervision failures](windows-engine-supervision-failures.md): diagnose restart exhaustion, forced shutdown, stale observations, and Job Object setup failures.

## Storage and recovery failures

- [Resolve local storage migration, corruption, and restore failures](local-storage-recovery-failures.md): diagnose compatibility windows, pre-migration snapshots, integrity, schema drift, restore artifacts, and scoped export denial.

## Synchronization failures

- [Resolve synchronization connection, queue, conflict, or cache failures](synchronization-failures.md): diagnose simulation/LAN/WAN health, authentication, encryption, backoff, partial discovery, relay failure, stream order, pending work, conflict, duplicate delivery, stale cache, scope, authorization, and durable state.
- [Resolve server authentication, tenant, and sync failures](server-authentication-and-sync.md): diagnose PostgreSQL, tokens, device proof, tenant isolation, compatibility, idempotency, snapshots, conflicts, and subscription resume.
- [Resolve relay, update distribution, and administration failures](server-plane-failures.md): diagnose relay lifecycle, signed manifests, channels, incompatible clients, backup, migration, audit, visibility, and support workflows.

## Configuration and authorization failures

- [Resolve configuration and authorization failures](configuration-authorization-failures.md): diagnose invalid patches/imports, revision conflicts, denial, last-owner protection, revocation, migration, and corruption.
- [Resolve scoped authorization, sync, adapter, and plugin denials](authorization-boundary-denials.md): diagnose relationship paths, inheritance, conditions, tenant/workspace isolation, boundary rejection, and mandatory audit failures.

## Privacy and secret failures

- [Resolve diagnostic privacy or secret-storage failures](privacy-and-secret-leakage.md): contain suspected leakage and diagnose redaction, sensitive-debug expiry, native credential-store, and encrypted-fallback failures.

No native settings workflow or user-facing Arabic error is implemented yet. Runtime failures expose stable localization message IDs for future shells.

Return to the [documentation index](../index.md) or review [protocol v1](../api/index.md).
