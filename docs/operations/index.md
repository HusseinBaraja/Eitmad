---
title: "Run Eitmad foundation checks"
description: "Safely verify Rust, engine diagnostics, generated contracts, Windows supervision, and documentation."
audience: "operations"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-12"
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
- Node.js and npm for pinned contract generation;
- .NET 8 for local Windows binding and process-supervision conformance;
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

5. Run non-mutating engine diagnostics:

   ```powershell
   cargo run -q -p eitmad-engine-cli -- diagnose
   ```

6. Install and validate generated contracts:

   ```powershell
   npm ci --ignore-scripts --prefix crates/contracts/codegen
   ```

   ```powershell
   npm run contracts:check --prefix crates/contracts/codegen
   ```

7. Run Windows binding conformance where .NET 8 is available:

   ```powershell
   dotnet run --project tests/contract-compatibility/csharp/Eitmad.ContractConformance.csproj -- tests/contract-compatibility/fixtures/protocol-v1.json
   ```

8. Build the Rust CLI and run Windows process supervision scenarios on Windows:

   ```powershell
   cargo build -p eitmad-engine-cli
   ```

   ```powershell
   dotnet run --project platform-adapters/windows/tests/Eitmad.Platform.Windows.Tests.csproj -- --engine target/debug/eitmad-engine-cli.exe
   ```

9. Audit documentation:

   ```powershell
   python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs
   ```

## Verify

In a healthy development environment, every applicable command should exit with code `0` and no warnings. Diagnostics should print one JSON report; an unhealthy required check may produce exit code `3`. Windows supervision prints `Windows process supervision scenarios passed.` after fake and real-engine checks. Swift binding conformance runs in macOS CI because Swift is not part of the Windows prerequisites.

## Recover

If a command fails, stop. Do not hide the warning or bypass the test. Fix the authoritative source or canonical document, then rerun the failed command and the full suite. The current foundation has no product state that requires rollback.

## Related tasks

- [Start developing Eitmad](../developer/index.md)
- [Run and diagnose the engine runtime](run-engine-runtime.md)
- [Extend Windows process supervision safely](../developer/subsystems/windows-process-supervision.md)
- [Review the documentation standard](../developer/contributing/documentation-standard.md)
