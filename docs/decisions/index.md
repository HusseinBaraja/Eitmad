---
title: "Review architectural decisions"
description: "Find decisions about Rust authority, shells, IPC, storage, synchronization, security, Arabic UX, and updates."
audience: "architecture"
page_type: "reference"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "an ADR is proposed, accepted, rejected, or superseded"
keywords:
  - "architectural decision records"
  - "ADR"
---

# Architectural Decision Records

> - Category: Explanation
> - Status: Active
> - Owner: Architecture maintainers
> - Review trigger: ADR lifecycle or architecture governance changes

This directory contains Eitmad's durable architectural decisions for future reference.

## Create an ADR

1. Copy the [ADR template](../developer/contributing/templates/adr.md).
2. Select the next unused four-digit sequence number. Numbers are never reused, including for rejected ADRs.
3. Name the file `NNNN-short-kebab-case-title.md`.
4. Start as `Proposed` and include decision owners and review triggers.
5. Review affected architecture, contracts, storage, sync, security, Arabic UX, operations, and compatibility.
6. Change status to `Accepted` or `Rejected` after review and add the ADR to the index below.

Accepted ADRs are immutable historical records except for typo fixes, link repairs, and status metadata. A changed decision requires a new ADR whose `Supersedes` field links the old ADR. Update both index entries and the old ADR's `Superseded by` field.

## Index

| ADR | Title | Status | Date | Supersedes |
| --- | --- | --- | --- | --- |
| [ADR-0001](0001-rust-core-product-authority.md) | Rust core is the product authority | Accepted | 2026-07-11 | None |
| [ADR-0002](0002-native-platform-ui-shells.md) | Use native platform UI shells | Accepted | 2026-07-11 | None |
| [ADR-0003](0003-separate-supervised-engine-process.md) | Run the engine as a separate supervised process | Accepted | 2026-07-11 | None |
| [ADR-0004](0004-headless-engine-mode.md) | Require a headless engine mode | Accepted | 2026-07-11 | None |
| [ADR-0005](0005-generated-or-validated-contracts.md) | Generate or mechanically validate platform contracts | Accepted | 2026-07-11 | None |
| [ADR-0006](0006-rust-owned-configuration.md) | Rust owns product configuration | Accepted | 2026-07-11 | None |
| [ADR-0007](0007-rust-owned-database.md) | Rust owns all product database access | Accepted | 2026-07-11 | None |
| [ADR-0008](0008-required-unified-synchronization.md) | Require one versioned synchronization protocol | Accepted | 2026-07-11 | None |
| [ADR-0009](0009-zero-trust-security-model.md) | Apply zero trust across every boundary | Accepted | 2026-07-11 | None |
| [ADR-0010](0010-rebac-authorization-foundation.md) | Use ReBAC as the authorization foundation | Accepted | 2026-07-11 | None |
| [ADR-0011](0011-arabic-first-user-experience.md) | Make Arabic-first UX a system requirement | Accepted | 2026-07-11 | None |
| [ADR-0012](0012-privacy-preserving-observability.md) | Make observability privacy-preserving by construction | Accepted | 2026-07-11 | None |
| [ADR-0013](0013-platform-native-update-adapters.md) | Use platform-native update adapters | Accepted | 2026-07-11 | None |
| [ADR-0014](0014-rust-derived-json-contracts.md) | Derive JSON contracts and native bindings from Rust | Accepted | 2026-07-12 | None |
| [ADR-0015](0015-contract-compatibility-window.md) | Support a rolling protocol compatibility window | Accepted | 2026-07-12 | None |
| [ADR-0016](0016-bounded-platform-process-supervision.md) | Bound platform engine process supervision | Accepted | 2026-07-12 | None |
| [ADR-0017](0017-windows-named-pipe-local-ipc.md) | Use bounded JSON frames over Windows named pipes | Accepted | 2026-07-12 | None |
| [ADR-0018](0018-bounded-resumable-local-ipc-events.md) | Use bounded resumable local IPC event streams | Accepted | 2026-07-13 | None |
| [ADR-0019](0019-sqlite-authority-storage.md) | Use SQLite for local authority storage | Accepted | 2026-07-18 | None |
| [ADR-0020](0020-direct-principal-scope-rebac-policy.md) | Use direct principal-to-scope ReBAC policy v1 | Accepted | 2026-07-18 | None |
| [ADR-0021](0021-checksummed-feature-storage-migrations.md) | Use checksummed feature-owned storage migrations | Accepted | 2026-07-18 | None |
