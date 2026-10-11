---
title: "Run the Manager and Receptionist end-to-end test environment"
description: "Start two real WPF clients with authenticated Rust server routes and persistent, isolated PostgreSQL test state."
audience: "developer"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-10-11"
review_triggers:
  - "account provisioning, desktop startup, server routes, workflow seeding, or test prerequisites change"
keywords:
  - "end-to-end"
  - "Manager Receptionist"
  - "workflow-environment.ps1"
  - "PostgreSQL"
  - "تسجيل الدخول"
---

# Run the Manager and Receptionist end-to-end test environment

Use `tools/workflow-environment.ps1` to start a dedicated PostgreSQL cluster, the real `eitmad-server` TLS host, and two real WPF clients. Each client supervises its own Rust engine and uses its own runtime directory. Stop and restart use the same state; neither action resets a database or a directory.

## Prerequisites

- Windows with an interactive desktop and access to Windows Credential Manager. Run all commands as the same Windows user.
- PowerShell 7.4 or later, .NET 8 SDK, and Rust `1.85.1` installed with rustup, including rustfmt and Clippy for development checks.
- The MSVC C++ build tools and native dependency build tools used by the Rust workspace, including CMake and NASM. The first build needs access to the locked Cargo and NuGet dependencies.
- PostgreSQL 17 Windows binaries: `initdb.exe`, `pg_ctl.exe`, `psql.exe`, and `createdb.exe` in one directory. An installed PostgreSQL service is not used. Do not point the script at an existing cluster or database.
- Free loopback ports `55439` and `58443`, and enough disk space for the normal `target` build cache and test state.
- A fresh checkout with commands run from the repository root. No production credentials, real customer data, or developer account seeder are needed.

## Start and sign in

Supply the PostgreSQL binary directory on the first launch. For an installation at `C:\Program Files\PostgreSQL\17\bin`:

```powershell
pwsh -File .\tools\workflow-environment.ps1 Start -PostgresBin 'C:\Program Files\PostgreSQL\17\bin' -VerifySignIn
```

The command builds the server, engine, provisioning example, and WPF shell. It uses the stable `target/toolchain-1.85.1` Rust cache to isolate the required compiler from other installed toolchains. Server readiness is checked over TLS with the generated CA trust anchor. `-VerifySignIn` runs the focused real-WPF integration test before it opens the two interactive clients. This test enters the generated credentials through the real sign-in controls, checks the role-specific surface, and reads server-confirmed quotations and orders through IPC and TLS. No preview engine or fake confirmation is used.

Sign in to the two windows with the usernames `manager` and `receptionist`. The passwords are generated independently. Open `tests/engine-integration/artifacts/workflow-e2e/secrets.json` locally to read the `manager_password` and `receptionist_password` fields. Do not print this file to a terminal, include it in logs, commit it, or paste its contents into a support report. Passwords, device signing seeds, and the server token key are retained only in this ACL-protected, ignored directory. Authenticated client token pairs use Rust-owned Windows Credential Manager entries with separate device-based references.

The default state is explicitly identified as disposable test state by `disposable-eitmad-e2e`. Its layout is:

| Path under the state directory | Purpose |
| --- | --- |
| `postgres/` | Dedicated PostgreSQL cluster; database `eitmad_workflow_test` |
| `manager/` | Manager SQLite authority, audit, checkpoints, and non-secret credential reference |
| `receptionist/` | Separate Receptionist SQLite authority, audit, checkpoints, and credential reference |
| `certificates/` | Private test TLS key and explicit CA trust anchor; renewed at startup |
| `secrets.json`, `provision.json` | Private settings and resumable provisioning journal |
| `processes.json` | Process IDs, executable paths, and start times used to identify owned processes |
| `accepted-work-proof.json` | Synthetic accepted order sources saved by the sign-in test and compared on later runs |
| `*.log` | Private service output; no passwords, access tokens, or raw IPC frames are printed |

The PostgreSQL application role is `NOSUPERUSER NOBYPASSRLS`. Rust applies the normal migrations and tenant isolation. The launcher clears server database credentials and token keys before it starts the shells.

## Seeded work and authority

`crates/server-connection/examples/workflow_environment.rs` calls the supported Rust identity, authentication, storage-import, and product operations. Account activation and subsequent logins use the control-plane authentication service with real registered devices and signed device proof. The WPF sign-in uses the existing imported-account credential verifier; it does not implement an online login dialog. Catalog transfer, customer sync, price publication, quotation approval, issuance, acceptance, and conversion use the actual authenticated TLS routes. The test does not write business rows with SQL or disable authentication, RLS, authorization, audit, or server confirmation.

The initial Manager is also the tenant bootstrap owner so it can provision the test environment. Its operational Manager role is granted separately. The Receptionist receives the organization and branch Receptionist relationships and cannot grant a Manager role; setup verifies this denial.

| Synthetic fixture | Durable initial state |
| --- | --- |
| Arabic material category, count unit, wood material, part category, and composed part | Manager-owned definitions with audit and catalog transfer |
| Arabic furniture category and wardrobe with a fixed size | Active definition with a server-published selling price |
| Arabic product category and mattress variant | Ready-made definition with a server-published selling price |
| Arabic customer contact | Branch-scoped, server-confirmed customer |
| Five quotations | Draft, pending discount approval above 5%, issued, accepted, and converted |
| Ready-made order | Created from the accepted quotation through server confirmation |

Images are not seeded. These records contain synthetic Arabic text only. The environment is intended for the [accepted Manager and Receptionist workflows](../developer/subsystems/manager-receptionist-workflows.md), not production operation.

Seed mutations use stable idempotency keys. After successful seeding, later starts authenticate both accounts and verify the retained catalog and workflows without recreating the fixtures. Existing accepted work is not replaced. A failure preserves local outboxes, journals, audits, and PostgreSQL state.

## Stop, restart, and inspect

```powershell
pwsh -File .\tools\workflow-environment.ps1 Stop
```

Stop identifies each owned process by PID, executable path, and start time. It ends the WPF dispatchers and their supervised engine process groups, stops the server, and performs a PostgreSQL fast shutdown. In-flight, uncommitted server transactions can roll back; accepted transactions and local committed work remain durable. For a normal manual client shutdown, use **إنهاء الاعتماد** in each system tray menu, then run `Stop` for the services.

Restart and repeat the sign-in proof:

```powershell
pwsh -File .\tools\workflow-environment.ps1 Start -VerifySignIn
```

For unchanged built source, omit builds:

```powershell
pwsh -File .\tools\workflow-environment.ps1 Start -SkipBuild -VerifySignIn
```

```powershell
pwsh -File .\tools\workflow-environment.ps1 Status
```

`-SkipBuild` requires the existing outputs from this checkout. Do not use it after changing relevant source. The PostgreSQL binary directory and ports are saved at first launch. Later starts reuse them.

## Isolate another test run

Choose a new, unused directory below `tests/engine-integration/artifacts` and different ports:

```powershell
pwsh -File .\tools\workflow-environment.ps1 Start -StateDirectory "$PWD\tests\engine-integration\artifacts\workflow-second" -PostgresBin 'C:\Program Files\PostgreSQL\17\bin' -DatabasePort 55440 -ServerPort 58444 -VerifySignIn
```

Supply the same `-StateDirectory` to `Stop`, `Status`, and subsequent `Start` commands. Existing directories without the exact disposable marker and paths outside the test-artifact root are refused. Reparse points in the state path are refused. There is no reset, drop, truncate, restore, or recursive-delete action.

## Recover a failed launch

Run `Status`, inspect the stage-specific error, and stop this environment before retrying. A failed launch can leave PostgreSQL or the server running so the evidence and accepted work remain available. Correct the prerequisite or service failure, then restart with the same directory.

If provisioning committed in PostgreSQL but its local journal could not be saved, setup fails closed. Preserve that environment for diagnosis. Use a new named disposable environment for a fresh test; do not hand-edit business tables or delete the old state. Keep the Windows credential store and the private state directory together under the same user. Moving only SQLite files to another account does not transfer authentication.

Return to [operations](index.md) or inspect [server authority](../developer/subsystems/server-authority.md) and [Windows supervision](../developer/subsystems/windows-process-supervision.md) when changing this path.
