---
title: "Operate and release Eitmad"
description: "Verify, package, deploy, update, back up, restore, and roll back Eitmad desktop and server foundations."
audience: "operations"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-10-21"
review_triggers:
  - "workspace verification, executable behavior, deployment, backup, or recovery changes"
keywords:
  - "foundation verification"
  - "cargo clippy workspace"
  - "audit_docs.py"
  - "E0658"
---

# Operate and release Eitmad

For a new Windows workshop, start with [Install Eitmad on the server and employee PCs](install-windows.md). Use the other tasks for release preparation, advanced deployment, maintenance, and recovery. Guided Windows setup, device pairing, scheduled local backups, backup administration, and unattended TLS renewal are implemented. Trusted signed offline distribution and clean-machine acceptance still need the evidence listed in [offline setup verification](verify-offline-windows-setup.md).

## Release and deployment tasks

- [Validate a release candidate](validate-release-candidate.md): run mandatory code, contract, security, Arabic/RTL, packaging, desktop, server, restore, and rollback gates.
- [Install Eitmad on the server and employee PCs](install-windows.md): follow the normal guided setup, create staff accounts, pair clients, and check the result.
- [Build and maintain Windows installation packages](package-windows-desktop.md): assemble and sign offline bundles; use advanced manual import, repair, removal, and upgrades with retained data.
- [Verify offline Windows Server and Client setup](verify-offline-windows-setup.md): repeat isolated proofs and record clean-PC, standard-user, pairing, firewall, reboot, DHCP, interruption, and restore acceptance.
- [Use the Windows setup backend and recover installation stages](prepare-windows-workshop.md): inspect Rust setup internals, private CLI requests, retained progress, and standalone preparation.
- [Prepare future macOS signing and notarization](prepare-macos-distribution.md): keep packaging blocked until the native shell and updater exist.
- [Prepare future Linux packages and updates](prepare-linux-distribution.md): choose managed formats and preserve package-manager authority.
- [Deploy staging and production server environments](deploy-server-environments.md): isolate profiles, terminate TLS, migrate, restore, promote, and roll back.
- [Run and recover the modular server authority](run-server-authority.md): configure and exercise the current combined binary.
- [Set up a stable Windows LAN server with TLS](setup-lan-server.md): manually provision a separate fixed endpoint or inspect retained trust and unattended certificate renewal.
- [Run the Windows host without an interactive login](run-windows-host-services.md): install persistent PostgreSQL and Rust services, inspect readiness, recover failures, and preserve data during uninstall.
- [Run the Manager and Receptionist test environment](run-workflow-test-environment.md): start one real WPF client with role-based sign-in, isolated PostgreSQL, authenticated TLS, synthetic Arabic workflows, and retained work across restart.
- [Run and diagnose the engine runtime](run-engine-runtime.md): start headless or supervised modes and interpret readiness.
- [Recover and export local storage](recover-local-storage.md): preserve and validate SQLite authority.
- [Back up and recover the Windows LAN installation](backup-lan-installation.md): schedule protected generations, retain explicit history, and recover Windows credentials with matching PostgreSQL and client state.

## Run the local foundation checks

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

10. On each release platform, run the synthetic native secret lifecycle through that platform's protected credential store. This test is ignored during routine hermetic runs because it mutates OS credential state and then cleans it up:

   ```powershell
   cargo test -p eitmad-secret-storage tests::os_native_backend_supports_secret_lifecycle -- --ignored --exact
   ```

11. Enforce repository ownership, migration checksum, secret/logging, direct shell authority, release plan, and Arabic root-direction rules:

   ```powershell
   python scripts/ci/check_repository_policy.py
   ```

12. Build unsigned validation artifacts on their supported build hosts:

   ```powershell
   python scripts/release/build_artifacts.py windows-desktop --version 0.0.0-ci --output dist
   ```

   ```powershell
   python scripts/release/build_artifacts.py server --version 0.0.0-ci --output dist
   ```

13. In an environment with PostgreSQL, follow the [server runbook](run-server-authority.md) and verify migrations `1–3`, bootstrap, authentication, tenant isolation, relay denial/lifecycle, signed update selection, administration, sync pull, snapshot fallback, subscription resume, readiness, backup, and restore.

## Verify

In a healthy development environment, every applicable command should exit with code `0` and no warnings. Diagnostics should print one JSON report; an unhealthy required check may produce exit code `3`. Windows supervision prints `Windows process supervision scenarios passed.` after fake and real-engine checks. Artifact builders print the artifact and manifest paths. These artifacts stay unsigned and not production-eligible. Swift binding conformance runs in macOS CI because Swift is not part of the Windows prerequisites.

## Recover

If a command fails, stop. Do not hide the warning or bypass the test. Fix the authoritative source or canonical document, then rerun the failed check and its direct dependent check. Rerun the full suite only for a release, CI or workspace-wide change, or evidence of broader impact. If CI reports `E0658` while a local stable build passes, inspect the reported syntax for a feature unavailable in Rust `1.85.1` and reproduce with the minimum-version check above. Preserve `eitmad.sqlite3` and its SQLite companion files before any recovery attempt; never edit or downgrade them manually.

## Related tasks

- [Start developing Eitmad](../developer/index.md)
- [Run and diagnose the engine runtime](run-engine-runtime.md)
- [Recover and export local storage](recover-local-storage.md)
- [Understand local storage and recovery boundaries](../developer/subsystems/local-storage.md)
- [Extend Windows process supervision safely](../developer/subsystems/windows-process-supervision.md)
- [Extend privacy-preserving observability safely](../developer/subsystems/privacy-preserving-observability.md)
- [Use Rust-owned secret storage safely](../developer/subsystems/secret-storage.md)
- [Extend synchronization and shared transports safely](../developer/subsystems/synchronization.md)
- [Run and recover the modular server authority](run-server-authority.md)
- [Extend the modular server authority safely](../developer/subsystems/server-authority.md)
- [Extend WAN relay coordination safely](../developer/subsystems/wan-relay-coordination.md)
- [Publish and evaluate signed updates safely](../developer/subsystems/update-distribution.md)
- [Extend server administration safely](../developer/subsystems/server-administration.md)
- [Resolve relay, update distribution, and administration failures](../troubleshooting/server-plane-failures.md)
- [Resolve diagnostic privacy or secret-storage failures](../troubleshooting/privacy-and-secret-leakage.md)
- [Review the documentation standard](../developer/contributing/documentation-standard.md)
