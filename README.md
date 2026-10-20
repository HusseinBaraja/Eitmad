# الاعتماد | Eitmad

Arabic-first operations system for الاعتماد furniture manufacturing.

This repository is a cross-platform native desktop workspace built around a separate, authoritative Rust engine. Native shells stay thin; Rust owns contracts, product behavior, storage, synchronization, authorization, update policy, observability, audit, and external boundaries.

Use English for reasoning and for communication with the user. Keep the application UI Arabic-first, and preserve exact Arabic UI labels and established domain terms when you refer to them.

Start with:

- [Repository policy](AGENTS.md)
- [Target architecture](docs/architecture/target-architecture.md)
- [Repository layout and ownership](docs/developer/repository-layout.md)
- [Documentation index](docs/index.md)

## Run the admin dashboard on Windows

To use a fresh Windows installation, first complete the [server setup](docs/operations/run-server-authority.md): configure PostgreSQL, TLS, and the required server settings, run `check-config` and `migrate`, bootstrap and activate the first Owner, and start the TLS server with `serve`. Follow [shared server account setup](docs/operations/run-server-authority.md#manage-shared-server-accounts) to register the branch and prepare an active account with a Manager assignment or a Receptionist assignment for that branch. Owner status alone is not an operational desktop role. Keep the server and PostgreSQL running.

Install the .NET 8 SDK and Rust 1.85.1 with rustup. From the repository root, build the debug Rust engine and start the native Windows dashboard:

```powershell
.\run.ps1
```

On the sign-in screen, enter the server account name and password. Expand **ربط هذا الجهاز بالمنشأة** and provide the HTTPS endpoint, trusted CA PEM file, tenant code, organization ID, and registered branch ID from the administrator, then select **دخول**. See [enroll or recover a Windows desktop](docs/operations/run-server-authority.md#enroll-or-recover-a-windows-desktop) for the full enrollment steps. The launcher does not create accounts. Sign-in and protected work require the live server; a retained local verifier does not authorize disconnected access under [ADR-0029](docs/decisions/0029-require-live-server-authority-for-protected-access.md).

To check the installation, sign in and save one synthetic customer or draft with a permitted account. Use **إنهاء الاعتماد** in the tray menu to stop the shell and engine; closing the window keeps the app in the tray. Start the app again under the same Windows user and runtime directory, leave enrollment closed, sign in online, and check that the saved work remains. For design and ownership details, see the [Windows operations shell guide](docs/developer/subsystems/windows-native-shell.md).

For an existing connected installation, use the same launcher with `-RuntimeDirectory <installation-directory>`. Rust reads its retained `connection.json`. Keep its server running. Use `-SkipBuild` only when source and dependencies are unchanged. See the [connected workflow test setup](docs/operations/run-workflow-test-environment.md) for an isolated setup with synthetic test records.

### Run an unattended Windows host

Use the [Windows host service guide](docs/operations/run-windows-host-services.md) to install PostgreSQL and the Rust server with separate service identities and retained data. After installation, run these commands in an elevated PowerShell window:

```powershell
.\run-services.ps1 Start
```

Start waits for verified HTTPS readiness. Windows also starts the installed services after reboot without an interactive login. Closing or exiting the Manager app leaves the host running. The command does not build binaries, open WPF, seed records, or change power settings.

For a local synthetic development environment, keep using its explicit owner:

```powershell
pwsh -File tools/workflow-environment.ps1 Start -ServicesOnly
```

For a new PC, start `run.ps1` with its own installation directory and use [desktop enrollment](docs/operations/run-server-authority.md#enroll-or-recover-a-windows-desktop). The launcher accepts a fresh directory and does not seed local accounts. Never copy another PC's runtime directory.

The development tool supports `-StateDirectory` and `-SkipBuild` for retained test state. It does not install Windows services. Use `run.ps1 -RuntimeDirectory <client-installation>` to open a client independently.

On Windows, another local server can lock the shared build executable. If a build reports `Access is denied` for `eitmad-server.exe`, stop that server through the command that started it, then retry. Use `-SkipBuild` only if the existing outputs match the current source and dependencies.

Inspect the services:

```powershell
.\run-services.ps1 Status
```

Stop the installed host explicitly when maintenance requires it:

```powershell
.\run-services.ps1 Stop
```

`Stop` retains work. `Uninstall` removes service registrations and preserves the database and all installed files. Use the same `-StateDirectory` and `-Name` for a non-default host. See the service guide for install inputs, least-privilege access, recovery, sleep requirements, and the live reboot acceptance checks.

## Furniture editor flow

The Windows Furniture editor uses six steps: `المعلومات`, `الأجزاء`, `المقاسات`, `الخيارات`, `التسعير`, and `المراجعة`. Managers set proposed prices for each fixed variant, then choose `حفظ كمسودة` or `حفظ التعريف` from the read-only review step. Saving the definition does not publish a selling price. Use `فتح التسعير` after saving an active definition, publish each variant's selling price, and wait for server confirmation before Receptionist sales. See [Furniture](docs/developer/subsystems/furniture.md) for the definition and publication boundaries.

## Ready-made Products manager

The Windows **المنتجات** page manages ready-made items through Rust-owned authorization, audit, durable definitions, supplier variants, images, and catalog transfer. Managers can add, edit, duplicate, and archive definitions. Commercial selling prices use the separate [Pricing capability](docs/developer/subsystems/pricing.md); see [Products](docs/developer/subsystems/products.md) for the storage and public catalog boundaries.

## Server-confirmed pricing

The separate Windows **التسعير** page lets a manager search product variants, filter by category, and open **تعديل سعر البيع** without entering the full Furniture editor. Rust returns costs, margins, option adjustments, and prices for separate ready-made Product and manufactured Furniture references. Publication requires an authenticated server receipt and creates an immutable, durable, audited price revision. Receptionist IPC projections omit internal costs and margins.

## Manager quotation review

The Windows **عروض الأسعار** page reads Rust-owned drafts and server-confirmed quotation state. Managers review and decide discount approval requests; Receptionists compose, issue, record acceptance, and convert quotations under Rust authorization. See [Quotations](docs/developer/subsystems/quotations.md) for persistence, approval, and retry behavior.

## Manager order review

The Windows **الطلبات** page reads server-confirmed orders with immutable commercial snapshots. Rust authorizes conversion, fulfillment changes, manufacturing transitions, and delivery; each role receives its permitted actions. See [Orders](docs/developer/subsystems/orders.md) for state, audit, and replay behavior.

## Manager work orders

The Windows **أوامر العمل** page separates manufactured Furniture from ready-made Products. Managers inspect production specifications, assign work and due dates, and start or complete manufacturing through Rust-owned server-confirmed transitions. The page omits commercial prices and margins. See [Work Orders](docs/developer/subsystems/work-orders.md) for the production boundary.

For routine changes, use the [focused check selection table](docs/developer/index.md#choose-the-smallest-normal-proof). Keep the full workspace and platform gates for [release-candidate validation](docs/operations/validate-release-candidate.md).

Production features must be organized by bounded product capability. Do not add generic `utils`, `common`, `shared`, `handlers`, or `services` buckets.
