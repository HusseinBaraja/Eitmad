---
title: "Extend persistent identity safely"
description: "Understand device, user, account, session, tenant, organization, and workspace persistence and offline attribution."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust identity and storage maintainers"
last_verified: "2026-10-17"
review_triggers:
  - "identity topology, session lifecycle, tenant isolation, offline policy, or audit attribution changes"
keywords:
  - "identity_sessions"
  - "TenantId"
  - "offline session"
  - "session attribution"
  - "هوية"
---

# Extend persistent identity safely

Rust owns stable identity IDs and their local persistence. Storage version 5 adds device, user, account, tenant, organization, workspace, and session records in `crates/storage/src/identity.rs`; native shells do not assert or write this topology.

## Model and ownership

`TenantId` is the isolation root. An account binds one `UserId` to one tenant. Organizations and workspaces belong to one tenant; a workspace may belong to an organization in that same tenant. A device is installation-local and tenant-neutral. A session binds its principal, account, user, device, tenant, optional organization/workspace, issue/expiry times, last observation, connectivity, and closure state.

The public storage boundary accepts typed IDs from `eitmad-contracts`. It never exposes a raw SQLite connection. Local installation bootstrap and desktop-account administration own topology writes and commit them with audit. Composite foreign keys reject cross-tenant account, organization, workspace, and session references.

## Offline session behavior

A persisted session may be `Online` or `Offline`. Connectivity does not bypass expiry or closure: `PersistentSession::is_locally_usable_at` requires an issued, unexpired, open session. `persist_desktop_session` and `close_desktop_session` commit sign-in and sign-out with audit. Closure matches the tenant, principal, device, and session; repeated sign-out adds no duplicate audit.

No bearer token, refresh token, or password is stored in the identity tables. Storage version 10 adds `desktop_accounts` for separate, provisioned Manager and Receptionist accounts and their Argon2 password verifiers. The installation owner is a provisioning identity; the process handshake projects a device principal without its owner relation. `DesktopAuthenticator` verifies a password, then issues a separate eight-hour durable user session. Offline sign-in requires the password. Session validation checks the account, device, tenant, scope, expiry, and closure before IPC dispatch. Sign-in or sign-out commits the session change and a user-attributed audit record in one transaction.

`AuthorityStore::provision_desktop_account` remains the trusted Rust import boundary for an already verified control-plane account. Manager administration uses the same `desktop_accounts` projection instead of a second account store. Protocol `1.8` adds revisioned create, display-name and role update, deactivation, and list operations. Creation stores an Argon2 verifier. Role changes and deactivation close all open sessions for the account. Deactivation also removes its active role relationship. Rust rejects a stale revision and any change that would leave no active Manager. Every accepted mutation and every denied or invalid attempt creates a redacted audit result; passwords and verifiers never enter audit.

The current engine does not yet import accounts from the server control plane. A new installation has no ordinary sign-in account until a trusted importer provisions the first Manager. Debug builds expose the explicit `seed-development-accounts` command for synthetic local testing. `run.ps1` invokes it before starting the Windows shell. Debug authentication accepts short passwords only for the two fixed seeded account IDs; all other accounts and release authentication keep the 12-character minimum. The seed command is absent from release builds, does not replace existing accounts, and does not print password verifiers or entered secrets.

Manager and Receptionist relationships produce distinct effective permissions. An active `DesktopSessionState` also carries the current Rust-owned `DesktopAccountRole`. The WPF shell maps that role to the existing Manager or Receptionist surface; it does not interpret permission decisions, infer a role from a username, or accept a shell-owned role switch. Rust still authorizes every command and query independently of the visible surface.

## Shared server account lifecycle

`crates/contracts/src/server_accounts.rs` defines version-one server account administration. `server/control-plane/src/identity/accounts.rs` owns transitions; `account_projection.rs` reads current assignments. The host exposes authenticated `POST /v1/accounts/list` and `POST /v1/accounts/transition`. Invitation activation uses `POST /v1/auth/activate`. See [server account operations](../../operations/run-server-authority.md#manage-shared-server-accounts) for the request sequence.

Administration requires an active device-bound session and an explicit Owner relationship on the authenticated tenant. Payloads cannot select another tenant. Manager and Receptionist relationships, usernames, and UI role names grant no Owner authority. Owner alone grants no operational role. Owner transfer and Owner deactivation are unsupported; an Owner can receive operational assignments separately.

Listing uses an exclusive account UUID cursor and a limit of 1 through 100. It includes pending, active, locked, and disabled accounts, their revisions, and organization assignments. It excludes credentials and the raw authorization graph. Organization membership remains after access is removed so disabled accounts remain identifiable.

`ChangeServerAccount` carries an idempotency key and one change:

| Change | Committed behavior |
| --- | --- |
| `invite` | Create a pending account at revision 1, its organization membership, explicit role relationships, and a seven-day invitation |
| `assign` | Require the current revision; replace Manager and Receptionist relationships only for the selected organization and its branches; preserve Owner and unrelated relationships |
| `deactivate` | Require the current revision; disable a non-Owner account, consume pending invitations, remove its user relationships, and revoke sessions and token families |
| `revokeSessions` | Require the current revision; revoke all account sessions and token families, retaining assignments and permitting later password login |

Activation and each accepted administrative update increment the account revision once. Assign also revokes every account session, including the caller's session when changing their own account. An empty assignment list removes operational grants. A Receptionist assignment needs at least one branch. Each role appears at most once, with at most 100 distinct registered branches in the selected organization. Foreign account, organization, or branch IDs fail closed. Duplicate usernames and duplicate role or branch entries are invalid.

An account can receive both operational relationships. Rust evaluates their separate permissions according to the [workflow matrix](manager-receptionist-workflows.md#action-and-permission-matrix): Managers cover the organization; Receptionist mutations require an assigned branch. An account role is not a client authorization claim.

Migration `0016_server_accounts.sql` adds revisions, retained organization membership, and mutation and activation receipts under forced tenant RLS. It backfills membership from retained organization relationships. A tenant lifecycle lock serializes administration and activation; login also locks its target account. State, relationships, revocation, successful audit, and the retry receipt commit together. A failed mandatory audit rolls back the complete change. Authenticated denials, invalid input, and revision conflicts have redacted audit results. Foreign account IDs are omitted from denial audit targets.

Receipts bind tenant, actor user, idempotency key, and the exact typed request through a keyed fingerprint. After current authorization succeeds, an exact retry returns its original success or terminal business failure, even after later revisions. Changed intent returns `eitmad.error.server-idempotency-mismatch.v1`. Unavailable transactions have no accepted receipt. A new intent after conflict needs a new key and current revision. Replaying a revocation receipt does not revoke newer sessions.

Invitation credentials and initial session tokens use separate HMAC contexts, random IDs, and the protected server token key. Receipt rows store no plaintext secret. An exact activation retry binds the complete request, including password and device key, through a keyed fingerprint. It returns the original session and token pair only while the original access token is unexpired, the refresh token is unconsumed, and account, device, session, and token family remain usable. Changed, revoked, rotated, or expired activation results fail authentication; use ordinary login for recovery. Retries never create another session or restore access. Deliver invitations through a protected channel; email delivery remains unimplemented.

This server lifecycle does not synchronize desktop passwords, offline sessions, or local account projections. The trusted Rust import boundary above remains separate. No native account-management screen or online desktop enrollment flow is added.

Focused proof is `server/host/tests/accounts.rs`, using real PostgreSQL with a non-superuser, non-`BYPASSRLS` role and the real Axum handlers, token authentication, and signed device proofs. Only foreign-tenant fixtures and audit fault injection use SQL; account operations use supported routes. Re-run this proof when account contracts, authorization, revisions, retries, or revocation change.

## Audit attribution

Mutation audit records keep the principal and scope and now also preserve optional `SessionId` and `DeviceId`. Configuration and authorization mutation builders populate both from `AuthorizationContext`. Session rows retain the account, user, tenant, organization, workspace, and offline snapshot needed to investigate attribution without placing personal display data in the audit row. Audit rows remain append-only and do not depend on a session foreign key, so closing or later retaining sessions cannot erase historical attribution IDs.

Manual restore preserves the operator's authentication evidence in the staged audit transaction and closes its maintenance session. See [manual SQLite recovery](../../operations/recover-local-storage.md#manual-sqlite-backup-and-restore) for the evidence retained and the credential and permission state restored from the backup.

## Arabic and mixed-direction data

Identity authority uses opaque UUIDs and does not branch on language. Future display names remain UTF-8 product data and must not become identifiers or tenant keys. Arabic labels such as `المستخدم`, `الحساب`, `المنشأة`, `مساحة العمل`, and `الجلسة` belong in localized shell copy; Rust IDs remain LTR and directionally isolated when displayed.

## Failure modes and recovery

- A missing referenced identity or cross-tenant reference rolls back the transaction and returns sanitized `StorageError`.
- An expired or closed session fails validation before IPC command or query dispatch.
- Corrupt, drifted, newer, or out-of-window storage prevents readiness before identity access.
- Offline operation never fabricates a tenant or silently moves a session between scopes.

Use [recover local storage](../../operations/recover-local-storage.md) before any database intervention. Do not edit identity tables or migration history manually.

## Tests and extension points

Colocated tests cover audited desktop-session persistence after reopen, offline expiry, durable closure, tenant-scoped lookup, and foreign-key isolation. Audit tests cover stored session/device attribution. Export tests prove tenant filtering and sensitive-table exclusion.

Add identity behavior in this vertical. Add the next immutable feature migration, preserve the declared storage compatibility window, and test denial, offline behavior, audit attribution, Arabic/mixed-direction display boundaries, and recovery. Also review [local storage](local-storage.md), [ADR-0022](../../decisions/0022-persistent-tenant-identity-and-safe-storage-recovery.md), and the [storage recovery](../../operations/recover-local-storage.md).
