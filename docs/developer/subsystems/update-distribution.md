---
title: "Publish and evaluate signed updates safely"
description: "Understand signed manifests, trusted keys, channels, staged rollout, compatibility, package metadata, authorization, and recovery."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "release infrastructure maintainers"
last_verified: "2026-08-24"
review_triggers:
  - "manifest signing, channels, rollout, compatibility, package hosting, key rotation, or update policy changes"
keywords:
  - "SignedUpdateManifest"
  - "Ed25519 update manifest"
  - "staged rollout"
  - "EITMAD_SERVER_UPDATE_PUBLIC_KEY"
---

# Publish and evaluate signed updates safely

The update plane hosts immutable Ed25519-signed manifests. The Rust update policy verifies signatures and decides eligibility; platform adapters may install the selected package but cannot choose a channel, percentage cohort, compatibility exception, or trusted key.

## Authority and boundaries

| Concern | Rust authority |
| --- | --- |
| Manifest, compatibility, rollout, package, client, and outcome contracts | `crates/contracts/src/updates.rs` |
| Signature verification and client eligibility | `crates/update-policy/src/lib.rs` |
| Authorized publication, channel rules, and manifest repository | `server/update-plane/src/lib.rs` |
| Device channel assignment | `server/control-plane/src/update_assignment.rs` |
| HTTP check and publication routes | `server/host/src/http.rs` |

The server stores no update signing private key. Release tooling signs canonical JSON bytes outside the host. The host receives a signing key ID and Ed25519 public key, verifies every manifest before publication, and returns the signature with the manifest so the engine can verify it again.

## Manifest and channel invariants

A schema version `1` manifest contains one channel, release version, publication time, staged rollout, compatibility rules, one or more package records, and an explicit revocation flag. Package metadata contains platform, architecture, package kind, HTTPS URL, non-zero byte size, and a lowercase 64-character SHA-256 digest.

The built-in channels are `stable`, `beta`, and `canary`. `stable` rejects semantic-version prereleases. A channel cannot contain duplicate manifest IDs or duplicate release versions. Platform, architecture, and package kind triples must be unique inside one manifest. Signed manifests are immutable JSON files under the dedicated manifest directory.

## Eligibility order

The engine-side policy verifies the Ed25519 signature and trusted key first. It then checks exact channel, revocation, pause, rollout start, whether the release is newer, current-version bounds and blocks, minimum protocol, required capabilities, deterministic device cohort, and exact platform/architecture package availability.

Protocol or capability mismatch, a blocked client, version outside the compatibility range, or missing package returns `Incompatible`. Pause, future start, channel mismatch, revocation, or cohort exclusion returns `Ineligible`. An equal or newer client returns `UpToDate`. Only a valid signed and compatible release returns `Available` with the selected package metadata.

## Authorization, scope, and audit

`POST /v1/updates/check` requires protocol `1.5`, the update-distribution capability, and an authenticated device proof. The profile device must match the session and its requested channel must match the Rust-owned effective device assignment. Manifest publication is limited to the configured `EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID`, a built-in channel, the publish action, and an explicit `eitmad.permission.server.update-manifest.publish.v1` relationship. It writes a redacted audit outcome. A failed publication audit removes the newly inserted manifest.

Manifest reads do not grant product access. Package URLs are metadata, not authorization credentials. A CDN or artifact service must enforce its own bounded distribution policy without changing the signed manifest.

## Configuration and key rotation

The combined host requires:

- `EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY`: dedicated durable manifest directory;
- `EITMAD_SERVER_UPDATE_OPERATOR_TENANT_ID`: operator tenant that owns global update distribution;
- `EITMAD_SERVER_UPDATE_KEY_ID`: open identifier for the trusted signing key;
- `EITMAD_SERVER_UPDATE_PUBLIC_KEY`: base64-encoded 32-byte Ed25519 public key.

To rotate a key, deploy a trusted-key overlap before publishing with the new key. The current combined host accepts one configured key, so multi-key rollout requires extending `ServerConfig` to load a reviewed key ring before the first new-key manifest is published. Do not replace a key in place while clients still require old manifests.

## Failure and recovery

| Failure | Preserved state | Safe action |
| --- | --- | --- |
| Invalid signature or unknown key | No manifest is published | Verify canonical bytes, key ID, and release signing pipeline |
| Invalid channel or package metadata | Existing channel files remain | Correct the manifest and sign a new immutable manifest |
| Incompatible client | No package is selected | Publish a compatible release or upgrade through a supported intermediate version |
| Paused, revoked, or outside rollout | Client keeps its current release | Resume or replace policy through a newly signed manifest |
| Repository unavailable | Existing files remain authoritative | Restore directory access; do not copy unsigned JSON into the directory |
| Publication denied | No release state changes; denial is audited | Grant the approved owner relationship or use the release workflow |

Use [server-plane troubleshooting](../../troubleshooting/server-plane-failures.md) for exact error IDs and safe checks.

## Arabic behavior, tests, and extension

There is no update UI in this checkpoint. Manifests remain locale-independent and contain no release-note prose. Future Arabic UI must localize states such as available, paused, incompatible, and revoked, and must render versions and hashes as isolated LTR text.

Signature tests reject changed content. Policy tests cover channel mismatch and incompatible protocol/version clients. Update-plane tests cover authorization denial, stable prerelease rejection, duplicate version rejection, and durable repository reload. Generated C# and Swift contracts include the protocol `1.5` types.

Run:

```powershell
cargo test -p eitmad-update-policy -p eitmad-update-plane -p eitmad-server
```

Related pages: [server operations](../../operations/run-server-authority.md), [ADR-0013](../../decisions/0013-platform-native-update-adapters.md), [ADR-0026](../../decisions/0026-compose-authorized-operational-server-planes.md), and [protocol 1.5 rollout](../../releases/protocol-1-5-operational-server-planes.md).
