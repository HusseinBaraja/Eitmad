---
title: "Validate a release candidate"
description: "Run mandatory CI, package integrity, desktop and server smoke, migration, security, Arabic, and rollback checks before promotion."
audience: "operations"
page_type: "task"
status: "active"
owner: "release infrastructure maintainers"
last_verified: "2026-10-31"
review_triggers:
  - "CI jobs, release artifacts, supported platforms, migrations, or promotion policy changes"
keywords:
  - "mandatory CI gates"
  - "release candidate validation"
  - "contract drift"
---

# Validate a release candidate

A release candidate can advance only after every mandatory check succeeds on the exact commit and artifact digest. `main` branch protection must require `validation-complete` and all dependency jobs listed in `deploy/ci-gates.md`. Administrators must not bypass failed, cancelled, or skipped release checks.

## Gate map

| Required evidence | CI enforcement |
| --- | --- |
| Rust format, minimum version, warnings, full workspace tests on the supported Windows engine host | `rust-quality` |
| Contract freshness and generated C#/Swift compatibility; Linux contract lint and tests | `contracts-and-ipc`, `macos-bindings`, `server-smoke` |
| Contract generator dependency vulnerabilities | `contracts-and-ipc` (`npm audit --audit-level=high`) |
| IPC and engine lifecycle | `rust-quality`, `windows-supervision` |
| Migrations and missing migration detection | Rust migration tests plus immutable `deploy/migrations.sha256` in `repository-policy` |
| Authorization, tenant isolation, audit redaction, secret storage, sync, updates | Focused workspace tests in `rust-quality` |
| Unsafe Rust, console, or C# `ILogger` calls; possible secret literals; direct shell database/config/secret access, including Rust and Swift environment APIs | `repository-policy` |
| Arabic/RTL, mixed-direction contracts, Windows shell behavior | `contracts-and-ipc`, `windows-desktop`, `macos-bindings`, `repository-policy` |
| Documentation impact, metadata, links, ownership | `repository-policy` and documentation audit |
| Desktop build, shell lifecycle, real-engine supervision | `windows-desktop`, `windows-supervision` |
| Linux server build, CLI, and safe configuration | `server-smoke` |
| Versioned Windows MSIX/client and server packages, integrity and retained service lifecycle; separate Linux server artifact | `package-windows`, `package-server` |

The workflow runs for every pull request and every push to `main` without path filters. Focused contract and supervision workflows are available through manual dispatch for diagnosis; their automatic coverage belongs to mandatory validation. The repository-policy job compares pull requests with their base SHA and pushes with their prior SHA. A manual run requires the operator to supply a base SHA.

Rust 1.85.1 compiler output and NuGet downloads are cached. CI omits development/test debug symbols to reduce linking and cache transfer time; debug assertions and overflow checks remain enabled. Packaging uses its separate `target/toolchain-1.85.1` cache and retains full release optimization, ThinLTO, inventory, SBOM, and integrity checks. Unsigned CI packages use 16 code generation units for parallel LLVM compilation; normal production builds retain the workspace release profile. Windows desktop and server packages build in separate matrix jobs alongside the other checks. Both must succeed before `validation-complete` passes. Unsigned artifact uploads do not imply release acceptance.

The test profile optimizes Argon2, image, and PNG dependencies to avoid unoptimized password hashing and image processing. Tests retain the real password parameters, image sizes and formats, memory limits, debug assertions, overflow checks, and authentication behavior. The workspace release profile is unchanged.

Clippy checks compilation and warnings for all workspace targets with Rust 1.85.1 in one pass. CI does not repeat the same compilation check with `cargo check`. Local publication validation retains both commands as required by `AGENTS.md`.

Rust quality and packaging cache compiler outputs with each commit and restore the latest compatible cache. The cache also records SHA-256 hashes and timestamps of tracked source files. `scripts/ci/cache_source_mtimes.py` restores a timestamp only when the current file has identical bytes; changed content receives a fresh timestamp and must rebuild. This avoids recompiling unchanged workspace crates after a fresh checkout. Missing or invalid metadata leaves normal Cargo freshness checks in place. Cache reuse never replaces compilation, lint, test, package, or integrity commands.

The `rust-test-timings-1`, `rust-test-timings-2`, and `rust-test-timings-3` artifacts contain Nextest JUnit reports for three required workspace count partitions. The first also contains the separate native credential report. Each test records its duration so slow tests can be identified without rerunning the gate with verbose output. Each partition uses its own Windows host and runs four tests concurrently; each test retains its own process. The first partition also runs formatting, minimum-version Clippy, and Cargo documentation tests. Every partition must pass. Paging fixtures keep a SQLite WAL connection open across bulk operations to avoid repeated connection-close checkpoints. Every mutation and durability setting remains active, and fixtures close that connection before testing persisted-state recovery.

The Rust gate uses pinned Nextest for concurrent workspace unit and integration tests, with no retries, followed by Cargo documentation tests. The Windows shell suite runs alongside engine compilation and supervision. Engine diagnostics reuse the executable already built in `windows-supervision`. To compare CI performance, record job and workflow durations for an initial cache-populating run and a rerun of the same commit. A warm run must execute all checks; a cache hit never replaces a validation command.

The shell suite runs in three required process-isolated groups selected by `scripts/ci/test_windows_shell.ps1`. Each group retains a separate WPF dispatcher. The third group includes every class outside the first two lists, so new tests remain covered automatically. Uploaded TRX reports record the executed tests and durations. Before publication, run the full unfiltered local shell suite as required by `AGENTS.md`.

Credential-routing unit fixtures use isolated encrypted storage instead of the OS credential store. This prevents their correctness checks from depending on shared native storage. The Rust gate separately runs the synthetic native Windows credential lifecycle test, including exact-ID cleanup; native failures remain visible and are not retried into success.

That base comparison makes existing server migration paths and bytes immutable. A schema change must add a new numbered migration and refresh `deploy/migrations.sha256`; changing or deleting a migration present at the base revision fails policy validation even if the manifest was refreshed.

## Promotion checks outside hermetic CI

CI cannot hold production signing keys or prove a live restore. Before promotion:

1. Verify the reviewed commit, source tag, artifact SHA-256, SBOM when available, and signing identities.
2. Verify self-signed publisher signatures, certificate validity, explicit machine trust, and tamper refusal on clean target systems. No timestamp is required.
3. Run the native protected-secret lifecycle on each release platform.
4. Deploy the exact digest to staging and run migration, authentication, denial, tenant-isolation, redaction, sync, update, backup, and restore checks.
5. Exercise an interrupted update and rollback or forward-recovery path.
6. Verify Arabic root RTL, mixed Arabic/Latin identifiers, keyboard flow, accessibility, and copy/paste on supported desktop systems.
7. Promote the same digest to the next channel. Do not rebuild between `canary`, `beta`, and `stable`.

For a Windows offline candidate, use the complete ZIP from [offline assembly](package-windows-desktop.md#prepare-the-combined-offline-payload). Verify its catalog, component inventories, publisher, version, SBOMs, and authenticated review digest. In isolated clean VMs with network access disabled, prove both roles, including a Server without preinstalled C++/Framework prerequisites and a Client with no global runtime installation. Test changed and missing PostgreSQL, runtime, script, MSIX, inventory, and setup-metadata files. Repeat tampering between initial verification and elevation, and verify refusal before elevated payload execution. Check protected-copy ACLs with a different UAC administrator, required-restart failure, retained state, service start without sign-in, scheduled backup and restore, per-user registration, and controlled upgrade. Do not perform these acceptance tests against a live workshop installation. Development build and synthetic catalog tests do not replace this evidence.

## Failure and recovery

Treat a missing, cancelled, or skipped check as a failure. Do not edit a generated binding, migration checksum, or package manifest to hide drift. Fix the Rust authority or source migration, regenerate through the approved command, update affected documentation, and rerun the full workflow.

For a release failure after promotion, pause the channel, preserve evidence, follow the platform or server rollback guide, and add reusable symptoms to troubleshooting documentation.

Unsigned Windows artifacts cannot satisfy production acceptance. The [Windows installation procedure](package-windows-desktop.md) lists signing prerequisites, clean-host installation and enrollment, retained repair/removal, interrupted-upgrade recovery, and coordinated restore evidence. A signed payload or hermetic package test alone does not meet those gates.

macOS and Linux desktop packaging remain disabled in `deploy/release-plan.toml`; neither platform has a runnable desktop shell or updater. Apply [ADR-0013](../decisions/0013-platform-native-update-adapters.md) when a native adapter is implemented. Swift binding conformance remains a required macOS CI check.

Related pages: [Windows packaging](package-windows-desktop.md), [known platform limits](../architecture/known-limitations.md), and [server deployment](deploy-server-environments.md).
