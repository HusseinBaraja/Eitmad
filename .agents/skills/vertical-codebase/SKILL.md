---
name: vertical-codebase
description: Organize Eitmad Rust code by product capability. Use when creating or changing module or crate ownership, moving code between capabilities, or reviewing a proposed Rust module boundary.
---

# Vertical Eitmad Codebase

Use this skill to keep the authoritative Rust engine grouped by product capability instead of technical type. Keep WPF presentation in `shells/windows/` and Windows process or OS mechanics in `platform-adapters/windows/`.

The goal is to colocate code that changes together, make ownership clear, and keep cross-feature dependencies explicit.

## Core Rule

Prefer a vertical home:

```text
src/
  notes/
    mod.rs
    commands.rs
    repository.rs
    persistence.rs
    validation.rs
    types.rs
    tests.rs
```

over a horizontal split:

```text
src/
  commands/
  services/
  repositories/
  models/
  utils/
  traits/
```

Technical type is secondary. Functionality, ownership, and change coupling decide placement.

For Rust, this means modules should usually represent product concepts, not implementation categories.

## Placement Workflow

1. Name what the code does in product terms.
2. Find the closest existing module or crate that owns that behavior.
3. Put private structs, enums, traits, helpers, commands, persistence code, validation, and tests next to the code that uses them.
4. Keep module visibility narrow with `pub(crate)` or private items by default.
5. Export only what another vertical is allowed to consume.
6. If code is shared by multiple verticals, determine whether it is its own domain module or crate. Ask only when missing product ownership would materially change the result.
7. If it is generic app infrastructure, place it behind an explicit infrastructure boundary.
8. If no vertical exists, create a sibling module with a narrow purpose instead of expanding a catch-all file.

Keep Rust-owned contracts, configuration, storage, sync, authorization, updates, audit, observability, external services, and background jobs with their owning capability. Do not move native presentation or OS integration into Rust to satisfy a generic vertical layout.

## Rust Module Rules

Prefer:

```rust
mod notes;
mod search;
mod settings;
mod sync;
```

Avoid turning `src/` into:

```rust
mod models;
mod services;
mod handlers;
mod repositories;
mod utils;
```

Inside each vertical, expose a small public surface:

```rust
// src/notes/mod.rs

mod note;
mod repository;
mod validation;
mod commands;

pub use note::Note;
pub use commands::{create_note, delete_note};
```

Keep internals private unless another module has a legitimate reason to depend on them.

Prefer:

```rust
pub(crate) struct NoteRepository;
```

over:

```rust
pub struct NoteRepository;
```

unless it is part of the module’s intended public API.

## Commands and Events

A top-level command or event registry is acceptable, but it must delegate into the owning capability:

```rust
// src/commands.rs

pub fn register() {
    notes::register_commands();
    settings::register_commands();
}
```

Do not place feature logic in the registry.

## Shared Code Test

Before moving code to a shared module or crate, answer:

* Which verticals use it now?
* Does it encode product behavior from one vertical?
* Could changing it for one vertical break another?
* Is there a stable public interface worth documenting?
* Is this truly generic infrastructure, or just one feature’s helper?
* Should this be a separate domain module instead of `utils`?

If the code mostly serves one vertical, keep it private there.

If several verticals use it with the same meaning, make that concern its own module or crate with explicit exports.

## Shared Crates

In a workspace, prefer crates with clear ownership:

```text
crates/
  authorization/
  configuration/
  contracts/
  storage/
  sync/
```

Avoid crates like:

```text
common/
utils/
shared/
types/
```

unless their public contract is stable and intentionally broad.

Good shared crates have a focused reason to exist:

```text
crates/
  app-config/
  app-database/
  app-platform/
  markdown-rendering/
```

## Tests

Keep tests near the behavior they verify.

Prefer:

```text
src/
  notes/
    validation.rs
    validation_tests.rs
```

or:

```rust
#[cfg(test)]
mod tests {
    use super::*;
}
```

over a distant test directory that mirrors technical layers.

Integration tests are fine when they test behavior across modules:

```text
tests/
  notes_sync_flow.rs
  settings_persistence.rs
```

## Refactor Signals

Use this skill when you see:

* One feature split across `models`, `services`, `commands`, `repositories`, and `views`.
* A helper used only by one feature but stored in `utils`.
* A trait created globally before multiple implementations exist.
* A broad service growing branches for unrelated product concerns.
* Tests far from the behavior they verify.
* Deep imports across feature internals.
* Duplicate helpers created because ownership is unclear.
* `pub` used where `pub(crate)` or private visibility would work.
* A shared crate becoming a junk drawer.

Refactor in small steps. Move the lowest-risk private code first, keep behavior stable, update imports, then run the focused checks required by `AGENTS.md`.

## Review Checklist

For each change, verify:

* The module or crate name describes what the code does.
* Related commands, domain types, validation, persistence, and tests are colocated where practical.
* Shared exports are deliberate and minimal.
* Visibility is as narrow as possible.
* Feature internals are not imported directly by unrelated verticals.
* Cross-vertical dependencies go through explicit public APIs.
* Tests cover the moved or newly colocated behavior.
* Native shells and platform adapters remain separate from Rust-owned product behavior.
