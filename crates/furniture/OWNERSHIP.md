# Furniture authority

`eitmad-furniture` owns organization-scoped Furniture definitions, separate categories, immutable revisions, exact Part composition references, definition costs, selection validation, authorization, and audit orchestration. Commercial catalog publication and server-confirmed selling prices remain outside this capability.

Contracts remain in `crates/contracts/src/furniture.rs`. Transactional storage and migration 18 remain in `crates/storage/src/furniture.rs`. The runtime dispatcher supplies typed IPC; the Windows shell stages unsaved fields and renders Rust results. See [the Furniture guide](../../docs/developer/subsystems/furniture.md) for lifecycle rules, failure handling, and focused checks.
