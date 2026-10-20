---
title: "Prepare a fresh Windows workshop"
description: "Create a new private PostgreSQL cluster, TLS material, runtime settings, and active first Owner through the Rust setup backend."
audience: "operations"
page_type: "task"
status: "active"
owner: "server and Windows deployment maintainers"
last_verified: "2026-10-20"
review_triggers:
  - "fresh setup contracts, PostgreSQL preparation, credentials, certificates, or service import changes"
keywords:
  - "prepare-windows-workshop"
  - "offline setup"
  - "first owner"
  - "fresh PostgreSQL cluster"
---

# Prepare a fresh Windows workshop

`eitmad-server prepare-windows-workshop` prepares a **new** Windows state directory using reviewed offline tools. It creates a private CA and matching leaf certificate, an isolated PostgreSQL 17 cluster with SCRAM authentication, separate random administrator and application credentials, an application database, migrations, and the active Owner/Manager and branch. PostgreSQL binds only to `127.0.0.1`. The command stops this cluster before it returns public enrollment coordinates and writes `prepared-workshop.json`.

This is an installer backend. It installs no service, firewall rule, desktop, or scheduled backup. The combined `EitmadSetup.exe`, automatic discovery, and pairing remain unimplemented. [ADR-0030](../decisions/0030-offline-guided-windows-setup.md) defines the selected user experience.

## Before you start

Use a local deployment operator who can create the intended protected directory. Service installation still requires administrator rights. Prerequisites are reviewed PostgreSQL 17 `bin`, `lib`, and `share` directories, the self-contained certificate executable in the server package, Windows PowerShell 5.1, packaged `deploy/windows/prepare-workshop.ps1` beside `host.psm1`, and a new local state path with an existing parent. Verify the package and tools through the [package procedure](package-windows-desktop.md) first. This backend does not authenticate an arbitrary script or vendor binary supplied by an operator.

Use the actual stable private listener address, reserved DNS name or matching private IPv4, two unused ports, and reviewed tool paths. The values below are machine-specific examples. Keep the request in memory and disable transcripts. Never put a password in process arguments.

### Authority and trust boundaries

The local deployment operator is trusted to select reviewed native tools. Discovery responses and remote application accounts cannot invoke this command. Rust owns generated secrets, credential validation, database provisioning, migrations, configuration, first-owner relationships, and mandatory product audit. The Windows adapter owns private-directory permissions, certificate API use, native process invocation, and PostgreSQL process startup and stop.

Protected assets are the first-owner credential, application and administrator database credentials, token key, CA signing key, leaf key, and new product state. Input is bounded to 16 KiB, unknown fields and unsupported versions are refused, account validation precedes native changes, and paths reject UNC input and reparse points. The new root must not exist. Child tools receive private input instead of secret arguments; inherited libpq environment settings are removed. Native output is discarded, administrator SQL statement logging is disabled, and errors contain stable identifiers. The completion marker is written only after PostgreSQL stops.

The command is not a sandbox for untrusted scripts or binaries. Malicious same-account or administrator processes can read process memory or change selected tools; authenticate the offline bundle before execution. A native failure, disk failure, or forced termination can leave incomplete new state. Atomic product initialization does not make filesystem and PostgreSQL cluster creation one transaction. Preserve incomplete state for operator diagnosis and keep the normal service installer's additional protected-parent and virtual-account checks.

## Prepare the new state

From the extracted server package in PowerShell 7.4 or later:

```powershell
$ownerPassword = Read-Host 'Owner password (at least 12 characters)' -AsSecureString
$setupEncoding = $OutputEncoding
$request = $null
try {
    $request = @{
        version = 1
        stateDirectory = 'C:\ProgramData\EitmadPrepared'
        platformScript = (Resolve-Path '.\deploy\windows\prepare-workshop.ps1').Path
        postgresBin = 'C:\ReviewedOfflineTools\PostgreSQL17\bin'
        certificateExecutable = (Resolve-Path '.\tools\lan-certificate\bin\Release\net8.0-windows\Eitmad.LanCertificate.exe').Path
        serverName = 'eitmad.workshop.test'
        listenAddress = '192.168.10.20'
        serverPort = 8443
        databasePort = 55432
        workshop = @{
            version = 1
            workshopName = Read-Host 'Workshop name'
            ownerUsername = Read-Host 'Owner username'
            ownerPassword = [System.Net.NetworkCredential]::new('', $ownerPassword).Password
        }
    }
    $OutputEncoding = [System.Text.UTF8Encoding]::new($false)
    $request | ConvertTo-Json -Depth 4 -Compress | & .\eitmad-server.exe prepare-windows-workshop
    if ($LASTEXITCODE -ne 0) {
        throw 'Preparation failed. Preserve the new state and inspect the stable error identifier.'
    }
} finally {
    if ($request) {
        $request.workshop.ownerPassword = $null
    }
    $ownerPassword.Dispose()
    $OutputEncoding = $setupEncoding
}
```

Success prints only the endpoint and public workshop identifiers. The Rust contract is `PrepareWindowsWorkshop` in `crates/contracts/src/setup.rs`, with bounded private JSON input, strict version-one validation, and unknown-field rejection. No ambient server configuration is required. The host composes the existing migrations and identity authority. The first-owner writes and audit remain atomic; see [first-owner initialization](run-server-authority.md#bootstrap-the-first-owner).

Rust disables SQL statement logging on its temporary administrator connection. Generated credentials and the CA password are retained under the private preparation root before product initialization. The native certificate adapter restricts this root to the current Windows account and SYSTEM. Protect it after preparation and include it in encrypted recovery: it contains CA renewal material, administrator recovery credentials, and the stopped cluster. Transfer only the public `trusted-ca.pem` and authenticated enrollment coordinates to clients. Preparation creates an empty update catalog with no signing private key or automatic-update assignment.

## Import into Windows services

The prepared root is the service installer's `LanDirectory`; its `cluster` child is `ClusterDirectory`. Use a **different new** `StateDirectory` for service installation. Retain the preparation root after import. The existing service adapter verifies the stopped primary cluster and copies it; it never uses the development database. Follow [install and operate](run-windows-host-services.md#install-and-operate), including package verification, reviewed WinSW input, and validated HTTPS readiness. Use the same PostgreSQL distribution that created the cluster.

Do not repeat first-owner initialization after import. Use the returned public identifiers to [enroll the server desktop and each client](run-server-authority.md#enroll-or-recover-a-windows-desktop), then use Users to create staff accounts.

## Recover from failure

An existing preparation directory always fails without replacement, including after an incomplete run. Invalid input fails before native creation. Native and database failures attempt to stop the new cluster and leave its files for diagnosis. If termination or power loss interrupts cleanup, inspect the retained cluster and stop only that cluster through reviewed PostgreSQL tools before service import. Do not delete a PID file or reset existing data.

A missing `prepared-workshop.json` means preparation did not finish. Retained credentials, when present, permit an authorized operator to inspect the tenant and audit state before a recovery decision. Failure before credential retention can leave a new incomplete native cluster without product data. Preserve it for diagnosis; this command provides no automatic resume or replacement path. Do not treat an uncertain result as a request to initialize again.

## Verify with isolated synthetic data

Build `eitmad-server` with Rust 1.85.1 and the LAN certificate tool in Release. Use a **new** evidence directory:

```powershell
python deploy/windows/test-workshop-preparation.py --postgres-bin C:\ReviewedOfflineTools\PostgreSQL17\bin --server target/toolchain-1.85.1/debug/eitmad-server.exe --certificate-executable tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe --root artifacts/fresh-workshop-UNIQUE
```

The test invokes the real preparation CLI with synthetic Arabic names and credentials. It checks rejection before native changes, fresh cluster creation, secret omission, stopped completion, refused reinitialization, validated HTTPS readiness, and first-owner sign-in. It starts only loopback listeners in its new cluster and stops its own processes. It installs no service and changes no firewall rule. This is not clean-machine offline installation, automatic startup, backup restore, native onboarding, or second-PC proof.

On 2026-10-20, this isolated executable proof passed with strict OpenSSL certificate verification. The certificate lifecycle tests, affected Rust strict Clippy, generated-contract checks, C# contract conformance, and documentation audit also passed. Existing installed services and databases were not used.

Next, use [Windows services](run-windows-host-services.md) or return to the [operations index](index.md).
