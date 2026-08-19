---
title: "Extend persistent identity safely"
description: "Understand device, user, account, session, tenant, organization, and workspace persistence and offline attribution."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust identity and storage maintainers"
last_verified: "2026-08-19"
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

Rust owns stable identity IDs and their local persistence. Storage version 5 adds device, user, account, tenant, organization, workspace, and session records in `crates/storage/src/identity.rs`; native shells may assert synthetic development identities but do not own or write this topology.

## Model and ownership

`TenantId` is the isolation root. An account binds one `UserId` to one tenant. Organizations and workspaces belong to one tenant; a workspace may belong to an organization in that same tenant. A device is installation-local and tenant-neutral. A session binds its principal, account, user, device, tenant, optional organization/workspace, issue/expiry times, last observation, connectivity, and closure state.

The public storage boundary accepts typed IDs from `eitmad-contracts`. It never exposes a raw SQLite connection. `persist_identity_topology` commits the tenant/user/account/organization/workspace graph atomically and verifies conflict results. Composite foreign keys reject cross-tenant account, organization, workspace, and session references.

## Offline session behavior

A persisted session may be `Online` or `Offline`. Connectivity does not bypass expiry or closure: `PersistentSession::is_locally_usable_at` requires an issued, unexpired, open session. `refresh_session` moves the last-seen time forward and records connectivity only for an active, unexpired session. `close_session` is tenant-scoped and idempotent.

No credential, bearer token, refresh token, password, or secret is stored in the identity tables. Persistence supplies identity continuity and attribution, not authentication proof. Production session issuance still requires a trusted authenticator and authorization path.

## Audit attribution

Mutation audit records keep the principal and scope and now also preserve optional `SessionId` and `DeviceId`. Configuration and authorization mutation builders populate both from `AuthorizationContext`. Session rows retain the account, user, tenant, organization, workspace, and offline snapshot needed to investigate attribution without placing personal display data in the audit row. Audit rows remain append-only and do not depend on a session foreign key, so closing or later retaining sessions cannot erase historical attribution IDs.

## Arabic and mixed-direction data

Identity authority uses opaque UUIDs and does not branch on language. Future display names remain UTF-8 product data and must not become identifiers or tenant keys. Arabic labels such as `المستخدم`, `الحساب`, `المنشأة`, `مساحة العمل`, and `الجلسة` belong in localized shell copy; Rust IDs remain LTR and directionally isolated when displayed.

## Failure modes and recovery

- A missing referenced identity or cross-tenant reference rolls back the transaction and returns sanitized `StorageError`.
- An expired or closed session cannot be refreshed and returns `false` without changing state.
- Corrupt, drifted, newer, or out-of-window storage prevents readiness before identity access.
- Offline operation never fabricates a tenant or silently moves a session between scopes.

Use [recover local storage](../../operations/recover-local-storage.md) before any database intervention. Do not edit identity tables or migration history manually.

## Tests and extension points

Colocated tests cover reopen persistence, offline expiry, online refresh, durable closure, tenant-scoped lookup, and foreign-key isolation. Audit tests cover stored session/device attribution. Export tests prove tenant filtering and sensitive-table exclusion.

Add identity behavior in this vertical. Add the next immutable feature migration, preserve the storage 2–5 compatibility window or release a deliberate new window, and test denial, offline behavior, audit attribution, Arabic/mixed-direction display boundaries, and recovery. Also review [local storage](local-storage.md), [ADR-0022](../../decisions/0022-persistent-tenant-identity-and-safe-storage-recovery.md), and the [storage v5 release note](../../releases/storage-v5-identity-recovery.md).
