---
title: "Build and maintain Windows installation packages"
description: "Build and sign offline Windows bundles, use advanced package installation, and preserve work through repair, removal, and manual upgrade."
audience: "operations"
page_type: "task"
status: "active"
owner: "release infrastructure maintainers"
last_verified: "2026-11-08"
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

Build the server ZIP and desktop installer ZIP from one source revision and version. The desktop ZIP contains an x64 MSIX with the WPF shell, Windows adapter, Rust engine, self-contained .NET runtime, and app-local Microsoft C++ runtime. The server ZIP contains Rust, service installation tools under `deploy/windows`, and the self-contained LAN certificate tool. Backup support is deferred; see [ADR-0033](../decisions/0033-defer-backup-support.md). Neither package contains a database, test account, credential, token, private certificate, or provisioned installation. No automatic updater is added.

Both Windows ZIPs include the canonical `docs/` tree with its repository paths unchanged. For the combined offline bundle, use [the installation guide](install-windows.md). For separate component ZIPs, use the manual procedures on this page. Keep the tree together so linked procedures, reference pages, and heading anchors remain available without a repository checkout. `release.json` inventories these documentation files. External links still require network access. Build and developer-test procedures still require the source checkout and their stated tools.

**Signing requirement:** use a locally created self-signed code-signing certificate for every Eitmad release, including controlled LAN tests. [ADR-0032](../decisions/0032-self-signed-windows-publisher.md) defines this policy. Unsigned artifacts remain engineering probes; the client refuses unsigned MSIX. A cloud account, paid certificate, hardware signing service, and timestamp service are not required. Signed builds retain `productionEligible: false` until [release acceptance](validate-release-candidate.md) passes.

## Build and sign

### Build both installers with one command

From a clean committed checkout, run `build-windows.cmd 0.1.19` at the repository root.
Double-click `build-windows.cmd` to enter the new version in a console instead. Both entry points
use `scripts/release/build_windows.py` to build and sign Server, Desktop, and Setup, authenticate
the combined input inventory, and assemble both offline setup EXEs. The command preserves the
component builder and installer checks described below.

Configure this build PC once in the Git-ignored `artifacts/windows-build/settings.json`:

```json
{
  "previousRelease": "dist/0.1.18.0",
  "previousReviewSha256": "<authenticated SHA-256 of that release's reviewed-inputs.json>",
  "certificateThumbprint": "<retained code-signing certificate thumbprint>",
  "publisher": "CN=Eitmad",
  "compiler": "C:/BuildTools/Inno Setup 7/ISCC.exe"
}
```

Paths can be absolute or relative to the repository root. Use a retained **complete** release
whose `inputs/` distribution trees and authenticated review are available. Obtain the review digest
from the trusted build handover; do not replace it with a newly calculated digest to accept changed
inputs. The settings contain public references only. The private signing key stays in the Windows
certificate store. Keep the Inno compiler outside temporary cleanup directories.

The build PC needs the tools listed below, Inno Setup 7.1.1+ within major version 7, and at least
10 GiB of free space; a fresh compiler cache can require more. Check prerequisites without
compiling or writing release output:

```powershell
.\build-windows.cmd -Version 0.1.19 -Check
```

Preflight checks the committed source state, signing identity, tool signatures and versions,
authenticated prerequisite hashes, executable architectures, and vendor signatures. The selected
Windows version must be higher than the retained release, and `dist/<windows-version>/` must not
already exist. Existing output is never replaced. The build reuses the normal compiler cache.

Success produces `EitmadServerSetup.exe`, `EitmadClientSetup.exe`, the three signed component ZIPs,
`inputs/`, their manifests, `reviewed-inputs.json`, `input-review-provenance.json`, `verification.json`,
`SHA256SUMS.txt`, and `BUILD-RECORD.md` under `dist/<windows-version>/`. Logs are under
`artifacts/windows-build/<windows-version>/`. The final check verifies the exact artifact set,
source and version identities, desktop package contents, publisher signatures, and digests.

On failure, the command stops with a nonzero exit code and a diagnostic or log path. A partial
output directory has no successful final verification record and must not be distributed. Preserve
it for diagnosis; inspect it before choosing a new version or removing that specific incomplete
output. The command does not install applications, download tools, change trust stores, run CI,
or grant production acceptance. Run the separate [release acceptance](validate-release-candidate.md)
gates before distribution. To select a different retained prerequisite set later, update its path
and independently authenticated digest together in the local settings.

### Build individual components

Use x64 Windows, PowerShell 7.4+ with `pwsh.exe` on PATH, Python 3.11+, rustup with Rust **1.85.1**, .NET 8 SDK, and Windows SDK x64 `makeappx.exe` and `signtool.exe`. Build-time signature checks, catalog creation, signing, and asset generation use PowerShell 7. Restore locked dependencies through approved Cargo/NuGet sources. The builder selects the Rust compiler explicitly and reuses `target/toolchain-1.85.1`. It publishes into a new temporary directory, independent of existing runtime data.

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

The desktop builder locates the newest numeric `VC/Redist/MSVC/<version>/x64/Microsoft.VC143.CRT` directory from installed Visual Studio instances through `vswhere.exe`. Use `--vc-runtime <REVIEWED_CRT_DIRECTORY>` to pin another approved redistribution source. It refuses missing files, mixed versions, versions below 14.40, non-x64 DLLs, untrusted Microsoft signatures, and conflicts with published runtime files. It never takes DLLs from System32 or installs a global runtime. `visual-cpp-runtime.json`, the payload inventory, and SBOM retain the exact shipped version and digests. Review redistribution rights and servicing status for the selected source before release.

`0.1.0` is a validation example, not a declared production release. Use one approved semantic version for both artifacts. Three numeric components map to `major.minor.patch.0` in MSIX and .NET file identities. For a Windows revision, use `--version 0.1.15+revision.1`: Rust and inventories retain this valid semantic version, while MSIX and .NET file identities use `0.1.15.1`. Each numeric component must be at most 65535. Revision metadata does not increase semantic-version precedence; this mapping supports the manual Windows package lifecycle. Prerelease labels are recorded in the Rust binary and inventory but do not increase the MSIX numeric version. Use a higher numeric version for every upgrade; never reuse a release version for different bytes.

The builder checks Rust `--version` and runtime diagnostic identity, gives .NET and MSIX the same version, validates migration hashes, and records revision, dirty-source status, architecture, local IPC version, and payload digests. `sbom.cdx.json` is a CycloneDX 1.6 dependency inventory from the resolved Rust normal/build graph and published .NET dependency manifests. It includes build inputs, excludes unrelated/test-only roots, and records exact bundled .NET runtime versions. Static linking can remove code from those inputs; the inventory does not claim that every source function ships. Signed payload digests bind this inventory to the release record. ZIP entries have sorted paths and fixed timestamps. Archive assembly is repeatable for identical payload bytes; compiler and external signing output are not claimed to be bit-reproducible. The `server` target still builds the separate Linux artifact.

Release candidates must come from a reviewed tag and clean checkout. Create the signing identity once on the build PC with PowerShell 7.4+:

```powershell
$signing = pwsh.exe -NoProfile -File deploy/windows/new-signing-certificate.ps1 | ConvertFrom-Json
$env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT = $signing.certificateThumbprint
$approvedPublisher = $signing.publisher
```

The default subject is `CN=Eitmad`. The script creates a three-year, non-exportable RSA 3072-bit SHA-256 key in `CurrentUser\My`. Its certificate permits code signing only and has `CA=false`. It trusts only the public certificate in the build user's `CurrentUser\Root` store so Windows Authenticode can verify the files. This changes the build user's trust store. Retain the private key there; never export it into a PFX, source tree, payload, or release handover. Save the returned public thumbprint, SHA-256 fingerprint, publisher, and expiry in the authenticated release record.

For later builds, set `EITMAD_SIGNING_CERTIFICATE_THUMBPRINT` to the retained public thumbprint and `$approvedPublisher` to the exact certificate subject. Do not create another key for each build. The signing script finds the latest installed Windows SDK x64 SignTool and verifies its Windows signature. It uses the selected local key and SHA-256 without network signing or timestamping.

```powershell
python scripts/release/build_artifacts.py windows-desktop --version 0.1.0 --publisher $approvedPublisher --signing-hook deploy/windows/sign-with-self-signed-certificate.ps1 --output dist
```

Use the same arguments for `windows-server` and `windows-setup`. The hook accepts `-Path` and throws on failure. Eitmad executables, scripts, catalogs, and MSIX must have a valid Windows signature from the selected self-signed certificate and exact publisher. Valid vendor signatures remain. File hashes, authenticated input reviews, and unsigned upstream exceptions retain their existing checks. No signature-bypass switch is added.

Executables are signed before MSIX assembly, then the final MSIX is signed. Packages retain license and notice files from the resolved Rust dependencies and exact published .NET runtime packages. The desktop bundle also provides these files under `docs/third-party`. The publisher must match the signing identity; see [Microsoft package signing](https://learn.microsoft.com/en-us/windows/msix/package/sign-app-package-using-signtool). Keep `Eitmad.Desktop` and its publisher stable. Publisher rollover requires a separately tested identity migration.

Keep packages under `dist`, outside Git. Transfer the ZIP digest and extracted `release.json` digest through an authenticated release channel. A checksum beside an untrusted download is not independent proof. The outer `.manifest.json` remains conservatively unsigned/non-production; a server ZIP also needs the authenticated release channel or approved detached release signature. Never edit generated inventory to pass verification. Package inventory is not the Rust signed update manifest. Do not create automatic-update assignments for this manual path; the [automatic update adapter](../developer/subsystems/update-distribution.md) remains unimplemented.

## Build the two standalone installers

The end-user requirement is **"click and run and follow in app guided setup"**. [ADR-0031](../decisions/0031-two-self-contained-windows-installers.md) requires `EitmadServerSetup.exe` and `EitmadClientSetup.exe`. Each contains its role's complete offline payload and opens native Arabic guidance. ZIPs below are component inputs and internal validation outputs.

Build reviewed desktop, server, and setup components from the same clean current source revision. Prepare and authenticate the full input review described below. Then run:

```powershell
python scripts/release/build_artifacts.py windows-installers --version 0.1.0 --payload-input C:\ReviewedOfflineInputs --payload-review C:\ReviewedRelease\offline-inputs.json --expected-review-sha256 $approvedReviewDigest --publisher $approvedPublisher --signing-hook deploy/windows/sign-with-self-signed-certificate.ps1 --output dist
```

This target requires Inno Setup **7.1.1+ within major version 7**, the local self-signed certificate signing hook, and new destination filenames. Install the official signed compiler, or set `EITMAD_INNO_COMPILER` to its `ISCC.exe`. The builder verifies the compiler's Windows signature and Pyrsys B.V. publisher before executing it, then checks its version. See [Inno Setup downloads](https://jrsoftware.org/isdl.php) and [ADR-0034](../decisions/0034-inno-windows-installation.md).

The builder verifies reviewed components, stages each fixed role, signs its catalog, and compiles `deploy/windows/eitmad.iss`. Inno signs its setup copies, uninstaller, and final EXE through the existing signing hook. Both roles must finish before either final EXE is published. The adjacent manifest records the technology, role, payload inventory digest, installed bundle directory name, catalog digest, and public certificate SHA-256. Authenticate it through the release handover. `productionEligible` remains false until separate release acceptance passes.

Server includes all reviewed components. Client includes the setup, desktop, and complete authenticated application component because the Rust setup backend runs from that component. Client omits PostgreSQL, WinSW, PowerShell 7, C++ redistributable installers, and Framework installers. Its installed MSIX contains no server tools. The temporary setup backend is not a Client host-service installation.

At first launch on each PC, Inno shows its Arabic standard wizard. If the exact publisher certificate is missing from either machine store, its trust page shows the compiled public SHA-256 fingerprint and requires explicit confirmation. Compare it with the independently authenticated handover. Inno checks its embedded certificate hash and invokes protected Windows `certutil.exe` with administrator approval for each missing store: `LocalMachine\Root` and `LocalMachine\TrustedPublisher`. It verifies the signed outer EXE after trust and before installation. It does not elevate unverified payload code. The application setup continues as the original user.

Setup starts installation, elevated verification, workshop preparation, and TLS renewal scripts with process-only `-ExecutionPolicy AllSigned`. Both entry scripts and imported modules must have trusted signatures. Machine publisher trust covers the original user, a different UAC administrator, and the SYSTEM renewal task. Setup does not change saved user/machine execution policy or override Group Policy. A managed script prohibition still stops setup. The signed catalog, inventory, exact publisher, and protected-copy checks remain required.

A self-signed EXE can show an initial Windows publisher or SmartScreen warning. Do not disable Windows protection. Obtain the EXE digest and certificate fingerprint through the independent handover. Inno extracts files using its signed package metadata and registers an installed uninstaller under `%LocalAppData%\Programs\Eitmad\<role>`. The complete catalog-bound payload stays under `bundles/<payloadDirectoryName>`, including files needed for interrupted setup and removal. Setup and Rust retain and authenticate the payload before using it. Rust enforces the role; launch arguments cannot select another role. Server elevation still authenticates a separate protected ProgramData copy.

Inno launches the existing WPF guided setup before showing its result. Cancellation or failure is not successful application installation. Keep the installed **استكمال الإعداد** and **إزالة الاعتماد** shortcuts available for recovery. Do not delete a bundle directory while it can be referenced by a setup journal.

The Inno template has compiled and signed both roles with a small synthetic payload on the build host. This is not a complete new release or clean-PC acceptance. Previous custom installer pairs remain historical evidence only. Record each new pair's current source revision, hashes, and fingerprint in its handover; complete [offline acceptance](verify-offline-windows-setup.md#current-verification-status) before declaring production support.

## Prepare the combined offline payload

The `windows-offline` target creates `eitmad-offline-<version>-win-x64-payload-signed.zip` from prebuilt, reviewed inputs for internal validation. It does not download dependencies, run payload executables, register MSIX, provision a database, or change Windows services, certificate stores, firewall rules, or installed runtimes. Use `windows-installers` above for end-user delivery. The internal `setup/EitmadSetup.exe` requires an explicit `--role server` or `--role client`; Rust accepts only the role in authenticated metadata. The full internal bundle defaults to Server metadata. Assembly does not grant production acceptance.

Use signed desktop, server, and setup ZIPs from the same release version and clean source revision. Extract them without changing their inventories. Include reviewed redistribution notices before creating and signing those component packages. Supply the following input tree; the collector copies these paths unchanged:

| Input path | Required reviewed content | Installation role |
| --- | --- | --- |
| `setup` | Self-contained x64 `EitmadSetup.exe`, native WPF resources, .NET 8 Windows Desktop runtime, approved signatures, and redistribution notices | Launcher for both roles; installs no host component for Client |
| `client/desktop` | Existing desktop ZIP contents: signed MSIX, installation scripts, and offline documentation; retain self-contained .NET and app-local x64 Microsoft C++ runtime | Client and Server |
| `server/application` | Existing server ZIP contents, including the self-contained LAN certificate tool and workshop preparation scripts | Server only |
| `server/postgresql` | Complete reviewed PostgreSQL 17 x64 `bin`, `lib`, and `share`; include initialization, service, and SQL tools | Server only |
| `server/winsw` | Reviewed `WinSW.NET461.exe` version 2.12.0 and redistribution terms | Server only |
| `server/powershell` | Extracted self-contained PowerShell 7.4+ x64 distribution, including its runtime and notices | Server only |
| `server/vc-redist` | Reviewed Microsoft C++ 14.40+ x64 offline installer named `vc_redist.x64.exe` and redistribution terms | Server only |
| `server/netfx` | Reviewed .NET Framework 4.8 full offline installer named `ndp48-x86-x64-allos-enu.exe` and redistribution terms for the WinSW Framework build | Server only |

Windows PowerShell 5.1, Windows `curl.exe` with Schannel, supported patched x64 Windows build 19041+, local NTFS, and MSIX policy remain OS prerequisites. They are recorded in `payload.json`. Do not copy system executables from an installed machine or bundle development toolchains. Setup verifies the whole distribution for both roles. After the one-time publisher trust approval, Client installs only the per-user MSIX and runs no Server prerequisite installer or elevated application install. Server uses its authenticated paths and installs missing C++ and Framework runtimes quietly with restart disabled. Exit code 3010 stops setup for an operator-controlled restart. PowerShell 7 remains an extracted app-local runtime for packaged operator tools; use `server/powershell/pwsh.exe` explicitly when those tools require it.

### Review manifest and trust

Keep the input review JSON **outside** the payload input tree. Authenticate its SHA-256 through the approved release channel before collection. Do not calculate a new approval digest from an untrusted download merely to make validation pass. No vendor versions, hashes, or licenses are inferred from installed software or accepted from filenames.

The review format has four root fields: `schemaVersion` (`1`), `version` (the exact Eitmad semantic version), `architecture` (`x64`), and `components`. The `components` object must contain exactly `setup`, `desktop`, `application`, `postgresql`, `winsw`, `powershell`, `vc-redist`, and `netfx`. Each component has these fields:

| Field | Review requirement |
| --- | --- |
| `version` | Exact reviewed product version. Eitmad components match the release; PostgreSQL uses `17.minor`; WinSW is `2.12.0`; PowerShell is 7.4+ within major 7; C++ is 14.40+ within major 14; Framework is `4.8` |
| `provenance` | Object with `url` (HTTPS acquisition URL) and `sha256` (lowercase 64-digit source archive or installer digest). Use a version-specific immutable release URL. Review binds these acquisition records to the extracted file list; collection does not fetch the source archive again |
| `licenses` | Nonempty array of objects with `name`, `version`, `expression` (SPDX expression or reviewed `LicenseRef-` identifier), and `files` (nonempty array of relative paths to shipped terms/notices). Include all redistributed inputs, including .NET, the Windows Desktop runtime, Microsoft C++, and transitive vendor libraries. Add terms before creating the original signed package inventory |
| `files` | Exact component-relative file map. Each entry has lowercase `sha256`; every EXE/DLL also declares `architecture`: `x64` or verified IL-only `anycpu`. PostgreSQL native files and runtime entry points must be x64. The C++ x64 installer and Framework offline installer can declare `x86` for their bootstrappers; this does not permit x86 runtime DLLs |
| Executable trust | Every EXE, DLL, PowerShell script/module, and MSIX file entry has the exact trusted certificate subject in `publisher`. Windows must report a valid Authenticode signature, matching subject, and an unexpired self-signed code-signing certificate for Eitmad content. No timestamp is required. Only PostgreSQL and WinSW can instead specify `unsignedReason`, documenting reviewed unsigned upstream content bound to its authenticated hashes. Both fields together are refused |

For example, an unsigned upstream WinSW file entry uses `sha256`, `architecture: "anycpu"`, and a nonempty `unsignedReason`. A signed Microsoft installer uses `sha256`, its verified PE `architecture`, and the exact Microsoft certificate subject in `publisher`. These are schema examples, not approved payload pins. There is no signature-bypass switch. Trusted signature validation requires Windows; an offline build host must already have the required trust-chain and revocation evidence. Unknown trust fails preparation. See Microsoft's [signature inspection](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.security/get-authenticodesignature) and [PE format](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format).

The collector rejects absent components, missing required tools, changed or unlisted files, unsafe Windows paths, links/junctions/hard links, opaque nested archives, and architecture mismatches. It checks the Eitmad release inventories and the desktop's inner MSIX inventory and runtime hashes. It refuses operational databases, cluster directories, credentials, certificates, keys, and device identity files even when listed in the review. Start with clean vendor distributions and clean package output, never an operational installation. Redistribution review remains responsible for classifying the allowlisted bytes; filenames cannot detect secrets deliberately hidden inside an approved binary.

Client MSIX validation also refuses external package dependencies and server tools, including PostgreSQL dump/restore/control executables and service wrappers. The native package verifier checks the self-contained .NET/Windows Desktop host files and runtime configuration in addition to app-local C++. A signed hash alone does not establish that an artifact meets the Client role. Follow [offline setup acceptance](verify-offline-windows-setup.md) for repeatable isolated checks and exact clean-PC/reboot steps; these checks do not install anything on the development machine.

### Collect and recheck without installation

During guided setup, the WPF adapter and Rust backend keep payload files open against writes and deletion. The embedded verifier authenticates the catalog and its publisher, reads the catalog inventory against an empty temporary directory, and verifies the root `release.json` hash from that inventory. It then checks every retained file's raw SHA-256 against this authenticated release inventory and rejects missing or unexpected files. The empty-directory catalog result alone never accepts a payload. This avoids incompatible file sharing in `Test-FileCatalog` and preserves the distinction between Windows PE catalog hashes and raw file hashes. A preflight failure leaves installation state untouched; preserve the failed release and use a newly reviewed, signed installer instead of editing extracted files.

On the release build host, set `$approvedReviewDigest` to the independently authenticated review digest, then run:

```powershell
python scripts/release/build_artifacts.py windows-offline --version 0.1.0 --prepare-payload --payload-input C:\ReviewedOfflineInputs --payload-review C:\ReviewedRelease\offline-inputs.json --expected-review-sha256 $approvedReviewDigest --output dist\eitmad-offline-payload
```

The optional `--prepare-payload` mode creates a new verified directory with `setup/`, `client/`, `server/`, `reviewed-inputs.json`, `setup-bundle.json`, and `payload.json`. The latter records role paths, component versions, provenance, shipped license files, file hashes, included .NET runtime versions, and the retained desktop C++ inventory. Source files remain unchanged. The collector validates the source and staged copy before publication. It refuses an existing destination and leaves no completed output after validation failure.

For the final signed distribution, use a clean reviewed checkout and the retained local self-signed certificate:

```powershell
python scripts/release/build_artifacts.py windows-offline --version 0.1.0 --payload-input C:\ReviewedOfflineInputs --payload-review C:\ReviewedRelease\offline-inputs.json --expected-review-sha256 $approvedReviewDigest --publisher $approvedPublisher --signing-hook deploy/windows/sign-with-self-signed-certificate.ps1 --output dist
```

The builder retains component inventories and SBOMs, creates an aggregate CycloneDX SBOM and root `release.json`, then creates and signs `offline.cat`. The catalog authenticates the complete extracted tree, including review, setup metadata, licenses, runtime files, MSIX, and inventories. Windows catalog version 2 uses SHA-256; see [Microsoft file catalogs](https://learn.microsoft.com/powershell/module/microsoft.powershell.security/new-filecatalog). The builder checks the trusted catalog signature, exact self-signed publisher certificate shared with setup, and contents before assembling a sorted ZIP with fixed entry timestamps. Failure publishes no completed ZIP. Identical staged bytes produce identical archive bytes; catalog generation and external signing are not claimed to produce identical bytes between builds. Without `--signing-hook`, the ZIP is labeled `unsigned` and setup rejects its catalog. Component signature checks remain mandatory.

The launcher retains read-only handles and checks the catalog before starting Rust. Rust verifies it before reading setup metadata. Server elevation authenticates the source again, copies it into a new administrator/SYSTEM-only `%ProgramData%/EitmadSetupPayload-<id>` directory, and verifies that copy before executing its backend. Rust binds tool paths and release pins to that copy and verifies it again before installation. Elevated execution and DLL loading use the protected copy. This release tree is retained for scheduled TLS renewal and interruption recovery; do not remove it while the renewal task references its backend. The outer installer handles the explicit publisher trust step before this workflow. The internal setup does not silently trust certificates; unsigned-MSIX rejection remains enabled.

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

To verify real local signing after building the setup project:

```powershell
pwsh.exe -NoProfile -File deploy/windows/test-self-signed-signing.ps1
```

This proof creates a temporary certificate and key in the build user's stores, verifies signed EXE and catalog acceptance without timestamps, and rejects changed bytes, a wrong publisher, and another key with the same subject. It removes only its own test key, trust entry, and files. It does not import machine trust, exercise UAC, install the app, or establish clean-PC acceptance.

Use `-CertificateThumbprint $env:EITMAD_SIGNING_CERTIFICATE_THUMBPRINT` to reuse the retained signing certificate and its existing build-user trust. This option imports no certificate and preserves the retained key. The proof also verifies a signed PowerShell module beyond the normal Windows path limit used by nested payload staging.

When the retained signing certificate expires, create a replacement, rebuild and sign the release, and repeat the guided trust check on every PC. Without timestamps, expiry stops signature acceptance. After all installed packages have moved to a replacement, an administrator can remove only the retired public thumbprint from `Cert:\LocalMachine\Root`. Remove the build-user public trust entry from `Cert:\CurrentUser\Root` when retired; preserve the private build key until its retention requirement ends. A lost or compromised key needs a new certificate, reviewed rebuilds, and explicit removal of the old trust entry on every PC. Do not change the publisher DN during routine renewal; test publisher identity migration separately if it must change.

## Target prerequisites

| Machine | Requirements |
| --- | --- |
| Every target | Supported, patched x64 Windows on local NTFS; build 19041+; x64 Windows PowerShell 5.1; sufficient disk space; optional disk encryption |
| Desktop | MSIX permitted by local policy; self-signed publisher trusted once per PC through the guided administrator approval; install as each intended ordinary Windows user; no elevated application install, global C++ runtime installation, Rust compiler, or .NET SDK required |
| Server | Target hosts are Windows Server 2022/2025 or Windows 11; elevation for services; `curl.exe`; Microsoft Visual C++ 2015–2022 x64 Redistributable 14.40+ and patched .NET Framework 4.8 for WinSW. Guided setup supplies reviewed PostgreSQL 17 and WinSW 2.12.0, installs missing C++/Framework prerequisites offline, and provisions the cluster. Manual import requires those prerequisites, a provisioned stopped cluster, reviewed tool digests, and PowerShell 7.4+ for manual LAN settings. Guided setup also registers the desktop MSIX; Server-edition AppX prerequisites and the complete guided path still need clean-edition acceptance |

Use [Microsoft's supported Redistributable](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) for the server. The desktop installer verifies the app-local C++ runtime inside the authenticated MSIX before registration; it does not require the machine-wide runtime registry entry. Microsoft [supports local deployment](https://learn.microsoft.com/en-us/cpp/windows/deployment-in-visual-cpp?view=msvc-170), but Windows Update does not service those copies. Rebuild and distribute the package for C++ and .NET runtime security fixes. The tools do not download prerequisites or disable security policy. Guided publisher trust imports the public code-signing leaf into the Windows Root store; it does not import a LAN TLS CA or private key. Guided setup derives the server endpoint from the Windows computer name and a private IPv4 address; keep the name resolvable on the LAN. Reserve a stable address for the manual fixed-endpoint path. The host must stay awake; review [power and reboot requirements](run-windows-host-services.md#sleep-power-and-reboot-requirements).

On 2026-10-20, an unsigned desktop validation ZIP built successfully with Rust 1.85.1 and self-contained .NET 8. Its outer inventory, inner MSIX inventory, ten app-local CRT files, and SBOM version `14.44.35211.0` were verified without package registration. Focused tests rejected changed and missing DLLs and verified that the per-user platform check does not query the global C++ runtime registry. A clean offline PC without a global C++ runtime remains **Needs verification**. This result does not prove production signatures or complete guided onboarding.

## Install the server

For a fresh workshop, use [guided Server setup](install-windows.md#1-install-the-server-pc). The following procedure is an **advanced manual import** from a separate server ZIP. It does not run the guided first-desktop or renewal flow. Use it only when an operator owns those separate tasks. The [preparation backend](prepare-windows-workshop.md) can create the stopped cluster and prepared LAN root instead of manual database and identity provisioning; do not bootstrap that prepared workshop again.

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

## Remove an Inno installation before a fresh setup

Close the desktop with **إنهاء الاعتماد** and close guided setup. Under the Windows user who installed it, open **إزالة الاعتماد** from the role's Start menu folder, or use its role-specific Windows Installed apps entry. Do not launch the uninstaller as administrator. Server removal requests administrator approval for its separate machine phase and interrupts connected employee PCs.

Approve Server removal with the Windows administrator account used for workshop preparation. The prepared directory remains private to that account and SYSTEM. Installed host directories use Administrators/SYSTEM ownership. Removal checks each policy separately; do not broaden permissions to bypass an ownership failure.

Inno first runs the signed native removal adapter. It checks both service owners, removes `EitmadServer` and `EitmadPostgres` through SCM, removes their Event Log sources, the owned `Eitmad-TlsRenewal` task, and owned firewall rules. It waits for service deletion; pending deletion requires a restart and retry, not manual registry removal. It then removes this user's MSIX after checking its publisher. Inno removes its tracked payload files, setup shortcuts, uninstall entry, and `HKCU\Software\Eitmad\Installation\<role>` keys. Client removal never performs the machine phase.

Records, settings, private recovery material, and setup journals are renamed into private `%LocalAppData%\EitmadRetained\<unique-id>` and, for Server, protected `%ProgramData%\EitmadRetained\<unique-id>`. The active `Eitmad`, `EitmadHost`, `EitmadPreparedWorkshop`, and setup-journal paths are cleared. Installed binaries and authenticated setup caches are removed; data files and native credentials referenced by retained records remain. Shared .NET/C++ prerequisites and publisher trust remain installed. Unauthenticated partial caches are preserved for inspection rather than guessed safe to delete.

A later install starts fresh and does not restore retained records automatically. Retention is not a backup or restore feature. Do not copy a retained database or identity into a new workshop. A cleanup error stops removal before Inno deletes the signed tools; correct the reported process, service, permission, or integrity problem and retry.

The manual `install-server.ps1 Uninstall` and `install-client.ps1 Uninstall` commands above keep their narrower repair semantics. Use the Inno entry point for complete removal and a fresh setup. Older custom installer EXEs do not gain this uninstaller until a new authenticated release is built and installed.

Validate native removal without changing real installation state:

```powershell
powershell.exe -NoProfile -File deploy/windows/test-uninstallation.ps1
```

Windows PowerShell 5.1 is required for this proof's Framework ACL APIs and matches the runtime adapter. On a build host with the retained signing certificate, compile and sign both real Inno templates with a small synthetic payload, without installing them:

```powershell
pwsh.exe -NoProfile -File deploy/windows/test-inno-package.ps1 -Compiler <Inno-Setup-7-ISCC.exe> -SigningCertificateThumbprint <retained-public-thumbprint>
```

## Controlled manual upgrade

End users can run the newer role-specific EXE and follow [Update an installed PC](install-windows.md#update-an-installed-pc). The guided server path validates ownership, stops both services, and selects the existing Upgrade or exact-target Repair path automatically. The commands below remain available for operators. Client `Install` also selects Install, exact-release Repair, or higher-version Upgrade from the current user's registration; explicit Repair and Upgrade still enforce their respective version rules.

1. Verify new artifacts, signatures, higher numeric versions, compatibility notes, and migration inventory. Reject changed or removed existing migration bytes. Keep original packages and authenticated records. Never downgrade binaries over migrated data.
2. Stop new work and close all clients. Keep them closed through acceptance. Preserve all existing server and desktop data. The current product has no backup or restore workflow.
3. Stop both server services. From the **new** server package, use its digest:

   ```powershell
   .\install-server.ps1 Upgrade -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher
   ```

   The tool checks the old inventory, higher version, immutable migration history, and stopped services. It retains the previous binary and inventory in `releases`, disables automatic service starts during replacement, records `release-pending.json`, replaces the binary atomically, runs explicit Rust `migrate`, then requires HTTPS readiness. Success restores automatic startup and clears the marker. PostgreSQL major upgrades are separate; the tool never replaces PostgreSQL or its cluster.
4. From each new desktop package, as its original Windows user:

   ```powershell
   .\install-client.ps1 Upgrade -ExpectedReleaseSha256 $releaseDigest -Publisher $approvedPublisher
   ```

   The client checks publisher, higher version, and immutable migration history. Windows deploys without a forced shutdown or downgrade. Run `Diagnose`, open the app, sign in online, and check retained work and receipt replay.
5. Require server readiness, authenticated work from every client, authorization denial, reconnect, and audit continuity before reopening access. Retain both artifact records.

On failure, preserve state. The server leaves services stopped/demand-start with its pending marker. Ordinary Start/Install/Upgrade refuse a pending release. Correct the cause and run `Repair` from the **exact interrupted target package**; the marker binds its version/digest and preserves the old inventory. Never substitute a different release as a retry. If repair cannot succeed, preserve the database and retained release files for operator investigation. Blind old-binary replacement can corrupt migrated data. Prefer forward recovery; never reverse migration SQL or discard newer work without an explicit recovery decision.

## Diagnostics and acceptance

Use server `Status` for SCM and validated health routes, client `Status` for package identity/version, and client `Diagnose` for Rust health while the app is closed. Inspect Windows **AppXDeploymentServer/Operational** for MSIX failures and Application events/private rotated logs for service failures. Report only stable error IDs, versions/digests, Windows build, service states, and applicable correlation IDs. Do not attach raw logs, environment dumps, activation output, connection files, credentials, private certificates, or databases.

Missing Redistributable: install the Microsoft prerequisite. Signature/publisher failure: stop and check release provenance. TLS failure: check endpoint name, public CA, expiry, clock, and network. Readiness failure: check PostgreSQL and Rust configuration/migrations. Enrollment denial: check account activation and branch/role assignments. Never bypass trust, authorization, or the firewall to fix these symptoms.

Use [current setup verification](verify-offline-windows-setup.md#current-verification-status) for observed packaging and signing results and the remaining acceptance limits. Keep release-specific checksums and source revisions with the generated installer handover, rather than copying old validation records into installation guidance.

The isolated runtime command is `python deploy/windows/test-runtime.py --postgres-bin <PostgreSQL17-bin> --server <extracted-server-exe> --certificate-tool <built-synthetic-certificate-tool-dll> --root <new-private-evidence-directory> --verify-desktop-enrollment-engine <extracted-engine-exe>`. Build the Windows test project first; the reused enrollment scenario invokes its focused controller test with `--no-build`. Synthetic clusters and keys stay outside packages and compiler output. The test needed an unrestricted process token because PostgreSQL cannot create its restricted child token inside this host's sandbox. It installs no services. This proof is not signed MSIX deployment, rendered UI acceptance, real SCM/reboot behavior, or a coordinated Windows image restore.

Before declaring support, use fresh server/client Windows hosts without developer SDK/runtime state. Record OS builds, prerequisite versions, artifact digests, and signatures. Verify install; reboot without login; TLS readiness; independent desktop enrollment; committed Arabic work and pending local work; repair; data-preserving uninstall/reinstall; higher-version upgrade; interrupted migration/repair; and rejection of altered bytes, wrong publishers, changed migrations, and incompatible binaries. Verify ACLs and the unvirtualized runtime after removal. Complete [mandatory release evidence](validate-release-candidate.md). Missing signing, clean hosts, or recovery proof blocks production acceptance, not independent package construction.

Related tasks: [Windows services](run-windows-host-services.md), [LAN trust](setup-lan-server.md), [server authority](run-server-authority.md), [backup and recovery](backup-lan-installation.md), and [operations index](index.md).
