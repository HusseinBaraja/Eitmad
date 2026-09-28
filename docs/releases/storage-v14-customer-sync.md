---
title: "Upgrade customer synchronization to storage version 14"
description: "Apply the customer sync exception migration and verify scoped PostgreSQL delivery, recovery, and rollback limits."
audience: "operations"
page_type: "release"
status: "active"
owner: "customer, storage, sync, and server maintainers"
last_verified: "2026-09-28"
review_triggers:
  - "customer sync protocol, storage version 14, or server domain behavior changes"
keywords:
  - "customer.sync-exceptions.v1"
  - "storage version 14"
  - "customer synchronization"
---

# Upgrade customer synchronization to storage version 14

Storage version `14` adds `customer.sync-exceptions.v1`. It retains rejected and conflicted customer delivery state across engine restarts without removing the contact or its pending outbox record. PostgreSQL migration `0005_customer_branches.sql` registers tenant-scoped branches under organizations. The server registers `eitmad.schema.customer.v1` version `1` and accepts local-first customer changes through the existing authenticated WebSocket route.

## Before rollout

- Use a PostgreSQL application role without `BYPASSRLS`. Apply the existing control, sync, audit, and administration migrations before starting the host.
- Register the intended branch under its organization and enroll the server account and device. A Manager needs an organization relationship; a Receptionist needs an exact branch relationship. The local customer sync cycle needs that trusted session and a Rust worker caller; normal engine startup does not yet configure or schedule it.
- Preserve the local SQLite database and its pre-migration backup. Do not copy one client's SQLite file to another device.

## Verify

1. Start the new engine on storage version `13` or another supported version. Confirm migration to `14` and the validated `eitmad.pre-migration-vN-to-v14.sqlite3` backup.
2. Create one synthetic Arabic contact in an authorized branch. Confirm `Pending` before delivery and `Confirmed` after the server echo and checkpoint projection.
3. Run the focused live test `two_isolated_customer_engines_recover_and_preserve_conflicts` against an empty disposable PostgreSQL database and development TLS certificates. It uses two isolated engine directories and two registered devices. Confirm exact replay has one server operation, concurrent edits retain the local value with `Conflicted`, and a different tenant cannot read the operation rows.
4. Restart a client with pending work. Confirm the original change ID is replayed and the customer projection catches up before the shared checkpoint advances.

## Failure and recovery

An unknown delivery result remains pending and must retry with the same idempotency key. A rejection or conflict keeps the local contact visible and holds its outbox work for a separate resolution path. A failed projection does not advance the shared engine checkpoint or acknowledge the server page.

An older engine cannot open version `14`. If no writes occurred after migration, stop the engine and restore the validated pre-migration backup before deploying an older compatible build. If writes occurred, preserve the current database and use a compatible forward change. Never delete customer outbox, exception, conflict, or PostgreSQL operation rows to force progress.

See [customer capability](../developer/subsystems/customers.md), [local storage](../developer/subsystems/local-storage.md), and [server authority](../developer/subsystems/server-authority.md).
