---
title: "Resolve generated contract drift"
description: "Diagnose and safely regenerate Rust-owned JSON schemas, registries, C#/Swift bindings, fixtures, and references."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust contract maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "contract generation paths, commands, outputs, or drift errors change"
keywords:
  - "contract drift detected"
  - "handwritten protocol strings found"
  - "Cannot unmarshal type"
  - "Quicktype"
---

# Resolve generated contract drift

Contract drift means a generated schema, registry, fixture, reference, or native binding no longer matches the Rust authority. Product data is not changed by these checks; stop shipping the affected binding until regeneration and conformance pass.

## Symptoms

- `contract drift detected` with a missing or differing path;
- `handwritten protocol strings found`;
- a C# `Cannot unmarshal type` failure;
- Swift `DecodingError.typeMismatch` or compile failure after a contract edit;
- CI job `rust-and-drift`, `windows-binding`, or `macos-binding` fails.

## Fast checks

1. Confirm the working tree and inspect the Rust change:

   ```powershell
   git status --short
   ```

2. Install the pinned local generator without lifecycle scripts:

   ```powershell
   npm ci --ignore-scripts --prefix crates/contracts/codegen
   ```

3. Reproduce drift and binding parity failures without writing tracked files:

   ```powershell
   npm run contracts:verify --prefix crates/contracts/codegen
   ```

## Causes and resolutions

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| Generated path differs | Rust contract changed without regeneration | Inspect `git diff -- crates/contracts/src` | Run generation, review every output, then rerun conformance |
| Generated path is missing | Output was deleted or new output was not committed | Inspect the path named by the check | Regenerate; do not recreate it manually |
| Handwritten protocol string found | Shell code declared an `eitmad.*` literal | Search the reported non-generated source | Replace it with generated `ProtocolIds` or a generated enum |
| C# or Swift decoder fails on the canonical fixture | Generator output cannot represent the Rust schema or fixture | Run the platform conformance executable | Fix the Rust representation or generator; never patch only one binding |
| Check differs immediately after generation | Generator or newline behavior is nondeterministic | Run generation twice and compare | Fix deterministic export/normalization before committing |
| Binding identifier parity test fails | A platform generator omitted a Rust-catalog identifier | Compare the named generated binding with `protocol-v1.json` | Fix the generator and regenerate both platforms; do not hand-edit one binding |

Regenerate all outputs together:

```powershell
npm run contracts:generate --prefix crates/contracts/codegen
```

Review schema and identifier changes for compatibility. A generated diff is evidence, not approval: verify version/capability policy and security impact before accepting it.

## Verify recovery

Run contract verification, focused Rust tests, and the available native conformance runner. In CI, `rust-and-drift` must pass before `windows-binding` or `macos-binding` starts. Full macOS Swift verification occurs in `macos-binding` CI when Swift is unavailable locally.

Do not revert unrelated user changes, edit generated files, delete product data, or include request payloads, secrets, identities, customer records, or permission graphs in an issue report. Safe escalation evidence includes the changed Rust type, generator version, failed output path, protocol version, capability list, sanitized correlation ID, and exact compiler/decoder error.

The owning capability is [the authoritative contract layer](../developer/subsystems/contract-layer.md). Exact contract behavior is in the [protocol v1 reference](../api/index.md), and compatibility decisions follow [the contract evolution process](../api/evolve-contracts-compatibly.md).
