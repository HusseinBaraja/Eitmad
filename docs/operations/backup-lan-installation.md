---
title: "Back up and recover the Windows LAN installation"
description: "Schedule consistent off-device recovery generations, retain explicit history, and restore PostgreSQL and Windows credential dependencies together."
audience: "operations"
page_type: "task"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-19"
review_triggers:
  - "backup cadence, retention, participant inventory, Windows credentials, PostgreSQL version, or measured recovery changes"
keywords:
  - "scheduled backup"
  - "bare metal recovery"
  - "DPAPI"
  - "pg_basebackup"
  - "recovery point objective"
  - "UNC path"
  - "Windows backup account"
---

# Back up and recover the Windows LAN installation

`deploy/windows/run-backup.ps1` supplies Windows Task Scheduler integration, a coordinated backup worker, an explicit retention policy, and failure reporting. `restore-backup.ps1` verifies a complete generation and stages its PostgreSQL cluster without replacing existing data. Rust continues to own product authorization, audit, schemas, and synchronization. These tools perform privileged installation recovery, outside product IPC; they do not grant disconnected product access or change business records.

**Deployment status:** the scripts and isolated data recovery have been checked. No off-device destination or backup identity was supplied for this installation, so no live task was installed. Windows image creation, replacement-machine image restore, and native credential recovery remain **Needs verification**. Do not treat this installation as recovery-ready until the replacement acceptance procedure below passes.

## Recovery objectives and scope

Use these initial operating targets. Record any approved change in the protected plan before installing tasks.

| Item | Initial policy |
| --- | --- |
| Maximum acceptable data loss (RPO) | 24 hours from the chosen complete generation's `cutUtc`; includes unsent client work |
| Maximum recovery time (RTO) | 4 hours from loss of service until the replacement server and all required clients pass authenticated work and receipt replay checks |
| Capture cadence | Every 12 hours, starting at local 02:00; initial maintenance windows are 02:00 and 14:00 |
| Backup execution budget | At most 3 hours; a daily-only capture would leave insufficient margin for a strict 24-hour RPO |
| Retention | Newest successful generation for each of the 14 most recent UTC dates with complete generations, plus newest successful generation for each of the 8 most recent Monday-based UTC weeks with complete generations; union of both sets |
| Operator checks | Hourly freshness and destination-availability task; inspect failures before the RPO expires |
| Planned restore drill | Before first use, after credential/storage changes, and at least quarterly |

All clients must be powered on and have their shell and engine closed during capture. An active engine causes failure; the tool does not kill it, discard pending work, or omit the PC. Adjust the maintenance windows with the operator before installation if 14:00 interrupts work. More frequent captures require a new measured downtime and capacity check. A missed window is an RPO risk, not a successful reduced backup.

Register every client runtime, every Windows user profile volume, the server host, the CA operator state, manifests, and the coordinator. Add a separate `operator` participant if CA state or scheduler credentials are on another PC. A `server` participant can also own client runtimes. The coordinator must itself appear in the imaged inventory. A missing PC rejects the entire generation.

| Recovery dependency | Protected copy |
| --- | --- |
| PostgreSQL records, audit, roles, SCRAM password verifiers, control sessions, receipts, checkpoints, and catalog image bytes | Complete PostgreSQL 17 physical base backup with streamed WAL and SHA-256 backup manifest |
| Each client's SQLite database, WAL/SHM, pending work, accepted receipts, device identity, `connection.json`, and public trust | Windows image of complete runtime and profile volumes while engine startup is blocked |
| Windows Credential Manager, DPAPI master keys, Windows account/profile state, and scheduled-task credentials | Critical Windows volumes plus all profile volumes; recover the original Windows installation and original user identity |
| Installed server settings, token key, TLS pair, PostgreSQL configuration/binaries, wrapper XML, `host.json`, and update manifests | Complete host volume image |
| Original LAN CA state, encrypted signing key, CA password dependencies, and external signing/fallback-key dependencies | Complete operator volume image plus independent recovery-key/password escrow |

Catalog images need no separate byte directory: their bytes are in PostgreSQL and SQLite. A profile-only file copy, a new Windows account with the same display name, or copying DPAPI ciphertext into a new account is not credential recovery. Hardware-backed keys, Windows Hello, EFS, third-party password managers, and encrypted fallback key providers must be inventoried and recovery-tested if used. Never assume that imaging alone makes a hardware-bound key portable. Do not reset the original Windows password offline to gain access.

## Prepare the protected destination and accounts

Use this manual setup procedure before installing the schedule. The following names are examples, not selected installation values. Replace them with the actual computer and account names:

```text
UNC path: \\BACKUP-PC\EitmadRecovery
Windows task account: EITMAD-SERVER\EitmadBackup
Storage account: BACKUP-PC\EitmadBackupStore
```

The coordinator runs the scheduled tasks and must stay powered on during backup and status checks. The server can be the coordinator. The storage computer must be a separate physical Windows computer. The task account runs the coordinator; the storage account accesses the backup share. They have separate passwords and permissions.

1. Use Windows Pro/Enterprise or Windows Server with supported `wbadmin`, VSS, SMB encryption, BitLocker management, Windows PowerShell 5.1, and Task Scheduler. Use PostgreSQL 17 tools matching the installed server. Keep WinRE recovery media, network/storage drivers, and replacement disks of sufficient size available.
2. Select an independent Windows storage computer with capacity for the retained server and client volume images. It must not be any source computer, an alias to source storage, or a junction. On that computer, open **Computer Management → Local Users and Groups → Users** and create the dedicated local storage account, such as `EitmadBackupStore`. Enter its password privately. Create a new dedicated folder, open **Properties → Sharing → Advanced Sharing**, and share it as `EitmadRecovery`. Use the actual storage computer name in the UNC path. The tool currently supports a Windows SMB target, not an arbitrary NAS attestation.
3. In the share permissions, grant the storage account **Change** and **Read**. In **Properties → Security → Advanced**, grant it **Modify** on the folder, subfolders, and files. Keep Administrators and SYSTEM full control; remove other allowed access, including inherited broad access. Apply these restrictions only to the new backup folder and share. Do not put this share on a client-visible product share. Enable BitLocker on its backing volume and save the recovery key independently. Then enable SMB encryption from elevated PowerShell on the storage computer, replacing the example share name if needed:

   ```powershell
   Set-SmbShare -Name EitmadRecovery -EncryptData $true
   ```

   The tool checks encryption, access policy, and physical machine UUIDs before capture. The source task has retention rights; this is not immutable ransomware storage. Keep an independently controlled offline copy when that failure boundary is required, under its own explicit retention policy. The tool never prunes that copy.
4. Retain the storage BitLocker recovery key, source BitLocker recovery keys, original Windows account passwords, storage recovery login, CA password, and any external signing/fallback keys in an offline recovery kit or independently recoverable password manager. Keep this kit apart from the source PCs and backup disk. Test unlocking a spare encrypted volume and accessing the recovery kit. TPM auto-unlock on the lost computer is not an independent key copy. Do not place recovery keys in the plan, task arguments, logs, chat, or beside the images.
5. Configure authenticated WinRM **HTTPS** on the exact source and storage names under normal Windows administration. Use valid trusted certificates and narrowly scoped firewall rules. The tool does not enable remoting, change `TrustedHosts`, bypass TLS checks, enable CredSSP, or change domain delegation. Each remote session needs administrative backup/service rights. SMB credentials are passed through the authenticated session, avoiding the remoting second-hop requirement.
6. Create a PostgreSQL `LOGIN REPLICATION` backup role through the approved administrator channel, with a strong escrowed password and no superuser, role-creation, database-creation, or RLS-bypass rights. Add only a loopback `host replication <backup-role> 127.0.0.1/32 scram-sha-256` HBA rule and reload PostgreSQL. Physical backup reads the complete dedicated cluster; do not grant this role to application users. Allow at least two WAL sender connections. No source tablespaces, configuration includes, key files, or storage dependencies may point outside the inventoried volumes.
7. On the coordinator, use **Computer Management → Local Users and Groups → Users** to create a dedicated task account, such as `EitmadBackup`. For this manual setup, add it to the coordinator's local Administrators group, then sign in as that account and open an elevated Windows PowerShell console. Do not use an everyday operator account. Keep its password in Windows Task Scheduler/LSA and independent escrow. Do not use S4U for a task that needs network access. Store all credential XML files with `Get-Credential | Export-Clixml` under this same account on this same Windows installation. Windows encrypts these files with CurrentUser DPAPI. Use separate files for the SMB login, PostgreSQL backup login, and each administrative WinRM login. The storage account's share permissions do not grant WinRM administration rights; use a separate administrative login for the storage check. Do not supply passwords in command arguments or scripts.

Record the chosen UNC path, Windows task account, and storage account names before continuing to [install the schedule](#install-and-run-the-schedule). Keep passwords and keys out of chat and repository files. Use the [recovery objectives](#recovery-objectives-and-scope): 02:00 and 14:00 local captures, 24-hour maximum data loss, 4-hour recovery, and 14 daily plus 8 weekly generations. A daily-only 02:00 capture has no duration or failure margin for a strict 24-hour RPO; the current plan validator rejects `intervalHours: 24` with `rpoHours: 24`.

See the [Windows backup command](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/wbadmin-start-backup), [SMB share encryption](https://learn.microsoft.com/en-us/powershell/module/smbshare/set-smbshare), [SMB mapping parameters](https://learn.microsoft.com/en-us/powershell/module/smbshare/new-smbmapping), and [PostgreSQL base backup](https://www.postgresql.org/docs/17/app-pgbasebackup.html) references for platform prerequisites.

## Install and run the schedule

Use an elevated console as the task account. Create a **new** private `C:\ProgramData\EitmadBackup` directory and private `status` child. Use the existing host module's `Protect-Directory` helper, which restricts access to Administrators and SYSTEM. If a separate task account is not an administrator, explicitly grant its required access and validate the resulting ACLs. Do not weaken an existing operational directory's ACLs to make the command pass.

Copy `backup.psm1`, `backup-worker.ps1`, `run-backup.ps1`, `restore-backup.ps1`, and `host.psm1` from `deploy/windows` into that protected directory. Copy `backup.example.json` as `plan.json`; replace synthetic machine names, paths, and deployment ID. Generate a new ID with `[Guid]::NewGuid().ToString('N')`. Set `recoveryKitId` to the non-secret reference for the independently held recovery kit. Retain password/key versions needed by every retained generation; a current password alone may not unlock an older Windows image. Update this reference when dependencies rotate. The reference is recorded in each completion manifest; key material is never recorded there. Do not store operational state in the repository or `target`.

Set `share` to the selected UNC path, `storage.computer` to the storage computer's trusted WinRM HTTPS name, and `storage.shareName` to the share name. Keep `time: "02:00"`, `intervalHours: 12`, `rpoHours: 24`, `rtoHours: 4`, and retention `daily: 14`, `weekly: 8`. The task account is selected by the Windows identity running `Install`; it is not a separate plan field.

Supply the real runtime paths; the example is not an assertion about this machine's default path. Include any non-system drive that holds profiles, runtimes, trust, manifests, keys, or installation files. `requiredPaths` checks that retained dependencies exist on selected volumes; it does not discover an unknown key provider. Keep the coordinator's plan, scripts, credential files, and status directory on an included volume. Copy the protected host module to the server's configured `hostModule` path as well.

Capture each credential with an interactive hidden prompt. Example for the SMB credential, repeated with the correct file and identity for the other dependencies:

```powershell
Get-Credential | Export-Clixml -LiteralPath C:\ProgramData\EitmadBackup\smb.credential.xml
```

Enter the storage account in this prompt. Repeat for the PostgreSQL backup role and each WinRM administrator, using the exact credential file paths in the plan. These files cannot be decrypted by a different user or on a different Windows installation; copying them alone does not recover credentials. See [Windows credential export behavior](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/export-clixml?view=powershell-5.1).

First close all registered clients, confirm current server readiness, and perform a supervised capture:

```powershell
powershell -NoProfile -File C:\ProgramData\EitmadBackup\run-backup.ps1 -Action Install -PlanPath C:\ProgramData\EitmadBackup\plan.json
```

`Install` registers the `EitmadBackup` Application Event Log source, a repeating backup task, and an hourly status task. It prompts for the **current** Windows task account's password. Existing tasks are refused. A partial registration is retained for inspection; do not overwrite it blindly. Task names are `EitmadBackup-<deploymentId>-Run` and `EitmadBackup-<deploymentId>-Status`. Both run with highest privileges without an interactive login, start when available, ignore overlapping instances, and have a three-hour execution limit. Inspect their next run times with `Get-ScheduledTaskInfo`.

```powershell
powershell -NoProfile -File C:\ProgramData\EitmadBackup\run-backup.ps1 -Action Run -PlanPath C:\ProgramData\EitmadBackup\plan.json
```

Check the completed capture explicitly:

```powershell
powershell -NoProfile -File C:\ProgramData\EitmadBackup\run-backup.ps1 -Action Status -PlanPath C:\ProgramData\EitmadBackup\plan.json
```

Require exit `0`, a new complete off-device generation, verified server readiness, and successful subsequent `Status`. Install does not constitute acceptance; complete the replacement drill before enabling operational use. The scripts use PowerShell cmdlets and Windows-protected credentials, not passwords in native process arguments. Do not enable transcripts that expose operational inputs or attach private tool output to support tickets.

## Consistency and publication

The coordinator opens persistent authenticated sessions on **all** participants. Workers validate selected local NTFS paths, reject links/external tablespaces, and hold each registered `engine.authority.lock` with exclusive file sharing. This conflicts with engine startup and an already running engine. It also holds the server's `host.lock`, verifies service ownership through the existing host module, and stops the exact Rust server service. PostgreSQL remains running for `pg_basebackup`; all other database writers and administrative changes must be excluded during the window.

Only after every client barrier and the server stop does the coordinator record `cutUtc`. Each client retains its own pending outbox at that same stopped-write boundary. The server retains receipts for accepted requests, including requests whose reply never reached a client. No write drain that discards local work or forced synchronization is required.

The server runs `pg_basebackup --format=plain --wal-method=stream --checkpoint=fast --manifest-checksums=SHA256` into the new generation, then `pg_verifybackup --exit-on-error`. This includes roles and role password verifiers, which a database-only dump omits. Every participant runs `wbadmin start backup` with its selected complete volumes, `-allCritical`, `-vssCopy`, and a **unique** network target, then checks its Windows image catalog. Reusing a network target could overwrite the prior image, so the tool never does that.

After all captures, each worker checks its original boot identity and barriers. The server must still be stopped. The coordinator resumes the server and requires authenticated TLS readiness, then releases barriers and SMB mappings. Only then does it publish `complete.json`, including the cohort, cut time, duration, retention policy, and PostgreSQL manifest digest. A stopped client can be reopened after this step. Partial generations retain `incomplete.json`; they are never promoted after a failed or missing participant.

An unexpected service restart, a lost remote session, reboot, unavailable PC, active engine, full destination, invalid key/storage configuration, verification failure, or resume failure rejects completion. `finally` attempts to resume the server and release every partially acquired barrier. A killed coordinator or broken WinRM session can leave the server stopped; inspect and use the ordinary [host Start procedure](run-windows-host-services.md#install-and-operate). Do not manually mark a partial generation complete. Boot checks do not authorize simultaneous operators to write through another database or engine path.

## Retention and visible failures

Retention runs only after a new complete generation and successful resume. It recognizes only version-1 `complete.json` files whose deployment ID, generation ID, and entire participant list match the current plan. It retains the daily/weekly union above and deletes only other recognized generation directories immediately under that deployment root. It rejects reparse points before recursive deletion. It never deletes source runtimes, pre-restore/pre-migration SQLite files, unknown versions, foreign cohorts, partial generations, malformed markers, or a directory containing a `hold` file. Add `hold` before a restore or investigation. Remove it only under explicit operator direction. Inventory changes leave old cohorts outside automatic deletion; retain or retire those separately under an explicit policy.

Application Event Log `EitmadBackup` event `4100` means capture and retention completed. Event `4101`, exit `1`, or `eitmad.backup.failed.v1` means failure or stale/unavailable backup. Inspect protected `last-success.json` and `last-failure.json` timestamps and Task Scheduler's Last Run Result; an older failure file is retained after a later success. Status checks age from `cutUtc`, checks destination encryption and access policy, reads the selected off-device completion marker, and compares its PostgreSQL manifest digest. This is not a substitute for periodic full restore testing. Forward these sanitized events through existing Windows monitoring if available. A powered-off coordinator cannot report its own failure: the operator must check daily for a missing task/status run.

If the share fills, fix capacity or review the explicit retention policy. Unknown or partial generations require operator investigation; the tool will not delete them merely to recover space. If a task exceeds its limit or cannot decrypt its credential XML, inspect the retained task result, original account/profile, disk space, sessions, and exact services. Never respond by resetting Windows passwords, issuing a new CA/token key, or removing SQLite companions.

## Restore an isolated replacement installation

1. Start the recovery clock when service is declared lost. Record the incident time and selected generation's `cutUtc` separately. Choose one **complete** cohort; require its age to meet the RPO and add `hold`. Preserve the source machines and all newer work. Never combine a newer client image with an older server backup. Accepted requests known only to a newer client might have lost server receipts and could be replayed twice.
2. Use disconnected replacement machines or an isolated virtual network with no route to the live installation. Never boot a restored copy alongside the original device identity on the production LAN. Keep both original and replacement scheduler tasks disabled during the drill. Supply sufficient disk space and compatible firmware/storage drivers; record actual hardware and image sizes for the RTO.
3. In WinRE, use **System Image Recovery** to restore each participant's image from `<generation>\<node>\image`, including all backed-up data/profile volumes. Use the saved machine name and image version returned by `wbadmin get versions`. On Windows Server, [wbadmin start sysrecovery](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/wbadmin-start-sysrecovery) is available from the recovery environment. Disk recreation is destructive: select only the dedicated replacement disks. Unlock BitLocker using independent escrow. Never run system recovery against a source PC to test this procedure.
4. Sign in with each original Windows identity and its retained password. Check profiles, Credential Manager access through the Rust engine, DPAPI-protected coordinator/admin files, original CA fingerprint, and external key-provider unlock. Do not create replacement accounts, reset passwords offline, copy another client's secrets, or re-enroll devices as proof of recovery. A fallback-key installation also needs the original out-of-band fallback key and encrypted records. Test CA signing-key unlock without changing the CA.
5. Keep clients stopped. Stop the replacement host services through the existing launcher. The image's live PostgreSQL directory is preserved as evidence; install the **verified physical base backup**, not the live PostgreSQL files captured in the Windows image. Map the share through the original storage identity over encrypted SMB, using an interactive credential prompt or the recovered DPAPI file.

Validate and stage into a **new** directory under the replacement host, for example `C:\ProgramData\EitmadHost\data.recovered-<unique-id>`:

```powershell
powershell -NoProfile -File C:\ProgramData\EitmadBackup\restore-backup.ps1 -Generation <complete-generation-path> -PostgresBin C:\ProgramData\EitmadHost\postgres\bin -StageDirectory <new-private-stage-path>
```

This verifies the PostgreSQL manifest digest, full backup/WAL with [pg_verifybackup](https://www.postgresql.org/docs/17/app-pgverifybackup.html), and every Windows image catalog. Staging refuses an existing target. Preserve a failed stage; no source data is removed.

6. Under replacement-host administration, confirm **both exact services are stopped** and confirm that `data`, the new stage, and a new `data.image-preserved-<unique-id>` path are plain sibling directories within the restored host root. Rename `data` to the preserved name, then rename the verified stage to `data`. If the second rename fails, rename the preserved directory back before attempting Start. Use `Get-SafePath` and `Assert-PlainTree` from the host module to validate the exact paths before moving any directory. Reapply the existing `Protect-Directory` policy for `data`: Administrators/SYSTEM full control and `NT SERVICE\<Name>Postgres` Modify. Do not grant the Rust service direct database-file access. Keep the original PostgreSQL major version and full base-backup WAL; startup performs recovery from `backup_label`.
7. Restore the same private network endpoint and CA trust inside the isolated network. Run server `check-config`, then the ordinary host Start procedure and require TLS `/readyz` 204. Confirm application SCRAM login and the restricted role attributes. Keep original token/signing/TLS keys and manifests. Do not point the replacement at the source database. Run Rust diagnostics and normal online sign-in for each restored client, without editing retained connection/trust settings.
8. Compare records, immutable revisions, all prior audit rows, image hashes/bytes, connection/device identity, and pending outboxes with the synthetic fixture recorded at capture. Retry a request that the server accepted before the point but whose reply was lost. Require its original result and exactly one business effect. Reconnect twice after acknowledgements persist. Require customer, quotation-draft, catalog, and order pending work to become confirmed and stay confirmed. Check scope denial and current server account policy; a recovered verifier must not grant offline access.
9. Stop the clock only after every required client and credential dependency passes. Record lost writes after the cut, RPO age, Windows/image recovery time, PostgreSQL recovery time, first TLS readiness, first authenticated client work, complete RTO, image sizes, fixture counts/hashes, duplicate effects, and failures. Store private evidence beside the held generation and only sanitized results in documentation. A new password, new CA, new device, missing offline outbox, or untested key dependency fails acceptance.
10. For a drill, stop and disconnect the replacements and retain evidence. Resume only the original installation. For a real cutover, keep originals isolated, obtain the operator's selected recovery point, then reconnect replacement clients to the single replacement authority. Later data from preserved originals needs explicit reconciliation; never reconnect it to replay blindly. Re-enable the verified schedule after the replacement is accepted.

## Measured evidence

On 2026-10-19, this command created a fresh synthetic source cluster and a separate restored PostgreSQL 17 cluster on a different loopback port:

```powershell
python tools/run-lan-recovery-drill.py --postgres-bin target/direct-order-test/pgsql/bin --root artifacts/lan-recovery-20261019-b
```

Use a new evidence directory for each run. The wrapper never uses the installed host or a retained database credential. It initializes SCRAM credentials, runs the real two-client Rust recovery scenario, and stops its two clusters. Private evidence remains outside compiler output. Select Rust 1.85.1 explicitly in `RUSTC` and `PATH`; keep `CARGO_TARGET_DIR=target/toolchain-1.85.1` and `CARGO_INCREMENTAL=0`. This host's default Cargo child compiler otherwise selected Rust 1.96 and rejected cached metadata; no cache deletion was required.

| Proof | Result |
| --- | --- |
| PostgreSQL physical backup with a non-superuser replication role, streamed WAL, manifest verification, and new-cluster startup | Passed |
| Original application SCRAM password and restricted role in the new cluster | Passed; no role recreation |
| Full PostgreSQL table rowsets, including audit, images and receipts | Equal at the recovery point |
| Both SQLite table rowsets and prior audit evidence | Retained; existing manual restore adds its recovery audit |
| Furniture and product image bytes through authenticated reads | Passed |
| Pending customer, quotation-draft, and catalog work | Confirmed after recovery |
| Accepted order receipt and repeated reconnect | Original revision `2`; zero duplicate order effects |
| Measured database recovery interval | **11,231 ms**, from stopped server through physical backup/copy/start, SQLite restore, server restart, image reads, reconnect, and pending-work checks |
| Complete synthetic command, including build/setup | **44.661 seconds** |
| Windows images, lost-device DPAPI/Credential Manager recovery, off-device transfer, full RPO/RTO acceptance | **Not performed**; requires selected storage and isolated replacement Windows machines |

The data drill reuses native credentials under the current Windows user and restores existing SQLite installation identities. It does **not** prove new-machine Windows credential recovery, the scheduler's live operation, the 4-hour end-to-end RTO, or large-database throughput. The fixture has no loss relative to its recovery point; it does not measure a full day of business writes. Do not use its 11.231-second interval as the full installation RTO.

`powershell -NoProfile -File deploy/windows/test-backup.ps1` passed calendar retention, held/unknown/partial preservation, stale detection, successful cohort publication, and cleanup after failed capture with mocked Windows/remoting commands. Rust 1.85.1 focused compilation, formatting, and strict Clippy passed for the changed integration scenario. Complete the Windows replacement drill above before changing the deployment status.

Return to [operations](index.md), [manual SQLite recovery](recover-local-storage.md), [LAN TLS](setup-lan-server.md), and [Windows host services](run-windows-host-services.md).
