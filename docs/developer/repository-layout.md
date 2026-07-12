---
title: "Find the owner of each repository area"
description: "Map every Rust workspace, shell, server plane, and test area to its authority and extension boundary."
audience: "developer"
page_type: "reference"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "a workspace member, platform, server plane, test suite, or ownership boundary changes"
keywords:
  - "repository ownership"
  - "repository layout"
  - "vertical codebase"
  - "OWNERSHIP.md"
---

# Find the owner of each repository area

Place behavior that changes together in the nearest vertical product capability. Contracts and the engine lifecycle are implemented foundations; most product capability boundaries remain empty.

## Ownership map

| Area | Current authority | Canonical source |
| --- | --- | --- |
| `crates/contracts/` | External contracts and generation or validation inputs | `crates/contracts/OWNERSHIP.md` |
| `crates/engine-runtime/` | Implemented engine lifecycle, readiness, health, authority lock, and component coordination | [Engine runtime guide](subsystems/engine-runtime.md) |
| `crates/engine-cli/` | Implemented supervised, headless, and diagnostic entry point | `crates/engine-cli/OWNERSHIP.md` |
| `crates/storage/` | Database, migrations, transactions, and backups | `crates/storage/OWNERSHIP.md` |
| `crates/sync/` | Unified synchronization protocol | `crates/sync/OWNERSHIP.md` |
| `crates/authorization/` | Identity, scope, and ReBAC authorization | `crates/authorization/OWNERSHIP.md` |
| `crates/update-policy/` | Update eligibility, compatibility, and migration safety | `crates/update-policy/OWNERSHIP.md` |
| `crates/observability-audit/` | Redacted diagnostics and mutation audit | `crates/observability-audit/OWNERSHIP.md` |
| `crates/external-integrations/` | Named external-service adapters | `crates/external-integrations/OWNERSHIP.md` |
| `crates/extensions/` | Extension capabilities and host isolation | `crates/extensions/OWNERSHIP.md` |
| `shells/` | Thin native shells; Windows uses C# | `shells/OWNERSHIP.md` |
| `platform-adapters/` | Replaceable OS mechanics; Windows engine process supervision is implemented | [Windows process supervision](subsystems/windows-process-supervision.md) |
| `server/*-plane/` | Separate control, sync, relay, update, and admin boundaries | `server/OWNERSHIP.md` |
| `tests/` | Cross-boundary tests; unit tests remain near capabilities | `tests/OWNERSHIP.md` |
| `deploy/` | Executable packaging and deployment assets | `deploy/OWNERSHIP.md` |
| `docs/` | Knowledge graph organized by audience and task | [Documentation index](../index.md) |

## Addition rules

1. Name behavior with a product term from the [glossary](../glossary.md).
2. Colocate domain types, validation, storage, commands, and tests with their owning capability.
3. Export only a narrow surface. Use private items by default, `pub(crate)` for same-crate consumers, and narrowly scoped `pub` items only for deliberate cross-crate dependencies.
4. Put generic infrastructure behind an explicit boundary; do not move a single capability's helper into `utils`, `common`, or `shared`.
5. Document authority, contracts, invariants, failure modes, tests, and safe extension points in the [developer collection](index.md).

## Verification

This map matches the current `Cargo.toml` members, implemented runtime boundary, and `OWNERSHIP.md` files as of 2026-07-12. Empty source files do not imply unimplemented behavior.

Next, [run the foundation checks](../operations/index.md).
