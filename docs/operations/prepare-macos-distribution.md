---
title: "Prepare future macOS signing and notarization"
description: "Define the blocked macOS package, signing, notarization, updater, validation, and rollback path."
audience: "operations"
page_type: "task"
status: "active"
owner: "release infrastructure maintainers"
last_verified: "2026-08-26"
review_triggers:
  - "macOS shell, entitlements, signing, notarization, package, or updater changes"
keywords:
  - "macOS notarization"
  - "Developer ID"
  - "stapler validate"
---

# Prepare future macOS signing and notarization

macOS packaging is blocked because no runnable macOS shell or updater adapter exists. `deploy/release-plan.toml` keeps `macos_desktop.enabled = false`, and repository policy rejects premature enablement.

## Required production path

When the native shell exists, release engineering must:

1. Build a universal or separately declared architecture package from a clean reviewed tag.
2. Sign the app, embedded Rust engine, frameworks, helpers, and final PKG with the correct Developer ID identities. Apply the smallest reviewed hardened-runtime entitlements.
3. Verify nested code signatures before submission.
4. Submit the final artifact to Apple notarization, wait for acceptance, staple the ticket, and run Gatekeeper and `stapler validate` checks on a clean supported macOS version.
5. Verify Arabic RTL, mixed-direction input, VoiceOver, sandbox access, Keychain prompts, local IPC, engine supervision, update consent, and clean shutdown.

The signing key and Apple credentials must use the release secret manager and short-lived CI access. Do not store them in the repository, package, logs, update server, or notarization transcript.

## Future updater expectations

Rust owns signed-manifest verification, compatibility, channel, rollout, and migration preflight. The native macOS adapter owns download, package digest verification, Apple signature and notarization checks, user consent, installation, relaunch, and typed outcome reporting. It must support atomic replacement or preserve the prior app until the new app passes launch and IPC readiness.

Host immutable artifacts over HTTPS and promote the same digest through `canary`, `beta`, and `stable`. Reject an unstapled, changed, expired-policy, wrong-team, or unnotarized package before installation.

## Rollback expectation

Pause or revoke the signed manifest, then publish a higher-version recovery build. Do not overwrite an artifact URL. Preserve Rust-owned data and follow migration compatibility notes. A future updater must prove recovery after interrupted download, interrupted installation, failed first launch, and failed IPC negotiation before production enablement.

Related pages: [validate a release candidate](validate-release-candidate.md), [signed updates](../developer/subsystems/update-distribution.md), and [ADR-0013](../decisions/0013-platform-native-update-adapters.md).
