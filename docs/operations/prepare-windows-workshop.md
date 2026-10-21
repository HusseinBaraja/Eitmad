---
title: "Install or prepare a Windows workshop"
description: "Run Rust-owned Server or per-user Client installation, inspect retained progress, or prepare a fresh workshop separately."
audience: "operations"
page_type: "task"
status: "active"
owner: "server and Windows deployment maintainers"
last_verified: "2026-10-21"
review_triggers:
  - "fresh setup contracts, PostgreSQL preparation, credentials, certificates, or service import changes"
keywords:
  - "prepare-windows-workshop"
  - "offline setup"
  - "first owner"
  - "fresh PostgreSQL cluster"
  - "install-windows"
  - "interrupted installation"
---

# Install or prepare a Windows workshop

`eitmad-server install-windows` accepts a typed Server or Client request on private standard input and emits version-one JSON progress lines. Server installation prepares a new workshop, applies migrations, creates the active Owner/Manager and first branch, installs the existing services, configures Private-network `LocalSubnet` HTTPS access, verifies HTTPS readiness, then registers the desktop for the initiating Windows user. Client installation registers only the desktop package for that user, without elevation or server installation.

`eitmad-server prepare-windows-workshop` prepares a **new** Windows state directory using reviewed offline tools. It creates a private CA and matching leaf certificate, an isolated PostgreSQL 17 cluster with SCRAM authentication, separate random administrator and application credentials, an application database, migrations, and the active Owner/Manager and branch. PostgreSQL binds only to `127.0.0.1`. The command stops this cluster before it returns public enrollment coordinates and writes `prepared-workshop.json`.

The separate `prepare-windows-workshop` command installs no service, firewall rule, desktop, or scheduled backup. The native `EitmadSetup.exe` now composes the installation backend, Rust [discovery](../developer/subsystems/server-discovery.md), authenticated pairing, and automatic local backup scheduling. [ADR-0030](../decisions/0030-offline-guided-windows-setup.md) defines the selected user experience.

## Use native Arabic setup

Use the complete extracted [offline distribution](package-windows-desktop.md#prepare-the-combined-offline-payload). Setup checks its trusted, timestamped `offline.cat` and retains the source files before it starts Rust. Missing, changed, unsigned, or differently published payloads stop setup. Server elevation verifies a new administrator/SYSTEM-only copy before it runs any prerequisite or backend from that copy. Rust binds the elevated tool paths to the authenticated metadata. Client keeps per-user MSIX registration and runs no Server prerequisite installer. Preserve the protected `%ProgramData%/EitmadSetupPayload-<id>` tree: scheduled backup execution can reference it.

Launch `setup/EitmadSetup.exe` as the intended normal Windows user. Choose **خادم الورشة ومعه تطبيق الموظف** for a new host or **جهاز موظف** for a client. Do not run development setup against an installed workshop. The WPF project is `shells/windows-setup/Eitmad.WindowsSetup.csproj`; its executable uses the existing native theme and controls.

Server asks for the workshop name and Owner credentials once. Rust validates the reviewed `setup-bundle.json` inputs, derives this computer's DNS name and private IPv4 address, and selects the existing protected ProgramData installation paths. It refuses existing journals and state. Keep the Windows computer name resolvable on the workshop LAN. A missing private IPv4 or invalid computer name stops preflight; correct Windows network or naming settings before retrying only when **فحص حالة الإعداد** confirms that no journal exists.

Rust refuses a nonempty existing desktop data directory before opening a store, then creates the independent desktop key under the initiating user's normal engine directory. The elevated worker uses the existing first-desktop claim, verifies the Owner and key proof, and returns public trust and scope over the existing peer-PID-verified local pipe. The original process performs verified TLS enrollment under the initiating user's SID, then registers the same desktop MSIX used by clients. A different UAC administrator does not receive that user's device key, desktop registration, or connection settings. Setup does not open an application session; the Owner signs in in the normal desktop.

After HTTPS readiness, the elevated worker requests a fresh proof over the same private channel. The original user's Rust process signs it with the retained device key and refuses a changed device ID or public key. Only the proof crosses to the worker; the private key stays with the user. The existing five-minute proof limit, Owner authorization, first-desktop claim, and atomic audit still apply. A long UAC prompt or installation does not use up the enrollment proof's lifetime. If enrollment still fails, preserve the installation and key and use the first-desktop recovery procedure. Public handoff frames are bounded to 32 KiB and never enter the progress journal. Private input frames are bounded to 64 KiB; the worker reads the installation request, then the fresh proof, as separate JSON values. Passwords remain private stdin/pipe input and are not process arguments or progress text.

Server also offers a local backup folder and an optional external folder. An empty local selection uses `ProgramData/EitmadBackups`. Rust configures the existing backup backend for a daily capture and seven retained generations, registers its existing scheduled task, and requires a successful first local capture and any selected external copy before setup completes. Destination checks still require protected local NTFS folders and encrypted volumes; an external copy must be on a verified separate physical disk. A same-disk backup does not protect against disk loss. See [automatic local server backups](backup-lan-installation.md#automatic-local-server-backups), including restore verification. Setup does not claim restore acceptance.

Client installs only the per-user desktop. Open **الاعتماد**, select the discovered server, and enter the code from the Owner's **المستخدمون** → **ربط جهاز** screen. Discovery is untrusted; Rust authenticates pairing and retains this device's key and trust. Pairing does not sign in or create a staff account. Each employee then signs in with an individual account. The Owner uses **إدارة حسابات الموظفين** to return to the existing Users workflow. Codes are transient, expire in three minutes, and are cleared when the screen closes, the engine connection is lost, or the deadline passes. Do not include code screens in screenshots.

**فحص إعداد الاتصال** on sign-in checks Rust's retained connection state and offers discovery when enrollment is absent. A saved connection is not proof of live access. Sign-in and reconnect still require the [live server authority](../developer/subsystems/server-authority.md). Failure preserves keys, saved work, and trust; the shell cannot replace a connection silently.

For an interrupted pairing, choose **إعادة محاولة الربط نفسه** first. The saved completion needs no code and can recover a lost receipt even after the code expires. If the unfinished exchange expired or you choose to abandon it, select **ترك الطلب واستخدام رمز جديد**, then obtain a fresh code from the Owner. The new code field appears only after Rust confirms abandonment. This action preserves the device key, local data, and saved trust. It does not cancel an approval already committed on the server. If the receipt of a committed approval was abandoned, stop and preserve the runtime directory; do not delete keys or create a replacement device to force another pairing.

### Interrupted native setup

The window prevents ordinary close while installation runs. After process termination or power loss, reopen setup and use **فحص حالة الإعداد**. It reads the existing Rust journal and displays the last durable stage, failure, or active worker. Rust offers **فحص واستكمال الإعداد** only for a retained retryable attempt. Server recovery requests the current Owner password again; Client recovery needs no password or elevation. Use the same authenticated bundle and the original normal Windows user. Setup retains credential-free inputs, release pins, paths, scope, and the first device public key. It rechecks Windows registration and elevated host state before continuing. Readiness alone is not completion. A ready host can remain when backup, first-desktop enrollment, or MSIX registration fails; do not choose new paths to bypass that failure.

### Development proof and remaining acceptance

Use [offline Windows setup verification](verify-offline-windows-setup.md) for the repeatable isolated runner and the separate clean-PC procedure. Fresh preparation must reject a valid Owner password on an unapproved device with HTTP 401; it must not bypass first-desktop approval or independent pairing. Owner account usability and paired-client behavior are checked at their existing Rust boundaries. Preparation readiness alone does not establish first-desktop enrollment or whole-installer completion.

The focused `delayed_server_installation_refreshes_proof_and_enrolls_the_same_device` regression requires a fresh disposable `EITMAD_FIRST_DESKTOP_TEST_DATABASE_URL` owned by a non-superuser role without RLS bypass. It rejects a correctly signed proof aged by six simulated minutes, requests a fresh proof after host readiness, then verifies enrollment, same-device retry after a lost reply, sign-in, wrong-password and other-device denial, and denial/success audit records. Run it with `rustup run 1.85.1 cargo test --locked -p eitmad-server --bin eitmad-server delayed_server_installation_refreshes_proof_and_enrolls_the_same_device -- --ignored`. `deploy/windows/test-installation.ps1` checks the private refresh exchange without UAC or installation. These checks do not establish clean-PC installation or different-account UAC acceptance.

Use synthetic data and doubles for the native setup and discovery screens. `OnboardingRenderedTests` covers roles, scrollable forms, keyboard traversal, Arabic accessible names, pairing rejection, Owner denial, reconnect presentation, and interrupted progress without installing anything. The existing control-level system-color path was rendered; this does not verify Windows high-contrast mode. The development display is 1920 × 1080 physical pixels at 125% (1536 × 864 DIP). The rendered maximized window is approximately 1550.4 × 830.4 DIP, the default is 1338.4 × 752.8 DIP, and the minimum is 720 × 560 DIP. Exact 100% baseline captures remain **Needs verification**; no display setting was changed. Recovery screen checks on 2026-10-21 passed at those actual dimensions: Owner-password retry, Client retry, active-worker wait, and blocked inspection. Keyboard traversal and Arabic accessible names were checked for the recovery controls. These captures use synthetic state and do not prove UAC or MSIX installation.

The real `desktop_enrollment_recovers_interruption_expiry_and_credential_loss` test passed against a fresh disposable PostgreSQL cluster and temporary desktop directories with `EITMAD_DESKTOP_ENROLLMENT_ENGINE` set. It includes the Windows bridge/IPC, paired sign-in without manual settings, interrupted persistence, server restart, credential recovery, and Owner-code authorization before sign-in, during the live Owner session, and after sign-out. No installed service, product database, or system trust store was used. Windows high-contrast mode and clean-machine end-to-end setup, different-account UAC, signed MSIX, offline runtime prerequisites, first-desktop setup enrollment, scheduled capture, restore, and independent-client reconnect remain isolated-VM acceptance requirements.

## Before you start

Use a local deployment operator who can create the intended protected directory. Service installation still requires administrator rights. Prerequisites are reviewed PostgreSQL 17 `bin`, `lib`, and `share` directories, the self-contained certificate executable in the server package, Windows PowerShell 5.1, packaged `deploy/windows/prepare-workshop.ps1` beside `host.psm1`, and a new local state path with an existing parent. Verify the package and tools through the [package procedure](package-windows-desktop.md) first. This backend does not authenticate an arbitrary script or vendor binary supplied by an operator.

Use the actual stable private listener address, reserved DNS name or matching private IPv4, two unused ports, and reviewed tool paths. The values below are machine-specific examples. Keep the request in memory and disable transcripts. Never put a password in process arguments.

### Authority and trust boundaries

The local deployment operator is trusted to select reviewed native tools. Discovery responses and remote application accounts cannot invoke this command. Rust owns generated secrets, credential validation, database provisioning, migrations, configuration, first-owner relationships, and mandatory product audit. The Windows adapter owns private-directory permissions, certificate API use, native process invocation, and PostgreSQL process startup and stop.

Protected assets are the first-owner credential, application and administrator database credentials, token key, CA signing key, leaf key, and new product state. Input is bounded to 16 KiB, unknown fields and unsupported versions are refused, account validation precedes native changes, and paths reject UNC input and reparse points. The new root must not exist. Child tools receive private input instead of secret arguments; inherited libpq environment settings are removed. Native output is discarded, administrator SQL statement logging is disabled, and errors contain stable identifiers. The completion marker is written only after PostgreSQL stops.

The command is not a sandbox for untrusted scripts or binaries. Malicious same-account or administrator processes can read process memory or change selected tools; authenticate the offline bundle before execution. A native failure, disk failure, or forced termination can leave incomplete new state. Atomic product initialization does not make filesystem and PostgreSQL cluster creation one transaction. Preserve incomplete state for operator diagnosis and keep the normal service installer's additional protected-parent and virtual-account checks.

## Install Server or Client through Rust

Run from the intended user's **non-elevated** process. Use the authenticated extracted bundle and the existing [Windows package prerequisites](package-windows-desktop.md). The command currently runs from `server/application/eitmad-server.exe` for either role; Client executes this backend but installs no server payload. For the guided interface, use `setup/EitmadSetup.exe` from the complete reviewed offline bundle. Keep the extracted bundle available to the administrator selected in UAC. Use the reviewed PostgreSQL and WinSW inputs from that bundle. This command does not install missing system runtimes.

The authority is `server/host/src/windows_setup/installation.rs`; native mechanics are in packaged `deploy/windows/installation.ps1`, `installation.psm1`, and `installation-channel.cs`. Rust contracts in `crates/contracts/src/setup.rs` generate native bindings. The local request is bounded to 64 KiB and rejects unknown fields and unsupported versions. It is a private local deployment interface, not a remotely callable application command. The initiating user selects the installation; Windows authorizes elevated OS changes. Application Owner/Manager relationships and audit still use the existing [atomic first-owner authority](run-server-authority.md#bootstrap-the-first-owner).

### Client request

In PowerShell 7.4 or later, replace the public paths and authenticated release values for the reviewed package. Do not run this development example on an existing installation.

```powershell
$bundle = 'C:\ReviewedOfflineBundle'
$desktop = @{
    directory = Join-Path $bundle 'client/desktop'
    releaseSha256 = Read-Host 'Authenticated desktop release SHA-256'
    publisher = Read-Host 'Approved publisher'
}
$client = @{
    version = 1
    progressDirectory = Join-Path $env:LOCALAPPDATA 'EitmadSetupProgress'
    intendedUserSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    desktop = $desktop
}
$request = @{
    kind = 'client'
    payload = $client
}
$request | ConvertTo-Json -Depth 8 -Compress |
    & (Join-Path $bundle 'server/application/eitmad-server.exe') install-windows
if ($LASTEXITCODE -ne 0) {
    throw 'Installation failed. Preserve progress and existing data.'
}
```

Client verifies the signed package, refuses an installed desktop, and calls the existing `install-client.ps1 Install` registration. It checks the current SID and elevation status during preflight and again before registration. An elevated process or another Windows user is refused. Existing engine data and credentials are retained. Client makes no service, firewall, certificate-store, or database change.

### Server request and different-account elevation

Use the same `$bundle` and `$client` values above, without executing the Client request first. All four state/progress directories must be different new local paths with existing parents. The three server directories require protected, administrator-owned ancestors. This example uses `ProgramData`; the initiating user's progress remains in their own profile.

```powershell
$application = Join-Path $bundle 'server/application'
$password = Read-Host 'New application Owner password' -AsSecureString
$workshop = @{
    version = 1
    workshopName = Read-Host 'Workshop name'
    ownerUsername = Read-Host 'Owner username'
    ownerPassword = [Net.NetworkCredential]::new('', $password).Password
}
$server = @{
    version = 1
    client = $client
    server = @{
        directory = $application
        releaseSha256 = Read-Host 'Authenticated server release SHA-256'
        publisher = $desktop.publisher
    }
    preparation = @{
        version = 1
        stateDirectory = 'C:\ProgramData\EitmadPreparedWorkshop'
        platformScript = Join-Path $application 'deploy/windows/prepare-workshop.ps1'
        postgresBin = Join-Path $bundle 'server/postgresql/bin'
        certificateExecutable = Join-Path $application 'tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe'
        serverName = Read-Host 'Reserved server DNS name'
        listenAddress = Read-Host 'Stable private IPv4 address'
        serverPort = 58443
        databasePort = 55439
        workshop = $workshop
    }
    serviceStateDirectory = 'C:\ProgramData\EitmadHost'
    serviceName = 'Eitmad'
    serverProgressDirectory = 'C:\ProgramData\EitmadServerSetupProgress'
    winswExecutable = Join-Path $bundle 'server/winsw/WinSW.NET461.exe'
    winswSha256 = Read-Host 'Authenticated WinSW SHA-256'
}
try {
    $request = @{
        kind = 'server'
        payload = $server
    }
    $request | ConvertTo-Json -Depth 8 -Compress |
        & (Join-Path $application 'eitmad-server.exe') install-windows
    if ($LASTEXITCODE -ne 0) {
        throw 'Installation failed. Preserve all state and inspect the failed stage.'
    }
} finally {
    $workshop.ownerPassword = $null
    $password.Dispose()
    $server = $null
}
```

Disable transcripts and keep the private JSON in memory. The Windows adapter creates an ACL-restricted, local pipe before requesting UAC. It denies network identities and permits the initiating user, Administrators, and SYSTEM. Both ends verify peer process IDs; the elevated client uses anonymous impersonation level. Only public executable, pipe, and process coordinates enter arguments. Credentials travel through the pipe and child standard input. Native output and OS error text are withheld. See the Windows [client PID](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid) and [server PID](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeserverprocessid) APIs.

UAC can use a different administrator account. That worker runs only `install-windows-server`: preparation, migrations, first-owner initialization, stopped-cluster import, service installation, Private-only `LocalSubnet` firewall configuration, and HTTPS `/readyz`. It never registers the desktop. The original non-elevated process waits for verified server completion, then registers the desktop under its own SID. Rust maps elevated completion to successful HTTPS verification; overall `complete/succeeded` is emitted only after desktop registration also succeeds. During service installation, Rust also enables [unattended TLS renewal](setup-lan-server.md#renew-without-changing-trust) using the retained CA and generated unlock credential. It copies the certificate tool/runtime into protected persistent storage and registers an hourly SYSTEM check without secrets in task arguments. The existing service adapter validates the CA chain, certificate name and expiry, and HTTP 204 readiness without disabling TLS validation. PostgreSQL remains loopback-only. Unrelated firewall rules can still grant independent access; inspect them during deployment acceptance.

### Progress, interruption, and recovery

New service installations listen on IPv4 wildcard and add executable-scoped Private + LocalSubnet rules for the configured HTTPS port and UDP 58444. PostgreSQL remains loopback-only. Rust [server discovery](../developer/subsystems/server-discovery.md) preserves an installation UUID and returns typed, untrusted candidates. It cannot install trust or enroll a device. The native desktop presents candidates and submits the Owner code to Rust authenticated pairing. Development tests do not update the installed service or effective firewall policy.

Each progress line has `version`, increasing `sequence`, `stage`, and `status`. The contract defines the exact stage/status values. Rust writes each record with exclusive creation and flushes it before emitting it. Records contain no credentials, account input, private paths, or native error text. The original-user journal contains the combined flow; the separate server journal contains elevated stages. A failed stage emits `failed` and prevents later stages. A server that is ready can remain installed if desktop registration fails; do not initialize it again.

Inspect retained progress without changing an installation:

```powershell
$statusRequest = @{
    version = 1
    progressDirectory = $client.progressDirectory
}
$statusRequest | ConvertTo-Json -Compress |
    & (Join-Path $bundle 'server/application/eitmad-server.exe') windows-install-status
```

The result has `version`, the last durable `progress`, and `interrupted`. An exclusive live lock distinguishes a running process from a terminated process. Without a live lock, a journal is interrupted unless it ends with a known failure or overall `complete/succeeded`. Successful HTTPS verification alone is still incomplete. A torn final record falls back to the preceding valid record. The reader never promotes an uncertain stage to success. Query the server progress directory under the selected administrator account when the parent channel or process was lost; its worker can still be running.

Recovery is an explicit action, not an automatic background retry. Rust reopens the existing exclusive journal and appends records after the highest existing sequence, including a torn record. Each process starts its wire sequence at one. An active worker returns `wait`; a damaged or missing recovery input returns `inspectManually`. Passwords and device proofs are never retained in `recovery-input.json`. A changed Windows user, release, destination, workshop, or device binding is refused.

| Retained stage or observation | Safe recovery action |
| --- | --- |
| UAC cancelled or prerequisite interrupted before workshop creation | Retry from the original normal Windows user; approve UAC again and complete any required Windows restart first. |
| Complete preparation with `prepared-workshop.json` and a verified stopped PostgreSQL 17 cluster | Reuse it. Rust does not create another Owner, CA, token key, or cluster. |
| Incomplete preparation, missing marker, or uncertain Owner transaction | Stop for operator inspection. Preserve credentials, cluster, journals, CA material, and audit state. |
| Complete imported host, with zero, one, or two matching services | Reuse the imported database. Verify protected paths, release, CA, token, database route, wrapper digests, service executable and identity. Complete only missing owned service registrations and service configuration. Never import the preparation cluster again. |
| Partial host files, foreign service, changed release or trust, or pending release repair | Stop with `blocked` progress. Use the existing host recovery procedure; setup cannot infer a safe repair. |
| Private network or HTTPS failure | Reapply the existing Private-network rules and require normal certificate chain, name, expiry, and `/readyz` checks. Correct the network, service, or certificate cause before retry. |
| First-desktop approval committed but reply was lost | Reauthenticate the active Owner with a fresh proof from the original user device. Reuse only the same approved account, key, and scope. Different or revoked devices are refused; existing desktop trust must match byte for byte. |
| Backup configuration, schedule, or first capture interrupted | Retain the backup deployment ID and exact destinations. Probe retained generation and schedule state; reuse a verified successful result or retry the missing work. A different host or destination is refused. |
| MSIX registration failed or its reply was lost | Under the original non-elevated user, reuse a healthy registration only when publisher, version, and installed inventory match. Otherwise install the missing package or stop for the separate exact-release repair procedure. |

A `blocked` result disables native retry and directs the operator to inspect state. Preserve all files and use [host status and recovery](run-windows-host-services.md#install-and-operate) or the preparation guidance below. An operator must establish the database and audit result before selecting a repair. Do not erase journals, remove PID files, change paths, replace trust, or run first-owner initialization as a recovery shortcut. Repair and upgrade retain their separate recovery requirements.

### Safe development verification

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server --bin eitmad-server windows_setup
```

```powershell
powershell.exe -NoProfile -File deploy/windows/test-installation.ps1
```

Rust tests exercise Client and Server order, stage failures and retry, cancelled UAC, reuse without Owner initialization, blocked uncertain preparation, torn/interrupted journals, user/release/device binding, unchanged trust, backup deployment binding, input validation, and credential/proof redaction using isolated synthetic files and platform doubles. The native adapter test substitutes service and AppX actions, simulates a different elevated administrator and existing desktop, and tests real local pipe framing, UTF-8, peer PID refusal, and frame bounds. It installs no package, service, or firewall rule. Real UAC with another administrator, signed MSIX registration, and a fresh clean-machine service installation remain **Needs verification** in an isolated Windows VM; these tests do not claim that deployment proof. The separate executable preparation proof below checks real PostgreSQL, first-owner sign-in, and HTTPS without installing services.

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

The standalone preparation command remains fresh-only and refuses an existing directory. Guided installation recovery can reuse its bound preparation only when the public completion marker matches the requested endpoint and native inspection proves a complete, stopped PostgreSQL 17 preparation. An incomplete directory is refused without replacement. Invalid input fails before native creation. Native and database failures attempt to stop the new cluster and leave its files for diagnosis. If termination or power loss interrupts cleanup, inspect the retained cluster and stop only that cluster through reviewed PostgreSQL tools before service import. Do not delete a PID file or reset existing data.

A missing `prepared-workshop.json` means preparation did not finish. Retained credentials, when present, permit an authorized operator to inspect the tenant and audit state before a recovery decision. Failure before credential retention can leave a new incomplete native cluster without product data. Preserve it for diagnosis; this command provides no automatic resume for this uncertain state and no replacement path. Do not treat an uncertain result as a request to initialize again.

## Verify with isolated synthetic data

Build `eitmad-server` with Rust 1.85.1 and the LAN certificate tool in Release. Use a **new** evidence directory:

```powershell
python deploy/windows/test-workshop-preparation.py --postgres-bin C:\ReviewedOfflineTools\PostgreSQL17\bin --server target/toolchain-1.85.1/debug/eitmad-server.exe --certificate-executable tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.exe --root artifacts/fresh-workshop-UNIQUE
```

The test invokes the real preparation CLI with synthetic Arabic names and credentials. It checks rejection before native changes, fresh cluster creation, secret omission, stopped completion, refused reinitialization, validated HTTPS readiness, and first-owner sign-in. It starts only loopback listeners in its new cluster and stops its own processes. It installs no service and changes no firewall rule. This is not clean-machine offline installation, automatic startup, backup restore, native onboarding, or second-PC proof.

On 2026-10-20, this isolated executable proof passed with strict OpenSSL certificate verification. The certificate lifecycle tests, affected Rust strict Clippy, generated-contract checks, C# contract conformance, and documentation audit also passed. Existing installed services and databases were not used.

Next, use [Windows services](run-windows-host-services.md) or return to the [operations index](index.md).
