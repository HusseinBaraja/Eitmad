---
title: "ADR-0024: Prefer native secret stores with an encrypted Rust fallback"
description: "Records why Rust uses desktop credential stores first and permits an authenticated encrypted fallback only with an out-of-band key."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture and security maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "supported platforms, native credential stores, fallback cryptography, key provisioning, or secret recovery changes"
---

# ADR-0024: Prefer native secret stores with an encrypted Rust fallback

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-08-19
> - Decision owners: Architecture and security maintainers
> - Last verified: 2026-08-19
> - Review trigger: Supported platforms, native credential stores, fallback cryptography, key provisioning, or secret recovery changes
> - Supersedes: None
> - Superseded by: None

## Context and problem

The Rust engine needs durable credentials for external services and future authenticated product boundaries. Product configuration, SQLite, shell preferences, environment-driven logging, and plaintext files are unsafe secret stores. Some supported desktop sessions may lack a usable native credential service, so failing without an explicit fallback would block legitimate deployments.

## Decision drivers

- Keep secret material inside Rust and outside product/configuration data paths.
- Use operating-system protections and lifecycle integration where available.
- Avoid a misleading encrypted fallback whose key is stored beside its ciphertext.
- Preserve deterministic, typed, testable set/get/delete behavior across platforms.

## Considered options

- **Native store first with an out-of-band-key encrypted fallback:** selected.
- **Native store only:** strongest platform integration, but unavailable in some headless or damaged desktop sessions.
- **Rust encrypted files with a colocated key:** rejected because filesystem compromise reveals both key and ciphertext.
- **Secrets in configuration or SQLite:** rejected because snapshots, exports, backups, migrations, and diagnostics would gain secret-bearing paths.
- **Shell-owned secret storage:** rejected because shells are thin untrusted adapters and must not use product credentials.

## Decision

Rust MUST probe and prefer Windows Credential Manager, macOS Keychain, or Linux Secret Service. It MAY open the encrypted Rust fallback only when the native probe fails and a trusted 256-bit key arrives through an approved out-of-band unlock or deployment path. That key MUST NOT be persisted beside ciphertext or placed in product configuration.

Secret operations use typed `SecretId` values and non-serializable, zeroizing `SecretMaterial`. Backend errors are stable and sanitized. Backend selection is fixed for the store lifetime; an operation failure MUST NOT trigger silent mid-session failover.

The fallback uses AES-256-GCM, a fresh nonce, the typed identifier as authenticated additional data, opaque derived filenames, user-private permissions, locking, and recoverable replacement. Native shells receive only deliberate non-secret references.

## Architecture and contract impact

`eitmad-secret-storage` is a Rust infrastructure capability with a narrow set/get/delete surface. `eitmad-contracts::secrets` owns typed non-secret identifiers, but secret lifecycle is not exposed through protocol v1. A future product command must add explicit capability negotiation, ReBAC permission, scope, audit, stable errors, and shell-safe response contracts without returning material.

## Storage, backup, and recovery impact

Native credential stores and fallback files are separate from the Rust product database. Product database backup/export excludes them. Recovery must preserve native credential continuity or restore encrypted fallback state with the matching separately protected key. Manual ciphertext repair and plaintext migration are prohibited.

## Security impact

Secret material is absent from configuration snapshots, routine logs, IPC errors, crash reports, audit payloads, and local data export. Rust-owned wrappers zeroize memory and redact debug output. The fallback protects confidentiality and integrity only while its key remains separately protected; losing both backend access and key requires credential rotation.

## Arabic UX impact

No UI is implemented. Future Arabic unlock, replace, and delete workflows require RTL layout, localized warnings, bidi isolation for stable identifiers, accessible confirmation, and no post-set reveal by default.

## Consequences and tradeoffs

### Positive

- Desktop-native protection is the default.
- Headless or unavailable-native deployments have an explicit secure recovery path.
- Typed references prevent path injection and raw-secret configuration.

### Negative

- Native behavior and permissions require platform-specific CI.
- Out-of-band fallback-key provisioning and recovery remain deployment responsibilities.
- Secret state is not covered by ordinary SQLite backup.

### Risks

- A caller could ignore authorization/audit obligations because the storage API is internal; mitigate by exposing future product access only through an owning authorized vertical.
- Platform services can change behavior; mitigate with synthetic native lifecycle tests and fail-closed backend selection.
- Fallback-key loss makes ciphertext unrecoverable; mitigate with approved separately protected recovery or provider rotation.

## Verification

Windows native lifecycle, encrypted fallback lifecycle, wrong-key, tamper/identifier-swap, interrupted replacement, permissions, bounds, redacted debug, and leakage tests passed on 2026-08-19. macOS Keychain and Linux Secret Service native lifecycle remain required CI evidence before release on those platforms.

## Follow-up work

- Define the first product-owned secret command, permission, scope, and atomic audit boundary.
- Define deployment-specific fallback-key provisioning, rotation, backup, and disaster recovery.
- Add macOS and Linux native lifecycle jobs.
- Threat-model any future secret import, clipboard, or interactive unlock UI.

## Related decisions and documents

- [Secret storage authority](../developer/subsystems/secret-storage.md)
- [Privacy-preserving observability](../developer/subsystems/privacy-preserving-observability.md)
- [ADR-0009](0009-zero-trust-security-model.md)
- [ADR-0012](0012-privacy-preserving-observability.md)
- [Configuration authority](../developer/subsystems/configuration.md)
