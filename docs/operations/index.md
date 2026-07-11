---
title: "Run Eitmad foundation checks"
description: "Safely verify Rust formatting, builds, tests, the initial engine process, and documentation."
audience: "operations"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "workspace verification, executable behavior, deployment, backup, or recovery changes"
keywords:
  - "foundation verification"
  - "cargo clippy workspace"
  - "audit_docs.py"
---

# Run Eitmad foundation checks

These steps verify only the current foundation. No installable package, production service, database, backup, or runnable recovery flow exists yet.

## Prerequisites

- Rust `1.85` or newer, compatible with the workspace `rust-version`;
- Python 3 for the Markdown audit;
- commands run from the repository root without real customer data.

## Steps

1. Check formatting:

   ```powershell
   cargo fmt --all -- --check
   ```

2. Check all targets:

   ```powershell
   cargo check --workspace --all-targets
   ```

3. Reject Clippy warnings:

   ```powershell
   cargo clippy --workspace --all-targets -- -D warnings
   ```

4. Run workspace tests:

   ```powershell
   cargo test --workspace
   ```

5. Run the current diagnostic entry point:

   ```powershell
   cargo run -q -p eitmad-engine-cli
   ```

6. Audit documentation:

   ```powershell
   python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs
   ```

## Verify

Every command must exit with code `0` and no warnings. The current entry point prints no output; this is expected because `crates/engine-cli/src/main.rs` is an empty foundation.

## Recover

If a command fails, stop. Do not hide the warning or bypass the test. Fix the authoritative source or canonical document, then rerun the failed command and the full suite. The current foundation has no product state that requires rollback.

## Related tasks

- [Start developing Eitmad](../developer/index.md)
- [Review the documentation standard](../developer/contributing/documentation-standard.md)
