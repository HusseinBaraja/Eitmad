---
title: "ADR-0029: Require live server authority for protected access"
description: "Deny protected desktop access during disconnection while retaining local work and its authorship."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "identity and security maintainers"
last_verified: "2026-10-18"
review_triggers:
  - "offline access, server revalidation, session lifetime, or pending-work attribution changes"
keywords:
  - "offline authorization"
  - "saved password verifier"
  - "live server check"
---

# ADR-0029: Require live server authority for protected access

> - Status: Accepted
> - Date: 2026-10-18
> - Decision owners: Product developer
> - Supersedes: [ADR-0022](0022-persistent-tenant-identity-and-safe-storage-recovery.md), offline protected-access allowance only
> - Superseded by: None

## Context

Shared account administration must control access on other installations. A disconnected PC cannot receive a server revocation. Local user sessions expire after eight hours, but saved password verifiers have no maximum age and could previously issue another local session. The session limit therefore did not bound offline access. The product developer selected a required server connection for all protected access.

## Decision

Require a fresh authenticated server check for protected desktop access. A saved verifier and an unexpired local session are insufficient. Do not cache successful checks as an offline lease. Rust enforces this rule at the local IPC boundary and during idle subscription revalidation; the native shell only renders the result. Missing connection settings, invalid credentials, trust errors, timeouts, and network failure deny access.

A failed live check closes the local session and protected subscriptions. The shell clears protected views and returns to sign-in. Reconnection requires a new sign-in and does not reopen a closed session. Keep local sign-out, enrollment, and process shutdown available for recovery. Isolated test fixtures can use local authority without representing a production access path.

Connection loss alone does not deactivate an account or remove its imported assignments. Confirmed server denial follows the account-revocation path. Neither path deletes cached records or pending work, replaces its original author, or changes its session, device, scope, or idempotency identity. Server authority can reject replay under the original revoked identity.

The eight-hour local session expiry remains. The current recheck and network bounds are defined in the [identity authority](../developer/subsystems/identity-foundation.md#disconnected-authorization-limit). They describe failure detection, not instantaneous delivery of a remote revocation.

## Consequences and alternatives

Protected work is unavailable during server or network outages. This availability cost is accepted. Retained data remains available for authorized recovery after an online sign-in. No storage migration or verifier deletion is required; previous local sessions do not bypass the new check.

An unlimited saved-verifier policy was rejected because it cannot bound removed access. A fixed offline grace period was rejected in favor of the explicit live-connection requirement. Deleting retained work was rejected because it loses data without proving revocation.

## Arabic UX and verification

Remove **العمل دون اتصال** from sign-in. Explain that entry and work require a server connection and that data remains saved. Access-check failure returns to sign-in with **تعذر تأكيد الوصول إلى حسابك. تحقق من اتصال الخادم ثم سجل الدخول. تبقى بياناتك محفوظة.**

The focused runtime test covers loss of live authority and denies reopening a closed session after reconnection. The [two-client procedure](../operations/run-server-authority.md#verify-connected-desktop-accounts) checks outage denial, idle subscription closure, online recovery, account changes, retained caches, pending authorship, and sign-out without application restarts. Render the changed sign-in screen at the supported window sizes and record actual dimensions and display scaling.
