# AGENTS.md

## Instruction Priority

- Every instruction in this file is mandatory.
- Use `.agents/skills/caveman/SKILL.md` in lite mode.
- Use `.agents/skills/vertical-codebase/SKILL.md` to decide how to structure the codebase.
- After implementing and verifying every feature request, run `.agents/skills/maintain-project-documentation/SKILL.md` before considering the feature complete. Use its evidence map and change-impact rules to update every affected canonical page, index, glossary term, decision, and troubleshooting path in the same logical change.
- Commit incrementally when you complete a checkpoint.
- Follow the existing non-destructive git rules: never revert user changes unless explicitly asked.

## Project Snapshot

The project is an Arabic-first operations system for **الاعتماد**, a local furniture manufacturer that produces its own furniture. The goal is to replace and improve the company’s workflow with a faster, more reliable system that centralizes data, reduces manual pricing errors, and supports daily coordination between managers, receptionists, carpenters, installers, and workshop supervisors.

## Mission

Build a robust cross-platform desktop app foundation for **الاعتماد**.
The final app must be fast, efficient, low-resource, secure, sync-capable, update-capable, Arabic-first, well documented, and easy to extend with app-specific features.

## Core Rules

- Rust is the product authority.
- Native UI shells are thin platform adapters.
- Windows shell uses C#.
- macOS and Linux shells use native platform UI choices.
- UI shells must not directly own business logic, database access, config files, sync logic, permissions, external API calls, secrets, or domain validation.
- Rust owns contracts, engine runtime, config, storage, sync, authorization, updates, observability, audit, external services, and background jobs.
- Every completed feature must leave an unfamiliar engineer able to find its Rust authority, contracts, invariants, failure modes, tests, and safe extension points.
- Do not finish a feature until the documentation-maintenance skill and its focused documentation audit pass. A verified no-documentation-impact result must still come from running the skill workflow.
- Every major feature requires dedicated engineer documentation covering purpose, design, ownership, contracts, security, Arabic behavior, tradeoffs, tests, operations, failure recovery, and extension points.

## Testing Discipline

- Follow test-driven development when making code changes: add or update useful and meaningful tests as you go.
- When fixing a focused small problem, determine if it actually needs a test or not before going straight to coding.
- Keep tests focused on the behavior changed.
- A task is not complete until the app compiles, runs, and has been verified with no warnings or errors.
- Fix warnings and errors properly by addressing the underlying issue; do not silence, bypass, or hide them just to make output clean.
- Only call the task done after tests pass and the app has been run cleanly.


## PR Review Fixes

- When fixing PR issues submitted by CodeRabbit, apply minimal fixes and do not go overboard.
- Minimal change does not mean taking shortcuts; if the correct fix is more involved, make the correct fix.

## Git Workflow

- Never commit to main. If the project is checked out to main and the user asks for a task, create a new branch and do the work in there.
- If the user explicitly says no branch is needed, do not create one.
- Commit incrementally when a logical checkpoint is complete.
- Close all PowerShell/CMD instances you created during the session after you are done working and the codebase is clean and committed.


## Commit Message Skill

- Follow the `conventional-commit` skill workflow instead of inventing a commit message ad hoc.
- Use `skills/caveman-commit/SKILL.md` to draft commit messages, then keep the final message Conventional Commits compliant.
- Preserve the existing non-destructive git rules in this file when handling commit requests.

## Core Priorities

- Correctness and reliability first.
- Prefer small, focused modules over large monolithic systems.
- Avoid architecture that encourages hallucinated, stale, cross-tenant, or partial state.

## Maintainability

- Long-term maintainability is a core requirement.
- Do not hesitate to refactor existing code when that produces a cleaner system.

# Architecture

- The Rust engine is the system authority and runs as a separate process.
- Platform shells are thin clients using typed local IPC.
- The engine must also support headless and diagnostic operation.
- IPC uses Commands, Queries, and Subscriptions.
- Event sourcing is limited to audit-critical or history-critical domains.

# Ownership

Rust owns:
- domain logic
- contracts
- config
- database access
- authorization
- sync
- update policy

Shells must not duplicate Rust-owned schemas, DTOs, validation, config, or database logic.

# Contracts

Maintain one Rust-owned contract layer for IPC, config, identity, permissions, sync, updates, errors, versions, and capabilities.

Generate or validate platform bindings.

Version every external boundary and require capability negotiation.

# Storage and Sync

Support two product modes:
- local-first
- server-authoritative

Use one sync protocol across IPC, LAN, and WAN transports.

Split server infrastructure into control, sync, relay, update, and admin planes.

# Security
Use zero-trust across shells, engines, devices, peers, servers, and plugins.
Use relationship-based authorization as the core permission model.
Authorize every command and query in Rust.
Every record belongs to an explicit scope.
Every state-changing command produces an audit record.
# Arabic
Arabic support is foundational.
Every feature must account for RTL, bidirectional text, Arabic search, localization, mixed-language data, and Arabic-ready documents and reports.

# Updates
Rust owns update policy, compatibility, rollout, migration safety, and update state.
Platform adapters own installation.
Use signed server-hosted manifests and native platform updaters.
# Performance
Keep hot paths in Rust.
Use coarse-grained asynchronous IPC, subscriptions instead of polling, streaming for large payloads, and incremental sync.

# Delivery
Define contracts, permissions, storage, sync, and Arabic behavior before implementing a feature.
CI must reject contract drift, broken migrations, unsafe logging, and direct shell access to config or databases.

# Product Defaults
- Accounting and ERP: server-authoritative, ledger-grade history, strict audit, strong backups.
- Productivity: local-first, offline-first, conflict-aware.
- System tools: local-first, permission-gated, low-background-cost.
