# الاعتماد | Eitmad

Arabic-first operations system for الاعتماد furniture manufacturing.

This repository is a cross-platform native desktop workspace built around a separate, authoritative Rust engine. Native shells stay thin; Rust owns contracts, product behavior, storage, synchronization, authorization, update policy, observability, audit, and external boundaries.

Use English for reasoning and for communication with the user. Keep the application UI Arabic-first, and preserve exact Arabic UI labels and established domain terms when you refer to them.

Start with:

- [Repository policy](AGENTS.md)
- [Target architecture](docs/architecture/target-architecture.md)
- [Repository layout and ownership](docs/developer/repository-layout.md)
- [Documentation index](docs/index.md)

## Install on Windows

Follow [Install Eitmad on the server and employee PCs](docs/operations/install-windows.md). Use the complete reviewed offline bundle and open `setup\EitmadSetup.exe` as the intended normal Windows user.

- **Server:** select **خادم الورشة ومعه تطبيق الموظف**. Enter the workshop name and Owner credentials, choose backup destinations, and approve UAC. Setup creates the workshop and installs the services and this user's desktop.
- **Client:** select **جهاز موظف**. After installation, open **الاعتماد**, discover the server, pair with an Owner code, and sign in with an individual employee account. Client installation needs no administrator rights.

The guide covers prerequisites, first sign-in, pairing, verification, and interrupted setup. Installation uses local offline payloads; protected work still requires the live workshop server. A complete trusted signed bundle and clean-PC acceptance remain **Needs verification**. Unsigned build artifacts are for engineering validation. See [package building and maintenance](docs/operations/package-windows-desktop.md) for signing, repair, removal, and upgrade.

## Run from source on Windows

Install the .NET 8 SDK and Rust 1.85.1 with rustup. From the repository root:

```powershell
.\run.ps1
```

This development launcher builds the Rust engine and starts the native shell. It does not install the server or create accounts. Use a running workshop and independently pair the device, or use the [isolated workflow test environment](docs/operations/run-workflow-test-environment.md) for synthetic development data. Follow the [developer guide](docs/developer/index.md#1-run-the-local-windows-app) for runtime selection and focused checks.

For an existing connected runtime, pass `-RuntimeDirectory <installation-directory>`. Rust reads the retained connection settings. Use `-SkipBuild` only when source and dependencies are unchanged. Never copy another PC's runtime. Use **إنهاء الاعتماد** in the tray menu to exit the shell and engine; closing the window keeps the app in the tray.

Installed PostgreSQL and Rust services run separately from the desktop and start without Windows sign-in. Use the [Windows host service guide](docs/operations/run-windows-host-services.md) for status and maintenance. Use the [manual LAN guide](docs/operations/setup-lan-server.md) only for a separately provisioned endpoint; it is not a prerequisite for guided setup.

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
