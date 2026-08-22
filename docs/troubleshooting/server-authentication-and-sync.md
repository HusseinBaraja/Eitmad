---
title: "Resolve server authentication, tenant, and sync failures"
description: "Diagnose server configuration, PostgreSQL, device proof, token, isolation, compatibility, snapshot, idempotency, and conflict failures safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-08-22"
review_triggers:
  - "server error identifiers, authentication, tenant isolation, compatibility, or sync recovery changes"
keywords:
  - "server-authentication-failed"
  - "server-device-proof-invalid"
  - "server-snapshot-required"
  - "server-client-incompatible"
---

# Resolve server authentication, tenant, and sync failures

Preserve PostgreSQL, audit rows, operation history, conflict records, token-family state, and client checkpoints. Do not delete or edit them to make a retry succeed.

## Start with safe evidence

Record the UTC time, stable error identifier, HTTP status, correlation ID, tenant code, opaque tenant/device/account IDs, client product version, protocol range, capability set, schema range, server build, readiness result, and whether the operation was a first attempt or retry. Do not collect passwords, tokens, signatures, nonces, private keys, database URLs, domain payloads, Arabic customer text, or invitation tokens.

## Symptom table

| Symptom or identifier | Likely cause | Safe check | Recovery |
| --- | --- | --- | --- |
| `eitmad.error.server-config-invalid.v1` | Missing key, invalid listen address, partial TLS pair, unsafe plaintext, or pool size outside 2–128 | Run `check-config` with secret values hidden | Correct deployment configuration; do not enable plaintext on a remote listener |
| `eitmad.error.server-database-unavailable.v1` | PostgreSQL unavailable, URL/role wrong, or pool exhausted | Check `/readyz`, PostgreSQL service health, role, TLS, and connection limits | Restore database access; keep traffic disabled until ready |
| `eitmad.error.server-migration-failed.v1` | Permission, checksum/history, SQL, or incompatible schema failure | Preserve backup and inspect migration identifier and database error through restricted operator logs | Repair the cause or restore the pre-migration backup; do not edit migration history |
| `eitmad.error.server-authentication-failed.v1` | Tenant code, username, password, device, session, or token is invalid | Confirm tenant code and device registration without revealing credentials | Use the activation or sign-in flow again; keep the response intentionally non-specific |
| `eitmad.error.server-token-expired.v1` | Access, refresh, invitation, or session time limit passed | Compare server UTC time and token/session metadata | Refresh once if permitted, otherwise sign in or issue a new invitation |
| `eitmad.error.server-token-reuse.v1` | A rotated refresh token was presented again | Find the token-family revocation audit and device attribution | Stop retrying, sign in again, and investigate possible token copying |
| `eitmad.error.server-device-proof-invalid.v1` | Wrong key, stale timestamp, reused nonce, changed signed bytes, or device ID collision | Compare canonical proof construction, server time, and registered public-key fingerprint | Correct clock or signing code; re-enroll through an authorized flow if the key changed |
| `eitmad.error.authorization-denied.v1` | Missing owner/member/domain relation or wrong tenant/scope | Inspect the redacted denial audit and relationship path | Repair the relationship through its owner-authorized API; never alter the request claim only |
| Cross-tenant request returns data | Critical isolation defect or privileged database role bypass | Stop traffic and reproduce with synthetic tenants and the normal application role | Contain the deployment, rotate exposed credentials, preserve evidence, and repair Rust checks plus RLS before restart |
| `eitmad.error.server-client-incompatible.v1` | No protocol overlap, a required capability is missing, or schema range is absent | Compare both hello messages with protocol `1.4` requirements | Upgrade the older peer; do not translate or ignore required behavior |
| `eitmad.error.server-idempotency-mismatch.v1` | One key was reused for changed intent | Compare safe fingerprints and operation identifiers, not payload content | Keep the first result; use a new key only for a new user intent |
| `eitmad.error.server-snapshot-required.v1` | Requested checkpoint is older than retained history or no usable incremental range exists | Check the snapshot manifest, scope, schema, checkpoint, chunk count, and checksums | Download all chunks, verify them, replace scoped projection atomically, and resume from the snapshot checkpoint |
| An operation returns an open conflict | Base revision was stale and no safe domain merge exists | Inspect conflict ID, revisions, and provenance through an authorized view | Use the domain resolution workflow; keep both inputs and history |
| A subscription repeats an event | Client acknowledgement did not commit or reconnect resumed from an older cursor | Compare durable cursor and event ID | Apply idempotently, acknowledge only after local commit, then resume |
| License is denied | Expired beyond grace, suspended, unavailable outside allowed grace, or entitlement absent | Read effective license state through the authorized boundary | Repair provider state or entitlement; suspension must not receive grace |

## Check a failed server connection

1. Confirm `/livez`, then `/readyz`.
2. Confirm HTTPS certificate identity and time validity. Development plaintext must be loopback-only and explicitly enabled.
3. Confirm authentication headers contain the expected device ID, current timestamp, unique nonce, and signature over the exact canonical request.
4. Confirm the WebSocket first message is `eitmad.server.hello.v1`.
5. Confirm overlap with protocol `1.4`, every required server capability, and the registered domain schema.
6. Retry only when the error is retryable. Never retry token reuse, idempotency mismatch, scope mismatch, or incompatibility in a loop.

## Recover snapshot and history safely

Treat a snapshot as one scoped atomic replacement. Reject a manifest or chunk whose tenant, organization, schema, sequence, count, or checksum differs. Do not combine chunks from different snapshot IDs. Persist the complete projection and checkpoint before acknowledging it. If the client fails before commit, restart the same snapshot transfer or request a new manifest; do not advance its checkpoint.

Compaction must preserve the 90-day history floor and every checkpoint not covered by a complete snapshot. If required data was removed without a valid snapshot, restore PostgreSQL from a consistent backup and stop clients from advancing until integrity is confirmed.

## Verify recovery

Use two synthetic tenants and two registered devices. Confirm valid login and refresh rotation, rejection of a replayed nonce and refresh token, no cross-tenant reads, exact duplicate-operation replay, mismatched duplicate rejection, conflict preservation, snapshot checksum validation, subscription resume, and incompatible-client rejection. Confirm all accepted mutations and denials have redacted audit evidence.

For commands and deployment order, use [Run and recover the modular server authority](../operations/run-server-authority.md). For invariants and owners, use [Extend the modular server authority safely](../developer/subsystems/server-authority.md).
