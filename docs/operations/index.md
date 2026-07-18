---
title: "Run Eitmad foundation checks"
description: "Safely verify Rust, engine diagnostics, generated contracts, Windows supervision, and documentation."
audience: "operations"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "workspace verification, executable behavior, deployment, backup, or recovery changes"
keywords:
  - "foundation verification"
  - "cargo clippy workspace"
  - "audit_docs.py"
  - "E0658"
---

# Run Eitmad foundation checks

These steps verify the current foundation. Rust now owns a local SQLite authority database; packaging, automated backup, and production restore tooling are not implemented.

## Prerequisites

- rustup with Rust `1.85.1` installed for minimum-supported-version checks;
- the current stable Rust toolchain for daily development;
- Python 3 for the Markdown audit;
- Node.js and npm for pinned contract generation;
- .NET 8 for local Windows binding and process-supervision conformance;
- commands run from the repository root without real customer data.

## Steps

1. Check formatting:

   ```powershell
   cargo fmt --all -- --check
   ```

2. Check all targets with the minimum supported Rust version used by CI:

   ```powershell
   rustup run 1.85.1 cargo check --workspace --all-targets
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
   npm run contracts:verify --prefix crates/contracts/codegen
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

If a command fails, stop. Do not hide the warning or bypass the test. Fix the authoritative source or canonical document, then rerun the failed command and the full suite. If CI reports `E0658` while a local stable build passes, inspect the reported syntax for a feature unavailable in Rust `1.85.1` and reproduce with the minimum-version check above. Preserve `eitmad.sqlite3` and its SQLite companion files before any recovery attempt; never edit or downgrade them manually.

## Related tasks

- [Start developing Eitmad](../developer/index.md)
- [Run and diagnose the engine runtime](run-engine-runtime.md)
- [Extend Windows process supervision safely](../developer/subsystems/windows-process-supervision.md)
- [Review the documentation standard](../developer/contributing/documentation-standard.md)
