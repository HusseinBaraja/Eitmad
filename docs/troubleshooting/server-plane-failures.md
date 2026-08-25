---
title: "Resolve relay, update distribution, and administration failures"
description: "Diagnose authorization, relay lifecycle, signed manifest, channel, backup, migration, audit, and support workflow failures without crossing tenant boundaries."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-08-24"
review_triggers:
  - "relay, update distribution, administration, backup, migration, or support error behavior changes"
keywords:
  - "eitmad.error.relay-unavailable.v1"
  - "eitmad.error.update-manifest-invalid.v1"
  - "eitmad.error.admin-unavailable.v1"
  - "backup NotConfigured"
---

# Resolve relay, update distribution, and administration failures

These failures affect connection coordination, update selection, or operational evidence. They do not authorize editing business records. Preserve PostgreSQL, signed manifest files, correlation IDs, and tenant scope while you diagnose them.

## Symptoms and data safety

- Relay open, heartbeat, reconnect, or health returns `eitmad.error.relay-unavailable.v1`, `eitmad.error.relay-session-not-found.v1`, or authorization denial.
- Manifest publication returns `eitmad.error.update-manifest-invalid.v1` or `eitmad.error.update-distribution-unavailable.v1`.
- Update check returns `Incompatible`, `Ineligible`, or no package.
- Administration returns `eitmad.error.admin-unavailable.v1`, backup `NotConfigured`, migration `Pending`, or a failed support workflow.
- Audit access or tenant/device visibility is denied.

Pending sync work and existing manifests remain intact for normal denials and validation failures. A support hook may have committed before a final cross-plane audit failure. Do not repeat a destructive support action until you reconcile its workflow and correlation IDs.

## Fast checks

1. Record protocol version, exact error ID, route, state, tenant-scoped correlation ID, and time. Do not record access tokens, device proofs, public-key bytes, manifest private keys, relay payloads, or customer content.
2. Verify `x-eitmad-peer-hello` contains a base64url-encoded protocol `1.5` `PeerHello` with the route capability, then verify the caller has an active authenticated device session. Relay requires tenant membership; administration requires tenant ownership. Manifest publication also requires the configured operator tenant and `eitmad.permission.server.update-manifest.publish.v1`.
3. For peer relay, verify the target device is registered, in the same tenant, and not revoked.
4. For reconnect, compare the current time with `nextReconnectAt` and the attempt count with the limit of eight.
5. For update publication, verify key ID, Ed25519 signature, schema version `1`, exact channel, semantic version, HTTPS package URL, non-zero size, and lowercase SHA-256.
6. For update check, compare effective device channel, client version, protocol, required capabilities, rollout start/pause/percentage, platform, and architecture.
7. For administration, verify PostgreSQL migration versions `1`, `2`, and `3`, tenant RLS context, audit availability, and backup reporter state.

## Evidence to resolution

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| Relay denial before session creation | Missing relationship, wrong source device, or foreign peer | Compare session device and tenant-scoped relationships | Repair the approved relationship or request; never weaken the check |
| `eitmad.error.server-client-incompatible.v1` before authentication | Missing or incompatible HTTP negotiation | Decode only the synthetic `x-eitmad-peer-hello` and compare protocol/capability identifiers | Send protocol `1.5` and the exact route capability; do not bypass negotiation |
| `RetryNotDue` or reconnecting health | Backoff has not elapsed | Inspect `nextReconnectAt` and attempt count | Wait for the due time; restore peer/server route before retry |
| Relay not found for an owner support action | Wrong, expired, restarted, or foreign session | Verify session ID and tenant without listing another tenant | Let the client open a new session; do not fabricate metadata |
| Signature changes fail verification | Wrong key or modified manifest bytes | Verify canonical JSON with the release public key | Recreate and sign a new immutable manifest outside the server |
| `UpdatePlaneError::ReconciliationRequired(manifest_id)` | Success audit and repository rollback both failed | Stop publication and compare the named manifest file with the correlation-scoped audit outcome | Reconcile the named file through the release procedure before retrying |
| Stable rejects prerelease | Channel rule violation | Inspect semantic-version prerelease field | Publish to `beta` or `canary`, or sign a stable version |
| Client `Incompatible` | Version, protocol, capability, or package mismatch | Inspect the exact `UpdateIneligibilityReason` | Publish a supported intermediate or matching package |
| Backup `NotConfigured` | No authoritative status row | Check approved backup reporter deployment | Configure reporting and verify a real restore; do not mark success manually |
| Migration `Pending` or startup migration error | Migration `3` missing or prerequisite/checksum failure | Run read-only migration registry checks | Back up, repair prerequisite or restore correct migration source, rerun `migrate` |
| Admin unavailable after support request | Provider or final audit failed | Query tenant-scoped workflow by ID and audit by correlation | Reconcile the first result before starting another workflow |
| Cross-tenant device list is empty or denied | Tenant isolation worked | Confirm authenticated tenant ID | Use the correct tenant session; never use privileged SQL to merge views |

## Verify recovery

- Relay health returns the expected tenant counts, and a new synthetic session completes open, heartbeat, and close.
- Signature verification still rejects a one-byte manifest change.
- Update check selects only the assigned channel and exact platform/architecture package.
- Admin health, backup, migration, audit, tenant, and device routes succeed for a tenant owner and remain denied for a non-owner or another tenant.
- Every completed or denied action has a redacted audit row with the expected operation and correlation ID.

## Escalate safely

Escalate to connectivity maintainers for route-hook or reconnect failures, release maintainers for signature and compatibility failures, operations tooling maintainers for backup/migration/support state, and security maintainers for suspected cross-tenant disclosure.

Provide only stable error IDs, protocol/capability set, tenant-scoped correlation and workflow IDs, redacted status, timestamps, and synthetic reproduction steps. Never include tokens, passwords, private keys, proof signatures, relay frames, package contents, audit data from another tenant, or real device/user labels.

Related pages: [WAN relay](../developer/subsystems/wan-relay-coordination.md), [update distribution](../developer/subsystems/update-distribution.md), [server administration](../developer/subsystems/server-administration.md), [server operations](../operations/run-server-authority.md), and [privacy failures](privacy-and-secret-leakage.md).
