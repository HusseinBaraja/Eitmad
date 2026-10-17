---
title: "Run the Manager and Receptionist end-to-end test environment"
description: "Start one real WPF client for Manager and Receptionist with authenticated Rust routes and persistent, isolated PostgreSQL test state."
audience: "developer"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-10-17"
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

Use `tools/workflow-environment.ps1` to start a dedicated PostgreSQL cluster, the real `eitmad-server` TLS host, and one real WPF client with one supervised Rust engine. Sign in as Manager or Receptionist in the same window. Rust returns the account role and authorizes its operations. Stop and restart use the same state; neither action resets a database or a directory.

## Prerequisites

- Windows with an interactive desktop and access to Windows Credential Manager. Run all commands as the same Windows user.
- PowerShell 7.4 or later, .NET 8 SDK, and Rust `1.85.1` installed with rustup, including rustfmt and Clippy for development checks.
- The MSVC C++ build tools and native dependency build tools used by the Rust workspace, including CMake and NASM. The first build needs access to the locked Cargo and NuGet dependencies.
- PostgreSQL 17 Windows binaries: `initdb.exe`, `pg_ctl.exe`, `psql.exe`, and `createdb.exe` in one directory. An installed PostgreSQL service is not used. Do not point the script at an existing cluster or database.
- Free loopback ports `55439` and `58443`, and enough disk space for the normal `target` build cache and test state.
- A fresh checkout with commands run from the repository root. No production credentials, real customer data, or developer account seeder are needed.

## Start and sign in

The setup tool is an integration-test harness. It starts the ordinary application with an isolated installation. It does not select an application mode or require any saved order to remain in a particular state. Keep existing state directories and evidence; no rename or reset is required.

After initial setup, use `run-services.ps1` from PowerShell 7.4 or later to start only PostgreSQL and the TLS server. It accepts `Start` (default), `Status`, `Stop`, `-StateDirectory`, and `-SkipBuild` for unchanged Start outputs. The default state directory is the same `workflow-e2e` setup used below. The repository README lists the paired service and app commands.

The service command reuses this tool with `-ServicesOnly`. Start requires the retained settings, completed provisioning journal, and PostgreSQL cluster. It builds only the server, Rust connection provisioning example, and certificate tool. It checks TLS readiness and refreshes existing account connection references through Rust. It does not seed business records, build or open WPF, or run workflow tests. Missing provisioning, a missing database, or an already running owned process fails without resetting state. Stop ends only the owned server and PostgreSQL; exit the app first. Existing databases and desktop state remain in place.

Build outputs are shared across local setups. On Windows, a running server from another setup can block replacement of `eitmad-server.exe` with `Access is denied`. Stop that setup through its owning command before rebuilding. Do not stop unrelated processes by executable name. `-SkipBuild` is suitable only when source and dependencies are unchanged.

Use `Prepare` to build without starting services. `Start -NoDesktop` starts the connected test services and provisions the retained installation without opening a desktop. `-PrepareCustomization` adds the optional customization fixture through Rust authority; it requires an already seeded catalog. These actions belong to test setup, not application startup.

After setup, the engine installation contains Rust-owned `connection.json` references for the HTTPS endpoint, CA trust anchor, organization, branch, and credential route. Tokens and signing keys remain in Windows Credential Manager. Reopening this installation does not require inherited service environment variables. The local server and PostgreSQL must still be running.

Open that existing installation through the normal launcher:

```powershell
.\run.ps1 -RuntimeDirectory "$PWD\tests\engine-integration\artifacts\workflow-e2e\receptionist"
```

This builds the current engine and shell, then opens the usual sign-in screen without provisioning development accounts or changing saved work. For unchanged built source, add `-SkipBuild`. Supply the same installation directory on subsequent starts. A runtime-directory argument selects stored installation data, not a different product workflow.

`RetainedConnectionStartsWithoutLauncherEnvironment` verifies both role sign-ins, server-confirmed catalog, quotations and orders, signed-out denial, and unchanged order records across two engine starts with all inherited `EITMAD_*` variables cleared. It does not require fixed record numbers, totals, or delivery states. Run it through the existing test assembly with `EITMAD_E2E_ROOT` and `EITMAD_E2E_ENGINE` set to the prepared test installation and current engine. It performs no desktop interaction.

### Run the full connected workflow test

Supply the PostgreSQL binary directory on the first launch. For an installation at `C:\Program Files\PostgreSQL\17\bin`:

```powershell
pwsh -File .\tools\workflow-environment.ps1 Start -PostgresBin 'C:\Program Files\PostgreSQL\17\bin' -VerifySignIn
```

The command builds the server, engine, provisioning example, and WPF shell. It uses the stable `target/toolchain-1.85.1` Rust cache to isolate the required compiler from other installed toolchains. Server readiness is checked over TLS with the generated CA trust anchor. `-VerifySignIn` runs the focused real-WPF integration test before it opens the interactive client. The test alternates Manager and Receptionist sign-ins for three rounds without restarting the engine. It checks role routing, sign-out denial, cleared controls, retained drafts, and server-confirmed quotations and orders through IPC and TLS. It also creates and edits a customer as Receptionist, waits for each revision to be confirmed, then issues a quotation and requests discount approval with the new revision. No preview engine or fake confirmation is used.

The same test fills real WPF fields for material categories and units, raw materials, Part categories and composition, Product categories and supplier variants, Furniture sizes and customization limits, and local desktop accounts. It rejects a fractional material cost before saving Arabic digits, reopens saved Product fields, and retains exact query results in `database-field-proof.json`. A later run compares those records before creating new synthetic work. The Receptionist customer form supplies the mixed-sales journey; Manager search shows the shared quotation with that customer name, and native customer documents retain it. Organization-wide customer search remains unavailable.

To retain synthetic entry and result images, set `EITMAD_UI_CAPTURE_DIR` to an absolute private artifact directory before `Start -VerifySignIn`. The launcher preserves this capture setting while clearing inherited service settings. `capture-metadata.jsonl` records actual WPF dimensions and scaling for the field captures. Images are rendered native WPF controls; they do not prove physical keyboard entry, a printer job, or 100% display scaling.

Sign in with username `manager` or `receptionist`. The passwords are generated independently. Open `tests/engine-integration/artifacts/workflow-e2e/secrets.json` locally to read the matching `manager_password` or `receptionist_password` value. Enter only the value, without JSON quotes or commas. Do not print this file to a terminal, include it in logs, commit it, or paste its contents into a support report. Passwords, device signing seeds, and the server token key are retained only in this ACL-protected, ignored directory. Authenticated token pairs use Rust-owned Windows Credential Manager entries derived from the route and user identity.

Use **Alt+K** or the account button to switch accounts. The **تبديل الحساب** dialog opens with **إلغاء** focused. Cancel keeps the current account and form input. Confirm **تسجيل الخروج** only after saving unfinished input. Unsaved form fields are cleared. Saved drafts and accepted work remain in storage. Rust closes the desktop session; the shell waits for account activation and subscription shutdown, clears account screens and credentials, and returns to sign-in. The application retains one visible window and the same engine.

Each direct request and subscription resolves credentials from its author, tenant, and device. Rust checks that identity before refreshing tokens or sending traffic. A saved background operation retains its original author; account switching does not reassign it to the next user. Revoked or unavailable authority leaves work pending or denied instead of inventing server confirmation. A shared desktop cannot test simultaneous users or cross-device sync; use separate isolated test environments when those behaviors are the test target.

The default state is explicitly identified as disposable test state by `disposable-eitmad-e2e`. Its layout is:

| Path under the state directory | Purpose |
| --- | --- |
| `postgres/` | Dedicated PostgreSQL cluster; database `eitmad_workflow_test` |
| `manager/` | Retained provisioning authority and original Manager data; no interactive client starts here |
| `receptionist/` | Shared interactive desktop authority for both accounts, including saved Receptionist drafts, audit, checkpoints, and connection references |
| `certificates/` | Private test TLS key and explicit CA trust anchor; renewed at startup |
| `secrets.json`, `provision.json` | Private settings and resumable provisioning journal |
| `processes.json` | Process IDs, executable paths, and start times used to identify owned processes |
| `accepted-work-proof.json` | Synthetic accepted order sources saved by the sign-in test and compared on later runs |
| `*.log` | Private service output; no passwords, access tokens, or raw IPC frames are printed |

The PostgreSQL application role is `NOSUPERUSER NOBYPASSRLS`. Rust applies the normal migrations and tenant isolation. The launcher clears server database credentials and token keys before it starts the shells.

## Seeded work and authority

Use the [workflow test runbook](test-manager-receptionist-workflows.md) for the mixed-sales smoke, focused failure scenarios, evidence, and current readiness blockers. `-VerifySignIn` also runs the mixed journey through approval, issue, conversion, manufacturing, native document preview, and delivery, with exact-key retries and retained-order checks on later starts.

`crates/server-connection/examples/workflow_environment.rs` calls the supported Rust identity, authentication, storage-import, and product operations. Account activation and subsequent logins use the control-plane authentication service with real registered devices and signed device proof. The WPF sign-in uses the existing imported-account credential verifier; it does not implement an online login dialog. Catalog transfer, customer sync, price publication, quotation approval, issuance, acceptance, and conversion use the actual authenticated TLS routes. The test does not write business rows with SQL or disable authentication, RLS, authorization, audit, or server confirmation.

The initial Manager is also the tenant bootstrap owner so it can provision the test environment. Its operational Manager role is granted separately. The Receptionist receives the organization and branch Receptionist relationships and cannot grant a Manager role; setup verifies this denial.

| Synthetic fixture | Durable initial state |
| --- | --- |
| Arabic material category, count unit, wood material, part category, and composed part | Manager-owned definitions with audit and catalog transfer |
| Arabic furniture category and wardrobe with a fixed size | Active definition with a server-published selling price |
| Arabic product category and mattress variant | Ready-made definition with a server-published selling price |
| Arabic customer contact | Branch-scoped, server-confirmed customer |
| One mixed purchase | Converted quotation for one wardrobe and one mattress, total 280,000 YER; Completed manufacturing and Ready order, without delivery |
| Bundled synthetic item photos | Imported through Rust catalog-image authority and transferred to the server |

The fixtures are fictional and exist only for integration tests. Later starts preserve all accepted work, including deliveries and additional orders. There is no startup requirement to retain their initial states. Optional customization adds a separate furniture definition with two variants, custom dimensions, colors, and handle adjustments. Use the normal catalog, quotation, order, and production screens to test changes.

Seed mutations use stable idempotency keys. After successful seeding, later starts refresh server credentials and import only missing local accounts. Existing account identity must match, including inactive accounts. Setup preserves local role, display name, password verifier, activation state, relationships, revision, and sessions. It does not sign in to local fixture accounts or restore their initial access. Existing accepted work is not replaced. A failure preserves local outboxes, journals, audits, and PostgreSQL state.

The running engine retries pending customer changes on a Rust worker every two seconds when idle. Rust loads the saved connection references for customer transfer. For an installation without a connection file, the existing `EITMAD_CUSTOMER_SERVER`, `EITMAD_CUSTOMER_TRUST_PEM`, `EITMAD_CUSTOMER_CREDENTIAL_ID`, and `EITMAD_CUSTOMER_BRANCH_ID` environment configuration remains available. The credential setting is a route reference. Rust resolves each original author's account credential and checks current branch permission before transfer. Connection failure retains pending work for retry. Customer revisions must reach server confirmation before quotation validation can use them; restarting the environment is not required.

For existing two-window environments, the launcher retains both directories and uses the existing `receptionist/` database as the shared desktop. It registers Manager on that device through authenticated server login, imports the verified account, and transfers catalog definitions through the supported authenticated catalog operations. Receptionist drafts and server-accepted work stay in place. Original Manager-only local data remains in `manager/`; this is not a general database merge or restore tool.

## Stop, restart, and inspect

```powershell
pwsh -File .\tools\workflow-environment.ps1 Stop
```

Stop identifies each owned process by PID, executable path, and start time. It ends the WPF dispatcher and its supervised engine process group, stops the server, and performs a PostgreSQL fast shutdown. It also stops owned clients from earlier two-window runs. In-flight, uncommitted server transactions can roll back; accepted transactions and local committed work remain durable. For a normal manual client shutdown, use **إنهاء الاعتماد** in the system tray menu, then run `Stop` for the services.

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

If sign-in reports **بيانات الدخول غير صحيحة أو انتهت صلاحية الحساب.**, use `manager` or `receptionist` with its matching field value from the same environment's protected file. `admin` is not an account in this environment. A deactivated account stays deactivated after restart; use user management from an active Manager account to inspect its status. `-VerifySignIn` expects both original accounts to remain active with their original roles and credentials; omit it when intentionally testing changed account access. A server-route failure must be diagnosed separately from local credential rejection; do not replace tokens, weaken authentication, or reset data to hide it.

If valid sign-in briefly opens an account screen and then returns to sign-in, check the engine connection state separately from credential rejection. The IPC reader must retain a partial frame while responses and events are delivered; cancelling that read caused intermittent startup disconnects. Rebuild the current source and use `-VerifySignIn` after stopping the environment. See [IPC framing and concurrency](../developer/subsystems/local-ipc.md#framing-concurrency-and-large-payloads) for the regression and safe reader behavior.

Return to [operations](index.md) or inspect [server authority](../developer/subsystems/server-authority.md) and [Windows supervision](../developer/subsystems/windows-process-supervision.md) when changing this path.
