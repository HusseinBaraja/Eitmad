---
title: "Extend server administration safely"
description: "Understand least-privilege diagnostics, health, backup, migration, audit, fleet visibility, support workflows, storage, and recovery."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "operations tooling maintainers"
last_verified: "2026-08-24"
review_triggers:
  - "administrative permissions, diagnostics, backup, migration, audit, visibility, support, or operational storage changes"
keywords:
  - "administration plane"
  - "BackupStatus"
  - "SupportWorkflow"
  - "eitmad.error.admin-unavailable.v1"
---

# Extend server administration safely

The administration plane provides tenant-scoped operational evidence and approved support workflows. It is not a superuser or database-repair bypass. Every interface requires an authenticated tenant owner and writes a redacted audit outcome.

## Authority and storage

| Concern | Rust authority |
| --- | --- |
| Diagnostic, health, backup, migration, visibility, audit, and support contracts | `crates/contracts/src/administration.rs` |
| Authorization-first service boundary | `server/admin-plane/src/lib.rs` |
| PostgreSQL migration and checksum | `server/admin-plane/migrations/0003_admin_foundation.sql` and `src/database.rs` |
| Scoped operational queries and support persistence | `server/admin-plane/src/postgres.rs` |
| Relationship decisions and append-only audit | `server/control-plane/src/access.rs` |
| Authenticated HTTP routes and support hooks | `server/host/src/http.rs` and `src/planes.rs` |

Migration `3` creates `operations.backup_status` and `operations.support_workflows`. Both tables have explicit `tenant_id`, enable and force PostgreSQL row-level security, and use the transaction-local `eitmad.tenant_id` setting. Database enum values use snake_case text while contract JSON uses camelCase. The migration requires control migration `1` and sync migration `2`, reports `AdminDatabaseError::MissingPrerequisites` when either is absent, and records its checksum as `server.admin-foundation.v1`.

## Administrative interfaces

The `/v1/admin` routes expose diagnostics, component health, backup status, migration status, a bounded audit page, current-tenant visibility, current-tenant device visibility, support workflow start, and signed update publication. The authenticated tenant is always the scope; callers cannot select another tenant through a query parameter.

Backup status distinguishes `Current`, `Stale`, `Running`, `Failed`, and `NotConfigured`. A missing row returns `NotConfigured`; it does not invent a successful backup. Migration status compares the durable migration registry with required version `3`. Audit access returns at most 500 records and includes only stable operation, outcome, target kind, correlation, time, principal, tenant, and redacted failure code.

Tenant visibility reports enabled state, active-device count, active-session count, and last-seen time. Device visibility reports tenant, device, label, revoked state, and last-seen time. It does not expose public keys, tokens, nonces, passwords, domain payloads, or another tenant's devices.

## Support workflows

Support actions are reason coded with lowercase ASCII identifiers from 3 through 64 characters. The service creates a typed `SupportWorkflow`, the PostgreSQL provider persists `running`, the approved hook executes, and the provider commits `succeeded` or `failed` with a redacted error.

Implemented hooks collect diagnostics, acknowledge backup verification workflow execution, close a tenant relay session through the owner-only relay action, and revoke all active sessions for a tenant device. Device-session revocation rechecks ownership and device scope and commits the session mutation plus audit in one control-plane transaction. `RetryMigration` fails as invalid because migrations remain an operator startup procedure, not a remote repair button.

## Authorization, audit, and partial failure

`AdministrationService` checks exact actor tenant before calling `AdministrativeSecurity`. The production adapter requires the tenant-owner relationship. It audits every success, denial, invalid request, and provider failure. A read response is withheld if its audit append fails.

Support workflow state and the cross-plane audit use separate PostgreSQL transactions. If the support hook succeeds but the final administration audit fails, the workflow state remains durable and the client receives failure. Operators must use the correlation and workflow IDs to reconcile this partial failure; they must not repeat a destructive support action blindly.

## Failure and recovery

| Symptom | State guarantee | Safe recovery |
| --- | --- | --- |
| `eitmad.error.authorization-denied.v1` | Provider action did not start; denial audit is attempted | Repair the tenant-owner relationship |
| `eitmad.error.admin-unavailable.v1` | Response is withheld; durable workflow state may exist | Check PostgreSQL, RLS context, audit append, then reconcile by correlation ID |
| Backup `NotConfigured` | No successful backup is claimed | Configure the approved backup reporter and verify restore separately |
| Migration `Pending` or `Failed` | Server must not report ready after startup migration failure | Back up PostgreSQL, repair the migration prerequisite, and rerun `migrate` |
| Failed support workflow | Failure code remains redacted and tenant scoped | Correct the dependency; start a new reason-coded workflow only when the first outcome is known |

Use [server-plane troubleshooting](../../troubleshooting/server-plane-failures.md) and [server operations](../../operations/run-server-authority.md).

## Arabic behavior, tests, and safe extension

No operator UI is present. Stable identifiers and timestamps are locale independent. A future Arabic interface must localize status labels, render UUIDs and error IDs as isolated LTR runs, and never combine device labels from different tenants in one view.

Tests cover backup status completeness, invalid audit limits, administrative denial before data access, audit outcomes, cross-tenant device and support denial, forced-RLS migration text, and unauthenticated host routes. PostgreSQL projections use fallible row decoding so schema or type drift returns `AdministrativeError::Unavailable` instead of terminating the process. Add a provider only through `AdministrationDataSource` or `SupportWorkflowExecutor`; keep permissions narrow, queries bounded, failures redacted, and storage tenant scoped.

Run:

```powershell
cargo test -p eitmad-admin-plane -p eitmad-control-plane -p eitmad-server
```

Related pages: [server authority](server-authority.md), [authorization](authorization.md), [privacy-preserving observability](privacy-preserving-observability.md), [ADR-0026](../../decisions/0026-compose-authorized-operational-server-planes.md), and [protocol 1.5 rollout](../../releases/protocol-1-5-operational-server-planes.md).
