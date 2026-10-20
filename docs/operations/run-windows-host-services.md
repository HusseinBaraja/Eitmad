---
title: "Run the Windows host without an interactive login"
description: "Install, start, stop, inspect, recover, and uninstall persistent PostgreSQL and Rust Windows services."
audience: "operations"
page_type: "task"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-20"
review_triggers:
  - "Windows service installation, service identities, readiness, recovery, or host power requirements change"
keywords:
  - "run-services.ps1"
  - "Windows service"
  - "unattended PostgreSQL"
  - "WinSW"
  - "reboot readiness"
---

# Run the Windows host without an interactive login

`run-services.ps1` manages installed Windows services. WinSW 2.12.0 hosts the PostgreSQL postmaster and the separate Rust console process, with a separate wrapper and identity for each component. Windows starts both services without WPF, a terminal, or an interactive login. The Manager window and its supervised local engine do not own these services.

The implementation is in `deploy/windows/host.psm1`. Rust owns application validation, authentication, authorization, audit, migrations, and readiness. Use the [versioned Windows installation package](package-windows-desktop.md) to deploy without a checkout or compiler. Its `install-server.ps1` adds verified binary inventory, repair, and controlled manual upgrade. The installer copies existing provisioned state; it does not bootstrap accounts, seed records, or reset a database. The [workflow environment](run-workflow-test-environment.md) remains a separate development tool.

## Prerequisites and safe cutover

For a new workshop without a prepared database, use the [fresh Windows preparation backend](prepare-windows-workshop.md). It creates a separate stopped cluster, TLS material, runtime settings, and active first Owner. This service adapter then imports that prepared state through the procedure below.

- Use a supported Windows host with NTFS, Windows PowerShell 5.1 or later, `curl.exe`, and .NET Framework 4.8 for WinSW. Run each service command in an elevated PowerShell window. Runtime services need neither PowerShell nor the repository after installation.
- Complete [LAN TLS setup](setup-lan-server.md), including stable DNS/address, the saved `server.json`, certificate pair, and verified public CA. Preserve the original token key and client trust. Supply the saved LAN directory as `-LanDirectory`.
- Supply trusted PostgreSQL 17 binaries as `-PostgresBin` and a dedicated, provisioned PostgreSQL 17 primary cluster as `-ClusterDirectory`. Use a SCRAM application role with `NOSUPERUSER`, `NOBYPASSRLS`, `NOCREATEDB`, and `NOCREATEROLE`. The current runtime still applies migrations during startup; follow [migration and backup requirements](deploy-server-environments.md#deploy-and-migrate).
- Obtain `WinSW.NET461.exe` from the [official WinSW 2.12.0 release](https://github.com/winsw/winsw/releases/tag/v2.12.0). Use the host's patched .NET Framework 4.8. Verify provenance and retain its reviewed SHA-256 through an authenticated release channel. Pass that value as `-WinSwSha256`. The installer checks the digest and product version. It downloads no executable and does not execute a mismatched wrapper. Do not use a hash supplied by an unrelated download mirror.
- Back up the source database, manifests, settings, and TLS state. Stop the foreground Rust server and stop the source PostgreSQL cluster cleanly through their owner. A remaining `postmaster.pid`, recovery signal, reparse point, or external tablespace blocks import. Do not delete those files to bypass the check.
- Keep all source directories separate from the destination. Use a new local destination outside the repository and build cache, with administrator-owned ancestors that do not permit ordinary users to replace child directories. The default ProgramData location is intended for this. Review PostgreSQL configuration for absolute include, archive, recovery, extension, or external-file paths before relocation. The installer overrides the data, HBA, ident, listen-address, and port settings for the installed cluster; it does not rewrite arbitrary external paths. Remove broad HBA rules through normal database administration before cutover.
- Reserve enough free space for a full copy of the cluster, PostgreSQL binaries, manifests, and backup growth. Installation retains source data. After cutover, keep the source server stopped so it cannot accept divergent writes.

The installer copies `bin`, `lib`, and `share` from the PostgreSQL distribution, the saved Rust executable, the stopped cluster, manifests, and the current leaf certificate/key and public CA. It does not copy the CA signing key. Original LAN certificate state stays under its operator account for renewal.

## Install and operate

Run from the repository root. Replace the paths and the digest variable with reviewed local inputs; no secret belongs in this command:

```powershell
.\run-services.ps1 Install -LanDirectory C:\ProgramData\EitmadLan -ClusterDirectory C:\PostgresSource\data -PostgresBin C:\PostgreSQL17\bin -WinSwExecutable C:\Installers\WinSW.NET461.exe -WinSwSha256 $reviewedWinSwSha256
```

The default destination is `C:\ProgramData\EitmadHost`. Services are `EitmadPostgres` and `EitmadServer`. For an isolated acceptance host, use `-Name EitmadAcceptance -StateDirectory C:\ProgramData\EitmadAcceptance` on **every** command. Install first registers demand-start services, applies identities, ACLs, dependency and failure policy, then enables PostgreSQL automatic startup and delayed automatic server startup. It does not report application readiness or start a Manager app.

```powershell
.\run-services.ps1 Start
```

Start checks for occupied ports before starting a stopped component, starts PostgreSQL before Rust, then waits up to 150 seconds for verified HTTPS `/readyz` HTTP 204. The command validates the endpoint name through the retained public CA with `curl --cacert`; it does not install a CA or use `-k`. It ignores user curl configuration and proxy settings. Schannel uses [best-effort revocation](https://curl.se/docs/manpage.html#--ssl-revoke-best-effort) because the private LAN CA has no online revocation service; chain, hostname, expiry, and known-revocation checks remain active. A running service alone is not success. Existing running services are reused. A failure retains state and service registrations for diagnosis.

```powershell
.\run-services.ps1 Status
```

Status returns PostgreSQL and server SCM states, `Live`, `Ready`, and the data directory. `/livez` returns 204 while the HTTP host responds. `/readyz` returns 204 only while the host accepts readiness and its application database answers `SELECT 1` within two seconds; database loss returns 503. The check does not prove every business workflow, schema permission, or client network path. The pools reconnect after database recovery without replacing credentials or changing endpoints.

```powershell
.\run-services.ps1 Stop
```

Stop waits for Rust before stopping PostgreSQL. WinSW sends Ctrl+C; Rust clears readiness and allows up to 30 seconds for HTTP/TLS connections to drain. WinSW has a 40-second stop budget before its forced termination fallback. PostgreSQL receives Ctrl+C for fast shutdown: it rejects new connections, rolls back active transactions, and checkpoints before exit. The command waits up to 60 seconds per service and fails instead of killing unrelated processes. Database transactions preserve their normal commit/rollback rules. Closing or exiting WPF does not issue this command.

```powershell
.\run-services.ps1 Uninstall
```

Uninstall stops the services in the same order, removes their SCM registrations, and removes the client firewall rule created by the `Firewall` action below. Other firewall rules remain unchanged. It preserves **all** installed files: the database, secrets, certificates, manifests, binaries, logs, and `host.json`. It contains no database drop, reset, or recursive-delete path. After SCM releases deleted service handles, `Install` with the same destination and name reinstalls the retained host without recopying source data. Run `Firewall` again to restore client access. Do not supply a different cluster as a recovery shortcut.

## Connect the second PC yourself

For DHCP clients on a trusted workshop LAN, use the Private LAN scope. On the host, open PowerShell as Administrator in the repository folder:

```powershell
.\run-services.ps1 Firewall -PrivateLan -WhatIf
.\run-services.ps1 Firewall -PrivateLan
```

The first command previews the change. The second creates or replaces this installation's HTTPS rule with `RemoteAddress=LocalSubnet`, `Profile=Private`, its saved private IPv4 listener, HTTPS port, and installed executable. Edge traversal is blocked. Public and Domain profiles are excluded. Client addresses can change through DHCP without rule updates. The command does not change network profiles, open PostgreSQL, or enable discovery. Set only a trusted workshop connection to Private in Windows Settings. Existing unrelated rules can grant access independently; inspect them before acceptance. Pairing and account sign-in remain application requirements; automatic discovery and pairing are still pending implementation.

For a packaged host, run `install-server.ps1 Firewall -PrivateLan` in the verified server package instead. `-PrivateLan` and `-ClientAddress` cannot be combined. Both scopes use the same installation-owned rule, so changing the scope replaces its prior access. Uninstall removes that rule and preserves data. A live check must confirm access from a DHCP client on Private and no access through this rule on Public. Automated command tests use mocked policy and do not prove effective Windows firewall behavior.

### Optional exact client address

On the host, open PowerShell as Administrator in the repository folder and run:

```powershell
.\run-services.ps1 Firewall
```

Enter the second PC's IPv4 address at the prompt. On that PC, find it under **Settings > Network & internet > Wi-Fi or Ethernet > Properties > IPv4 address**. Press Enter with no address to cancel. Invalid input shows a message and asks again. You do not need to send the address to anyone.

The command creates a Private-network inbound rule for that one address, the installed server executable, and its saved HTTPS address and port. It prints the server URL for the client setup. Run the same command again to replace the previous client address. It accepts no subnet, range, wildcard, public address, or IPv6 address. Use router DHCP reservations for both PCs so their addresses stay stable. Set the trusted workshop connection to Private through Windows Settings; the command does not change network profiles.

On the second PC, open the app and expand **ربط هذا الجهاز بالمنشأة** on the sign-in screen. Enter the printed server URL in **عنوان الخادم**. Supply the verified public CA file, tenant code, organization and branch identifiers, and your own active account through [desktop enrollment](run-server-authority.md#enroll-or-recover-a-windows-desktop). Transfer only the public CA, never the host's private directory. Firewall access alone does not create an account or complete enrollment. For a fresh database, use [first-owner initialization](run-server-authority.md#bootstrap-the-first-owner) to create an active Owner/Manager and branch. The service installer still imports prepared state; it does not perform this initialization itself.

For a preview without a firewall change, run `Firewall -WhatIf`; it still asks for the address. For scripted input, use `Firewall -ClientAddress 192.168.50.21` with the real client address. The rule is `<Name>-Host-Client` (default `Eitmad-Host-Client`) in the local persistent policy store. The command refuses a matching rule owned by another installation. Existing broad rules can grant access independently; inspect them through [LAN firewall scope](setup-lan-server.md#allow-only-selected-clients-on-the-private-interface). The command does not remove unrelated rules or open PostgreSQL.

From the second PC, test the host address and saved HTTPS port with `Test-NetConnection`. Then sign in and complete an authorized operation. The firewall command does not prove TLS trust, enrollment, or client reconnection. Use `Status` on the host to check service readiness. For an isolated host, supply the same `-Name` and `-StateDirectory` used at installation.

## Identity, secrets, and storage

| Location or identity | Access and lifetime |
| --- | --- |
| `NT SERVICE\EitmadPostgres` | Password-free virtual account; reads PostgreSQL binaries and modifies only installed database storage within the host directory |
| `NT SERVICE\EitmadServer` | Separate password-free virtual account; reads application binaries, settings and TLS; modifies manifests and private server logs |
| `data/` | Persistent copied cluster; PostgreSQL, Administrators, and SYSTEM only |
| `private/database-url`, `private/token-key`, TLS files | Unencrypted files protected by NTFS ACLs; server read access, Administrators and SYSTEM full control; no PostgreSQL account access |
| `app/`, `postgres/` | Service identities have read/execute access only; operators replace artifacts while stopped |
| `manifests/`, `logs/server/` | Server write access; stdout/stderr roll at 10 MiB with four retained files per stream |
| `logs/postgres/` | PostgreSQL identity write access for its separate wrapper logs, with the same rotation limits |
| Source LAN directory and backups | Operator-owned; retained separately; CA signing key stays outside service access |

Use BitLocker and encrypted, access-controlled backups for protection at rest. Administrators and SYSTEM can read service secrets. Never copy private keys, URLs, tokens, cluster data, or logs to client PCs or screenshots. No service password, application secret, or CA password is placed in SCM arguments or WinSW XML. Rust reads the two secret files through `EITMAD_SERVER_DATABASE_URL_FILE` and `EITMAD_SERVER_TOKEN_KEY_FILE`; configuring both a value and its file is an error. Do not set ambient machine-wide `EITMAD_SERVER_*` secrets. Product changes still pass through Rust authorization and audit; service lifecycle changes are host administration, not product-state mutations.

The installer grants no administrator group membership or interactive logon. [Windows manages virtual account credentials](https://learn.microsoft.com/en-us/windows-server/identity/ad-ds/manage/understand-service-accounts#virtual-accounts). Review existing domain/computer-account network grants separately on domain-joined hosts.

## Failure, recovery, and updates

- Both services have two SCM restart actions, after 10 and 30 seconds, then no further restart. Failure counts reset after 3,600 seconds without failures; a manual successful Start does not immediately grant a new retry budget. There is no restart loop or machine reboot action. A deliberate Stop remains stopped until explicit Start or a later Windows boot.
- WinSW supervises `postgres.exe` directly so a postmaster crash reaches SCM as a failure. A live host test found that PostgreSQL 17.11's native `pg_ctl runservice` reported exit 0 after its postmaster was terminated, which prevented SCM recovery. Do not replace the wrapper with that native launcher without verifying crash detection.
- Rust waits at most 120 seconds for database connections at startup, with two seconds between failed connection attempts. Configuration and migration failures stop startup. Inspect private server logs and Event Viewer before an explicit Start. Do not use a new token key, replacement database, or fresh CA as a repair.
- During database interruption, liveness can remain true while readiness is false. Restore the same database/service, then require readiness and a real authenticated client operation. Explicitly stopping PostgreSQL through SCM requires stopping its dependent server first; a database crash or connection interruption is a separate failure case.
- For port conflicts, inspect `Get-NetTCPConnection` and its owning process. The launcher never selects a new port or stops another process. Fix the conflict or review an intentional endpoint change. At boot, a bind failure exits and consumes the same finite recovery budget.
- On partial installation, preserve the directory, inspect both exact service registrations and identities, and run Uninstall before retrying Install with retained state. Ownership checks refuse a different executable, identity, or cluster path. An incomplete copy without `host.json` is not a complete installation; recover from preserved source into a new destination after inspecting the failed copy.
- Use the [controlled manual upgrade](package-windows-desktop.md#controlled-manual-upgrade) for an inventoried Windows package. It gates backups, immutable migration history, stopped services, atomic replacement, explicit Rust migrations, and TLS readiness. Interrupted releases remain demand-start with a recovery marker. A legacy installation without `app/release.json` needs an explicitly verified baseline before using that command. Reuse the same secrets and cluster; never copy development output over a running service.
- Renew the leaf in the original [LAN certificate directory](setup-lan-server.md#renew-without-changing-trust). While services are stopped, back up the installed pair and copy the newly active matching certificate and key to `private/server-cert.pem` and `private/server-key.pem`. Preserve the installed ACLs, verify both files inherit the server-only ACL, then Start and test both clients. Never copy the CA signing key into the service directory. A changed source `active-tls.json` alone does not update the installed copy.

The installer registers each wrapper's Windows Application Event Log source before restricted-account startup. Uninstall retains these historical event-source registrations. PostgreSQL fatal diagnostics go to its private rotating stderr log, with the logging collector and statement logging disabled by command-line overrides. Server logs use stable error codes. Collect only sanitized service states and error identifiers for support. Install and accept [scheduled LAN backup and recovery](backup-lan-installation.md) separately. Log retention, TLS expiry monitoring, disk monitoring, and UPS maintenance remain explicit operator responsibilities.

## Sleep, power, and reboot requirements

Services run while the screen is locked or the user is signed out. They cannot serve clients while Windows sleeps, hibernates, shuts down, or loses power. In Windows power settings, set sleep and hibernation to **Never while plugged in** for this host. Screen-off and screen-lock can remain enabled. Review lid-close behavior if a laptop is used. Keep the host on AC and a tested UPS. If operation after power loss is required, review firmware AC-recovery settings with the hardware owner.

Inspect the active power plan with `powercfg /getactivescheme` and `powercfg /query`. The installer does not execute power changes, disable updates, change NIC power management, install firewall rules, or change global shutdown timeouts. Record any separately approved power change. For a planned maintenance reboot, run Stop and confirm both services are stopped before restarting Windows. Windows can impose a shorter shutdown deadline during an OS restart; no wrapper can guarantee the full application drain budget after power loss or forced termination. Allow PostgreSQL crash recovery time after an unclean shutdown. Follow [LAN firewall scope](setup-lan-server.md#allow-only-selected-clients-on-the-private-interface) using the **installed** executable `C:\ProgramData\EitmadHost\app\eitmad-server.exe`; a rule for a source executable does not cover the installed copy. Keep PostgreSQL loopback-only.

## Verify before unattended use

Run the non-administrative lifecycle command checks:

```powershell
powershell -NoProfile -File deploy/windows/test-host.ps1
```

They exercise real Windows argument quoting and mocked SCM operations. They prove command order, readiness gating, conflict refusal, service ownership checks, finite recovery configuration, and uninstall data retention at the command boundary. They do not prove SCM installation or ACL behavior under virtual accounts.

Build the Rust server with the required toolchain and run the isolated runtime check with a **new** evidence directory:

```powershell
python deploy/windows/test-runtime.py --postgres-bin C:\PostgreSQL17\bin --server target/toolchain-1.85.1/debug/eitmad-server.exe --certificate-tool tools/dev-certificate/bin/Release/net8.0/Eitmad.DevCertificate.dll --root artifacts/unattended-runtime-UNIQUE
```

This check uses synthetic SCRAM credentials and a separate cluster on free loopback ports. It checks late database startup, readiness during loss and recovery, HTTPS bind conflict, and successful Ctrl+C shutdown. It retains private evidence and stops its own processes. Add `--verify-client-reconnect` to run the existing authenticated reconnect proof from [server operation](run-server-authority.md#run-the-direct-desktop-connection-test) with a fresh disposable database in that cluster. Add `--verify-workshop-setup` to check the real private-input initialization CLI, invalid input, mandatory audit rollback, concurrent initialization, active-owner sign-in, initial branch, staff creation, and the existing invitation account lifecycle in separate fresh databases. Select the required Rust compiler in `PATH` as well as Cargo. Local transport proof does not prove independent-PC reconnection.

Complete this acceptance table on an elevated Windows test host, with two independently enrolled PCs and synthetic data. Record the time, service identities, actual endpoint, and result without credentials or records:

| Check | Required evidence |
| --- | --- |
| Installed security | SCM `qc`, `qfailure`, and `qsidtype` show the two virtual identities, database dependency, automatic/delayed start, and two retries then none; `icacls` confirms ordinary users cannot read secrets or change binaries and each identity has only its listed access |
| Boot without login | Reboot at an approved time; leave the host signed out. From a client, verified `/readyz` becomes 204 within the agreed boot/recovery window; installed service PIDs and readiness are recorded afterward |
| Client reconnect | Both existing clients recover against the unchanged endpoint, trust, and credentials; a synthetic authorized operation reaches the other client without re-enrollment |
| Manager exit | Close the Manager window, then exit its tray process; services and ready endpoint remain available and the other PC can complete an authorized operation |
| Database interruption | Interrupt only the test database; observe live 204 and ready 503. Restore it; observe ready 204 with retained records and a client operation. Test a crash separately from an intentional dependent-service Stop |
| Port conflict and restart bound | On the isolated host, reserve the configured port before startup; the unrelated listener survives, the host stays unready, and recovery stops after the configured attempts. Release the port and use explicit Start |
| Graceful stop and reboot | Stop during synthetic work; confirm readiness drains, services stop within their budgets, accepted work remains, and an interrupted transaction does not report success |
| Preserving uninstall | Record a synthetic accepted record and the cluster identity; Uninstall, confirm registrations are absent and files remain; reinstall retained state, Start, sign in, and read the same record |

On 2026-10-19, the Rust focused tests, strict Clippy, optimized build, documentation audit, and PowerShell command checks passed. The isolated PostgreSQL/HTTPS check passed late database startup, live/ready separation, outage recovery, port conflict, Ctrl+C shutdown, and the existing authenticated Rust client reconnect test.

Live Windows installation checks passed with separate virtual accounts: verified TLS readiness; rejection of a wrong CA and hostname; denied non-elevated secret access; actual postmaster crash and automatic recovery with the same Rust PID; clean PostgreSQL shutdown; an occupied HTTPS port with exactly three starts (initial attempt plus two retries), followed by no further restart; preservation of the unrelated listener; and explicit recovery after releasing the port. Uninstall/reinstall retained the cluster identity, token key, and CA trust and became ready without importing again. The current WPF window was closed and its real tray Exit callback invoked by a temporary integration harness; both service PIDs and TLS readiness remained unchanged. That Manager profile was new and unenrolled.

The fresh shared installation uses `C:\ProgramData\EitmadHost`; separate operator TLS and stopped staging state are retained outside service access. The first owner and software-update publisher are not enrolled. No development cluster was imported. Operator CA and database administrator recovery credentials use CurrentUser DPAPI; retain the Windows profile and arrange independent protected recovery before operational use. Sleep and hibernation were already Never on AC; no power policy changed. The firewall remains unchanged until the second PC's exact address is supplied.

Boot without interactive login and two independently enrolled Windows PCs remain **Needs verification**. A one-time SYSTEM startup observer can record service states, session IDs, verified readiness, time since boot, and whether an interactive login is present, without starting any service. Its result must show a boot later than the installation checks. Leave the host signed out for five minutes after the approved restart, then inspect the protected evidence. This local observer does not replace a real client operation. Verify stable host addressing/name resolution, finish first-owner activation and independent enrollment through the supported setup, then complete both-PC reconnect and authorized-operation checks. Do not call this host accepted until those checks pass.

Return to [operations](index.md), [LAN setup](setup-lan-server.md), or [server recovery](run-server-authority.md).
