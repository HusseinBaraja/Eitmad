---
title: "Resolve configuration and authorization failures"
description: "Diagnose invalid configuration, revision conflict, denial, last-owner protection, revocation, and authority-store failure safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust configuration, authorization, and storage maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "configuration, authorization, migration, or revocation error behavior changes"
keywords:
  - "eitmad.error.config-invalid.v1"
  - "eitmad.error.authorization-denied.v1"
  - "eitmad.error.authorization-last-owner.v1"
  - "authorizationRevoked"
---

# Resolve configuration and authorization failures

Use stable codes, revisions, scope, correlation ID, and operation identifiers. Never collect raw configuration values, secrets, bearer tokens, database contents, or relationship graphs as support evidence.

## Symptoms and safe recovery

| Evidence | Meaning | Safe action |
| --- | --- | --- |
| `eitmad.error.config-invalid.v1` | Empty/oversized/duplicate/unknown patch, wrong value kind, unsupported scope, invalid/non-canonical tag, or malformed import | Correct typed input; use canonical BCP 47 such as `ar-YE`; do not edit SQLite |
| `eitmad.error.config-revision-conflict.v1` | `expectedRevision` is stale | Query a fresh snapshot, reconcile user intent, then submit a new idempotency key against the returned revision |
| `eitmad.error.config-unavailable.v1` | Storage/schema/read failure, or a commit succeeded but its durable event publication is still pending | Retry the exact same command with the same idempotency key once; if unavailable persists, stop mutations, run diagnostics, preserve the runtime directory, and escalate with sanitized health/error IDs |
| `eitmad.error.authorization-denied.v1` | Policy grants no permission in the exact scope | Confirm authenticated principal and scope, then ask an owner to inspect bounded relationships; do not grant via shell state |
| `eitmad.error.authorization-policy-conflict.v1` | Expected policy revision is stale | Owner refreshes the relationship page and retries the intended mutation with a new idempotency key |
| `eitmad.error.authorization-relation-invalid.v1` | Unknown relation or missing relationship | Refresh owner relationship state and use generated relation identifiers |
| `eitmad.error.authorization-last-owner.v1` | Revocation would remove the final persisted owner | Grant another persisted owner first, verify policy revision, then revoke; never bypass the guard |
| `eitmad.error.authorization-unavailable.v1` | Authorization storage failed, or a relationship commit succeeded but policy-event publication remains pending | Retry the exact same command with the same idempotency key once; if unavailable persists, stop mutations and preserve the runtime directory for diagnosis |
| `authorizationRevoked` | Active `1.2` subscription lost permission | Discard buffered later state, refresh identity/policy through an authorized path, and resubscribe only after access is restored |
| Older connection closes after policy mutation | A `1.0`/`1.1` client was revoked | Reconnect with compatible bindings; do not interpret the close as permission to reuse cached configuration |

## Migration or corruption

Run non-mutating diagnostics against the same runtime directory. A missing database is healthy before first authoritative start; a corrupt or newer database is unhealthy. Do not delete WAL files, run ad hoc SQL, retry migration with another engine, or downgrade across storage versions. Stop all authorities, preserve the full runtime directory, and follow the release rollback limits.

## Verify recovery

Query the redacted snapshot or effective permissions in the same scope. Confirm revisions are monotonic, a no-op does not increment, one real mutation emits one event, and no protected value appears in output. Then run focused tests and the [foundation checks](../operations/index.md).

Implementation details are in [configuration](../developer/subsystems/configuration.md), [authorization](../developer/subsystems/authorization.md), and the [protocol 1.2 release](../releases/protocol-1-2-configuration-authorization.md).
