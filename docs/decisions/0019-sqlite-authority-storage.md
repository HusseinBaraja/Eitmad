---
title: "ADR-0019: Use SQLite for local authority storage"
description: "Records the local SQLite file, migration, transaction, privacy, and compatibility policy for configuration and authorization."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture, storage, and security maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "authority database technology, encryption, migration, transaction, or recovery policy changes"
keywords:
  - "ADR-0019"
  - "eitmad.sqlite3"
  - "rusqlite"
---

# ADR-0019: Use SQLite for local authority storage

> - Status: Accepted
> - Date: 2026-07-18
> - Decision owners: Architecture, storage, and security maintainers
> - Supersedes: None
> - Superseded by: None

## Context

Configuration, direct scoped relationships, mutation audit, and durable idempotency need one Rust-owned local transaction boundary. Shell-owned files, multiple databases, and memory-only state cannot provide atomic revisions, migration safety, or crash recovery.

## Decision

Use exact-pinned `rusqlite` with bundled SQLite. Store `eitmad.sqlite3` under the engine runtime directory. Rust exposes narrow repositories only; no shell, plugin, or other crate receives a raw database handle.

Open the store as an engine runtime component after acquiring authority and before readiness. Enable foreign keys, WAL mode, a five-second busy timeout, and user-private directory/file permissions where the OS supports them. Apply ordered transactional migrations for configuration, relationships, then audit/idempotency. Roll back the entire migration or mutation on failure and expose only sanitized structured errors.

Successful state mutations and audit outcomes commit together. Idempotency records hash canonical input and store a safe response. Diagnostics open an existing database read-only, reject corruption or a newer storage version, and do not create or migrate a missing database.

## Consequences

Local mutations gain atomicity, durable optimistic revisions, and one recoverable file. WAL creates companion files during operation, so backup and packaging must treat the database as SQLite state rather than copying an arbitrary open file. Downgrades across a newer storage version remain unsupported.

Initial protection relies on OS-local permissions. Database encryption and key management are required before production settings may contain sensitive plaintext. Raw secrets remain prohibited regardless of future encryption.

## Alternatives

- Shell configuration files were rejected because they create a second authority and cannot atomically join audit/idempotency.
- One database per vertical was rejected because cross-cutting mutation evidence would not share a transaction boundary.
- An ORM was rejected for this bounded schema because narrow SQL repositories make migrations and transaction behavior explicit.

## Verification

Tests cover fresh creation, ordered upgrade from seeded schemas, preserved values, migration rollback, corruption/newer-version compatibility, private paths, atomic configuration and relationship mutations, append-only audit, and idempotency. See [configuration](../developer/subsystems/configuration.md), [authorization](../developer/subsystems/authorization.md), and [startup recovery](../troubleshooting/engine-startup-failures.md).
