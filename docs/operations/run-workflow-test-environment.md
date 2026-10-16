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

### Prepare and launch the local demo

Use `run-demo.ps1` for the presentation. It uses this connected workflow with fixed state at `tests/engine-integration/artifacts/demo-family-purchase` and loopback ports `55459` (PostgreSQL) and `58463` (TLS server). This separate environment preserves the previous `demo-presentation` state and its services. It does not use the local-only `run.ps1` account seeder. Keep this state under the same Windows user. The default PostgreSQL binaries on this PC are at `target/direct-order-test/pgsql/bin`; supply `-PostgresBin` if they are installed elsewhere. That path and the ports are retained in private settings.

The fresh connected seed contains one fictional family purchase: a two-door walnut wardrobe at 185,000 YER and a double mattress at 95,000 YER, one of each, with no discount and a 280,000 YER total. Both have bundled synthetic catalog photos. The same customer and specifications link the converted quotation, order, customer documents, and completed wardrobe work order. Rust completes production and leaves the order Ready, without recording delivery. The customer, address, workshop assignment, and non-dialable all-zero phone are fictional. Prices are demo values, not a market quotation. Photo prompts are retained beside the `crates/server-connection/examples/workflow_environment/photos/README.md`.

Before the meeting, while package sources are available, prepare all build outputs and check the PostgreSQL binaries:

```powershell
pwsh -NoProfile -File .\run-demo.ps1 Prepare
```

Prepare builds the Rust server, engine, provisioning example, WPF shell, certificate tool, and test assembly. It does not start services or claim demo readiness. Repeat Prepare after changing relevant source or dependencies. Keep the prepared checkout and its build outputs on the demo PC.

From stopped services, use one launch command:

```powershell
pwsh -NoProfile -File .\run-demo.ps1
```

Start uses prebuilt outputs without a package restore or build. It starts the retained PostgreSQL cluster and local TLS server, provisions through Rust, and runs `DemoRolesCanReadFamilyPurchase` through the real supervised engine and authenticated routes. The check signs in as both roles, reads confirmed prices and photo assets, and compares the linked family quotation, order, printable documents, and Manager work-order specifications. It permits separate catalog samples and quotations. It requires the family order to remain Ready and permits Receptionist delivery. Normal Start adds no customers, quotations, or orders. Restart preserves accepted work; it does not reseed or undo delivery. After rehearsing delivery, readiness fails until another purchase is prepared through normal authorized workflows; it never resets the delivered order.

Wait for `DEMO READY` and exit code zero. The launcher requires every selected test to pass; a skipped or missing test cannot pass readiness. The private `demo-ready.json` links the fresh TRX result and `demo-workflow-proof.json`. A new launch clears old readiness proof, and Stop removes the ready marker. These files record a completed check, not continuous health monitoring. Leave PostgreSQL, the server, and the opened app running before the meeting. Local services use loopback and need no store internet after preparation. Actual disconnected operation still needs a rehearsal on the demo PC.

For this demo, read `manager_password` and `receptionist_password` locally from the protected `tests/engine-integration/artifacts/demo-family-purchase/secrets.json` before the meeting. Use the sign-in names `manager` and `receptionist`. Do not print or copy private settings into evidence or logs.

For CLI verification without opening or driving the desktop:

```powershell
pwsh -NoProfile -File .\run-demo.ps1 -NoDesktop
```

This checks the same Rust workflow and leaves the local services running. It does not verify visible sign-in, keyboard input, or the display. After a CLI-only check, stop this setup before using the normal launch command:

```powershell
pwsh -NoProfile -File .\run-demo.ps1 Stop
```

Use `run-demo.ps1 Status` to inspect owned processes. A PC reboot, visible role sign-in, and a manual quotation rehearsal remain separate checks. The full `-VerifySignIn` rehearsal below remains available; the focused demo check does not replace its field-editor, order, or delivery coverage. On failure, keep the state, inspect the stage error, then Stop before retrying. Do not reset databases or print private settings.

### Recover the demo and keep rehearsal separate

Use the default `-Environment Presentation` for the retained family purchase. Use `-Environment Rehearsal` for practice. Each name selects its own retained database, account credentials, local authority, and process journal. The launcher prints the selected name before each action. Keep only one demo desktop open so that identical synthetic record numbers cannot be mistaken for the other environment.

| Environment | State directory under `tests/engine-integration/artifacts/` | Database / TLS ports |
| --- | --- | --- |
| Presentation (default) | `demo-family-purchase` | `55459` / `58463` |
| Rehearsal | `demo-family-rehearsal` | `55475` / `58479` |

Before the meeting, prepare the rehearsal through the same launcher. On its first Start, supply the installed PostgreSQL binary path with `-PostgresBin` if needed. Start once to seed the family purchase; Stop; then Start with `-PrepareCustomization` to add the separate sample. These operations use Rust authority and preserve partial and accepted work. Neither environment has a reset action. Preparation is a one-time step; later starts omit `-PrepareCustomization`.

```powershell
pwsh -NoProfile -File .\run-demo.ps1 -Environment Rehearsal -NoDesktop -PostgresBin "$PWD\target\quotation-test-env\pgsql\bin"
```

```powershell
pwsh -NoProfile -File .\run-demo.ps1 Stop -Environment Rehearsal
```

```powershell
pwsh -NoProfile -File .\run-demo.ps1 -Environment Rehearsal -PrepareCustomization
```

Use this short recovery route with the selected environment:

1. **Start:** from stopped services, run `pwsh -NoProfile -File .\run-demo.ps1 -Environment Presentation`. Wait for `DEMO READY` and exit code zero. This reads the retained family purchase through both accounts without rebuilding. Before the audience arrives, obtain both matching passwords from this environment's protected settings. Keep them available privately; never put them in the runbook, screenshots, or logs.
2. **Confirm readiness:** run `pwsh -NoProfile -File .\run-demo.ps1 Status -Environment Presentation`. Expect `server`, `desktop`, and `postgres` to be running. Sign in as `receptionist`; confirm the records below. The ready marker is historical proof, not a health monitor. Keep this tested app session open.
3. **Accidental close:** closing the main window hides it. Open the Windows notification area, including hidden icons if needed. Double-click **الاعتماد · مركز العمليات**, or right-click it and select **فتح مركز العمليات**. Do not run Start again while it is in the tray.
4. **Switch accounts:** save unfinished input, press **Alt+K**, then select **تسجيل الخروج** in **تبديل الحساب**. Sign in as `manager` or `receptionist` with the same environment's matching password. Unsaved input clears; accepted records remain. Cancel with **إلغاء** to keep the current form.
5. **Exit from tray:** right-click the icon and select **إنهاء الاعتماد**. This ends the desktop and its supervised engine. Closing the window is not Exit.
6. **Stop local services:** run `pwsh -NoProfile -File .\run-demo.ps1 Stop -Environment Presentation`. Expect `Clients, server, and PostgreSQL stopped. Accepted work is retained.` Status must show stopped processes. Stop also ends a remaining owned desktop safely.
7. **Resume:** repeat Start with the same environment name and locate the same record numbers. Start refuses running processes; use the tray to reopen, or Stop before Start. A failed readiness check never restores an earlier commercial state. Retain the state and use the recorded fallback if recovery would interrupt the meeting.

For practice, replace `Presentation` with `Rehearsal` in every step. Exit and stop Rehearsal before opening Presentation. Do not copy its database, credentials, or process journal into Presentation.

### Try the prepared family story

In Presentation, use Receptionist to open the customer **أسرة سامر السعدي**, converted quotation `QT-2026-00001`, and Ready order `OR-2026-00001`. The order contains **خزانة السكينة ببابين** at 185,000 YER and **مرتبة الراحة المزدوجة** at 95,000 YER, quantity one each, zero discount, **280,000 YER** total. Open the saved quotation or order document to show the same customer, specifications, and total. Leave this order undelivered for the meeting. A converted quotation cannot be converted again.

Switch to Manager and open Orders → `OR-2026-00001` → **فتح تصنيع الطلب**. Work order `WO-2026-00001` is already Completed. Show its wardrobe specification; no new completion is needed. Switch back to Receptionist. The order remains Ready. Delivery changes durable state; perform a delivery rehearsal only in Rehearsal.

The separate sample **خزانة السكينة حسب المقاس**, quotation `QT-2026-00002`, is Issued: 1400 × 2100 × 600 mm, walnut, brass handle, quantity one, zero discount, **200,000 YER**. Its smaller variant costs 185,000 YER plus the 15,000 YER handle. The larger variant with brass costs 240,000 YER. Use the issued sample for inspection; keep Presentation unchanged. In Rehearsal, this unconverted sample can be used for acceptance, conversion, production, and delivery practice. Once converted or delivered, create a fresh quotation through the app and use its newly issued numbers. Restart does not undo those transitions.

The strict demo gate requires one family order in Ready state. After creating another rehearsal order or delivering the family order, resume the retained rehearsal without that gate:

```powershell
pwsh -NoProfile -File .\tools\workflow-environment.ps1 Start -StateDirectory "$PWD\tests\engine-integration\artifacts\demo-family-rehearsal" -SkipBuild
```

Stop that environment first if its processes are still running. This command preserves all work and opens the app to prepare fresh records; it does not claim the original Ready-order gate passed. Continue to use `run-demo.ps1 Stop -Environment Rehearsal` and `Status -Environment Rehearsal` for its owned services.

For a fresh manufacturing order in separate verification or Rehearsal state, a Manager can use Orders → **بدء أمر العمل** → **إكمال أمر العمل** after server confirmation. The [Orders guide](../developer/subsystems/orders.md#native-workflow) explains refresh sequencing, cached reads, and exact retry after an uncertain reply. A connected button-handler test verifies this direct route with test-generated WPF renders; it does not establish a physical click-through. The existing Presentation family work order is already Completed and must remain Ready for the meeting.

Before the meeting, keep a local visual fallback made from the verified family proof and bundled synthetic photos. Label it **Recorded evidence**, include its verification time and environment, and show 280,000 YER for the family story. It is a saved snapshot, not current application state or proof of a live click. The action 13 enlarged document fixture is a separate synthetic **47,500 YER** example; do not present it as this family purchase. The local action 14 route, recorded fallback, measured restart proof, and remaining manual checks are saved under `artifacts/demo-fixes-20261016/`.

### Prepare the optional customization sample

After the family demo has been seeded, stop its services, run Prepare for the current source, then use:

```powershell
pwsh -NoProfile -File .\run-demo.ps1 -PrepareCustomization -NoDesktop
```

This adds `خزانة السكينة حسب المقاس` as a separate active furniture item. It preserves the original family purchase. The two standard sizes are 1200 × 2000 × 600 mm at 185,000 YER and 1600 × 2000 × 600 mm at 225,000 YER. Both permit custom width 1000–1800 mm, height 1800–2400 mm, and depth 500–700 mm. Both accept walnut or white and a black handle at no extra charge or a brass handle at an extra 15,000 YER. Custom dimensions have no separate surcharge in this sample; the selected variant determines the base price. These are synthetic demo values.

Rust saves and audits the new definition, synchronizes it through authenticated TLS, and publishes both current prices with server confirmation. As Receptionist, the command selects 1400 × 2100 × 600 mm, walnut, and the brass handle on the smaller variant. It creates and issues one separate quotation for the existing fictional family at 200,000 YER, with no discount. It compares the server quotation specification and total with the local evaluation. It also checks the larger variant at 240,000 YER and rejects width 1801 mm locally with a Dimensions/Invalid error and no total. It does not accept or convert the extra quotation, change the original order, or record delivery.

`customization-proof.json` in the protected demo state records the published public variants, chosen specification, server-issued quotation, and local rejection. Stable idempotency keys resume partial preparation without creating another sample or quotation. Changed source definitions or rehearsal edits can cause a conflict; retain the state and inspect the failed stage instead of resetting or overwriting work. Normal Start does not run this preparation.

Stop the CLI setup before the next normal launch. In the visible app, sign in as Receptionist and rehearse the same selection and out-of-range error. The CLI proof does not verify the displayed quotation, Arabic layout, input, or focus. If that rehearsal cannot be completed before the meeting, show the original fixed-size family purchase and describe customization separately.

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
| `receptionist/` | Shared interactive desktop authority for both accounts, including saved Receptionist drafts, audit, checkpoints, and route reference |
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
| Five quotations | Draft, pending discount approval above 5%, issued, accepted, and converted |
| Ready-made order | Created from the accepted quotation through server confirmation |

Images are not seeded. These records contain synthetic Arabic text only. The environment is intended for the [accepted Manager and Receptionist workflows](../developer/subsystems/manager-receptionist-workflows.md), not production operation.

Seed mutations use stable idempotency keys. After successful seeding, later starts refresh server credentials and import only missing local accounts. Existing account identity must match, including inactive accounts. Setup preserves local role, display name, password verifier, activation state, relationships, revision, and sessions. It does not sign in to local fixture accounts or restore their initial access. Existing accepted work is not replaced. A failure preserves local outboxes, journals, audits, and PostgreSQL state.

The running engine retries pending customer changes on a Rust worker every two seconds when idle. The launcher supplies `EITMAD_CUSTOMER_SERVER`, `EITMAD_CUSTOMER_TRUST_PEM`, `EITMAD_CUSTOMER_CREDENTIAL_ID`, and `EITMAD_CUSTOMER_BRANCH_ID`. The credential setting is a route reference. Rust resolves each original author's account credential and checks current branch permission before transfer. Connection failure retains pending work for retry. Customer revisions must reach server confirmation before quotation validation can use them; restarting the environment is not required.

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
