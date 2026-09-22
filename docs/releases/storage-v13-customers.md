---
title: "Upgrade to protocol 1.9 and storage version 13 customers"
description: "Roll out branch-scoped customer contacts, generated bindings, atomic local-first storage, and safe rollback limits."
audience: "operations"
page_type: "release"
status: "active"
owner: "customer, contract, storage, and runtime maintainers"
last_verified: "2026-09-22"
review_triggers:
  - "protocol 1.9, storage version 13, customer schema, or compatibility behavior changes"
keywords:
  - "protocol 1.9"
  - "storage version 13"
  - "customer.initial.v1"
  - "eitmad.schema.customer.v1"
---

# Upgrade to protocol 1.9 and storage version 13 customers

Protocol `1.9` adds typed customer create, update, get, search, subscription, and event contracts. Migration `12` (`customer.initial.v1`) adds scoped customer search state and the bounded customer sync outbox. Migration `13` binds the singleton local desktop branch to the installation tenant. The executable storage boundary is version `13`.

## Compatibility and rollout

- Fresh storage version `0` creates version `13` directly.
- Supported versions `2` through `12` upgrade after a validated `eitmad.pre-migration-vN-to-v13.sqlite3` backup.
- Version `1`, incomplete or changed history, schema drift, and versions newer than `13` fail closed.
- An older engine cannot open storage version `13`. Do not perform an in-place binary rollback after customer writes commit.
- Deploy the Rust engine, protocol catalog, JSON schema, C#/Swift bindings, and consuming adapters from one generated set.
- A peer must negotiate `eitmad.capability.customer.v1` and `eitmad.schema.customer.v1` version `1` before using the feature.

## Verify

1. Start the engine and confirm storage migration `customer.initial.v1` has the expected checksum.
2. In a synthetic authorized branch scope, create an Arabic customer and confirm revision `1`, one redacted audit, one publication item before delivery, and one pending sync change.
3. Restart and confirm exact stored display text and scoped search results.
4. Confirm a same-phone create returns an advisory while preserving two UUIDs.
5. Run the focused customer, storage, runtime, contract-generation, and documentation checks.

## Rollback and recovery

If version `13` accepted no writes, stop the engine and restore the validated pre-migration artifact before deploying an older compatible engine. If customer or other writes committed, restoring that artifact loses those writes. Preserve the database and use an explicit compatible migration instead.

Never delete customer tables, clear outboxes, or edit migration history. Follow [recover local storage](../operations/recover-local-storage.md) and [maintain customer contact records](../developer/subsystems/customers.md).

