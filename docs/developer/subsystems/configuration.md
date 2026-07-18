---
title: "Extend Rust-owned configuration safely"
description: "Understand configuration defaults, validation, revisions, redaction, persistence, import, export, and change events."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust configuration maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "a configuration key, validator, sensitivity, persistence rule, import format, or event changes"
keywords:
  - "eitmad-configuration"
  - "eitmad.config.locale.primary.v1"
  - "ConfigSnapshot"
  - "ConfigurationChanged"
---

# Extend Rust-owned configuration safely

`eitmad-configuration` is the sole authority for product settings. Native shells may read generated `ConfigSnapshot` values, submit typed `UpdateConfiguration` patches, and observe `ConfigurationChanged`; they do not own defaults, validation, persistence, imports, exports, redaction, versions, or migrations.

## Ownership and flow

| Concern | Authority |
| --- | --- |
| Wire types and identifiers | `eitmad-contracts::config`, commands, queries, and events |
| Registry, defaults, validation, redaction, patch orchestration, import/export | `eitmad-configuration` |
| Scoped values, revisions, audit, and idempotency transactions | `eitmad-storage` |
| Read/write/import/export authorization | `eitmad-authorization` policy v1 |
| Dispatch and post-commit events | `eitmad-engine-runtime::ProductDispatcher` |

The implemented organization setting is `eitmad.config.locale.primary.v1`. It is public text, defaults to canonical `ar-YE`, requires application restart, and accepts only a valid canonical BCP 47 language tag. Configuration schema version starts at `1`; a new scope starts at revision `0`.

## Snapshots and patches

A snapshot contains every registered setting in stable key order. Rust begins with registry defaults, overlays scoped persisted values, and redacts protected entries before they cross IPC. Only the `organization` scope is supported.

A patch must contain 1–64 unique registered keys, the correct typed value kind, and `expectedRevision`. Rust validates the whole patch before opening the mutation transaction. A real change commits atomically and increments the scope revision once. A same-value patch succeeds without a revision increment or event. Revision conflict, failure, and idempotent replay also publish no event.

Durable idempotency hashes canonical operation input. Reusing a key with the same input returns its recorded redacted snapshot without another revision, audit mutation, or event. Reusing it with different input fails as invalid contract use.

## Redaction and secret rules

Public values may appear in snapshots and exports. `Sensitive` and `Secret` definitions always become `ConfigReadValue::Redacted` at IPC and export boundaries. Secret definitions may persist only opaque `SecretReferenceId` values; raw secret material is never a valid configuration write. Audit rows contain changed identifiers and revisions, never configuration values.

SQLite currently relies on user-private OS-file protection. Do not register production sensitive plaintext until database encryption and key management have an approved design and implementation.

## Import and export

Import/export are Rust service APIs, not shell IPC or CLI operations. JSON format v1 is deterministic and limited to 64 KiB on import. It contains format version, configuration schema version, source revision, sorted public entries, and sorted identifiers for redacted entries. It never contains a source scope ID or protected value.

Import merges included public entries into an authorized target scope through the same validation, optimistic revision, idempotency, audit, and persistence path as a patch. Omitted and redacted entries remain unchanged. Malformed, zero, or future format/schema versions fail without changing configuration.

## Authorization, events, and failures

Members may query and subscribe. Config managers and owners may patch, import, and export. Rust denies absent relationships by default. After a real commit, the dispatcher publishes one coalescible `ConfigurationChanged` containing the committed redacted snapshot. Active subscriptions reauthorize after policy changes and before every delivery.

Stable IPC failures use `eitmad.error.authorization-denied.v1`, `eitmad.error.config-invalid.v1`, `eitmad.error.config-revision-conflict.v1`, or `eitmad.error.config-unavailable.v1`. Shells localize `messageId` and never parse prose. Use [configuration and authorization recovery](../../troubleshooting/configuration-authorization-failures.md) for safe diagnosis.

## Arabic-first behavior

The default locale is `ar-YE`. Registry values and deterministic JSON preserve UTF-8, including mixed Arabic/Latin text, without inserting presentation bidirectional controls. No settings UI exists yet, so RTL layout, localized labels, focus order, and accessibility remain shell work. A future shell must render generated metadata and errors without copying validation.

## Tests and safe extension

Focused tests cover defaults, ordering, valid and invalid patches, type/key/scope checks, canonical BCP 47 validation, conflicts, rollback, no-op behavior, durable replay, sensitive/secret redaction, deterministic import/export, future versions, size bounds, and mixed UTF-8.

To add a setting, define it in the Rust registry with a stable key, type, default, sensitivity, restart requirement, validator, and scope. Add redaction, migration, import/export, Arabic/mixed-direction, and failure tests before exposing it. Regenerate bindings only when the external contract changes. See [ADR-0019](../../decisions/0019-sqlite-authority-storage.md), [protocol 1.2](../../releases/protocol-1-2-configuration-authorization.md), and [authorization policy](authorization.md).
