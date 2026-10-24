---
title: "ADR-0032: Require a self-signed Windows publisher certificate"
description: "Sign Windows installers locally and guide explicit one-time trust for the public code-signing certificate on each PC."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "product developer"
last_verified: "2026-10-24"
review_triggers:
  - "publisher signing, certificate trust, expiry, key retention, or installer elevation changes"
keywords:
  - "self-signed certificate"
  - "publisher trust"
  - "offline signing"
  - "اعتماد الناشر والمتابعة"
---

# ADR-0032: Require a self-signed Windows publisher certificate

> - Status: Accepted
> - Date: 2026-10-24
> - Decision owners: Product developer
> - Supersedes: ADR-0030 and ADR-0031, publisher signing and first-use Client certificate elevation only
> - Superseded by: None

## Context

The product developer requires a self-signed certificate as the only Eitmad Windows publisher signing identity. The delivery requirement remains **"click and run and follow in app guided setup"**, with the two EXEs defined by [ADR-0031](0031-two-self-contained-windows-installers.md).

Windows must explicitly trust this identity before normal EXE, catalog, script, and MSIX verification can succeed. Self-signing does not provide public identity verification, initial download authenticity, or SmartScreen reputation.

## Decision

Create and retain one local, non-exportable RSA 3072-bit signing key in the build user's Windows `CurrentUser\My` store. Its self-signed certificate uses SHA-256, code-signing-only EKU, and `CA=false`. Default validity is three years. Keep the publisher subject stable and use the same exact certificate for Eitmad setup, payload catalogs, and final installers. Preserve valid vendor signatures. No cloud signing account, paid certificate, hardware signing service, or external timestamp is required.

Retain Windows signature verification, exact publisher checks, authenticated review digests, file catalogs, and protected extraction. A self-signed certificate is an explicit trust choice, not permission to accept unsigned, changed, expired, or untrusted files. Catalog and setup signatures must use the same certificate bytes; matching subject names alone are insufficient.

Each outer EXE embeds only the public certificate, with a compiled SHA-256 pin. Before payload extraction, the Windows adapter checks that the embedded certificate is a valid public code-signing leaf and matches the EXE signer. If machine trust is absent, native Arabic guidance shows the publisher and public fingerprint. The user compares the fingerprint with the independent release handover and confirms it. Windows then requests administrator approval to import the public leaf into `LocalMachine\Root` using the protected system `certutil.exe`. Authenticode requires this store for ordinary EXE and script signatures. The certificate is not a CA and contains no private key, but its root-store entry grants Windows trust to code signed by this key. State that scope in the UI.

This one-time trust operation requires administrator approval on both Server and Client PCs. Continue application installation under the original normal user. Client still installs only its per-user desktop package and no host service or global runtime. Existing valid machine trust skips the import step. Never elevate unverified bundled code to establish trust. Rust continues to authenticate catalogs, enforce roles, and own installation policy; the native adapter owns Windows certificate-store mechanics.

The initial Windows publisher or SmartScreen warning can occur before the wizard opens. Distribute the EXE digest and public certificate fingerprint through an independent authenticated handover. Do not disable Windows protection. Do not package private signing keys, exportable PFX files, or provisioned LAN TLS identities. Publisher code-signing trust and workshop HTTPS trust remain separate.

## Consequences

Signing can run locally without a signing network service. Each PC needs one guided administrator trust approval. The build user also needs the public certificate in `CurrentUser\Root` to verify output. Without timestamps, expiry stops acceptance; renewal requires a valid replacement certificate, rebuilt releases, and renewed explicit trust on each PC.

A compromised key can sign code trusted by those PCs. Retain the non-exportable key only on the protected build PC. Replace a lost or compromised key, rebuild affected releases, and remove the retired public certificate from each PC by its exact thumbprint. Do not silently trust a replacement or remove unrelated certificates. The [packaging guide](../operations/package-windows-desktop.md#build-and-sign) owns commands and recovery instructions.

## Verification

Use real Windows signing to prove EXE and SHA-256 catalog acceptance without timestamps. Reject changed bytes, wrong publishers, another key with the same subject, CA certificates, other EKUs, expired certificates, and private-key resources. Inspect the trust screen at the native rendering baselines and verify confirmation, keyboard access, and LTR fingerprint presentation. Clean-PC acceptance must observe trust approval and cancellation, unchanged Client privilege after trust, offline installation, and the existing LAN acceptance path. Focused tests do not establish clean-PC or production acceptance.

Return to the [decision index](index.md).
