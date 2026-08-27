---
title: "Package, sign, update, and roll back Windows desktop"
description: "Build the Windows validation bundle and prepare the future signed MSIX release path without moving update policy out of Rust."
audience: "operations"
page_type: "task"
status: "active"
owner: "release infrastructure maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "Windows packaging, certificate, installer, update adapter, or rollback behavior changes"
keywords:
  - "Windows MSIX signing"
  - "Authenticode"
  - "Windows rollback"
  - "تحديث"
---

# Package, sign, update, and roll back Windows desktop

The current repeatable path builds an unsigned, framework-dependent `win-x64` ZIP for validation. It includes the WPF shell, platform adapter, and Rust engine. It is not a production installer. Production distribution must use MSIX, Authenticode, an approved timestamp service, and a tested native update adapter.

## Build the validation package

Use a clean Windows runner with Rust `1.85`, .NET 8, and locked repository dependencies:

```powershell
python scripts/release/build_artifacts.py windows-desktop --version 0.0.0-ci --output dist
```

The command builds release binaries and writes the ZIP plus a JSON manifest with its byte count and SHA-256 digest. The builder uses fixed ZIP timestamps and sorted paths and excludes PDB debug symbols from the distributable bundle. The manifest contains `signed: false` and `productionEligible: false` by design.

## Prepare a production MSIX

The future release workflow must:

1. Build from a reviewed tag and a clean, locked checkout.
2. Embed one product version in the shell, engine, MSIX identity, signed update manifest, and release record.
3. package the shell, engine, required runtime files, visual assets, protocol handler, and installer declarations as MSIX.
4. Sign every executable and the final MSIX with the organization code-signing certificate. Keep its private key in a hardware-backed signing service, not on a build runner or server.
5. Add a trusted RFC 3161 timestamp and verify the signature, chain, publisher identity, package contents, and SHA-256 digest on a clean Windows host.
6. Install, start, upgrade, repair, and uninstall in a Windows sandbox. Verify Arabic root RTL, mixed `CNC-04` text, named-pipe IPC, engine containment, shutdown, restart limits, and preservation of Rust-owned data.

Certificate rollover needs an overlap release signed by a publisher identity accepted by both old and new installation paths. Do not change the MSIX publisher identity without a migration plan because Windows treats it as a different package authority.

## Host and release an update

Upload the signed MSIX to an immutable HTTPS object URL. Deny overwrite and public listing. Publish the SHA-256 digest and exact byte size in an externally signed `SignedUpdateManifest`. The server stores only the public verification key; the signing private key stays in the release system.

Use `canary` for engineering devices, `beta` for staging, and `stable` for production. Promote the same verified artifact digest between channels. Rust verifies the manifest and selects the channel, compatibility, cohort, and package. The Windows adapter may download, recheck the package digest and Authenticode signature, ask for the required consent, install, and report the typed result. It must not override Rust eligibility.

## Verify

Require all checks in [Validate a release candidate](validate-release-candidate.md). On clean Windows test machines, verify install, update from the oldest supported release, launch, IPC negotiation, Arabic UI, repair, uninstall, and rejection of a changed package byte or untrusted signer.

## Roll back or recover

Pause the signed rollout first. Do not replace the object at an existing URL. Publish a new signed manifest that revokes the faulty release and offers a higher-version recovery build. Windows installer version rules can block an in-place downgrade, so use a forward-fix package unless an approved MSIX downgrade mechanism has been tested.

If the release changed a storage schema, follow its release note. Restore a pre-migration backup only while the engine is stopped and only when the older binary cannot read the new schema. Preserve the failed package, manifest, audit correlation ID, and redacted installer logs for incident review.

## Current gaps

MSIX authoring, production certificate integration, and the Windows native update adapter are not implemented. This gap blocks production desktop distribution. The unsigned ZIP is for CI and engineering validation only.

Related pages: [signed update authority](../developer/subsystems/update-distribution.md), [Windows process supervision](../developer/subsystems/windows-process-supervision.md), and [ADR-0013](../decisions/0013-platform-native-update-adapters.md).
