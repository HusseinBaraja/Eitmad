---
title: "Validate a release candidate"
description: "Run mandatory CI, package integrity, desktop and server smoke, migration, security, Arabic, and rollback checks before promotion."
audience: "operations"
page_type: "task"
status: "active"
owner: "release infrastructure maintainers"
last_verified: "2026-08-26"
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
| Contract freshness and generated C#/Swift compatibility | `contracts-and-ipc`, `macos-bindings` |
| IPC and engine lifecycle | `contracts-and-ipc`, `windows-desktop` |
| Migrations and missing migration detection | Rust migration tests plus immutable `deploy/migrations.sha256` in `repository-policy` |
| Authorization, tenant isolation, audit redaction, secret storage, sync, updates | Focused workspace tests in `rust-quality` |
| Unsafe Rust, console, or C# `ILogger` calls; possible secret literals; direct shell database/config/secret access, including Rust and Swift environment APIs | `repository-policy` |
| Arabic/RTL, mixed-direction contracts, Windows shell behavior | `contracts-and-ipc`, `windows-desktop`, `macos-bindings`, `repository-policy` |
| Documentation impact, metadata, links, ownership | `repository-policy` and documentation audit |
| Desktop build, shell lifecycle, real-engine supervision | `windows-desktop` |
| Linux server build, CLI, and safe configuration | `server-smoke` |
| Repeatable Windows and Linux-hosted server artifacts | `package-windows`, `package-server` |

The workflow runs for every pull request and every push to `main` without path filters. Existing focused workflows can give faster feedback, but they do not replace mandatory validation. The repository-policy job compares pull requests with their base SHA and pushes with their prior SHA. A manual run requires the operator to supply a base SHA.

That base comparison makes existing server migration paths and bytes immutable. A schema change must add a new numbered migration and refresh `deploy/migrations.sha256`; changing or deleting a migration present at the base revision fails policy validation even if the manifest was refreshed.

## Promotion checks outside hermetic CI

CI cannot hold production signing keys or prove a live restore. Before promotion:

1. Verify the reviewed commit, source tag, artifact SHA-256, SBOM when available, and signing identities.
2. Verify production signatures and timestamps on clean target systems.
3. Run the native protected-secret lifecycle on each release platform.
4. Deploy the exact digest to staging and run migration, authentication, denial, tenant-isolation, redaction, sync, update, backup, and restore checks.
5. Exercise an interrupted update and rollback or forward-recovery path.
6. Verify Arabic root RTL, mixed Arabic/Latin identifiers, keyboard flow, accessibility, and copy/paste on supported desktop systems.
7. Promote the same digest to the next channel. Do not rebuild between `canary`, `beta`, and `stable`.

## Failure and recovery

Treat a missing, cancelled, or skipped check as a failure. Do not edit a generated binding, migration checksum, or package manifest to hide drift. Fix the Rust authority or source migration, regenerate through the approved command, update affected documentation, and rerun the full workflow.

For a release failure after promotion, pause the channel, preserve evidence, follow the platform or server rollback guide, and add reusable symptoms to troubleshooting documentation.

Related pages: [Windows packaging](package-windows-desktop.md), [macOS expectations](prepare-macos-distribution.md), [Linux expectations](prepare-linux-distribution.md), and [server deployment](deploy-server-environments.md).
