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

Install the .NET 8 SDK and Rust 1.85.1 with rustup. From the repository root, build the debug Rust engine, provision the synthetic development accounts, and start the native Windows dashboard with one command:

```powershell
.\run.ps1
```

Use `admin` with `admin` or `rec` with `rec` on the sign-in screen. These short synthetic credentials are accepted only by debug builds after the explicit seed command that `run.ps1` runs. Existing role changes, credentials, and deactivation are preserved on restart. Release builds do not contain that command and keep the 12-character password minimum. Closing the window keeps the app in the system tray. Use **إنهاء الاعتماد** in the tray menu to stop the shell and engine. For design and ownership details, see the [Windows operations shell guide](docs/developer/subsystems/windows-native-shell.md).

For an existing connected installation, use the same launcher with `-RuntimeDirectory <installation-directory>`. Rust reads its retained `connection.json`; no development accounts are added. Keep its server running. Use `-SkipBuild` only when source and dependencies are unchanged. See the [connected workflow test setup](docs/operations/run-workflow-test-environment.md) for the current provisioning boundary and synthetic test records. The normal local installation has no server connection until one is configured, so server-confirmed sales actions remain unavailable there.

### Start the local services

After completing the [connected workflow setup](docs/operations/run-workflow-test-environment.md), use PowerShell 7.4 or later from the repository root to start PostgreSQL and the TLS server:

```powershell
.\run-services.ps1
```

Wait for `SERVICES READY`. The command builds the required service tools, starts the retained database and server, checks TLS readiness, and refreshes the installation's connection references. It leaves the app closed and preserves business records. It requires completed setup and does not add test records.

Then open the normal app against the same installation:

```powershell
.\run.ps1 -RuntimeDirectory "$PWD\tests\engine-integration\artifacts\workflow-e2e\receptionist"
```

Both commands support `-SkipBuild` when the relevant source and dependencies are unchanged. For another retained setup, pass `-StateDirectory <setup-directory>` to `run-services.ps1` and use its `receptionist` child as the app's `-RuntimeDirectory`.

On Windows, another local server can lock the shared build executable. If a build reports `Access is denied` for `eitmad-server.exe`, stop that server through the command that started it, then retry. Use `-SkipBuild` only if the existing outputs match the current source and dependencies.

Inspect the services:

```powershell
.\run-services.ps1 Status
```

Exit the app through its tray menu, then stop the services:

```powershell
.\run-services.ps1 Stop
```

Stop retains stored work and does not close the app. Start refuses an already running setup; use Status before restarting. Keep the same `-StateDirectory` for Start, Status, and Stop. These commands control the retained local development setup; they do not install system services.

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
