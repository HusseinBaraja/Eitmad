---
title: "Run and recover the modular server authority"
description: "Configure, migrate, bootstrap, start, verify, back up, and recover the combined Eitmad server safely."
audience: "operations"
page_type: "task"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-24"
review_triggers:
  - "server configuration, CLI, migrations, health routes, TLS, backup, or recovery changes"
keywords:
  - "eitmad-server migrate"
  - "eitmad-server bootstrap"
  - "EITMAD_SERVER_DATABASE_URL"
  - "readyz"
---

# Run and recover the modular server authority

Run the combined control, sync, relay, update, and administration server only with a dedicated PostgreSQL database, protected token key, trusted update public key, dedicated manifest directory, and TLS. Development plaintext is permitted only on an explicit loopback bind.

For a new Windows workshop, follow [guided server and client installation](install-windows.md). It provisions PostgreSQL, TLS, the first Owner, services, and the first desktop. The commands on this page are for advanced manual deployment, account administration, and recovery; they are not extra installation steps after guided setup. Use [manual stable LAN setup](setup-lan-server.md) only when you need to provision a fixed endpoint separately.

## Before you start

- Use a PostgreSQL role that can create the owned schema during migration but cannot bypass row-level security during normal operation.
- Back up the database before each migration or binary rollback.
- Generate a random 32-byte token key and encode it with URL-safe base64 without padding. Store it in the deployment secret manager. Do not rotate it without a token invalidation plan.
- Provide a certificate and private key for every non-loopback listener.
- Provision a dedicated update-manifest directory that only the server service account and approved release workflow can write.
- Obtain the reviewed Ed25519 public key and key ID. Do not put the signing private key on the server.
- Do not use customer credentials or data in command transcripts.

## Configure the process

| Environment variable | Required | Meaning |
| --- | --- | --- |
| `EITMAD_SERVER_DATABASE_URL` or `EITMAD_SERVER_DATABASE_URL_FILE` | Yes | PostgreSQL connection URL or UTF-8 file containing it; secret; choose exactly one source |
| `EITMAD_SERVER_TOKEN_KEY` or `EITMAD_SERVER_TOKEN_KEY_FILE` | Yes | URL-safe base64 32-byte HMAC key or UTF-8 file containing it; secret; choose exactly one source |
| `EITMAD_SERVER_LISTEN` | No | Listener, default `127.0.0.1:8443` |
| `EITMAD_SERVER_TLS_CERTIFICATE` | For TLS | PEM certificate path |
| `EITMAD_SERVER_TLS_PRIVATE_KEY` | For TLS | PEM private-key path; secret path |
| `EITMAD_SERVER_ALLOW_INSECURE_LOOPBACK` | Development only | Must be `true` to serve plaintext on loopback |
| `EITMAD_SERVER_MAX_CONNECTIONS` | No | Total PostgreSQL connection budget, 4 through 192; default 16. The server splits it across control, sync, and administration pools. The sync pool includes one shared approval notification listener. |
| `EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY` | Yes | Dedicated durable directory for immutable signed manifest JSON files |
| `EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID` | Yes | Tenant UUID for the operator scope allowed to publish global update channels |
| `EITMAD_SERVER_UPDATE_KEY_ID` | Yes | Trusted update signing-key identifier |
| `EITMAD_SERVER_UPDATE_PUBLIC_KEY` | Yes | Base64-encoded 32-byte Ed25519 public key; not secret |

Validate without connecting to PostgreSQL:

```powershell
cargo run -q -p eitmad-server -- check-config
```

Expected output is `server configuration is valid`. Configuration debug output redacts the database URL, token key, and private-key path.

For unattended Windows hosting, use [installed services](run-windows-host-services.md). The service configuration contains protected secret-file references. Empty, unreadable, or ambiguous secret sources fail validation. A file may end in CR/LF; its contents are never logged.

## Migrate before bootstrap or serve

Stop new work and preserve the existing database, then run:

```powershell
cargo run -q -p eitmad-server -- migrate
```

Expected output is `server migrations are current`. This command reads only `EITMAD_SERVER_DATABASE_URL` and the optional `EITMAD_SERVER_MAX_CONNECTIONS`; it does not require token, listener, TLS, or update runtime settings. It applies the current control, sync, administration, and canonical audit migration sets, currently `1–18`, before it exits. See [schema ownership](../developer/subsystems/server-authority.md#storage-scope-and-audit-invariants) for the capability migrations. Migration files are immutable after release. Stop if the process emits `eitmad.error.server-database-unavailable.v1` or `eitmad.error.server-migration-failed.v1`; preserve the database and diagnose the PostgreSQL service, permissions, RLS, and schema history.

## Bootstrap the first owner

For an empty Windows host, [guided setup](install-windows.md#1-install-the-server-pc) creates the PostgreSQL cluster, runtime settings, and active Owner/Manager. The separate [preparation backend](prepare-windows-workshop.md) creates the same initial workshop state without installing the services or desktop. Neither path needs another bootstrap or activation step. Use the commands below only for a separately provisioned empty database.

For a new workshop installation, use `eitmad-server initialize-workshop`. This local deployment command accepts version-one JSON on standard input, creates an active Owner with the Manager role and a first branch, and returns only public enrollment coordinates. It creates no activation invitation. Rust commits the tenant, organization, account, password verifier, branch, relationships, publication event, and mandatory audit together. It does not install PostgreSQL, configure Windows services, enroll a desktop, or provide the combined installer UI.

Use the dedicated application database and full runtime configuration above. The command applies normal migrations before initialization. Run it as the local deployment operator with access to those protected settings. It is not a network endpoint, and an application account or discovered server cannot invoke it remotely. Do not use a superuser or a role with `BYPASSRLS`.

From the extracted server package in PowerShell 7.4 or later:

```powershell
$setupPassword = Read-Host 'Owner password (at least 12 characters)' -AsSecureString
$setupEncoding = $OutputEncoding
$setupRequest = $null
try {
    $OutputEncoding = [Text.UTF8Encoding]::new($false)
    $setupRequest = @{
        version = 1
        workshopName = Read-Host 'Workshop name'
        ownerUsername = Read-Host 'Owner username'
        ownerPassword = [Net.NetworkCredential]::new('', $setupPassword).Password
    }
    $setupRequest | ConvertTo-Json -Compress | & .\eitmad-server.exe initialize-workshop
    if ($LASTEXITCODE -ne 0) {
        throw 'Workshop initialization failed. Preserve state and inspect the error identifier.'
    }
} finally {
    if ($setupRequest) {
        $setupRequest.ownerPassword = $null
    }
    $setupPassword.Dispose()
    $OutputEncoding = $setupEncoding
}
```

Do not print or save `$setupRequest`, enable transcripts, or put the password in command arguments. UTF-8 preserves Arabic names in the native input pipe. The bounded request has `version`, `workshopName`, `ownerUsername`, and `ownerPassword`; its Rust authority is `crates/contracts/src/setup.rs`. Unknown fields, unsupported versions, invalid names, and invalid passwords fail closed. The public result has `tenantCode`, `tenantId`, `organizationId`, `branchId`, and `ownerAccountId`, plus `version`. Use those coordinates for [desktop enrollment](#enroll-or-recover-a-windows-desktop), then use the [native Users screen](#use-the-native-users-screen) to add staff.

Any existing tenant blocks initialization, including one created by the invitation bootstrap below. Concurrent initializations share the same advisory lock as invitation bootstrap; only one can create the first tenant. After an uncertain result, inspect retained tenant and audit state through the deployment operator. Do not rerun setup as a password reset, change a tenant code, or replace the database. Use the existing owner recovery procedure for an active Owner.

### Invitation bootstrap

Use the separate invitation flow only when an operator needs to activate the initial account separately.

Run this once for a new tenant. Quote display names that contain spaces:

```powershell
cargo run -q -p eitmad-server -- bootstrap al-eitmad "الاعتماد" "مصنع الاعتماد" owner
```

The command prints tenant, organization, and account IDs, one activation token, and its expiry. Capture the activation token through an approved secret channel. It cannot be recovered from PostgreSQL because only its keyed hash is stored. Activate the account through `POST /v1/auth/activate` before the invitation expires.

A `bootstrap` invocation with a missing or extra argument prints usage and exits with a failure code (`eitmad.error.contract-invalid.v1`); it never exits successfully without creating the tenant. Concurrent bootstrap calls are serialized by a PostgreSQL advisory lock; exactly one call creates the first tenant and every other call is rejected.

Do not repeat bootstrap after an uncertain result. First check the tenant and audit state through an approved administrative query or database operator under a documented incident procedure. A repeated tenant code is rejected.

## Manage shared server accounts

Use the supported Rust contracts in `crates/contracts/src/server_accounts.rs` and the normal TLS host. No provisioning example or direct SQL is required. The [identity guide](../developer/subsystems/identity-foundation.md#shared-server-account-lifecycle) defines authorization, revision, receipt, and secret-handling rules.

1. Initialize the first active Owner as described above, use the invitation bootstrap, or sign in an existing Owner through `POST /v1/auth/login`.
2. For each administrative request, send a current bearer token, a fresh signed device proof in `x-eitmad-device-proof`, and a base64url `PeerHello` in `x-eitmad-peer-hello`. Negotiate protocol 1.5 or newer and `eitmad.capability.server-administration.v1`. A UI role name is not authorization evidence.
3. Register each needed branch with `POST /v1/customer-branches` using `RegisterBranchRequest`.
4. Call `POST /v1/accounts/list` with `ListServerAccounts`, initially `after: null` and `limit: 100`. Continue from `nextAfter` until it is null. Use the returned account revision for changes.
5. Call `POST /v1/accounts/transition` with `ChangeServerAccount`. Use a new UUID idempotency key for each new intent. An `invite` change supplies organization ID, username, and explicit role/branch assignments. The response contains an account receipt and an activation credential. Deliver the credential through a protected channel; do not print it in diagnostics.
6. The invited user submits `ActivateAccountRequest` to `POST /v1/auth/activate` with that credential, a valid password, and their device registration. Possession of the invitation authenticates this step. The response supplies the user's device-bound session and tokens. Read the account list again before its next revisioned change.
7. Use `assign` with account ID, current `expectedRevision`, organization ID, and the complete desired assignments for that organization. Both operational roles can be assigned explicitly; an empty list removes both. All affected account sessions close. A caller changing their own assignment must sign in again.
8. Use `revokeSessions` to close all current sessions while retaining the account, or `deactivate` to disable a non-Owner account and remove its relationships. Both require account ID and current revision. Owner deactivation and transfer are unsupported.

Success returns HTTP 200 after commit. Invalid input returns 400, denied scope or authority returns 403, and a stale revision, changed retry intent, or protected Owner returns 409. Invalid, revoked, or expired credentials return 401; unavailable authority returns 503. After a lost response, retry the identical request and key with a fresh device proof. Do not change the payload under the old key. After a conflict, read current state and submit a new intent with a new key. Exact retries preserve their original receipt, including terminal business errors. A revoked caller must first authenticate again.

An identical activation retry returns the original token pair only while that result is usable and unrotated. Otherwise use password login. Neither retry path restores revoked sessions. Preserve the server token key: it protects token hashes, request fingerprints, and retry reconstruction. Desktop enrollment and online credential recovery use the supported flow below. All protected desktop access requires live server confirmation under the [offline authorization rule](../developer/subsystems/identity-foundation.md#disconnected-authorization-limit).

### Verify account administration

Create a fresh disposable database with the helper below, then run in the same PowerShell session:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server --test accounts server_accounts_work_through_authenticated_routes -- --ignored --exact
```

This scenario uses the real handlers and PostgreSQL RLS. It verifies role separation, tenant isolation, stale revisions, concurrent exact retries, activation recovery, branch and role replacement, audit rollback, deactivation, pagination, and token/session revocation. It does not verify native rendering or TLS; the separate direct desktop connection scenario covers TLS.

### Use the native Users screen

Sign in online with an account that has both a Manager surface assignment and tenant Owner authority. Open **المستخدمون**. Use **إضافة مستخدم** to create a shared account with a display name, username, password, and role. Use **تعديل** for display-name or role changes and **تعطيل** for confirmed deactivation. Rust sends the intent to the authenticated server and waits for its committed receipt. A role change closes that user's active sessions; sign in again for the current role. Owner deactivation remains unsupported.

On a list-service failure with access still confirmed, account changes remain disabled and retained rows show **غير مؤكد**. **إعادة التحميل** restores current state. A failed live access check, including an outage, closes the session and clears protected views. Sign in again online after connectivity returns. Do not copy another device's data to restore access. See the [disconnected rule and detection bounds](../developer/subsystems/identity-foundation.md#disconnected-authorization-limit).

### Verify connected desktop accounts

Build the current Rust engine and Windows test project. Prepare a fresh disposable PostgreSQL database and TLS certificates using the [direct connection prerequisites](#run-the-direct-desktop-connection-test). Set `EITMAD_DESKTOP_ENROLLMENT_ENGINE` to the absolute engine executable path. Then run:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route shared_accounts_revoke_two_live_engines_and_preserve_offline_authorship -- --ignored
```

The scenario starts two isolated engines, uses real Windows IPC and controller clients, creates a shared account, edits its role through Users, and deactivates it. It saves pending work before interrupting the server, then checks idle-session closure, denied cached reads, writes, offline sign-in, and subscriptions during the outage. It changes access while disconnected, restores the connection, and verifies online recovery, retained records, and exact original authorship. It asserts no application restart after enrollment. Run `UsersPresentationTests`, `DesktopSessionControllerTests`, `UsersRenderedTests`, and `DesktopEnrollmentRenderedTests` for native presentation, session fencing, Arabic copy, and rendered controls. Rendered tests use synthetic data; record actual display size and scaling with captures.

## Recover a forgotten Owner password on the server

Run `eitmad-server recover-owner <tenant-code> <owner-username>` locally on the server with a PostgreSQL superuser connection supplied through `EITMAD_SERVER_DATABASE_URL` or its protected `_FILE` setting. The Rust command verifies the database role and loopback connection. Ordinary server application credentials are denied. Protect administrator credentials and use a local masked prompt to send the new password to the command through stdin. Never put a password in command arguments, shell history, logs, or chat.

Supply the exact password as UTF-8 stdin, without an added newline. Use 12 to 256 characters and at most 1024 UTF-8 bytes. The command operates only on the specified active account with a tenant Owner relationship. It does not create accounts, replace identifiers, change assignments, or modify business records. Password replacement, existing session and token-family revocation, account revisions, change notification, and `eitmad.server.authentication.owner-password-recovered.v1` audit commit in one transaction. An audit or storage failure rolls back the change. This command does not run migrations or start a network listener.

Success prints `Owner password recovered; existing sessions revoked`. Sign in online with the new password and check the desktop profile. Existing clients retain their runtime data and connection settings. If the command reports `eitmad.error.server-owner-recovery-failed.v1`, check the local administrator connection, exact tenant and account, active Owner status, password limits, and current audit schema. Correct the cause before retrying; no database reset is required.

The focused `accounts` integration test `local_owner_password_recovery_preserves_scope_and_revokes_sessions` requires a fresh disposable database named `eitmad_owner_recovery_*`, an ordinary RLS application connection in `EITMAD_DIRECT_TEST_DATABASE_URL`, and a local superuser connection to that same database in `EITMAD_RECOVERY_TEST_ADMIN_URL`. It checks role and scope denials, invalid password rejection, audit rollback, preserved identities, old password rejection, new password login, and session/token revocation. Run only against disposable test data:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server --test accounts local_owner_password_recovery_preserves_scope_and_revokes_sessions -- --ignored
```

## Enroll or recover a Windows desktop

Approve a new device through [Rust pairing](../developer/subsystems/server-authority.md#pair-a-discovered-server-and-approve-its-device) before ordinary sign-in. The [native pairing and recovery controls](prepare-windows-workshop.md#use-native-arabic-setup) use the same Rust authority. Existing registered devices and high-entropy invitation activation remain supported. Manual connection entry alone cannot approve an unknown device.

For the first server desktop, guided setup uses `DesktopEnrollment::first_desktop_request` to create the client key and signed public request locally. It passes that secret-bearing `LoginRequest` JSON to `eitmad-server enroll-first-desktop` through private stdin on the server, with its protected local database connection. The authenticated elevation channel returns `SetupDesktopHandoff`, including the running service's retained server UUID. `DesktopEnrollment::retain_first_desktop` retains the original endpoint, public CA, workshop scope, and device binding before ordinary sign-in. Keep `desktop-first-server.json`, `connection.json`, the CA files, `desktop-device.json`, and the original user's secret store. The Server PC desktop then uses the same discovery-based recovery as independent clients after app or service restart and supported Private IPv4 address changes. Do not replace its URL with a discovered IP or regenerate its device key. The local approval command permits only the original first device, requires the active Owner password, and fails closed on audit failure. Do not expose it over HTTP or place its input in command arguments, logs, or chat.

The focused `first_desktop_retains_recovery_through_restart_and_revocation` direct-route scenario uses the disposable PostgreSQL and TLS setup below. It checks private first-desktop approval, HTTPS sign-in, settings reload, server restart, substituted installation identity, and revoked approval. Run it with `rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route first_desktop_retains_recovery_through_restart_and_revocation -- --ignored`. The certificate-backed `recovery::tests::` library tests also exercise first-desktop retention through discovered loopback address changes, unchanged TLS name and CA, changed trust, and credential rejection. They inject candidate results into the existing connection path; they do not prove Windows Private-network broadcast or firewall behavior. Two-PC acceptance remains required for that network boundary.

For a discovered client, an authenticated Owner opens a code with `POST /v1/pairing/open`. The Rust client calls `DesktopEnrollment::pair`, then uses its returned connection settings for account sign-in. A code expires after three minutes or five starts and approves only one device. The exact pending completion can recover a lost receipt after expiry. Retry it first. Use **ترك الطلب واستخدام رمز جديد** (`DiscardPendingPairing`) only for explicit abandonment, then request a fresh Owner code. Abandonment removes only the pending exchange; it does not revoke server approval, replace committed trust, or change the device key. If approval already committed and its receipt was abandoned, another exchange is denied for that device. Preserve the runtime directory and all keys instead of replacing the identity.

### Manual connection entry for an approved device

For a new employee PC, use [discovery and Owner-code pairing](install-windows.md#3-install-and-pair-each-employee-pc). The steps below are a diagnostic or recovery path for an already approved device, or the supported high-entropy invitation flow. Entering connection settings cannot approve a new device.

Use an existing active server account with one explicit Manager assignment or a Receptionist assignment for the selected branch. The administrator supplies the HTTPS endpoint, trusted CA PEM file, tenant code, organization ID, and existing branch ID. Owner status alone is not an operational desktop role; guided setup creates the first Owner with Manager duties. The TLS server and PostgreSQL must be running.

1. Open the installed **الاعتماد** app from Start under its original Windows user. For a source checkout, use `run.ps1` with the approved device's runtime directory supplied through `-RuntimeDirectory`. Keep that device's original runtime and key.
2. Enter the server account name and password. Sign-in and protected work require a server connection.
3. Expand **ربط هذا الجهاز بالمنشأة**. Enter **عنوان الخادم**, **ملف شهادة الثقة**, **رمز المنشأة**, **معرّف المنشأة**, and **معرّف الفرع** from the administrator. Identifiers and paths use LTR input within the Arabic screen.
4. Select **دخول**. Rust validates server trust, verifies this installation's approved device, and confirms the account assignment. After settings commit, the supervisor reopens the engine and enters the Rust-selected role.
5. Close and reopen the application under the same Windows user and runtime directory. Leave the setup section closed and sign in. Rust uses the retained connection references and renews server access with password login.

For expired or missing account credentials, use the same normal online sign-in. The installation key is separate from account tokens. The flow preserves local records, pending changes, and device identity. After a failed or interrupted attempt, correct the input and retry; the password box is cleared, while connection inputs remain available. An initial settings commit can require one supervised engine restart. Cancellation or process interruption does not require deleting or recreating the database.

| Symptom | Safe action |
| --- | --- |
| Trust or connection rejection | Verify the endpoint, hostname, CA file, and server availability with the administrator. Do not bypass certificate validation. |
| Account or scope rejection | Verify the tenant code, credentials, active account status, organization, branch, and explicit operational role. Existing installation scope is not replaced. |
| Existing settings have no tenant code | Expand setup once and supply the original connection, trust, organization, and branch plus the tenant code. |
| Server is unavailable | Protected access is unavailable. Keep the runtime data and retry online sign-in when the server returns. |
| Both device key and account credentials are lost | Preserve the runtime and contact the operator. Automatic replacement of the registered signing identity is not supported. |

A new PC must enroll independently. Never clone another PC's runtime directory or copy token material between PCs. The [desktop enrollment authority](../developer/subsystems/server-authority.md#retained-desktop-connection) defines persistence and failure boundaries. The workflow environment remains a synthetic provisioning harness; it is not required for desktop enrollment.

### Verify desktop enrollment

Build the current engine and Windows test project. Prepare a fresh disposable PostgreSQL database and TLS certificates as described under [direct desktop connection testing](#run-the-direct-desktop-connection-test). Set `EITMAD_DESKTOP_ENROLLMENT_ENGINE` to the absolute path of that engine executable to include the Windows controller and named-pipe path, then run:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route desktop_enrollment_recovers_interruption_expiry_and_credential_loss -- --ignored
```

## Start and verify

Start the foreground server:

```powershell
cargo run -q -p eitmad-server -- serve
```

The process applies migrations before it starts listening. Keep migration privileges separate in a hardened deployment even though the combined development binary supports this convenience. The executable registers its customer, catalog, quotation, order, and work-order handlers before listening. Verify that negotiation includes the required schema and capability before accepting product traffic. Library tests can compose an empty registry; that is not the normal executable configuration.

Verify:

1. `GET /livez` returns success when the process can serve requests.
2. `GET /readyz` returns success after startup and migration checks while the application database answers `SELECT 1` within two seconds. It returns 503 during database loss or shutdown. It does not prove that a product domain is registered. `/livez` can still return 204 during a database outage. Startup waits up to 120 seconds for database connections; migration or configuration failures stop startup. Runtime pools reconnect to the same database after an interruption.
3. Protocol `1.4`, `1.5`, and current-minor `1.6` synthetic clients can authenticate, send `eitmad.server.hello.v1`, negotiate every required server capability, verify the product's required schema list, and then close cleanly.
4. A synthetic tenant cannot read another tenant's scoped records.
5. Relay and administration routes reject unauthenticated and unauthorized requests and produce redacted audit rows.
6. A changed manifest byte fails Ed25519 verification; the assigned channel selects only an exact compatible platform package.
7. The administration migration projection uses foundation version `3` as its minimum; verify the complete migration registry separately through `migrate`, not only its `Current` status.
8. Logs contain stable error identifiers and no URL credentials, token values, passwords, private keys, proof signatures, relay frames, or domain payloads.

Do not route traffic from the load balancer until readiness and tenant-isolation checks pass.

## Interrupted changes and recovery

Backup and restore commands are deferred by [ADR-0033](../decisions/0033-defer-backup-support.md). Do not discard database files or replace a migrated database with older binaries. Stop inbound traffic, preserve the database and release records, correct the failure, and use the same release for forward repair. Never reverse migration SQL by hand.

Do not delete operation history until a complete snapshot covers it and retained client checkpoints no longer need it. Keep at least the configured 90-day history floor.

## Current verification limit

The repository provides real TLS/PostgreSQL scenarios for desktop authentication, catalog delivery, quotations, orders, and revocation below. Use [local storage inspection](recover-local-storage.md) to check retained desktop state. These checks do not establish production deployment, clean-machine installation, physical-LAN access, or replacement-machine credential recovery. Use [offline setup verification](verify-offline-windows-setup.md) for those remaining acceptance checks.

For failures, use [server authentication and sync failures](../troubleshooting/server-authentication-and-sync.md) or [relay, update, and administration failures](../troubleshooting/server-plane-failures.md). For design ownership, use [the modular server authority](../developer/subsystems/server-authority.md).

## Run the direct desktop connection test

For a local Windows setup, dot-source the repository helper. It downloads the pinned PostgreSQL 17.11-3 Windows x64 archive from [EDB](https://www.enterprisedb.com/download-postgresql-binaries), checks SHA-256 `4B8DB0930C38F6EF845DB919551DEDDA3B6B845AEB0927B3D79A6E8E9E4537CF`, initializes a private cluster, starts it on loopback port 55432, creates a fresh disposable database and generates TLS certificates.

```powershell
. ./tools/dev-database.ps1 -Action NewDatabase
```

The helper sets `EITMAD_DIRECT_TEST_*` in the current PowerShell session. Run the test in that same session. Each invocation creates a new database; use one per live scenario. The test role is `eitmad_test`, without superuser, `BYPASSRLS`, role creation or database creation rights. Random credentials, the cluster, archive and logs remain under ignored `target/direct-order-test`, restricted to the current Windows account. TLS files remain under ignored `target/direct-route-cert`; no system trust store is modified. Do not print or commit credentials, connection URLs, certificates or keys. The helper starts a local process, not a Windows service.

Stop that test cluster when it is no longer needed:

```powershell
./tools/dev-database.ps1 -Action Stop
```

Stopping preserves the cluster and databases for inspection. No product database is reset or removed. If PostgreSQL or certificates already exist elsewhere, use the manual setup below.

Use a new, empty, disposable PostgreSQL database that the test role can migrate. The test bootstraps one synthetic Arabic tenant and must not run against product data. The local certificate tool creates a seven-day CA, a `localhost` server certificate and key, and an unrelated CA for the rejection check. It writes only to the ignored `target/direct-route-cert` directory. Do not commit those files.

```powershell
dotnet run --project tools/dev-certificate/Eitmad.DevCertificate.csproj --configuration Release -- target/direct-route-cert
```

Set `EITMAD_DIRECT_TEST_DATABASE_URL` to the disposable database URL through the local secret channel, then run:

```powershell
$certDir = (Resolve-Path target/direct-route-cert).Path
$env:EITMAD_DIRECT_TEST_CERTIFICATE = Join-Path $certDir 'server-cert.pem'
$env:EITMAD_DIRECT_TEST_PRIVATE_KEY = Join-Path $certDir 'server-key.pem'
$env:EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE = Join-Path $certDir 'trusted-ca.pem'
$env:EITMAD_DIRECT_TEST_WRONG_CERTIFICATE = Join-Path $certDir 'wrong-ca.pem'
cargo test -p eitmad-server-connection --test direct_route real_server_authentication_tls_sync_and_reconnect -- --ignored --exact
```

The test starts the real HTTPS/WebSocket host with a registered synthetic sync schema. It requires a valid signed device proof and token, receives and acknowledges a real `SyncMessage::Changes` from PostgreSQL, delivers cancellation, rejects an invalid token and unrelated CA, rotates an expiring token pair, stops the server, observes transport loss, and reconnects after the shared backoff. A missing database or certificate path fails the command; it is not a skipped success. The test does not exercise discovery or relay routing.

## Verify catalog delivery and recovery

Use the direct-route certificate settings above and a new disposable PostgreSQL database. The migration role must have neither superuser nor `BYPASSRLS` rights; the test checks those privileges. Use a fresh database for each real-server scenario because each bootstraps its own synthetic tenant.

```powershell
cargo test --locked -p eitmad-server-connection --test direct_route catalog_reaches_separate -- --ignored
```

This scenario checks dependency interruption, separate Manager and Receptionist clients, public field omission, tenant RLS, private-stream denial, denied Product definition history/snapshots, published image references, a failed snapshot commit, server and client restart, exact cost dependencies, and archive propagation. It does not verify native rendering. Restore the same authorized route after interruption; do not delete checkpoints, history, or transfer work. See [catalog replication](../developer/subsystems/synchronization.md#catalog-replication).

## Verify quotation-to-order recovery

Create a fresh disposable database with the helper above, then run:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route orders_cross_client -- --ignored
```

The test covers competing Receptionist conversions, principal-bound exact retry, changed-key rejection, immutable accepted prices, Furniture-only work, Products-only readiness, Manager notes and cancellation, delivery uniqueness, invalid transitions, direct role/scope denials, restricted production fields and documents, active subscription role revocation, forced tenant RLS, migration reapplication and server restart. Native pending/rejected rendering is checked separately by `OrderAuthorityTests` and `OrderAuthorityRenderedTests`.

## Verify established-session revocation

Use another fresh disposable database and the same certificate settings:

```powershell
rustup run 1.85.1 cargo test --locked -p eitmad-server-connection --test direct_route revoked_session_cannot_submit -- --ignored
```

The test establishes a real TLS WebSocket, revokes its durable server session, and submits a customer mutation before the periodic session check. The server must return an authentication failure without a sync result or customer record. This verifies per-request enforcement on an existing connection.
