---
title: "Resolve diagnostic privacy or secret-storage failures"
description: "Contain suspected secret leakage and diagnose redaction, sensitive-debug expiry, native credential-store, or encrypted-fallback failures safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust security and reliability maintainers"
last_verified: "2026-08-19"
review_triggers:
  - "redaction, sensitive-debug, secret backend, incident response, or stable secret error identifiers change"
keywords:
  - "secret in logs"
  - "sensitive debug expired"
  - "eitmad.error.secret-storage-corrupt.v1"
  - "eitmad.error.secret-fallback-key-required.v1"
  - "تسرب سر"
  - "انتهاء التشخيص الحساس"
---

# Resolve diagnostic privacy or secret-storage failures

Treat an actual secret in logs, IPC errors, crash output, configuration snapshots, or audit as a security incident. Stop sharing the artifact, restrict access, and rotate the affected credential through its owning provider. Routine application work may continue only after the security owner confirms scope and containment.

## Symptoms

- a credential-like value appears in JSON diagnostics, a crash report, an IPC error, configuration export, or audit;
- sensitive fields appear after the approved debug expiry;
- `eitmad.error.secret-fallback-key-required.v1`;
- `eitmad.error.secret-storage-unavailable.v1`;
- `eitmad.error.secret-storage-corrupt.v1`;
- Arabic reports such as `تسرب سر` or `انتهاء التشخيص الحساس`.

No Arabic user-facing error copy is implemented. These phrases are support search terms, not approved UI labels.

## Fast checks

1. If a real secret is visible, do not paste it into an issue, chat, or terminal. Record only the artifact type, timestamp, component, event/error ID, correlation ID, platform, and whether sensitive debug was active.
2. Confirm whether the field's `ObservationContract` classifies it as `Metadata`, `Sensitive`, or `Secret`.
3. Check sensitive-debug `expiresAt` against the event timestamp. At equality, the mode is expired even if a caller retained an earlier redaction context.
4. Confirm the selected secret backend from `SecretBackendKind` without listing account names or values.
5. For fallback corruption, preserve the encrypted record and matching key through the restricted security route. Do not open, edit, or publish either.
6. Reproduce only with a synthetic sentinel and the focused tests from the owning subsystem pages.

## Causes and resolutions

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| Undeclared field rejected | Missing or stale observation contract | Compare field name and value kind in Rust | Add a reviewed classification; do not bypass the contract |
| Sensitive field visible before expiry, secret field redacted | Expected temporary sensitive-debug behavior | Verify permission, enable audit, localized warning message ID, access restriction, and expiry | Disable the session when no longer needed and persist the disable audit |
| Sensitive field visible at or after expiry | Redaction bound was bypassed or event time is wrong | Run the copied-context exact-expiry unit test | Route the field through `ObservationContract::redact` with the authoritative event timestamp |
| Free text exists internally but not on the wire | IPC external projection worked | Inspect only stable code and correlation | Replace raw internal construction with typed metadata |
| `secret-fallback-key-required` | Native store probe failed without an approved key | Check OS credential-store availability | Restore native service or use the approved out-of-band unlock path |
| `secret-storage-unavailable` | Selected backend is inaccessible | Check platform service and user-private directory availability | Recover the selected backend; do not silently create a second store |
| `secret-storage-corrupt` | Wrong key, tampering, swapped identifier, truncated file, or unsupported format | Compare only format version and recovery provenance | Restore encrypted backup plus matching key, or rotate the credential |
| Real secret appears anywhere prohibited | Contract or boundary regression | Preserve restricted evidence and identify earliest emitted artifact | Contain, rotate, add a sentinel regression test, and follow security incident handling |

## Verify recovery

Run the observability, contracts, engine, and secret tests. On the affected OS, run the explicit synthetic native lifecycle. Confirm the sentinel is absent from serialized logs, errors, crash reports, audit, configuration snapshots, and persisted fallback bytes. Confirm diagnostic output still contains a usable correlation ID and stable code.

## Escalate safely

Escalate to security and the owning Rust maintainer with sanitized timestamps, correlation IDs, stable event/error IDs, component, app version, platform, backend kind, sensitive-debug status/expiry, and focused test results. Never include secret bytes, fallback keys, raw provider errors, credential-store account listings, plaintext/ciphertext files, customer data, database content, config snapshots containing the suspected value, or unrestricted crash artifacts.

Related authority: [privacy-preserving observability](../developer/subsystems/privacy-preserving-observability.md), [secret storage](../developer/subsystems/secret-storage.md), [configuration redaction](../developer/subsystems/configuration.md), and [ADR-0024](../decisions/0024-native-secret-storage-with-encrypted-fallback.md).
