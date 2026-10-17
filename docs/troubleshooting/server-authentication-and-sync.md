---
title: "Resolve server authentication, tenant, and sync failures"
description: "Diagnose server configuration, PostgreSQL, device proof, token, isolation, compatibility, snapshot, idempotency, and conflict failures safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-17"
review_triggers:
  - "server error identifiers, authentication, tenant isolation, compatibility, or sync recovery changes"
keywords:
  - "server-authentication-failed"
  - "server-device-proof-invalid"
  - "server-snapshot-required"
  - "server-client-incompatible"
  - "server-subscription-ack-unsupported"
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
| Direct route reports `EncryptionRequired` | The server certificate is untrusted, expired, or issued for another host | Check the HTTPS host name and the configured PEM trust anchor against the server certificate; do not collect its private key | Replace the local trust anchor or server certificate through the approved deployment process; never disable validation |
| Direct route reports `AuthenticationFailed` | The stored session is absent, rejected, or cannot be refreshed | Check device registration, account/device IDs, server time, and redacted authentication audit | Activate or sign in again through the Rust-owned credential flow |
| `eitmad.error.authorization-denied.v1` | Missing owner/member/domain relation or wrong tenant/scope | Inspect the redacted denial audit and relationship path | Repair the relationship through its owner-authorized API; never alter the request claim only |
| Cross-tenant request returns data | Critical isolation defect or privileged database role bypass | Stop traffic and reproduce with synthetic tenants and the normal application role | Contain the deployment, rotate exposed credentials, preserve evidence, and repair Rust checks plus RLS before restart |
| `eitmad.error.server-client-incompatible.v1` | No protocol overlap, a required capability is missing, or schema range is absent | Compare both hello messages with protocol `1.4` requirements | Upgrade the older peer; do not translate or ignore required behavior |
| `eitmad.error.server-idempotency-mismatch.v1` | One key was reused for changed intent | Compare safe fingerprints and operation identifiers, not payload content | Keep the first result; use a new key only for a new user intent |
| `eitmad.error.server-snapshot-required.v1` | Requested checkpoint is older than retained history, no usable incremental range exists, or a stale base revision named a record the server has never stored (or removed by compaction) | Check the snapshot manifest, scope, schema, checkpoint, chunk count, and checksums; for a stale base revision confirm the record exists server-side | Download all chunks, verify them, replace scoped projection atomically, and resume from the snapshot checkpoint |
| `eitmad.error.server-subscription-ack-unsupported.v1` | The client sent `eitmad.server.acknowledge.v1`; the server does not persist connection-level subscription acknowledgements yet | Confirm the client uses `eitmad.sync.acknowledge.v1` for durable checkpoints instead of the rejected message | Keep event application idempotent, rely on scoped cursor resume, and track the acknowledgement support follow-up |
| A live WebSocket closes during traffic without an error frame | Periodic session revalidation found an expired access token, ended session, or revoked device | Compare closure time with token/session lifetime and device revocation audit rows | Reauthenticate with fresh credentials; do not disable revalidation to keep sockets open |
| `bootstrap` exits with a usage failure (`eitmad.error.contract-invalid.v1`) | Wrong argument count for `eitmad-server bootstrap` | Count arguments: command plus tenant code, tenant name, organization name, owner username | Rerun with exactly four values; quote Arabic display names that contain spaces |
| An operation returns an open conflict | Base revision was stale and no safe domain merge exists | Inspect conflict ID, revisions, and provenance through an authorized view | Use the domain resolution workflow; keep both inputs and history |
| A subscription repeats an event | Client acknowledgement did not commit or reconnect resumed from an older cursor | Compare durable cursor and event ID | Apply idempotently, acknowledge only after local commit, then resume |

## Check a failed server connection

1. Confirm `/livez`, then `/readyz`.
2. Confirm HTTPS certificate identity and time validity. Development plaintext must be loopback-only and explicitly enabled.
3. Confirm authentication headers contain the expected device ID, current timestamp, unique nonce, and signature over the exact canonical request.
4. Confirm the WebSocket first message is `eitmad.server.hello.v1`.
5. Confirm overlap with protocol `1.4`, every required server capability, and the registered domain schema.
6. Retry only when the error is retryable. Never retry token reuse, idempotency mismatch, scope mismatch, or incompatibility in a loop.

## Credential contention and session ownership

The Rust direct route serializes each stored rotating token pair by credential reference. HTTP reads, connection setup, refresh, and `store_session` / `store_account_session` persistence share that ownership. A waiter uses its remaining monotonic deadline and loads the credential only after admission. Short concurrent use does not immediately become `Unavailable`. Refresh runs once for the due pair; later owners read the persisted replacement. Session persistence cannot race an earlier refresh and be overwritten by its old result.

HTTP requests use separate sockets and immutable driver state, so requests on one client do not compete for a driver mutex. Connection establishment uses one ten-second budget for credential admission, refresh, TCP/TLS, and WebSocket negotiation. It retains credential ownership through authentication failure cleanup so an older connection attempt cannot delete a newly stored session. Standalone session persistence has bounded ten-second admission. Native credential-store calls remain synchronous and cannot be interrupted by this network deadline.

Desktop sign-in and sign-out use the local Rust session authority. Sign-out closes the desktop session and does not delete the reusable server credential. IPC reauthorization withholds in-flight results from closed sessions. Credential admission does not change that boundary. Account-derived references and user/tenant/device checks remain required before refresh. HTTP scope validation and server authorization remain required before data access.

Use the [correlated phase diagnostic](local-ipc-failures.md#read-the-bounded-diagnostic-events) to compare `CredentialAdmission`, `CredentialRefresh`, TLS, receive, and confirmation durations. Admission reports success after waiting, deadline exhaustion, or poison; DNS keeps its independent bounded resolver admission. No trace from a new build establishes the cause of an older cached response.

The focused proof is in `credential_admission.rs`, `catalog_image.rs`, and `credential_contention_tests.rs`. Run the owner library tests normally. The ignored TLS contention tests require fresh disposable certificates through `EITMAD_DIRECT_TEST_CERTIFICATE`, `EITMAD_DIRECT_TEST_PRIVATE_KEY`, and `EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE`; they verify concurrent confirmation, single refresh, session replacement ordering, and a shared establishment deadline. Keep certificate keys and credentials private.

## Recover snapshot and history safely

Treat a snapshot as one scoped atomic replacement. Reject a manifest or chunk whose tenant, organization, schema, sequence, count, or checksum differs. Do not combine chunks from different snapshot IDs. Persist the complete projection and checkpoint before acknowledging it. If the client fails before commit, restart the same snapshot transfer or request a new manifest; do not advance its checkpoint.

Compaction must preserve the 90-day history floor and every checkpoint not covered by a complete snapshot. If required data was removed without a valid snapshot, restore PostgreSQL from a consistent backup and stop clients from advancing until integrity is confirmed.

## Verify recovery

Use two synthetic tenants and two registered devices. Confirm valid login and refresh rotation, rejection of a replayed nonce and refresh token, no cross-tenant reads, exact duplicate-operation replay, mismatched duplicate rejection, conflict preservation, snapshot checksum validation, subscription resume, and incompatible-client rejection. Confirm all accepted mutations and denials have redacted audit evidence.

For commands and deployment order, use [Run and recover the modular server authority](../operations/run-server-authority.md). For invariants and owners, use [Extend the modular server authority safely](../developer/subsystems/server-authority.md).
