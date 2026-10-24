---
title: "Verify offline Windows Server and Client setup"
description: "Repeat isolated setup tests and record clean-PC installation, pairing, firewall, reboot, DHCP, and restore evidence."
audience: "operations"
page_type: "task"
status: "active"
owner: "product developer"
last_verified: "2026-10-24"
review_triggers:
  - "offline payload validation, installation stages, network recovery, supported Windows editions, or acceptance results change"
keywords:
  - "offline acceptance"
  - "ordinary-user Client"
  - "Private Public firewall"
  - "boot without sign-in"
  - "interrupted setup recovery"
---

# Verify offline Windows Server and Client setup

Use this procedure to assess one exact offline release. A successful development test does not accept an installer. Keep each failed result and mark checks without observed evidence **Needs verification**. Never run installation, reboot, profile changes, or restore against a development machine's live installation.

The product authority and required outcomes are [ADR-0030](../decisions/0030-offline-guided-windows-setup.md) , [ADR-0031](../decisions/0031-two-self-contained-windows-installers.md), and [ADR-0032](../decisions/0032-self-signed-windows-publisher.md). Acceptance must start with the appropriate EXE and satisfy **"click and run and follow in app guided setup"**. Use [Windows packaging](package-windows-desktop.md) for authenticated release preparation and [native setup](prepare-windows-workshop.md#use-native-arabic-setup) for stage meanings and recovery limits. Server and Client below are **installer roles**. The current MSIX targets `Windows.Desktop`; Windows Server operating-system edition support is not established by a build-number check. If a Windows Server SKU is required, record the exact edition and AppX support and run this procedure there. It remains unverified until it passes on that edition.

## Run isolated development checks

Prerequisites: a source checkout, Rust 1.85.1, .NET 8 SDK, Python 3.11+, Windows PowerShell 5.1, reviewed PostgreSQL 17 test binaries, and at least 5 GiB free. These are development prerequisites; they are not Client installation requirements.

```powershell
$proof = Join-Path $env:TEMP ('eo-' + [Guid]::NewGuid().ToString('N').Substring(0,8))
python deploy/windows/test-offline-setup.py --postgres-bin C:\ReviewedTestTools\pgsql\bin --root $proof
```

The runner reuses `target/toolchain-1.85.1`, selects its compiler explicitly, and creates fresh private test roots and loopback clusters. It refuses an existing root. It runs existing payload, package, installation-channel, service-adapter, discovery, backup-adapter, setup-journal, fresh-preparation, account, TLS, pairing, two-customer-engine, retained-route, and physical PostgreSQL restore tests. It installs no MSIX, service, runtime, task, trust certificate, or firewall rule. Private logs stay in the selected directory. `results.json` lists only completed commands and always has `fullAcceptance: false`; a failure retains prior results and exits nonzero.

After a failure, correct the cause and use a new root with `--from-check <failed-check-name>`. Earlier checks appear under `notRun`, not `passed`. Retain the earlier root as evidence; do not repeat unchanged successful suites. For example, `--from-check fresh-preparation` uses the current previously built binaries and runs preparation and its dependent runtime/restore checks.

Pairing tests exercise the Rust authority and interrupted completion. Address tests move loopback listeners, retain the original TLS name/trust/device, and reject mismatched identity or TLS. These tests do not prove native pairing between PCs or a real DHCP lease change. Backup tests use real data capture, copy, hashing, staging, restored CA renewal and authenticated clients, with substituted administrator/service/disk observations. They do not prove lost-profile recovery, independent encrypted media or unattended boot. The runtime harness uses the documented synthetic certificate verification profile; it still validates CA trust and hostname. Installer readiness uses Windows Schannel with normal name, chain and expiry checks.

Check acceptance-probe verdicts without accessing a network:

```powershell
pwsh.exe -NoProfile -File deploy/windows/test-offline-machine-probes.ps1
```

## Prepare clean offline PCs

1. Use three disposable supported x64 Windows PCs or isolated VMs: **S** for Server, **C1** and **C2** for ordinary-user Client. Use local NTFS and distinct Windows standard users without Administrators membership. Keep separate lab administrator credentials for each PC to approve the one-time publisher trust step and Server service installation. Record Windows edition/build, machine identity, network adapter, scaling, and date. Start each repeat from a clean image, never by deleting product state. Retain failed images as evidence.
2. Copy only `EitmadServerSetup.exe` to S and only `EitmadClientSetup.exe` to C1/C2. Use the exact signed pair and independently authenticated EXE/archive/catalog digests, public certificate SHA-256, and publisher from the release handover. An unsigned development probe, a different diagnostic test identity, or a manually arranged payload cannot pass this acceptance. A rebuilt script or binary requires a new reviewed/signed installer; do not replace files inside the signed catalog.
3. Copy `test-offline-machine.ps1` and `verify-offline.ps1` from the same reviewed source revision into `C:\LabChecks`. Authenticate those files through the handover too. They are lab probes, not installed application prerequisites. Windows PowerShell 5.1 is sufficient. Do not install Python, .NET, C++, an SDK or a compiler on these PCs for this drill.
4. Disconnect the lab router's WAN and all alternate uplinks, retaining the local LAN and its DHCP. Record this physical isolation and the adapters/routes. Do not merely block one website. Begin without the Eitmad publisher certificate trusted. Require the guided public fingerprint confirmation and administrator approval while offline. Keep signature and certificate validation enabled.
5. Open the appropriate EXE as the intended **standard user**. Require the native publisher trust screen. Compare its fingerprint with the independent handover and confirm it. Cancel the Windows prompt once: require no payload execution and a clear retry path. Reopen, confirm, and approve the certificate import as the lab administrator. Inspect that only the expected public code-signing leaf was added to `LocalMachine\Root`, with no private key or TLS CA. Require signature verification and automatic extraction followed by the correct role's native screen. Do not select installation yet. Record that no archive extraction, terminal command, runtime installer, or manual certificate procedure was required from the end user. Record the guided trust UAC separately from the Client installation baseline. Lab observers can now inspect the automatically extracted directory. Set both digests from that role's authenticated handover, not from local files:

```powershell
$approvedCatalog = Read-Host 'Authenticated offline.cat SHA-256'
$approvedPayload = Read-Host 'Authenticated embeddedPayloadSha256'
$labBundle = Join-Path $env:LOCALAPPDATA ('Eitmad\Installers\' + $approvedPayload)
powershell -NoProfile -File C:\LabChecks\test-offline-machine.ps1 -Stage Baseline -EvidenceDirectory C:\LabEvidence -BundleDirectory $labBundle -ExpectedCatalogSha256 $approvedCatalog
```

Baseline verifies the signed catalog and records the user, service names, firewall rule names and global runtime versions. It refuses existing Eitmad state, common development tools and a separate .NET installation. Retain the image inventory as well: PATH checks alone cannot prove every possible SDK is absent. These observations do not by themselves prove offline execution.

## Install the roles and connect independent users

1. On S, continue the Server wizard opened by `EitmadServerSetup.exe`. Enter a synthetic Arabic workshop name and a new synthetic Owner. Choose local backup storage; record the same-disk warning and select a separate supported external destination for the recovery drill. Authorize UAC with the separate lab administrator. Do not start setup from the administrator desktop. Require confirmed completion, then open the desktop under the original standard user. A runtime-requested restart is not completion; preserve the stage and follow the interruption procedure below.
2. On C1 and C2, continue the Client wizard opened by `EitmadClientSetup.exe` under each intended standard user. After the recorded one-time certificate trust approval, require no further UAC or separate runtime installer/download prompt. Require confirmed completion and launch from Start. Do not elevate Client application installation or install a runtime to make this pass. Under the same user run:

```powershell
powershell -NoProfile -File C:\LabChecks\test-offline-machine.ps1 -Stage Client -EvidenceDirectory C:\LabEvidence
```

   Require healthy per-user MSIX, retained self-contained runtime files, no package dependency, no server executable, no service or firewall registration change, no global runtime version change, and no Eitmad ProgramData installation. This compares registrations and known paths; the recorded offline launch is the proof that no separate download was needed.
3. On S, open a lab administrator window and run the Server observation. It checks the two separate automatic virtual-account services, PostgreSQL loopback listeners and Eitmad ActiveStore firewall scope/program/protocol/ports. It does not establish that no other rule grants access.

```powershell
powershell -NoProfile -File C:\LabChecks\test-offline-machine.ps1 -Stage Server -EvidenceDirectory C:\LabEvidence
Get-NetConnectionProfile | Select-Object InterfaceIndex, NetworkCategory
Get-NetFirewallProfile -PolicyStore ActiveStore | Select-Object Name, Enabled, DefaultInboundAction
```

4. As the application Owner on S, use the existing Users workflow to create distinct synthetic staff users with the required branch roles. On C1 discover S, obtain a fresh Owner pairing code through the native pairing control, pair, and sign in as staff. Repeat independently on C2 with a **new** code. Never copy runtimes, credentials or device identities. An expired, reused or wrong code must fail without committing replacement trust. Record only success/failure, not codes, passwords, tokens, device keys or account-response bodies.
5. Under an authorized staff role on C1, create a synthetic customer through the existing workflow. Record its synthetic name and confirmed revision. Require the same confirmed record on C2 in the authorized branch. Check a user denied that scope cannot read it. A local optimistic row or a discovery candidate alone is not confirmation.
6. Close each app using its tray Exit action; reopen from Start and sign in. Require no repeated pairing/settings entry, the same confirmed customer, and another authorized change reaching the other client. Sign out and switch staff accounts; require the previous session's protected content to clear.
7. Copy only the public CA from S through the authenticated handover and compare its DER fingerprint as described in [LAN TLS verification](setup-lan-server.md#enroll-each-pc-independently). Do not install it into the Windows root store. On each client use the canonical retained name, for example:

```powershell
powershell -NoProfile -File C:\LabChecks\test-offline-machine.ps1 -Stage Private -EvidenceDirectory C:\LabEvidence -Endpoint https://LAB-SERVER:58443/ -TrustedCa C:\LabTrust\trusted-ca.pem -ServerAddress 192.168.50.10
```

   Replace the name and address with S's actual values. Require validated `/readyz` HTTP **204** and failed client-to-PostgreSQL TCP. Use the same `curl.exe --disable --noproxy '*' --ssl-revoke-best-effort --cacert <ca> <endpoint>/readyz` with an unrelated CA and then a different name resolving to S; both must fail TLS. Never use `-k`. Keep these negative probes separate from enrolled settings.

## Prove effective firewall, boot and DHCP behavior

1. On the isolated S console, record the actual LAN interface index. Confirm Windows Firewall is enabled. Under its administrator, inspect all enabled inbound Allow rules in `Get-NetFirewallRule -PolicyStore ActiveStore`, including their application, port, address and service filters. Investigate independent broad allow rules. A correct Eitmad rule does not override another rule.
2. With S on Private, run the Private probe above on both clients and perform an authorized change. On S's lab console run `Set-NetConnectionProfile -InterfaceIndex <recorded-index> -NetworkCategory Public`. This is a **lab-only operator action**; the application must never change the profile. Record the actual resulting profile.
3. Keep the services running. On S verify loopback readiness with the original TLS name: `curl.exe --disable --noproxy '*' --ssl-revoke-best-effort --resolve LAB-SERVER:58443:127.0.0.1 --cacert C:\LabTrust\trusted-ca.pem --head https://LAB-SERVER:58443/readyz`. Require HTTP 204. On C1 and C2 repeat the probe with `-Stage Public`. Require failed HTTPS **TCP** and failed PostgreSQL TCP, and no usable discovery candidate after its expiry. A certificate error or stopped server is not proof of firewall denial. Restore only that S interface to its original Private profile and require Private readiness and authorized work again. Record all three observations.
4. Close S's desktop and tray process. From C1 require another confirmed operation on C2. Restart only `EitmadServer` on S using `Restart-Service EitmadServer` under the lab administrator. Require both existing clients to reconnect without pairing or trust replacement and retain accepted records.
5. Arrange an observed reboot window. From S's lab console run `shutdown.exe /r /t 0`. Leave S at the Windows sign-in screen; do not sign in or open an RDP session. On C1 probe every five seconds for up to five minutes, retaining UTC times. Require 204 and an authorized change reaching C2 **before any Windows sign-in on S**. Afterward sign in on S and record service start times, accounts and boot time. Local service configuration alone does not pass this check.
6. Record S/C1/C2 addresses and S's certificate fingerprint. Change the lab DHCP reservation to a different free address, then at the affected PC's physical console use `ipconfig /release` and `ipconfig /renew` on the lab adapter. Repeat separately for a client and S. Confirm that the address actually changed; a renewed identical lease does not exercise this case. Wait for old discovery offers to expire. Require rediscovery by the retained S identity, validated HTTPS with the original TLS name, unchanged CA/device enrollment, normal sign-in, and another cross-client confirmed change. Record reconnect duration. Do not edit connection JSON, copy another device or issue a new pairing code as a workaround.

## Interrupt setup and restore a backup

1. Use separate clean snapshot clones, synthetic data and the same pinned bundle for each interruption. Power off the lab VM through its hypervisor while setup shows preparation, service installation, firewall, HTTPS verification, backup configuration or desktop registration. Do not interrupt the development machine or a real workshop. Preserve the powered-off image before recovery. A physical-PC power-loss drill needs its own disposable machine and operator window.
2. Boot the clone and reopen setup as the original user. Require the last durable failed/interrupted stage and no false completion. Inspect the elevated server journal with `windows-install-status` using [the exact status request](prepare-windows-workshop.md#progress-interruption-and-recovery); the worker may have finished after the parent stopped. Require existing state to be refused without creating another Owner, resetting a cluster, changing CA, or losing accepted records. A torn record must not become success.
3. Use **فحص واستكمال الإعداد** when the inspected state offers it, under the original normal Windows user and with the same authenticated bundle. Supply the current Owner password for Server recovery. This is an explicit, verified continuation; setup does not resume automatically in the background. Require reuse of the same Owner, device, trust, cluster, and destinations. An active worker requires waiting; a blocked state requires operator inspection. Follow the stage-specific [installation recovery](prepare-windows-workshop.md#progress-interruption-and-recovery) and [host recovery](run-windows-host-services.md#install-and-operate). Use packaged `install-server.ps1 Status` with `-StateDirectory C:\ProgramData\EitmadHost -Name Eitmad` before manual host actions. Use `Start` only for a complete owned installation, or the documented original-package `Repair` after its prerequisites pass. An incomplete preparation with no verified repair path remains a failed recovery case; do not delete state or change paths to bypass it. Repeat sign-in, readiness and cross-client work after recovery.
4. Under the lab administrator, inspect `Get-ScheduledTask -TaskName 'EitmadServerBackup-*'` and run the configured capture through the documented [local server backup procedure](backup-lan-installation.md#configure-and-schedule). Confirm a new complete generation and current status. Follow that page's protected destination and encryption requirements. A same-disk generation alone cannot pass disk-loss recovery.
5. On a disconnected replacement lab host, verify and stage the chosen external generation with packaged `deploy/windows/restore-backup.ps1 -ServerLocal -Generation <complete-generation> -PostgresBin <reviewed-pg-bin> -StageDirectory <new-private-stage>`. Require hash/manifest verification, refusal of an existing stage and refusal of a deliberately tampered **copy**. Preserve the original generation. Staging alone is not a working restore.
6. Complete [replacement acceptance](backup-lan-installation.md#restore-an-isolated-replacement-installation) with the retained CA renewal material, original application credentials, PostgreSQL state and client Windows profile/credential dependencies. Reconcile acknowledged and pending synthetic work; require the same accepted revision/audit history, authorized clients and no duplicate effect after retry. Record recovery point age and full recovery time. If Windows/DPAPI or hardware-key recovery was not exercised, mark it **Needs verification**.

## Record the result

For each row retain release/catalog digest, source revision, exact command or UI action, actual machine/edition, UTC time, expected/actual outcome and sanitized evidence location. Keep private journals and synthetic credentials out of reports. Use **Passed**, **Failed**, or **Needs verification**; an available script is not a passed result.

| Required boundary | Evidence required for acceptance |
| --- | --- |
| Clean offline Server and standard-user Client | Real signed catalog, offline launches, Client baseline comparison, one explicit publisher trust approval per PC, no elevated Client app install/download/toolchain, healthy services |
| Discovery, independent pairing and staff authorization | Native flow on distinct devices and accounts, expired/replayed code refusal, authorized change visible on C2 |
| HTTPS and PostgreSQL | Client validated 204; wrong-name/wrong-CA refusal; database inaccessible from both clients |
| Effective profiles | Private success, Public TCP/discovery denial while locally ready, restored Private success; active-policy review |
| Restart, boot and DHCP | Existing clients recover; operation before S Windows sign-in; observed new lease with retained identity/trust |
| Interrupted setup and backup | Preserved stage/state, observed repair, external-generation activation, accepted/pending data and credential recovery |

### Standalone installer implementation evidence, 2026-10-24

The new distribution path uses one shared WPF implementation and Rust-enforced Server/Client metadata. Focused development evidence is retained under `artifacts/two-installers-verification`; it is not clean-PC acceptance.

- The adapter extraction suite passed 13 cases: archive digest rejection, traversal, duplicate names, reserved paths, links, changed or unexpected retained content, retained file handles, and unsigned EXE rejection. Windows trust also accepted the installed signed .NET host in a separate positive probe.
- `OnboardingRenderedTests` passed 17 cases, including the Server, Client, preparation, and recovery screens. The affected screens were captured and inspected at the maximized, default, and minimum sizes on the existing 1920 × 1080 physical display at 125%. Actual dimensions were approximately 1550.4 × 830.4, 1338.4 × 752.8, and 720 × 560 DIP. Exact 100% baselines remain **Needs verification**. No display setting was changed.
- Single-file publishing produced one self-contained EXE for each role with a synthetic archive, without loose runtime files. These unsigned probes contain no installable product payload and are not release artifacts.
- Focused Rust setup tests, the role-specific payload builder tests, generated-contract verification, and C# conformance cover the changed boundaries. The fresh-database integration test retains its explicit prerequisite and was not run as part of this change.

No approved signing identity or complete newly reviewed signed component set was available. The final installer pair, offline standard-user installation, two-device native LAN use, and unattended Server reboot acceptance remain **Needs verification**. Signing and integrity checks remain enabled.

### Observed development evidence, 2026-10-21

The following checks ran on the development Windows host with synthetic data. No live installation was changed and no branch was created. The runner was resumed at failed checks with fresh roots; unchanged successful suites were retained. None of these results is whole-installer acceptance.

| Check actually run | Observed result and limit |
| --- | --- |
| `python -m unittest discover -s scripts/ci/tests -p test_offline_payload.py -v` | 17 tests passed, including real Windows rejection of unsigned synthetic signatures/catalogs. Final expanded Client-policy cases also passed in a focused rerun |
| `pwsh.exe -NoProfile -File deploy/windows/test-package.ps1` | Passed inventory, app-local C++, self-contained .NET, external-package/server-tool refusal and migration checks with synthetic packages |
| `test-installation.ps1`, `test-host.ps1`, `test-discovery-interfaces.ps1`, `test-local-backup.ps1` through Windows PowerShell | Passed existing isolated adapter/channel, mocked SCM/firewall/profile and local-backup checks; no real service, task or firewall installation |
| `rustup run 1.85.1 cargo test --locked -p eitmad-server --bin eitmad-server windows_setup` | 10 setup and journal tests passed, including retained failed/interrupted/torn state and Client-only stages |
| `rustup run 1.85.1 cargo test --locked -p eitmad-server-discovery` | 11 tests passed, including loopback UDP; no physical LAN discovery proof |
| `dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter FullyQualifiedName~OfflineSetupPayloadTests` | One launcher rejection test passed. A first build attempt hit a file lock; serial rerun passed without warnings. No native UI rendering was performed |
| Runner `fresh-preparation` | Real fresh PostgreSQL 17/SCRAM, migrations and retained setup, validated HTTPS 204 and HTTP 401 for an unapproved Owner device passed. The old expectation of HTTP 200 was corrected; account usability is tested separately |
| Runner `tls-accounts-pairing-two-clients` | Account/staff authorization and audit rollback, pairing rejection/interrupted completion, real TLS sync, two isolated customer engines, host restart and retained-identity loopback address recovery passed |
| Runner `backup-restore` with `--server-backup` | Real capture/copy/staging, restored CA renewal, new PostgreSQL cluster, two recovered clients, rowsets/image bytes, pending work confirmation and zero duplicate order effects passed. Recovery interval 115,147 ms; whole command 135.216 s. Privilege/service/disk observations were substituted; Windows image and lost-profile credentials were not recovered |
| `pwsh.exe -NoProfile -File deploy/windows/test-offline-machine-probes.ps1` | Synthetic verdict tests passed for valid 204, readiness/TLS failures, database exposure, and TCP versus certificate failure on Public |
| Existing `eitmad-desktop-0.1.0-offline.1-win-x64-unsigned.zip`, current `Read-EitmadPackage -AllowUnsignedValidation` and `Assert-DesktopRuntime` | Static inventory/runtime checks passed without package registration. This older unsigned component is not the finished signed offline bundle |
| `python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs` and focused Python compilation | Documentation audit passed for 110 pages; changed Python harnesses compiled |

Private results are retained in Windows TEMP under `eitmad-offline-acceptance-20261021-c` (adapter/build/discovery results), `eitmad-offline-acceptance-20261021-d` (corrected preparation and runtime; its restore attempt failed the path-length guard), and `eo-20261021-e` (successful restore). The runner now refuses overly long roots before starting tests. Package-only evidence is under `eo-msix-6dba9014`. Keep these private; publish only sanitized summaries. Full evidence-root paths are environment-specific.

A complete trusted signed offline bundle was not available. Clean offline Server/Client installation, actual standard-user launch without prerequisites, native pairing/staff sign-in on separate PCs, effective Private/Public access, boot without sign-in, physical DHCP and replacement-machine credential/media recovery remain **Needs verification**. No clean-PC, real firewall, reboot or physical DHCP acceptance was performed.

Return to [operations](index.md), [package preparation](package-windows-desktop.md), or [backup recovery](backup-lan-installation.md).
