---
title: "Install Eitmad on the server and employee PCs"
description: "Use the offline Windows setup to create a workshop, install employee desktops, pair each device, and verify the installation."
audience: "operations"
page_type: "task"
status: "active"
owner: "server and Windows deployment maintainers"
last_verified: "2026-10-24"
review_triggers:
  - "guided setup, target prerequisites, pairing, backup, renewal, or recovery changes"
keywords:
  - "Windows installation"
  - "offline setup"
  - "EitmadServerSetup.exe"
  - "EitmadClientSetup.exe"
  - "click and run and follow in app guided setup"
  - "server and client"
  - "device pairing"
---

# Install Eitmad on the server and employee PCs

**Delivery requirement:** "click and run and follow in app guided setup". [ADR-0031](../decisions/0031-two-self-contained-windows-installers.md) requires `EitmadServerSetup.exe` and `EitmadClientSetup.exe`, each with its required offline payload embedded. End users do not run scripts, extract a ZIP, install runtimes separately, or configure certificates and firewall rules manually.

Use the two installers from the same release. Install the server first. It keeps the workshop database and also installs the desktop app for the Windows user who starts setup. Each employee PC installs only the desktop app and connects to that server. Each EXE fixes its role; there is no second role-choice screen.

Use Server for a **new workshop** and Client for a **new employee PC** joining a running workshop. For a PC where Eitmad is already installed, use [repair or upgrade](package-windows-desktop.md#repair-and-data-preserving-uninstall). Do not create another workshop to repair a failed installation.

**Current release limit:** standalone packaging, authenticated automatic extraction, fixed roles, and the existing guided installation flow are implemented. A complete reviewed self-signed installer pair and clean-PC installation acceptance remain **Needs verification**. Unsigned packaging probes are not usable installers. See [offline setup verification](verify-offline-windows-setup.md) for the evidence still required. The steps below describe the implemented path; they do not claim a live installation has passed.

## Before you start

- Obtain the appropriate signed EXE and its authenticated release record from the release operator. If you are building it, use [standalone installer preparation](package-windows-desktop.md#build-the-two-standalone-installers). Component ZIPs and developer script folders do not meet this delivery requirement.
- Use patched x64 Windows with local NTFS storage and a policy that permits the signed MSIX. See [target prerequisites](package-windows-desktop.md#target-prerequisites) for host versions and Windows Server edition limits.
- On every PC, have Windows administrator credentials for the one-time publisher trust prompt. The server also needs administrator approval to install its services. Start setup from the normal Windows account that will use the desktop; a different administrator can approve the prompt. On clients, start setup from the intended normal Windows account; a separate administrator can approve the certificate import.
- Connect all PCs to the same trusted workshop LAN. Set the workshop interface to **Private** through Windows settings. Setup does not change the network profile. The server's Windows computer name must resolve to its private IPv4 address. Keep that name unchanged and keep the server powered on, connected, and awake.
- Prepare protected backup folders on unlocked, fully BitLocker-encrypted NTFS volumes. The default local destination is `%ProgramData%\EitmadBackups`, so its volume must meet these checks too. An optional external destination must be on a different physical disk. A local backup on the server's disk does not protect against disk loss.

End users need no Rust compiler, .NET SDK, database commands, or internet download during setup. The bundle contains the desktop runtimes and reviewed server tools. Server setup installs missing C++ and .NET Framework prerequisites from the bundle. Client setup installs no global runtime, service, database, or firewall rule. On first launch, setup shows the self-signed publisher and its public certificate fingerprint. Compare it with the authenticated handover, confirm it, then select **اعتماد الناشر والمتابعة**. Windows asks for administrator approval to trust that exact public code-signing certificate in its machine Root store. This applies to both Server and Client. The private signing key stays on the build PC. Later launches reuse existing valid trust. An initial Windows publisher or SmartScreen warning can still occur before the wizard opens; self-signing does not provide public reputation. See [publisher trust and signing](package-windows-desktop.md#build-the-two-standalone-installers).

## 1. Install the server PC

1. Verify `EitmadServerSetup.exe` against the authenticated release record.
2. Open it as the intended normal Windows user. Do not select **Run as administrator**. Complete the guided publisher trust step if shown. Setup then verifies and extracts its embedded files automatically.
3. Confirm that **إعداد خادم الورشة** appears.
4. Enter **اسم الورشة**, **اسم مستخدم المالك**, and **كلمة مرور المالك**. The password must contain at least 12 characters. Retain it securely. This creates the first active application Owner with Manager duties and the first branch; no separate activation step is required.
5. Under **وجهات النسخ الاحتياطي**, leave the local destination empty to use the default, or select a protected folder. Select an external folder if a suitable encrypted separate disk is available. Both selected destinations must be available for the first capture.
6. Select **بدء التثبيت** and approve Windows UAC. Leave setup open until it reports **اكتمل التثبيت**. If a prerequisite requests a restart, use the recovery steps below after restarting; a restart request is not successful completion.
7. Select **فتح الاعتماد**, or open **الاعتماد** from Start under the same normal Windows user. Sign in with the Owner username and password entered above. The first server desktop is already enrolled; it does not need an Owner pairing code.

Setup creates the private PostgreSQL cluster and TLS trust, applies migrations, installs the `EitmadPostgres` and `EitmadServer` services, and verifies HTTPS readiness. It also configures unattended certificate renewal, enrolls the original user's desktop, configures daily backups with seven retained generations, verifies the first capture and any selected external copy, and registers the desktop app. Readiness alone is not whole-setup completion.

The installed services start without Windows sign-in. Closing the desktop does not stop the server. PostgreSQL stays on loopback. Guided setup permits the Eitmad HTTPS and discovery traffic only from `LocalSubnet` on **Private** networks. Do not add a firewall rule for each employee address. Keep the server awake; see [power and reboot requirements](run-windows-host-services.md#sleep-power-and-reboot-requirements).

## 2. Create employee accounts

While signed in as the application Owner, open **المستخدمون**. Create an individual account for each employee and assign the required Manager role or Receptionist role and branch. Use [the Users workflow](run-server-authority.md#use-the-native-users-screen) for account changes.

Application Owner authority is separate from Windows administrator rights. Pairing approves a device; it does not create an employee account or give that account access.

## 3. Install and pair each employee PC

1. Open the release's `EitmadClientSetup.exe` as the normal Windows user who will use the app. Complete the guided publisher trust step if shown. Setup then verifies and extracts its embedded files automatically.
2. Confirm **إعداد جهاز الموظف**, then select **بدء التثبيت**. After publisher trust, the application installs for the original user without elevation. Wait for **اكتمل التثبيت**, then select **فتح الاعتماد** or open **الاعتماد** from Start.
3. In the app, select **البحث عن الخوادم** and select the intended server. If needed, select **فحص إعداد الاتصال** on sign-in to open connection setup.
4. On the already connected server desktop, sign in as Owner. Open **المستخدمون** → **ربط جهاز**, then select **إظهار رمز ربط جديد**. Share this code only with the person pairing the intended PC.
5. On the employee PC, enter that code and select **ربط هذا الجهاز**. Use a fresh code for each PC. Codes expire after three minutes and approve only one device. Do not photograph or record them.
6. After pairing succeeds, sign in with the employee's individual account. Close the app with **إنهاء الاعتماد** in the tray menu, reopen it from Start, and sign in again. Pairing and manual connection entry should not be required again.

The app authenticates server trust during pairing and keeps an independent device key for this Windows user. Do not copy runtime folders, keys, or private server files between PCs. Normal installation does not require entering IP addresses, certificate paths, tenant codes, or scope IDs. [Manual connection entry](run-server-authority.md#enroll-or-recover-a-windows-desktop) is an advanced recovery path for an already approved device.

## 4. Check the result

- Sign in on the server desktop and on each employee PC with their intended accounts. Protected work requires the live LAN server even when installation used an offline bundle. Internet access and a live workshop server are separate requirements.
- With a permitted account, save a synthetic customer and confirm that another authorized PC can read the server-confirmed record. Do not treat a local pending row as server confirmation.
- Exit and reopen each desktop. Check that sign-in works without pairing again and that the confirmed record remains.
- On the server, reopen `EitmadServerSetup.exe` and select **إدارة النسخ الاحتياطي**. Approve the administration prompt and check the schedule, last successful local capture, and any external copy. Follow [backup management and restore verification](backup-lan-installation.md#manage-backups-after-installation). A successful capture does not prove replacement-machine recovery.
- Before depending on unattended operation, verify client access after a planned server reboot while the server is still at the Windows sign-in screen. Use [offline setup acceptance](verify-offline-windows-setup.md) for the controlled reboot, firewall, DHCP, and restore checks.

## If setup or pairing stops

Keep the original installer EXE, installation folders, and original Windows user. Reopen the **same authenticated installer EXE** and select **فحص حالة الإعداد**. If offered, select **فحص واستكمال الإعداد**. Server recovery asks for the current Owner password; Client recovery requires no password or elevation. Wait if another worker is still active. A blocked or uncertain result needs [stage-specific installation recovery](prepare-windows-workshop.md#progress-interruption-and-recovery).

A server can remain installed when a later enrollment, backup, or desktop stage fails. Do not delete its database, journals, certificates, or device key, and do not select new paths to start again. Repair and upgrade use their [separate package lifecycle procedures](package-windows-desktop.md#repair-and-data-preserving-uninstall).

If no server appears, check that the server is running, all PCs use the workshop Private LAN, and the server computer name resolves correctly. Keep Windows Firewall and TLS validation enabled. Use [host status](run-windows-host-services.md#install-and-operate) for diagnosis; do not start a second foreground server over the installed services.

If pairing was interrupted, select **إعادة محاولة الربط نفسه** first. It can recover a lost completion receipt without another code. If the exchange expired and cannot complete, select **ترك الطلب واستخدام رمز جديد**, then obtain a fresh code from the Owner. This preserves the device identity and does not revoke a server approval already committed. If approval was committed but its receipt was abandoned, preserve the runtime and use [desktop recovery](run-server-authority.md#enroll-or-recover-a-windows-desktop); do not replace the key to force another pairing.

## Maintenance

Keep the authenticated bundle and protected `%ProgramData%\EitmadSetupPayload-<id>` copy available: recovery and scheduled backups can use them. Do not clear these files as temporary installation output.

Use [backup management](backup-lan-installation.md#manage-backups-after-installation) to change destinations or check failures, [TLS renewal status](setup-lan-server.md#renew-without-changing-trust) to inspect certificate renewal, and [package maintenance](package-windows-desktop.md#repair-and-data-preserving-uninstall) for repair, removal, and controlled upgrades. Return to the [operations index](index.md) for other tasks.
