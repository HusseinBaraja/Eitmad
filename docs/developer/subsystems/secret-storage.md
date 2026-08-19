---
title: "Use Rust-owned secret storage safely"
description: "Store typed secrets in native desktop credential stores with an authenticated encrypted fallback only when native storage is unavailable."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust security and platform-integration maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "secret identifiers, native backends, fallback cryptography, key provisioning, authorization, audit, or platform support changes"
keywords:
  - "eitmad-secret-storage"
  - "Windows Credential Manager"
  - "macOS Keychain"
  - "Linux Secret Service"
  - "FallbackEncryptionKey"
  - "SecretId"
---

# Use Rust-owned secret storage safely

`eitmad-secret-storage` owns secret material and its desktop persistence. It prefers the operating system's credential store and opens the Rust-owned AES-256-GCM fallback only when a native write/read/delete probe fails and the caller supplies a trusted 256-bit key out of band.

## Authority and supported backends

| Platform | Preferred backend | Current verification |
| --- | --- | --- |
| Windows | Windows Credential Manager | Synthetic set/get/delete lifecycle passed on 2026-08-19 |
| macOS | Keychain | Implemented through `keyring`; native lifecycle needs macOS CI verification |
| Linux desktop | Secret Service | Implemented through `keyring` with Rust crypto; native lifecycle needs Linux desktop CI verification |
| Native store unavailable | Rust-owned encrypted files | Lifecycle, corruption, recovery, permission, and ciphertext tests passed on Windows |

Backend selection occurs once in `SecretStore::open`. The write/read/delete native availability probe runs at most once per engine process, and its result is reused by later store construction. If that first probe fails, later opens in the same process remain on the encrypted fallback; restore the platform service and restart the engine before expecting a new native probe. A mid-session native error returns a sanitized availability error; it does not silently switch stores or create split secret state.

Native shells never call these stores for product secrets. Rust uses `SecretId`, composed of a validated `SecretKind` and `SecretReferenceId`, while configuration may carry only the non-secret reference. Raw material has no serde implementation, prints as `[REDACTED]`, is bounded to 2 KiB for consistent desktop credential-store behavior, rejects empty values, and zeroizes owned memory on drop.

## Safe set, get, and delete

`SecretStore::set` creates or replaces one typed value and consumes the `SecretMaterial`. `get` returns `Option<SecretMaterial>` without serialization. `delete` is idempotent and distinguishes `Deleted` from `NotFound`. Errors expose only stable identifiers such as `eitmad.error.secret-storage-unavailable.v1`; provider messages, paths, account names, and secret bytes are discarded.

This is an internal capability, not an IPC command or CLI. Any product command that calls `set` or `delete` must authenticate, authorize through Rust ReBAC, bind the operation to an explicit scope, and commit a redacted audit result at its owning command boundary. Reads require authorization and should produce a security-access audit where the product threat model requires it. Audit identifiers may reference the secret record or a one-way fingerprint, never its value.

## Encrypted fallback invariant

The fallback key must come from a trusted out-of-band deployment or user-unlock flow. It is consumed as `FallbackEncryptionKey`, zeroized on drop, and never read from product configuration, command arguments, logs, audit, crash reports, or the fallback directory. Without that key, native-store failure returns `eitmad.error.secret-fallback-key-required.v1` and storage does not open.

Each fallback record uses:

- AES-256-GCM with a fresh 96-bit random nonce;
- the canonical typed secret identifier as authenticated additional data, preventing ciphertext swaps between identifiers;
- a SHA-256-derived filename, avoiding identifier/path injection;
- a user-private directory and files (`0700`/`0600` on Unix and a current-user ACL on Windows);
- Unix temporary records created as `0600` before any ciphertext is written;
- a cross-process file lock hardened once when the backend opens plus an in-process mutex;
- a Windows user SID resolved once per fallback backend and reused for directory, lock, and temporary-file ACL hardening;
- `/findsid` verification that requires the target path in successful command output instead of trusting process exit status alone;
- a private temporary write, flush, recoverable deterministic backup, and replacement flow.

Ciphertext, nonce, format marker, filenames, and lock metadata are not secret values, but the directory still requires restricted access. A wrong key, modified record, identifier swap, invalid format, or oversized ciphertext returns `eitmad.error.secret-storage-corrupt.v1` without exposing cryptographic details.

## Configuration, IPC, diagnostics, and backup boundaries

Secret material must never enter `ConfigSnapshot`, configuration export, SQLite product tables, IPC errors, normal logs, crash reports, audit payloads, local data exports, or support bundles. `eitmad-configuration` already converts both `Sensitive` and `Secret` definitions to `ConfigReadValue::Redacted`; a secret setting accepts only `SecretReferenceId`.

Fallback files are deliberately outside the product database contract. Database backup/export does not capture them. A production recovery design must coordinate native credential continuity or separately encrypted fallback recovery without copying plaintext into SQLite or support artifacts.

## Arabic-first behavior

No secret-management UI exists. Future Arabic UI must use localized labels, RTL layout, clear destructive confirmation for deletion, and bidi isolation for LTR provider/error identifiers. It must never reveal the secret after set, place it on the clipboard without explicit bounded user action, or use Arabic customer data in secret-store account names.

## Failure modes and recovery

| Error | Meaning | Safe recovery |
| --- | --- | --- |
| `eitmad.error.secret-empty.v1` | Empty material rejected | Supply a real bounded secret through the authorized flow |
| `eitmad.error.secret-too-large.v1` | Material exceeds 2 KiB | Store the intended credential, not a document or payload |
| `eitmad.error.secret-fallback-key-required.v1` | Native probe failed and no fallback key exists | Restore native credential-store availability or supply an approved out-of-band key |
| `eitmad.error.secret-storage-unavailable.v1` | Selected backend operation or ACL verification failed | Preserve identifiers and correlation only; retry after platform recovery; restart before retrying a process-cached failed native probe |
| `eitmad.error.secret-storage-corrupt.v1` | Fallback authentication or format failed | Stop using the value; restore the encrypted record and matching key or rotate through the owning provider |

Never repair ciphertext manually, copy secret bytes into configuration, or print a provider error for diagnosis. Use [privacy and secret leakage troubleshooting](../../troubleshooting/privacy-and-secret-leakage.md).

## Tests and safe extension

Unit tests cover typed lifecycle, required fallback keys, empty/oversized material, encrypted-at-rest bytes, wrong keys, identifier swaps, interrupted replacement recovery, idempotent deletion, redacted debug/errors, Windows SID-result parsing, and Windows ACL enforcement. A `cfg(unix)` test verifies directory mode `0700` and record mode `0600`; run it on Unix CI. An ignored native lifecycle test is run explicitly on supported hosts because it touches the OS credential store with a synthetic value and always attempts cleanup.

```powershell
cargo test -p eitmad-secret-storage
```

```powershell
cargo test -p eitmad-secret-storage tests::os_native_backend_supports_secret_lifecycle -- --ignored --exact
```

Before adding a platform, implement a native probe and lifecycle adapter, keep the fallback key out of config, add platform permission tests, and update [ADR-0024](../../decisions/0024-native-secret-storage-with-encrypted-fallback.md). Related authority: [configuration redaction](configuration.md) and [privacy-preserving observability](privacy-preserving-observability.md).
