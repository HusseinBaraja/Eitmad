# AGENTS.md

## Product and Priorities

This repository builds **الاعتماد**, an Arabic-first operations system for a local furniture manufacturer. It is a cross-platform native desktop product with a separate Rust engine.

Optimize work in this order:

1. Correctness, security, and data integrity.
2. Clear Arabic-native UI and efficient daily workflows.
3. Small, maintainable changes.
4. Fast implementation and low agent usage.

The user is the only developer. Do not add team process, coordination documents, compatibility layers, abstractions, or infrastructure unless the current product needs them.

Use ASD-STE100 Simplified Technical English when you communicate with the user. Keep code, identifiers, exact quotations, error text, and established domain terms unchanged. State the outcome first. Use concise paragraphs. Use lists only for parallel items, steps, or comparisons. Do not repeat the plan, restate completed work, or add a generic closing summary.

## Instruction Precedence and Agent Behavior

Apply repository instructions in this order:

1. The user's explicit instruction for the current task.
2. The hard invariants in this file, unless the user explicitly asks to change an invariant itself.
3. Applicable repository-local instructions and skills.
4. The working defaults in this file.

Within the same level, prefer the more specific instruction. A skill is scoped guidance. It does not expand the task, grant new permissions, or override the user's requested outcome. Read a skill only when its trigger applies.

If a skill conflicts with the current task, follow the precedence above. If a skill requires a pause, confirmation, unfinished work, or a change of direction, identify the exact skill and the controlling instruction and explain the conflict briefly. If a referenced skill is unavailable, do not invent it or block unrelated safe work; follow the applicable repository rules and report the missing skill.

Infer intent and scope from the request, prior conversation, and repository evidence. When the user asks for action, perform the authorized work to completion. Do not stop at a plan, status report, or non-blocking question. Ask only when a missing decision can materially change behavior, data, security, destructive scope, or visible UX.

Complete all authorized reversible work before requesting approval for an irreversible or external action. Do not ask for approval for read-only work, reversible local work, or an action the user already authorized.

Use subagents only when independent work can run in parallel and materially reduce time or improve confidence. Prefer direct work for focused tasks. Do not delegate overlapping edits or small investigations. Keep one agent responsible for integration and the final result.

## Hard Invariants

* Rust is the product authority and runs as a separate process. It supports supervised desktop, headless, and diagnostic operation.
* Native UI shells are thin platform adapters. Windows uses C# and WPF on .NET 8. Future shells use suitable native platform UI technology.
* A shell must not own or duplicate business logic, domain validation, database or config-file access, authorization, audit, sync, update policy, external API access, secrets, or Rust-owned schemas and DTOs.
* Rust owns contracts, engine runtime, configuration, storage, sync, authorization, updates, observability, audit, external services, and background jobs.
* Use zero trust and relationship-based authorization. Authorize every command and query in Rust. Every record has an explicit scope. Every product-state mutation creates an audit record.
* Use typed, versioned Commands, Queries, and Subscriptions across IPC. Generate or validate native bindings from Rust-owned contracts.
* Keep hot paths in Rust. Prefer coarse asynchronous IPC, subscriptions instead of polling, streaming for large payloads, and incremental sync.
* Never expose secrets, bootstrap tokens, authorization graphs, raw IPC frames, or customer data in UI, examples, screenshots, or logs.
* Never revert, overwrite, delete, or reformat unrelated user work. Do not use destructive Git commands without explicit authorization for the exact operation.
* Repository documentation prose is English. Application UI and user-facing search terms are Arabic-first.

Use the [repository ownership map](docs/developer/repository-layout.md), [target architecture](docs/architecture/target-architecture.md), [decision records](docs/decisions/index.md), and [glossary](docs/glossary.md) for detail. Do not restate them in new files.

## Scope, Work Method, and Stop Rules

These are working defaults. The user can override them unless a hard invariant is affected.

* Make the smallest complete change that resolves the requested outcome.
* Plan in proportion to risk. Keep the plan brief for focused work. Use a detailed plan only for multi-boundary work or irreversible decisions.
* Fix the verified root cause within scope. Do not hide it behind another conditional, compatibility path, or duplicate implementation.
* Keep one product concern in one change. Do not fix adjacent issues unless they block correctness or verification.
* Prefer vertical product capabilities over generic `utils`, `common`, `shared`, `handlers`, or `services` buckets.
* Reuse suitable existing dependencies, patterns, controls, contracts, helpers, and test mechanisms. Add new machinery only when the current requirement cannot be met safely without it.
* Use local code, focused tests, logs, Git history, and canonical documentation as primary evidence. Browse only for an external specification, dependency behavior, current fact, or when local evidence is insufficient. Treat a diagnostic hypothesis as unverified until evidence confirms it.
* For a review, explanation, diagnosis, or plan, inspect and report. Edit only when the user asks for a change.
* For a change or fix, make safe in-scope edits and run focused verification without asking for routine approval.

For implementation:

1. Define the outcome and owning product capability.
2. Mark only the applicable boundaries: Rust behavior, contract, authorization and audit, storage and sync, native shell, Arabic UI, operations, or documentation.
3. Inspect the owner and its focused evidence.
4. Implement the smallest correct patch. Update direct callers and generated artifacts with it.
5. Run the smallest proof that can detect a defect in the changed behavior.
6. Inspect the final diff once for unrelated edits, temporary files, debug code, and accidental generated output.
7. Stop when the requested outcome is complete and the applicable focused proof passes.

Do not continue with optional cleanup, broad refactoring, extra tests, repeated screenshots, or speculative edge cases after the stop condition. Once applicable focused checks pass, broaden or repeat verification only when a new change, failure, or unresolved risk requires it. Do not repeat a successful tool call or check unless relevant code changed. After two failed attempts with the same approach, reassess the cause before another attempt.

## Pre-Release Compatibility

There is no public production baseline. Do not build backward compatibility for hypothetical users or versions.

Treat compatibility as required only when the current task, repository, or user instruction contains evidence of a real supported release, external consumer, durable user data, or published artifact. Do not run a separate compatibility investigation without such evidence.

* If no real compatibility requirement exists, prefer the clean current design. Update the authority, direct callers, generated bindings, fixtures, and focused tests together.
* Do not add deprecated aliases, adapters, dual writes, migration chains, feature flags, or old protocol branches only to preserve unreleased behavior.
* Continue to version external boundaries and negotiate capabilities. Support only declared ranges.
* Preserve real local data, supported external consumers, and published artifacts when they exist. Never delete or reset data without explicit authorization.

## Native Arabic UI and UX

This is native software. Apply native platform conventions before web conventions.

### Native behavior

* Use WPF and Windows behavior for controls, input, window chrome, system tray, DPI, keyboard, focus, UI Automation, high contrast, and accessibility.
* Let Windows own caption buttons, resizing, snapping, the system menu, and window placement unless a proven product need requires custom chrome.
* Keep the UI thread free of IPC, file, network, and database work.
* Render Rust-owned state with explicit loading, unavailable, empty, denied, conflict, and failure states. Do not invent success in the shell.
* Preview fixtures can own temporary presentation state only. Label them clearly and never imply that data was saved, authorized, audited, or synchronized.
* Use native layout and vector resources. Do not reproduce browser breakpoints, CSS structure, or JavaScript interaction models in WPF.

### Arabic-first behavior

* Set the root direction and language explicitly. On Windows, use `FlowDirection="RightToLeft"` and approved Arabic language metadata at the shell boundary.
* Use Arabic as the default visible language. Reuse approved terms from `docs/glossary.md`.
* Write short, direct Arabic labels for local furniture and workshop users. Avoid unclear literal translations.
* Isolate LTR identifiers, paths, model numbers, dimensions, dates, percentages, Latin abbreviations, and mixed values inside RTL layouts.
* Treat physical placement and reading direction separately so RTL does not move icons, amounts, popups, or technical values to the wrong side.
* Preserve stored user text. Apply Arabic normalization to search and matching, not as a destructive rewrite.
* Never use color as the only state signal. New interactive controls need an Arabic accessible name, visible keyboard focus, and a keyboard path.
* Design for realistic Arabic strings, mixed-direction data, narrow windows, high DPI, and text expansion. Do not scale the complete page to hide overflow.

### Focused UI verification

For a user-visible UI change, inspect the real rendered affected screen once after implementation is stable when a suitable Windows rendering environment is available. Use synthetic Arabic data.

If the required rendering environment is unavailable, complete the available build and static verification, state that rendered verification was not performed, and continue other safe in-scope work. Never claim visual, RTL, focus, or accessibility behavior that was not verified.

* Check the normal size. Add one compact or wide size only when responsive behavior changed.
* Check keyboard, focus, popup placement, and accessible names only for new or changed interactions.
* Check high contrast or text scaling only when colors, typography, sizing, or custom templates changed.
* Use a rendered check for visual quality. XAML string scans and snapshots do not prove layout, shaping, focus, or usability.
* Run another visual pass only after a relevant code change or while a known visible defect remains.
* Do not copy a web reference pixel for pixel. Preserve its product intent with native behavior and the Eitmad visual system.

## Tests and Verification

Tests are risk controls, not a quantity target. Run relevant existing tests first. Add or update a test only when existing evidence cannot detect a credible regression in changed consequential behavior.

A credible regression usually concerns:

* domain calculations, validation, and state transitions;
* authorization, scope, audit, privacy, or secrets;
* storage, schema, recovery, sync, conflicts, or idempotency;
* public contracts, serialization, bindings, versions, or capabilities;
* Arabic search normalization or mixed-direction formatting logic;
* non-trivial UI state, command availability, reconnect, or failure recovery;
* a reproduced defect with a stable regression check.

Before adding a test, identify the accepted behavior, the regression that existing evidence misses, and the smallest stable boundary that can detect it.

Do not add a test only for static copy, spacing, color, a trivial property, simple delegation, generated output, framework behavior, or an implementation detail. Do not use brittle source-text assertions when behavior or focused rendering is the real proof.

Use the smallest applicable verification:

* Rust: format the affected crate and run its focused test or filter. Add affected-crate check or Clippy only when useful.
* C# or WPF logic: build the affected project and run the narrow shell or adapter test.
* Contract: regenerate bindings and run the affected conformance check.
* UI-only visual change: build the shell and perform the focused rendered check when the required environment is available. Do not run Rust workspace tests when no Rust boundary changed.
* Cross-boundary runtime change: run one integrated engine-and-shell path after focused component checks pass.
* Documentation: audit changed pages when supported. Use the full documentation audit only for shared navigation, indexes, or documentation-system changes.

Do not run repository-wide checks, the full application, every platform suite, or a release checklist for a focused change. Run full gates only for a release, an explicit request, CI or workspace configuration, or a change that can affect most workspace members. CI owns exhaustive repository validation.

Applicable checks must pass without new warnings. Fix the cause; do not silence it. After failure, rerun the failed check and its direct dependent check, not the complete suite. Once applicable focused checks pass, broaden or repeat testing only when a new change, failure, or unresolved concern justifies it. Do not claim an application run, platform, RTL, visual, or accessibility state that was not verified.

## Documentation Impact

Use `.agents/skills/maintain-project-documentation/SKILL.md` once when a change affects a user workflow or Arabic term; a public contract; authorization, audit, privacy, or scope; durable data, sync, recovery, installation, update, or operations; ownership or a durable decision; or a reusable troubleshooting path.

If none applies, report no documentation impact and do not edit documentation or run a broad documentation audit. If it applies, update only affected canonical pages, indexes, glossary terms, decisions, and troubleshooting paths. Link to authority instead of copying it. Create a dedicated subsystem page only for a major capability with a real reader task.

## Git and Review

* Never commit to `main`. Before committing work on `main`, create a descriptive branch without a tool or vendor prefix unless the user explicitly says not to create a branch.
* Preserve user changes and keep unrelated files out of commits.
* Create local commits at suitable checkpoints during authorized work, using agent judgment without asking for routine confirmation. Commit a coherent, verified change when a meaningful milestone or the task is complete. Keep one concern per commit, avoid incomplete checkpoints, and follow an explicit user instruction to defer or omit commits.
* Before each commit, use `.agents/skills/caveman-commit/SKILL.md`, then keep the message compliant with the `conventional-commit` skill.
* A local commit does not authorize remote changes. Do not push, rebase, create a pull request, merge, or otherwise modify remote state unless the user asks for that action.
* When publishing, push the active branch first and create only a regular ready-for-review pull request unless the user requests another state.
* Keep one concern per pull request. If its summary needs an unrelated “also,” split it.

Workflow-specific review rules, including rules for a named review service or bot, belong in the relevant skill or workflow file instead of this root file.

## Navigation

* `crates/`: Rust capabilities and authority.
* `server/`: server planes and PostgreSQL support.
* `shells/windows/`: Arabic-first WPF presentation.
* `platform-adapters/windows/`: Windows process, IPC, credential, and OS mechanics.
* `tests/`: cross-boundary and contract conformance.
* `docs/`: canonical project knowledge.

Start from [README.md](README.md), the [documentation index](docs/index.md), and the [operations index](docs/operations/index.md). Use `rg` and `rg --files` to find exact owners before broad inspection.
