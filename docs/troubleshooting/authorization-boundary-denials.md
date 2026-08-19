---
title: "Resolve scoped authorization, sync, adapter, and plugin denials"
description: "Diagnose deny-by-default relationship decisions and audit failures without exposing policy graphs or cross-tenant data."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "authorization, security, and audit maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "authorization decisions, scope context, boundary gates, error identifiers, or audit persistence changes"
keywords:
  - "eitmad.error.authorization-denied.v1"
  - "plugin rejected"
  - "sync rejected"
  - "cross-workspace"
  - "رفض الصلاحية"
---

# Resolve scoped authorization, sync, adapter, and plugin denials

`eitmad.error.authorization-denied.v1` means Rust found no valid relationship path for the exact actor, action, object, tenant, workspace, and optional conditions. The protected action did not run. Do not infer which tuple was absent from the user-visible denial.

## Symptoms

- A command or query returns `eitmad.error.authorization-denied.v1`.
- A sync operation is rejected before negotiation, pull, push, or acknowledgement work runs.
- An external provider adapter is not invoked.
- A plugin capability callback is not invoked.
- A query response is withheld with `eitmad.error.authorization-unavailable.v1` because mandatory audit persistence failed.
- Future Arabic UI may show a localized equivalent of `رفض الصلاحية`; no native denial UI exists yet.

## Fast checks

1. Use the correlation ID to locate only the sanitized audit envelope.
2. Confirm the authenticated session carries the expected tenant and, when the object or operation is workspace-scoped, the expected workspace. Tenant-wide work may omit workspace context. Never change scope to make a denial disappear.
3. Confirm the requested object's embedded tenant/workspace matches the session.
4. Confirm a rule exists for the exact action and object kind.
5. Confirm a direct principal relation or role-membership path grants one registered rule relation.
6. For inherited access, confirm every explicit parent edge stays in the same tenant/workspace and the graph is no deeper than 16 edges.
7. Confirm required condition attributes use canonical keys and exact values.
8. If audit failed, check storage readiness, migration version 6, disk state, and schema drift before retrying.

## Causes and resolutions

| Evidence | Cause | Resolution | Verify |
| --- | --- | --- | --- |
| Actor tenant differs from object tenant | Cross-tenant request or confused deputy | Stop and correct the caller's object/context binding; do not copy a tuple across tenants | The original cross-tenant request remains denied |
| Workspace target differs from actor workspace | Cross-workspace request | Use the authorized workspace or a separately reviewed tenant-wide object | Same-workspace synthetic request follows policy; crossed request denies |
| No action/object-kind rule | Capability was never modeled | Add a versioned rule with owner review, tests, and documentation | Missing rule denies; new rule grants only named relations |
| Role has object relation but actor lacks role membership | Incomplete delegation | Add/recover the authorized membership through the owning workflow | Actor without membership still denies |
| Parent graph cycles or exceeds 16 | Invalid or overly deep model | Repair the owning data; flatten the hierarchy deliberately | Bounded decision completes and only intended descendants inherit |
| Equality/`all`/`any` condition fails | Missing or non-canonical attribute | Correct authoritative attribute derivation; never trust UI assertions | Matching synthetic attributes allow; mismatches deny |
| Plugin/provider code ran despite denial | Boundary bypass defect | Disable the affected integration, preserve evidence, and escalate as a security event | Rejection test proves callback is not invoked |
| Audit insert fails or schema is below 6 | Storage unavailable or incomplete migration | Stop normal work and follow storage recovery; never disable audit validation | Audit row contains tenant/workspace, target, result, correlation, and redacted error |

## Verify recovery

Use synthetic IDs in one test tenant/workspace. Verify direct allow, absent-relation deny, inherited allow, cross-workspace deny, sync reject, plugin reject, and a complete redacted audit row. Do not use production customer records to probe authorization.

## Escalate safely

Collect protocol/storage versions, boundary kind, stable action/object kind, outcome, error code, timestamp, correlation ID, and whether the callback executed. Do not attach raw tuples, principal/customer UUIDs, attributes, Arabic customer text, tokens, provider payloads, database files, or raw errors. Treat any executed denied action or cross-tenant data exposure as a security incident and mark the audit extension `SecurityEvent` only through the future owning workflow.

Return to [authorization and audit ownership](../developer/subsystems/authorization.md), [local IPC troubleshooting](local-ipc-failures.md), or [storage recovery](local-storage-recovery-failures.md).
