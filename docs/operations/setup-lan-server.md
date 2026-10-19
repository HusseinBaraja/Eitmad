---
title: "Set up a stable Windows LAN server with TLS"
description: "Retain a LAN endpoint, protect server keys, enroll independent PCs, and renew TLS without changing client trust."
audience: "operations"
page_type: "task"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-18"
review_triggers:
  - "LAN launcher, TLS issuance, firewall scope, or desktop enrollment changes"
keywords:
  - "LAN TLS"
  - "certificate renewal"
  - "private firewall"
  - "lan-server.ps1"
---

# Set up a stable Windows LAN server with TLS

Use `tools/lan-certificate` and `tools/lan-server.ps1` for a retained Windows LAN installation. This path does not call `workflow-environment.ps1`, generate test accounts, refresh client trust, or reset data. Rust remains the authority for server configuration validation, authentication, authorization, audit, and product data. Manual server-address entry is sufficient; discovery is not required.

## Before setup

- Use Windows, PowerShell 7.4 or later, .NET 8, and a built `eitmad-server.exe`. Build the server with Rust 1.85.1 as described in [server operation](run-server-authority.md). Keep the executable in a location that ordinary clients cannot modify.
- Run certificate setup, settings capture, server startup, and renewal as one dedicated Windows account. The tool restricts its new directory and private keys to that account and SYSTEM. Use an encrypted disk and a protected backup. Do not use a shared folder, repository, compiler cache, junction, or symlink for operational state.
- Reserve the server IPv4 address in DHCP, or configure a fixed address outside the DHCP pool. This launcher supports RFC1918 IPv4 LAN binds. Example addresses below are synthetic and must be replaced.
- Choose one stable name, such as `eitmad.workshop.test`, and make LAN DNS resolve it to the reserved address on both PCs. If LAN DNS is unavailable, use the reserved IPv4 address as the certificate name and endpoint. Do not enter a bare IP for a DNS-only certificate. Do not depend on `.local` discovery.
- Confirm that the correct Windows interface has the **Private** network profile. Keep the firewall enabled. Do not add router port forwarding or publish this private CA endpoint on the Internet.
- Prepare the dedicated PostgreSQL service, database, role, backups, token key, and update public-key settings from [server operation](run-server-authority.md#before-you-start). Keep the database on this server. Bootstrap and activate accounts through that guide; this LAN tool does not create product identities or business records.

## Keep PostgreSQL private

Set `listen_addresses = '127.0.0.1'` in the service's `postgresql.conf`. Keep SCRAM authentication in `pg_hba.conf`, with application access limited to loopback and the dedicated database and role. Restart PostgreSQL through its service owner. Do not remove or reset an existing cluster.

Check the actual listener on the server:

```powershell
Get-NetTCPConnection -State Listen -LocalPort 5432 | Select-Object LocalAddress, LocalPort
```

Use the actual database port when different. Only loopback listeners are allowed. The LAN launcher requires a database URL with host `127.0.0.1` and refuses startup if any listener on the configured database port uses a non-loopback address. It creates no database firewall rule. Inspect and remove any unrelated broad PostgreSQL allow rule under the machine's normal administration procedure.

## Create the retained CA and first certificate

From the repository root:

```powershell
dotnet build tools/lan-certificate/Eitmad.LanCertificate.csproj --configuration Release --nologo
dotnet tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.dll initialize C:\ProgramData\EitmadLan eitmad.workshop.test
```

The destination must be new. For an address-only deployment, replace `eitmad.workshop.test` with the reserved address, such as `192.168.50.10`. The tool adds that exact DNS name or IP to the Subject Alternative Name extension. It does not add unrelated names or localhost.

Enter a unique CA password of at least 16 characters at the hidden prompt. Store it in the operator's password manager, separate from the state directory. It is required for renewal. Do not put it in a command argument, script, transcript, or scheduled task. The CA signing key uses password-encrypted PKCS#8 with AES-256 and PBKDF2-SHA256. The Rust server's PEM private key is unencrypted for unattended startup and protected by NTFS access control. Neither private key goes to clients.

The CA lasts five years. Server certificates last 90 days. The tool prints the CA certificate's SHA-256 fingerprint. Record it through an authenticated channel. Initialization refuses an existing directory. Renewal cannot create a replacement CA.

| Retained path | Purpose |
| --- | --- |
| `identity.json` | Stable certificate name and format version |
| `trusted-ca.pem` | Public CA certificate; the only TLS file to distribute to clients |
| `ca-key.encrypted.pem` | Password-protected CA signing key; server/operator only |
| `tls/<bundle>/` | Immutable server certificate and protected private-key pair |
| `active-tls.json` | Atomically replaced pointer to one complete pair |
| `server.json` | Retained endpoint, executable, bind address, and private deployment environment |

Keep the directory and password in separate protected backups under the [LAN recovery procedure](backup-lan-installation.md). Include the CA operator machine if it differs from the server. Loss of the CA signing key or password prevents ordinary renewal. Preserve existing data and client trust if setup is interrupted; inspect the private directory before retrying. There is no reset operation.

## Save and check server settings

Set the required deployment environment variables from [the server configuration table](run-server-authority.md#configure-the-process) through the approved secret channel in a private PowerShell process. Use the same dedicated account. Do not print the environment. In particular, retain the original token key and use `127.0.0.1` in the database URL. Do not copy secrets from the disposable test harness.

```powershell
pwsh -File tools/lan-server.ps1 Save -StateDirectory C:\ProgramData\EitmadLan -ServerExecutable C:\Eitmad\eitmad-server.exe -ListenAddress 192.168.50.10 -Port 8443
pwsh -File tools/lan-server.ps1 Check -StateDirectory C:\ProgramData\EitmadLan
```

Expected output includes `server configuration is valid`. `Save` records the deployment environment only after the Rust validator accepts it. It refuses to overwrite saved settings. Close the secret-input process after capture. `Check` and later `Start` work without inherited server variables. The launcher clears ambient server variables for the child and restores its caller's environment afterward. It never passes server secrets to the desktop launcher.

For an intentional port, executable, or secret change, stop the server, back up `server.json`, edit it locally under the state owner, and run `Check`. Keep its endpoint and port consistent. Do not change the identity name or CA as an ordinary settings edit. Existing desktop connection settings reject a different endpoint or trust during recovery.

## Allow only selected clients on the private interface

Run an elevated PowerShell process as the **same account** that owns the state. Select exact client addresses, preferably with DHCP reservations:

```powershell
& .\tools\lan-server.ps1 Firewall -StateDirectory C:\ProgramData\EitmadLan -ClientAddress @('192.168.50.21', '192.168.50.22')
```

The rule allows inbound TCP only on the saved HTTPS port, saved local address, saved server executable, Private profile, and selected remote addresses. Edge traversal is blocked. A narrow private `/24` through `/32` CIDR is also accepted. Public addresses, `Any`, and broad subnet ranges are refused. The command requires Windows firewall administration rights. It refuses an existing rule rather than silently changing its scope. See [Microsoft's rule parameter reference](https://learn.microsoft.com/en-us/powershell/module/netsecurity/new-netfirewallrule?view=windowsserver2025-ps).

Add `-WhatIf` to preview the firewall command without changing policy.

Record the printed rule name. Inspect its profile, application, port, and address filters with `Get-NetFirewallRule` and the associated `Get-NetFirewall*Filter` commands. To retire it, use `Remove-NetFirewallRule -Name <recorded-rule-name>`. For a deliberate address or executable change, inspect and remove that exact rule, then recreate it. Do not disable the firewall or add an `Any` allow rule to fix a connection failure.

## Start and retain the endpoint

```powershell
pwsh -File tools/lan-server.ps1 Start -StateDirectory C:\ProgramData\EitmadLan
```

The server runs in the foreground. Use Ctrl+C for normal shutdown. Restart with the same command and state. This does not regenerate certificates, change the token key, or replace client trust. PostgreSQL must already be running privately. The Rust host applies its normal migrations before listening; follow the existing backup and migration procedure before a binary upgrade.

For unattended operation, follow [Windows host service installation](run-windows-host-services.md). It imports the saved LAN settings, current certificate pair, and a stopped provisioned cluster into protected persistent storage. PostgreSQL and Rust use separate virtual service accounts and start without an interactive login. Keep the original CA state for renewal. A firewall rule must refer to the installed executable. Do not configure a competing foreground launcher or scheduled task for the same ports and cluster.

## Enroll each PC independently

Copy **only** `trusted-ca.pem` through an authenticated channel. On each PC, compare its certificate fingerprint with the operator's recorded value before first enrollment:

```powershell
$ca = [Security.Cryptography.X509Certificates.X509Certificate2]::CreateFromPem([IO.File]::ReadAllText('C:\EitmadTrust\trusted-ca.pem'))
try {
    $ca.GetCertHashString([Security.Cryptography.HashAlgorithmName]::SHA256)
} finally {
    $ca.Dispose()
}
```

Use PowerShell 7.4 or later for this command. This hashes the certificate DER, not the PEM file bytes. Do not import this CA into the system-wide Windows root store; the Rust connection uses an explicit application trust file.

Follow [desktop enrollment](run-server-authority.md#enroll-or-recover-a-windows-desktop) on each PC. Enter `https://eitmad.workshop.test:8443/` (or the matching reserved IP endpoint), the verified public CA file, tenant code, organization, branch, and the user's active account credentials. Each PC generates its own device identity. Never copy another PC's runtime directory, server settings, signing key, or tokens.

Rust copies initial public trust into that installation and retains `connection.json`. Close and reopen the app with the same Windows user and runtime directory. Leave setup closed and sign in normally. No inherited server environment is required. Ordinary leaf renewal requires no new client files or edits.

## Renew without changing trust

Schedule renewal before the 90-day leaf expiry, for example every 60 days. Check server time and client time first. Under the state owner:

```powershell
dotnet tools/lan-certificate/bin/Release/net8.0-windows/Eitmad.LanCertificate.dll renew C:\ProgramData\EitmadLan
pwsh -File tools/lan-server.ps1 Check -StateDirectory C:\ProgramData\EitmadLan
```

Enter the retained CA password. Renewal generates a new leaf key, signs a certificate for the retained name with the same CA, writes a new immutable pair, then atomically updates `active-tls.json`. A wrong password or failed write preserves the previous active pointer. Concurrent certificate operations are refused. Existing TLS sessions use the old certificate until the host restarts.

Stop the server, start it with the same saved settings, and repeat secure readiness and both-PC sign-in checks. Compare the public CA fingerprint before and after; it must remain identical. Clients keep their existing CA and settings. There is no validation bypass, trust-download step, or private-key transfer.

Old bundles remain for recovery. For rollback after a failed restart, stop the host and restore a backed-up `active-tls.json` that points to an existing, unexpired pair from this same CA. Run `Check` before startup. Never roll back to an expired or compromised key. Do not delete retained state or customer data.

When fewer than 91 days remain on the CA, renewal refuses to issue another 90-day leaf. Plan an explicit operator-reviewed CA transition before that point. CA replacement after expiry or compromise is a separate trust change; this tool and desktop recovery do not silently replace existing trust. This private CA has no revocation service. A compromised CA or server key requires an incident response, not a validation exception or ordinary restart.

## Verify the configuration and two-PC outcome

Run automated checks with a **new synthetic** evidence directory:

```powershell
$env:EITMAD_LAN_TEST_ROOT = Join-Path $PWD ('artifacts/lan-tls-' + [Guid]::NewGuid().ToString('N'))
dotnet test tools/lan-certificate/tests/Eitmad.LanCertificate.Tests.csproj --configuration Release --nologo
pwsh -File tools/test-lan-server.ps1 -TestRoot $env:EITMAD_LAN_TEST_ROOT -ServerExecutable "$PWD\target\toolchain-1.85.1\debug\eitmad-server.exe"
$env:CARGO_TARGET_DIR = Join-Path $PWD 'target/toolchain-1.85.1'
$env:CARGO_INCREMENTAL = '0'
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --lib lan_tls_tests -- --ignored
```

Use the repository's required compiler on PATH for Cargo subprocesses. The certificate tests verify matching pairs, retained CA, wrong-password rollback, refused reinitialization, and rejected broad key permissions. The launcher checks use the real Rust `check-config` command, mocked firewall/listener commands, and firewall `-WhatIf`; they do not modify machine firewall policy. The Rust test performs TLS handshakes with the operational DNS and IP certificates, rejects wrong names and unrelated CAs, and accepts renewed leaves through retained trust. It uses in-memory transport and does not prove LAN routing or two-PC operation.

On 2026-10-18, these automated checks passed with .NET 8 and Rust 1.85.1. Focused Rust Clippy, retained desktop-settings tests, and the documentation audit also passed. The launcher was exercised under host PowerShell `7.7.0-preview.5`; PowerShell 7.4 was not separately exercised. No live two-PC, reboot, or real firewall-policy acceptance result has been recorded.

Complete this live acceptance check with two independent Windows PCs and synthetic accounts and records. Record the actual endpoint, date, and pass/fail result without passwords, tokens, keys, or customer data:

| Check | Required result |
| --- | --- |
| DNS/address and readiness | Both PCs reach `https://<saved-name>:<port>/readyz` using `curl.exe --cacert <verified-ca.pem> <url>` without `-k`; HTTP 204 |
| Independent enrollment | Both PCs sign in through the saved endpoint and their own runtime/device identities; a synthetic server-confirmed change reaches the other authorized PC |
| Wrong name | Use a different DNS alias resolving to the same server; HTTPS fails name validation. For a DNS certificate, the bare IP must also fail. Do not alter an enrolled installation's settings for this test |
| Wrong trust | Use an unrelated CA in an independent disposable test runtime or `curl --cacert`; TLS fails, and no settings are committed |
| Client restart | Exit and reopen both apps under the same users and runtime directories; normal sign-in succeeds with setup closed |
| Server restart/reboot | Use the installed host service acceptance procedure, retaining PostgreSQL, keys, and settings; both clients reconnect and accepted work remains |
| Leaf renewal | Renew, restart the host, and sign in on both PCs without distributing another CA or changing connection settings; CA fingerprint unchanged |
| Database isolation | `Test-NetConnection <server-address> -Port <database-port>` fails from both PCs; HTTPS remains reachable |
| Firewall limits | The recorded rule is Private-only with the selected addresses, executable, and HTTPS port; unselected clients cannot use this rule |

Live two-PC checks require two PCs and their network. Local automated results are not a substitute. If a check fails, preserve state and diagnose name resolution, clock, SAN, CA fingerprint, Private profile, exact rule scope, and PostgreSQL loopback binding. Never use `-k`, accept-all certificate callbacks, a fresh CA, or copied private keys as a repair.

Return to [operations](index.md), [server account and enrollment procedures](run-server-authority.md), or the separate [localhost workflow test harness](run-workflow-test-environment.md).
