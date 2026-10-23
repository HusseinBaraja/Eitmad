---
title: "Build and maintain Windows installation packages"
description: "Build and sign offline Windows bundles, use advanced package installation, and preserve work through repair, removal, and manual upgrade."
audience: "operations"
page_type: "task"
status: "active"
owner: "release infrastructure maintainers"
last_verified: "2026-10-23"
review_triggers:
  - "Windows packaging, publisher, signing, prerequisites, service setup, migrations, enrollment, or lifecycle evidence changes"
keywords:
  - "Windows installation"
  - "MSIX signing"
  - "manual upgrade"
  - "data-preserving uninstall"
  - "install-server.ps1"
  - "install-client.ps1"
---

# Build and maintain Windows installation packages

For a new workshop or employee PC, follow [Install Eitmad on the server and employee PCs](install-windows.md). This page is for release builders and operators who need package verification, manual service import, repair, removal, or upgrade. The manual procedures below are alternatives to guided setup; do not run them after a successful guided installation.

Build the server ZIP and desktop installer ZIP from one source revision and version. The desktop ZIP contains an x64 MSIX with the WPF shell, Windows adapter, Rust engine, self-contained .NET runtime, and app-local Microsoft C++ runtime. The server ZIP contains Rust, service installation tools, backup tools under `deploy/windows`, and the self-contained LAN certificate tool. Each service and backup module has one copy under `deploy/windows`; run its `restore-backup.ps1` to verify a recovery generation. The backup example contains placeholder paths only; create the protected operational plan outside the package. Neither package contains a database, test account, credential, token, private certificate, or provisioned installation. No automatic updater is added.

Both Windows ZIPs include the canonical `docs/` tree with its repository paths unchanged. For the combined offline bundle, use [the installation guide](install-windows.md). For separate component ZIPs, use the manual procedures on this page. Keep the tree together so linked procedures, reference pages, and heading anchors remain available without a repository checkout. `release.json` inventories these documentation files. External links still require network access. Build and developer-test procedures still require the source checkout and their stated tools.

**Release status:** unsigned artifacts are for engineering validation. They are not production-ready. The production client installer refuses unsigned MSIX. A separate [self-signed test path](#test-msix-with-a-local-certificate) is available for controlled test PCs. Production requires the approved publisher, hardware-backed signing service, trusted certificate chain, approved RFC 3161 timestamp service, and clean-machine acceptance. These external prerequisites remain explicit gates. Even signed builds retain `productionEligible: false` until [release acceptance](validate-release-candidate.md) passes.

## Build and sign

Use x64 Windows, PowerShell 7.4+ with `pwsh.exe` on PATH, Python 3.11+, rustup with Rust **1.85.1**, .NET 8 SDK, and Windows SDK x64 `makeappx.exe`. Build-time signature checks, catalog creation, signing, and asset generation use PowerShell 7. Restore locked dependencies through approved Cargo/NuGet sources. The builder selects the Rust compiler explicitly and reuses `target/toolchain-1.85.1`. It publishes into a new temporary directory, independent of existing runtime data.

```powershell
python scripts/release/build_artifacts.py windows-server --version 0.1.0 --output dist
```

```powershell
python scripts/release/build_artifacts.py windows-desktop --version 0.1.0 --output dist
```

```powershell
python scripts/release/build_artifacts.py windows-setup --version 0.1.0 --output dist
```

The setup target builds `EitmadSetup.exe` with self-contained .NET 8 Windows Desktop, app-local C++, versioned file identities, SBOM, and release inventory. It accepts the desktop's `--publisher`, `--signing-hook`, and `--vc-runtime` options. The server also accepts `--vc-runtime` and includes app-local C++ beside Rust and the certificate tool, so they can start before Server prerequisites are installed. An unsigned setup ZIP is a build-validation artifact; setup refuses to launch a backend without a complete trusted offline catalog.

The desktop builder locates the newest numeric `VC/Redist/MSVC/<version>/x64/Microsoft.VC143.CRT` directory from installed Visual Studio instances through `vswhere.exe`. Use `--vc-runtime <REVIEWED_CRT_DIRECTORY>` to pin another approved redistribution source. It refuses missing files, mixed versions, versions below 14.40, non-x64 DLLs, untrusted Microsoft signatures, missing timestamps, and conflicts with published runtime files. It never takes DLLs from System32 or installs a global runtime. `visual-cpp-runtime.json`, the payload inventory, and SBOM retain the exact shipped version and digests. Review redistribution rights and servicing status for the selected source before release.

`0.1.0` is a validation example, not a declared production release. Use one approved semantic version for both artifacts. Three numeric components map to `major.minor.patch.0` in MSIX and .NET file identities. Prerelease labels are recorded in the Rust binary and inventory but do not increase the MSIX numeric version. Use a higher numeric version for every upgrade; never reuse a release version for different bytes.

The builder checks Rust `--version` and runtime diagnostic identity, gives .NET and MSIX the same version, validates migration hashes, and records revision, dirty-source status, architecture, local IPC version, and payload digests. `sbom.cdx.json` is a CycloneDX 1.6 dependency inventory from the resolved Rust normal/build graph and published .NET dependency manifests. It includes build inputs, excludes unrelated/test-only roots, and records exact bundled .NET runtime versions. Static linking can remove code from those inputs; the inventory does not claim that every source function ships. Signed payload digests bind this inventory to the release record. ZIP entries have sorted paths and fixed timestamps. Archive assembly is repeatable for identical payload bytes; compiler and external signing output are not claimed to be bit-reproducible. Signing timestamps intentionally change bytes. The `server` target still builds the separate Linux artifact.

Production candidates must come from a reviewed tag and clean checkout. The selected service is **Azure Artifact Signing Public Trust**, with Microsoft-managed HSM keys and Microsoft RFC 3161 timestamping at `http://timestamp.acs.microsoft.com`. The publisher is the **exact subject DN issued to the verified certificate profile**, copied from Azure after identity validation. Do not invent a legal identity or pin a short-lived certificate thumbprint. See [service resources and subject identity](https://learn.microsoft.com/en-us/azure/artifact-signing/concept-resources-roles) and [Microsoft's signing integration](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations).

External prerequisites remain: an eligible Azure subscription/account, accepted public identity validation, a Public Trust profile, a signing-node managed/workload identity with the Certificate Profile Signer role, the reviewed x64 Artifact Signing client library, compatible Windows SDK SignTool, .NET 8 runtime, and network access to the regional service and timestamp endpoint. Availability depends on Microsoft's identity eligibility rules. No Azure resources, paid service, or verified legal identity were supplied or created by this change.

Set `EITMAD_SIGNTOOL`, `EITMAD_SIGNING_DLIB`, and `EITMAD_SIGNING_METADATA` to approved local tool/configuration paths. The metadata contains `Endpoint`, `CodeSigningAccountName`, and `CertificateProfileName`, not credentials. Keep authentication in the signing node's managed/workload identity. Review and pin tool versions/digests in the release environment. The supplied adapter follows Microsoft's SignTool/dlib integration:

```powershell
python scripts/release/build_artifacts.py windows-desktop --version 0.1.0 --publisher $approvedPublisher --signing-hook deploy/windows/sign-with-artifact-signing.ps1 --output dist
```

Use the same arguments for `windows-server`. The hook accepts `-Path`, signs one file through the selected hardware-backed service with SHA-256 and an approved RFC 3161 timestamp, and throws on failure. It must not read a private key from the checkout or package. The repository has no PFX/password parameter, self-signed trust bypass, or production key. Executables, DLLs, scripts, and final MSIX are verified for trusted signatures and timestamps. Eitmad files, scripts, and MSIX must match the approved publisher; trusted vendor runtime signatures can remain.

Executables are signed before MSIX assembly, then the final MSIX is signed. The publisher must match the signing identity; see [Microsoft package signing](https://learn.microsoft.com/en-us/windows/msix/package/sign-app-package-using-signtool). Keep `Eitmad.Desktop` and its publisher stable. Publisher rollover requires a separately tested identity migration.

Keep packages under `dist`, outside Git. Transfer the ZIP digest and extracted `release.json` digest through an authenticated release channel. A checksum beside an untrusted download is not independent proof. The outer `.manifest.json` remains conservatively unsigned/non-production; a server ZIP also needs the authenticated release channel or approved detached release signature. Never edit generated inventory to pass verification. Package inventory is not the Rust signed update manifest. Do not create automatic-update assignments for this manual path; the [automatic update adapter](../developer/subsystems/update-distribution.md) remains unimplemented.

## Prepare the combined offline payload

The `windows-offline` target creates `eitmad-offline-<version>-win-x64-payload-signed.zip` from prebuilt, reviewed inputs. It does not download dependencies, run payload executables, register MSIX, provision a database, or change Windows services, certificate stores, firewall rules, or installed runtimes. Extract the complete ZIP before launching `setup/EitmadSetup.exe` as the intended ordinary Windows user. Assembly does not grant production acceptance.

Use signed desktop, server, and setup ZIPs from the same release version and clean source revision. Extract them without changing their inventories. Include reviewed redistribution notices before creating and signing those component packages. Supply the following input tree; the collector copies these paths unchanged:

| Input path | Required reviewed content | Installation role |
| --- | --- | --- |
| `setup` | Self-contained x64 `EitmadSetup.exe`, native WPF resources, .NET 8 Windows Desktop runtime, approved signatures, and redistribution notices | Launcher for both roles; installs no host component for Client |
| `client/desktop` | Existing desktop ZIP contents: signed MSIX, installation scripts, and offline documentation; retain self-contained .NET and app-local x64 Microsoft C++ runtime | Client and Server |
| `server/application` | Existing server ZIP contents, including the self-contained LAN certificate tool and workshop preparation scripts | Server only |
| `server/postgresql` | Complete reviewed PostgreSQL 17 x64 `bin`, `lib`, and `share`; include initialization, service, SQL, dump/restore, `pg_basebackup.exe`, and `pg_verifybackup.exe` tools | Server only |
| `server/winsw` | Reviewed `WinSW.NET461.exe` version 2.12.0 and redistribution terms | Server only |
| `server/powershell` | Extracted self-contained PowerShell 7.4+ x64 distribution, including its runtime and notices | Server only |
| `server/vc-redist` | Reviewed Microsoft C++ 14.40+ x64 offline installer named `vc_redist.x64.exe` and redistribution terms | Server only |
| `server/netfx` | Reviewed .NET Framework 4.8 full offline installer named `ndp48-x86-x64-allos-enu.exe` and redistribution terms for the WinSW Framework build | Server only |

Windows PowerShell 5.1, Windows `curl.exe` with Schannel, supported patched x64 Windows build 19041+, local NTFS, and MSIX policy remain OS prerequisites. They are recorded in `payload.json`. Do not copy system executables from an installed machine or bundle development toolchains. Setup verifies the whole distribution for both roles. Client installs only the per-user MSIX; it runs no Server prerequisite installer and requests no elevation. Server uses its authenticated paths and installs missing C++ and Framework runtimes quietly with restart disabled. Exit code 3010 stops setup for an operator-controlled restart. PowerShell 7 remains an extracted app-local runtime for packaged operator tools; use `server/powershell/pwsh.exe` explicitly when those tools require it.

### Review manifest and trust

Keep the input review JSON **outside** the payload input tree. Authenticate its SHA-256 through the approved release channel before collection. Do not calculate a new approval digest from an untrusted download merely to make validation pass. No vendor versions, hashes, or licenses are inferred from installed software or accepted from filenames.

The review format has four root fields: `schemaVersion` (`1`), `version` (the exact Eitmad semantic version), `architecture` (`x64`), and `components`. The `components` object must contain exactly `setup`, `desktop`, `application`, `postgresql`, `winsw`, `powershell`, `vc-redist`, and `netfx`. Each component has these fields:

| Field | Review requirement |
| --- | --- |
| `version` | Exact reviewed product version. Eitmad components match the release; PostgreSQL uses `17.minor`; WinSW is `2.12.0`; PowerShell is 7.4+ within major 7; C++ is 14.40+ within major 14; Framework is `4.8` |
| `provenance` | Object with `url` (HTTPS acquisition URL) and `sha256` (lowercase 64-digit source archive or installer digest). Use a version-specific immutable release URL. Review binds these acquisition records to the extracted file list; collection does not fetch the source archive again |
| `licenses` | Nonempty array of objects with `name`, `version`, `expression` (SPDX expression or reviewed `LicenseRef-` identifier), and `files` (nonempty array of relative paths to shipped terms/notices). Include all redistributed inputs, including .NET, the Windows Desktop runtime, Microsoft C++, and transitive vendor libraries. Add terms before creating the original signed package inventory |
| `files` | Exact component-relative file map. Each entry has lowercase `sha256`; every EXE/DLL also declares `architecture`: `x64` or verified IL-only `anycpu`. PostgreSQL native files and runtime entry points must be x64. The Framework offline bootstrapper can declare `x86` because it carries both target architectures |
| Executable trust | Every EXE, DLL, PowerShell script/module, and MSIX file entry has the exact trusted certificate subject in `publisher`. Windows must report a valid Authenticode signature, matching subject, and timestamp. Only PostgreSQL and WinSW can instead specify `unsignedReason`, documenting reviewed unsigned upstream content bound to its authenticated hashes. Both fields together are refused |

For example, an unsigned upstream WinSW file entry uses `sha256`, `architecture: "anycpu"`, and a nonempty `unsignedReason`. A signed Microsoft installer uses `sha256`, its verified PE `architecture`, and the exact Microsoft certificate subject in `publisher`. These are schema examples, not approved payload pins. There is no signature-bypass switch. Trusted signature validation requires Windows; an offline build host must already have the required trust-chain and revocation evidence. Unknown trust fails preparation. See Microsoft's [signature inspection](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature) and [PE format](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format).

The collector rejects absent components, missing required tools, changed or unlisted files, unsafe Windows paths, links/junctions/hard links, opaque nested archives, and architecture mismatches. It checks the Eitmad release inventories and the desktop's inner MSIX inventory and runtime hashes. It refuses operational databases, cluster directories, credentials, certificates, keys, and device identity files even when listed in the review. Start with clean vendor distributions and clean package output, never an operational installation. Redistribution review remains responsible for classifying the allowlisted bytes; filenames cannot detect secrets deliberately hidden inside an approved binary.

Client MSIX validation also refuses external package dependencies and server tools, including PostgreSQL dump/restore/control executables and service wrappers. The native package verifier checks the self-contained .NET/Windows Desktop host files and runtime configuration in addition to app-local C++. A signed hash alone does not establish that an artifact meets the Client role. Follow [offline setup acceptance](verify-offline-windows-setup.md) for repeatable isolated checks and exact clean-PC/reboot steps; these checks do not install anything on the development machine.

### Collect and recheck without installation

On the release build host, set `$approvedReviewDigest` to the independently authenticated review digest, then run:

```powershell
python scripts/release/build_artifacts.py windows-offline --version 0.1.0 --prepare-payload --payload-input C:\ReviewedOfflineInputs --payload-review C:\ReviewedRelease\offline-inputs.json --expected-review-sha256 $approvedReviewDigest --output dist\eitmad-offline-payload
```

The optional `--prepare-payload` mode creates a new verified directory with `setup/`, `client/`, `server/`, `reviewed-inputs.json`, `setup-bundle.json`, and `payload.json`. The latter records role paths, component versions, provenance, shipped license files, file hashes, included .NET runtime versions, and the retained desktop C++ inventory. Source files remain unchanged. The collector validates the source and staged copy before publication. It refuses an existing destination and leaves no completed output after validation failure.

For the final signed distribution, use a clean reviewed checkout and the existing Artifact Signing environment:

```powershell
python scripts/release/build_artifacts.py windows-offline --version 0.1.0 --payload-input C:\ReviewedOfflineInputs --payload-review C:\ReviewedRelease\offline-inputs.json --expected-review-sha256 $approvedReviewDigest --publisher $approvedPublisher --signing-hook deploy/windows/sign-with-artifact-signing.ps1 --output dist
```

The builder retains component inventories and SBOMs, creates an aggregate CycloneDX SBOM and root `release.json`, then creates and signs `offline.cat`. The catalog authenticates the complete extracted tree, including review, setup metadata, licenses, runtime files, MSIX, and inventories. Windows catalog version 2 uses SHA-256; see [Microsoft file catalogs](https://learn.microsoft.com/powershell/module/microsoft.powershell.security/new-filecatalog). The builder checks the trusted catalog signature, exact approved publisher, timestamp, and contents before assembling a sorted ZIP with fixed entry timestamps. Failure publishes no completed ZIP. Identical staged bytes produce identical archive bytes; catalog generation and external signing are not claimed to produce identical bytes between builds. Without `--signing-hook`, the ZIP is labeled `unsigned` and setup rejects its catalog. Component signature checks remain mandatory.

The launcher retains read-only handles and checks the catalog before starting Rust. Rust verifies it before reading setup metadata. Server elevation authenticates the source again, copies it into a new administrator/SYSTEM-only `%ProgramData%/EitmadSetupPayload-<id>` directory, and verifies that copy before executing its backend. Rust binds tool paths and release pins to that copy and verifies it again before installation. Elevated execution and DLL loading use the protected copy. This release tree is retained for scheduled backup execution and interruption recovery; do not remove it while a backup task references its backend. Setup does not install production signing certificates or weaken unsigned-MSIX rejection.

Recheck a prepared directory, or the complete extracted distribution, before use:

```powershell
python scripts/release/build_artifacts.py windows-offline --version 0.1.0 --output dist\eitmad-offline-payload --verify-payload --expected-review-sha256 $approvedReviewDigest
```

This repeats file, architecture, package, and signature validation and compares the generated inventory with `payload.json`. Authenticate the review digest independently again at the consuming boundary. Altering both the review and payload metadata cannot satisfy an unchanged approved review digest. To recover from rejection, correct the reviewed acquisition or source package, obtain a new approved review when bytes change, and select a new output directory. Never edit generated hashes as a repair.

Focused automated proof uses temporary synthetic inputs and installs nothing:

```powershell
python -m unittest discover -s scripts/ci/tests -p test_offline_payload.py -v
```

Synthetic archive tests substitute signature approval and catalog signing only. A Windows test uses real catalog creation and hash comparison to reject changed, missing, and extra files, with signature approval substituted for that hash test. Separate checks use the real trust boundary to reject unsigned catalogs and PE files. The launcher test refuses missing and unsigned catalogs before starting a backend. Runtime-selection tests substitute registry and process boundaries and check restart refusal. These tests do not prove trusted production payload acceptance, runtime installation, UAC, or a clean offline host. See [release acceptance](validate-release-candidate.md) before distribution.

## Target prerequisites

| Machine | Requirements |
| --- | --- |
| Every target | Supported, patched x64 Windows on local NTFS; build 19041+; x64 Windows PowerShell 5.1; sufficient disk space; encrypted disk and protected backups |
| Desktop | MSIX permitted by local policy; approved publisher chain trusted through enterprise/public distribution; install as each intended ordinary Windows user; no administrator rights, global C++ runtime installation, Rust compiler, or .NET SDK required |
| Server | Target hosts are Windows Server 2022/2025 or Windows 11; elevation for services; `curl.exe`; Microsoft Visual C++ 2015–2022 x64 Redistributable 14.40+ and patched .NET Framework 4.8 for WinSW. Guided setup supplies reviewed PostgreSQL 17 and WinSW 2.12.0, installs missing C++/Framework prerequisites offline, and provisions the cluster. Manual import requires those prerequisites, a provisioned stopped cluster, reviewed tool digests, and PowerShell 7.4+ for manual LAN settings. Guided setup also registers the desktop MSIX; Server-edition AppX prerequisites and the complete guided path still need clean-edition acceptance |

Use [Microsoft's supported Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) for the server. The desktop installer verifies the app-local C++ runtime inside the authenticated MSIX before registration; it does not require the machine-wide runtime registry entry. Microsoft [supports local deployment](https://learn.microsoft.com/en-us/cpp/windows/deployment-in-visual-cpp?view=msvc-170), but Windows Update does not service those copies. Rebuild and distribute the package for C++ and .NET runtime security fixes. The tools do not download prerequisites, disable security policy, or import a root CA. Guided setup derives the server endpoint from the Windows computer name and a private IPv4 address; keep the name resolvable on the LAN. Reserve a stable address for the manual fixed-endpoint path. The host must stay awake; review [power and reboot requirements](run-windows-host-services.md#sleep-power-and-reboot-requirements).

On 2026-10-20, an unsigned desktop validation ZIP built successfully with Rust 1.85.1 and self-contained .NET 8. Its outer inventory, inner MSIX inventory, ten app-local CRT files, and SBOM version `14.44.35211.0` were verified without package registration. Focused tests rejected changed and missing DLLs and verified that the per-user platform check does not query the global C++ runtime registry. A clean offline PC without a global C++ runtime remains **Needs verification**. This result does not prove production signatures or complete guided onboarding.

## Install the server

For a fresh workshop, use [guided Server setup](install-windows.md#1-install-the-server-pc). The following procedure is an **advanced manual import** from a separate server ZIP. It does not run the guided first-desktop, backup, or renewal flow. Use it only when an operator owns those separate tasks. The [preparation backend](prepare-windows-workshop.md) can create the stopped cluster and prepared LAN root instead of manual database and identity provisioning; do not bootstrap that prepared workshop again.

1. Extract the server ZIP into a new local staging directory. Compare ZIP and inventory digests with the authenticated release record. In elevated x64 PowerShell, set `$releaseDigest` and `$approvedPublisher` to its non-secret values, then run:

   ```powershell
   .\install-server.ps1 Verify -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher
   ```

2. Provision a **new dedicated** PostgreSQL 17 primary cluster and application database through normal PostgreSQL administration. Keep it loopback-only, with SCRAM authentication. Use an application role with `NOSUPERUSER`, `NOBYPASSRLS`, `NOCREATEDB`, and `NOCREATEROLE`, with schema privileges for Rust migrations. Never initialize over existing data or use the development harness. See [server prerequisites](run-server-authority.md#before-you-start).
3. Create retained LAN trust with the packaged tool, using the actual stable DNS name or reserved private IPv4:

   ```powershell
   .\tools\lan-certificate\bin\Release\net8.0-windows\Eitmad.LanCertificate.exe initialize C:\ProgramData\EitmadLan eitmad.workshop.test
   ```

   Enter the CA password privately and retain its independent recovery copy. Follow [LAN TLS setup](setup-lan-server.md) for database URL, original random token key, update public key, and manifest directory. Use packaged `tools/lan-server.ps1 Save` with `-ServerExecutable` pointing to the packaged `eitmad-server.exe`. The certificate assembly is already built; no SDK is needed. Clear secret environment values after capture.
4. With PostgreSQL running, use the exact packaged server for `migrate` and one-time `bootstrap` under the protected deployment environment. Follow [first-owner activation and operational assignments](run-server-authority.md#bootstrap-the-first-owner). Capture activation material privately and activate through the documented Rust HTTP route. Assign the real organization/branch and Manager/Workshop access. The installer creates no account. Never repeat bootstrap after an uncertain result.
5. Check LAN configuration, back up the cluster/settings/TLS/manifests, then stop source Rust and PostgreSQL cleanly. Keep source and destination separate. Import into a **new** ProgramData destination:

   ```powershell
   .\install-server.ps1 Install -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher -LanDirectory C:\ProgramData\EitmadLan -ClusterDirectory C:\PostgresSource\data -PostgresBin C:\PostgreSQL17\bin -WinSwExecutable C:\Installers\WinSW.NET461.exe -WinSwSha256 $reviewedWinSwSha256
   ```

6. Run `install-server.ps1 Start`, then `Status`. Both services must be Running, with `Live` and `Ready` true. Configuration, migration, or TLS failures stop acceptance. The import retains its source; keep that source stopped after cutover to prevent divergent writes.
7. For workshop DHCP clients, run `install-server.ps1 Firewall -PrivateLan`. This scopes HTTPS and discovery to `LocalSubnet` on Private networks. The alternative `Firewall` action without `-PrivateLan` prompts for one client address and replaces the previous single-client rule. Use [the service firewall procedure](run-windows-host-services.md#connect-the-second-pc-yourself) for the chosen scope; PostgreSQL stays loopback-only. Guided setup already applies Private LAN rules.

Protected storage is `%ProgramData%\EitmadHost`: read-only binaries in `app`/`postgres`, server-only settings/TLS in `private`, and PostgreSQL storage in `data`. Separate password-free virtual service accounts run Rust and PostgreSQL. Administrators and SYSTEM retain control. Keep CA operator state outside the service tree. Links, foreign service registrations, active clusters, and unsafe ancestors are rejected. See [service identities and storage](run-windows-host-services.md#identity-secrets-and-storage).

## Test MSIX with a local certificate

Use this path to test MSIX installation without Azure on PCs you control. It does not grant production approval or replace [release acceptance](validate-release-candidate.md). The production signing tools and installer keep their existing checks.

Prerequisites on the build laptop: the authenticated unsigned desktop ZIP, its SHA-256 from the build record, Windows SDK x64 MakeAppx and SignTool, Windows PowerShell 5.1, and the Windows .NET Framework 4.x C# compiler. The input app already includes its .NET 8 runtime and Rust engine; the builder does not copy source code into the package. Client PCs need the [desktop prerequisites](#target-prerequisites).

Build once with new work and output directories:

```powershell
.\deploy\windows\build-test-client.ps1 -SourceArchive $unsignedDesktopZip -ExpectedSourceSha256 $sourceZipDigest -WorkDirectory artifacts\local-msix-test\work -OutputDirectory artifacts\local-msix-test\client
```

The builder verifies the ZIP and both original payload inventories before repacking. It creates `Eitmad.Desktop.Test` with publisher `CN=Eitmad Local Test` and an Arabic test label. A small Windows launcher passes the existing runtime-directory argument to the unchanged shell; both Start-menu activation and direct launcher activation use `%LOCALAPPDATA%\Eitmad\TestClient\engine`. This path retains test data after package removal. Product authorization, validation, and audit stay in Rust. Test accounts can change the connected server tenant; use a test tenant when the data must be disposable.

The builder creates a one-year, non-exportable RSA code-signing key in the build user's `CurrentUser\My` Windows certificate store. It exports only `Eitmad-Local-Test.cer`. Keep that private key on the build laptop. For another build, supply the retained public thumbprint with `-CertificateThumbprint`; changing certificates requires a separate reviewed client trust import. The test MSIX is signed with SHA-256 without a production timestamp. Expiration requires a new valid build and trust check.

Transfer only the output directory as a ZIP. Compare its ZIP digest through the authenticated handover channel. The generated command files contain the exact public-certificate and MSIX SHA-256 pins; `test-release.json` records the same public evidence. These pins are meaningful only when the bundle itself came from the trusted handover.

On each client:

1. Extract the complete bundle. Right-click `Trust-TestCertificate.cmd` and select **Run as administrator**. This verifies both hashes and the code-signing-only leaf certificate, imports only that public certificate into `LocalMachine\TrustedPeople`, then verifies the MSIX signature. If that signature check fails, a newly imported certificate is removed. It never imports a root CA, enables Developer Mode, or changes machine execution policy. Trust applies to all Windows users on that test PC; use it only for this controlled test.
2. Close the Administrator window. Double-click `Install-TestApp.cmd` as the intended normal Windows user. It checks the pinned trusted signature and prerequisites, uses normal `Add-AppxPackage`, and confirms the separate test identity. No unsigned-install switch is used. Each command sets execution policy only for its own PowerShell process.
3. Open **الاعتماد - اختبار** from Start. Follow [discovery and Owner-code pairing](install-windows.md#3-install-and-pair-each-employee-pc) with this test client's own runtime, then sign in with its intended account. The HTTPS server CA is separate from this code-signing certificate; authenticated pairing establishes server trust.
4. Close and reopen the test app and sign in again. Verify retained connection settings and connected work on the actual client. Signature verification on the build laptop alone does not prove client installation or application behavior.

Verify the signature and meaningful rejection cases on a Windows host where the test certificate is already trusted:

```powershell
.\deploy\windows\test-local-signing.ps1 -BundleDirectory artifacts\local-msix-test\client
```

For a hash mismatch, retain the error and obtain the original bundle; do not edit a pin or generated inventory to continue. For signature or trust failure, compare the certificate, clock, and thumbprint. Do not weaken Windows policy or the production installer. If installation fails after trust succeeds, retain the Windows AppXDeploymentServer error and the runtime.

To remove only the test app as its Windows user, close it and run `Get-AppxPackage -Name Eitmad.Desktop.Test | Remove-AppxPackage`. This preserves the separate test runtime. When no users need the signing certificate, an administrator can remove exactly `Cert:\LocalMachine\TrustedPeople\<certificateThumbprint>` from the trusted handover record. Retain the runtime and build key if later test work still needs them. See [Microsoft's certificate trust guidance](https://learn.microsoft.com/en-us/windows/msix/package/create-certificate-package-signing).

## Install and enroll desktops

For normal Client setup, follow [install and pair each employee PC](install-windows.md#3-install-and-pair-each-employee-pc). Use the command below only for separate desktop-package registration. From the extracted desktop package, as the intended ordinary Windows user:

```powershell
.\install-client.ps1 Install -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher
```

Open **الاعتماد** from Start and follow [discovery and Owner-code pairing](install-windows.md#3-install-and-pair-each-employee-pc), then sign in with the user's active account. Manual connection entry under **ربط هذا الجهاز بالمنشأة** is for an already approved device or supported invitation flow; it cannot approve an unknown device. Use [desktop recovery](run-server-authority.md#enroll-or-recover-a-windows-desktop) when needed. Never copy private server files or another device's runtime. Close/reopen and sign in normally without inherited server environment variables.

Rust retains records, pending work, receipts, connection, and device identity in `%LOCALAPPDATA%\Eitmad\engine`, with owner-private ACLs and Windows protected credentials. MSIX declares `unvirtualizedResources` and disables AppData file virtualization so package removal preserves this directory. This uses [Microsoft's persistent AppData mechanism](https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization); verify it on supported Windows baselines before release.

Keep server and clients on the same release. Record both versions before cutover. Rust IPC/server negotiation still rejects incompatible protocols and schemas; package metadata cannot grant authorization or bypass negotiation.

## Repair and data-preserving uninstall

Close desktops and wait for engines to exit. The tools never kill a process to obtain access. Repair uses the **original package, version, publisher, and digest**.

Stop both server services, then run:

```powershell
.\install-server.ps1 Repair -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher
```

Repair replaces the application binary/inventory, runs Rust migrations, and requires HTTPS readiness. It retains the cluster, token key, TLS, manifests, identities, and logs. Missing installed release metadata stops repair; do not invent a baseline for a validation ZIP. It does not fix damaged PostgreSQL files, replace secrets, or repair a foreign service. After service removal, `Install` with the same destination/name reinstalls retained state without importing another cluster. Wrapper/PostgreSQL damage requires reviewed original dependencies and the service recovery procedure.

For the desktop:

```powershell
.\install-client.ps1 Repair -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher
```

Windows redeploys the same signed package and the tool checks registration. It never calls Windows Reset. If Windows cannot repair package files in place, use the data-preserving Uninstall below and reinstall the original package under the same user/publisher. Verify retained work and online sign-in before returning the PC.

`install-server.ps1 Uninstall` removes owned service registrations and its firewall rule, preserving **all** server files. `install-client.ps1 Uninstall` removes this user's package and retains engine data and credentials. Do not select **Reset**, delete runtime files, remove Credential Manager entries, reset Windows passwords, or initialize a database as part of removal. Reinstall the same identity under the same Windows account to resume. Device decommissioning is separate work.

## Controlled manual upgrade

1. Verify new artifacts, signatures, higher numeric versions, compatibility notes, and migration inventory. Reject changed/removed existing migration bytes. Keep original packages and authenticated records. Never downgrade binaries over migrated data.
2. Rehearse the exact next server/client packages against an isolated restored recovery generation. Verify migrations, account sign-in, tenant denial, pending-work replay, receipt idempotency, and confirmed records. Record source/target versions, digests, recovery point, and results. `-RestoreVerified` explicitly attests that this drill passed; the switch cannot prove a restore occurred.
3. Stop new work and close all clients. Take a fresh complete [LAN recovery generation](backup-lan-installation.md), including server/CA state, all runtime/profile volumes, Credential Manager/DPAPI dependencies, and PostgreSQL. Verify the PostgreSQL backup and all Windows image catalogs with `restore-backup.ps1`. Record hashes, cut time, migration baseline, and participant inclusion. Keep clients closed from this recovery point through acceptance; do not omit unsent work.
4. Stop both server services. From the **new** server package, use its digest:

   ```powershell
   .\install-server.ps1 Upgrade -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher -BackupGeneration $completeGeneration -RestoreVerified
   ```

   The tool checks the old inventory, higher version, immutable migration history, stopped services, complete backup/image verification, and this host's inclusion. It retains the previous binary/inventory in `releases`, disables automatic service starts during replacement, records `release-pending.json`, replaces the binary atomically, runs explicit Rust `migrate`, then requires HTTPS readiness. Success restores automatic startup and clears the marker. PostgreSQL major upgrades are separate; the tool never replaces PostgreSQL or its cluster.
5. From each new desktop package, as its original Windows user:

   ```powershell
   .\install-client.ps1 Upgrade -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher -BackupGeneration $completeGeneration -RestoreVerified
   ```

   The client checks publisher, higher version, migration history, complete-generation marker, and this user's runtime inclusion. The full backup/image verification is the preceding operator/server check; the client does not run `wbadmin`. Windows deploys without a forced shutdown or downgrade. Run `Diagnose`, open the app, sign in online, and check retained work and receipt replay.
6. Require server readiness, authenticated work from every client, authorization denial, reconnect, and audit continuity before reopening access. Retain the recovery point and both artifact records.

On failure, preserve state. The server leaves services stopped/demand-start with its pending marker. Ordinary Start/Install/Upgrade refuse a pending release. Correct the cause and run `Repair` from the **exact interrupted target package**; the marker binds its version/digest and preserves the old inventory. Never substitute a different release as a retry. If repair cannot succeed, restore the complete coordinated pre-upgrade point with the original binary and Windows credential dependencies. Blind old-binary replacement can corrupt migrated data. Prefer forward recovery; never reverse migration SQL or discard newer work without an explicit recovery decision.

## Diagnostics and acceptance

Use server `Status` for SCM and validated health routes, client `Status` for package identity/version, and client `Diagnose` for Rust health while the app is closed. Inspect Windows **AppXDeploymentServer/Operational** for MSIX failures and Application events/private rotated logs for service failures. Report only stable error IDs, versions/digests, Windows build, service states, and applicable correlation IDs. Do not attach raw logs, environment dumps, activation output, connection files, credentials, private certificates, or databases.

Missing Redistributable: install the Microsoft prerequisite. Signature/publisher failure: stop and check release provenance. TLS failure: check endpoint name, public CA, expiry, clock, and network. Readiness failure: check PostgreSQL and Rust configuration/migrations. Enrollment denial: check account activation and branch/role assignments. Never bypass trust, authorization, or the firewall to fix these symptoms.

Local verification on 2026-10-19 used validation version `0.1.0` on the development Windows host:

| Proof | Result |
| --- | --- |
| Desktop MSIX and Windows server ZIP builds; MakeAppx manifest validation; payload inventories and excluded-state scan | Passed; unsigned validation artifacts |
| Packaged Rust CLI and diagnostic identity; matching .NET/MSIX identities | `0.1.0`; local IPC `1.28` |
| Python repository/package tests | 19 passed, including dependency graph scope, deterministic archive bytes, migration drift, unsafe versions, and excluded operational state |
| Windows package, existing host, and release lifecycle scenarios | Passed with real temporary files and mocked SCM; tamper rejection, repair, backup gate, failed migration, exact-target recovery, retained work/credential sentinels |
| Rust workspace unit/integration tests and doctests; strict all-target Clippy; formatting | Passed with Rust 1.85.1; environment-dependent tests remain ignored unless explicitly run |
| Complete Windows shell suite | 406 passed, 5 environment-dependent tests skipped; focused adapter tests passed after the version identity change |
| Packaged server with isolated PostgreSQL 17 and TLS | Passed late database startup, loss/recovery with unchanged PID, port conflict, and graceful CTRL_C shutdown |
| Existing enrollment integration with packaged engine | Passed trusted TLS, wrong-CA denial, interrupted setup, reopen, retained state, credential recovery, and the real Windows named-pipe session controller |
| Missing signing configuration | Explicit refusal; binary digest unchanged |
| Documentation and PowerShell parsing | Passed; documentation audit covered 106 pages |

The isolated runtime command is `python deploy/windows/test-runtime.py --postgres-bin <PostgreSQL17-bin> --server <extracted-server-exe> --certificate-tool <built-synthetic-certificate-tool-dll> --root <new-private-evidence-directory> --verify-desktop-enrollment-engine <extracted-engine-exe>`. Build the Windows test project first; the reused enrollment scenario invokes its focused controller test with `--no-build`. Synthetic clusters and keys stay outside packages and compiler output. The test needed an unrestricted process token because PostgreSQL cannot create its restricted child token inside this host's sandbox. It installs no services. This proof is not signed MSIX deployment, rendered UI acceptance, real SCM/reboot behavior, or a coordinated Windows image restore.

Before declaring support, use fresh server/client Windows hosts without developer SDK/runtime state. Record OS builds, prerequisite versions, artifact digests, and signatures. Verify install; reboot without login; TLS readiness; independent desktop enrollment; committed Arabic work and pending local work; repair; data-preserving uninstall/reinstall; backup/restore; higher-version upgrade; interrupted migration/repair; and rejection of altered bytes, wrong publishers, absent backups, changed migrations, and incompatible binaries. Verify ACLs and the unvirtualized runtime after removal. Complete [mandatory release evidence](validate-release-candidate.md). Missing signing, clean hosts, or recovery proof blocks production acceptance, not independent package construction.

Related tasks: [Windows services](run-windows-host-services.md), [LAN trust](setup-lan-server.md), [server authority](run-server-authority.md), [backup and recovery](backup-lan-installation.md), and [operations index](index.md).
