---
title: "Prepare future Linux packages and updates"
description: "Define Linux package choices, repository signing, updater boundaries, validation, and rollback before the shell exists."
audience: "operations"
page_type: "task"
status: "active"
owner: "release infrastructure maintainers"
last_verified: "2026-08-26"
review_triggers:
  - "Linux shell, package formats, repository signing, sandbox, or updater changes"
keywords:
  - "DEB RPM AppImage"
  - "Linux package signing"
  - "Linux updater"
---

# Prepare future Linux packages and updates

Linux desktop packaging is blocked because no runnable Linux shell or updater adapter exists. Repository policy requires `linux_desktop.enabled = false` until both are implemented and validated.

## Package choices

Use DEB as the first managed package for Debian and Ubuntu deployments. Add RPM when Fedora or enterprise RPM-family support has an owner and test matrix. AppImage can support portable evaluation, but it must not become the managed production default because repository policy, dependency updates, desktop integration, and rollback are weaker. Flatpak is a later option when portal, sandbox, and background-engine behavior are proven.

Each package must declare architecture, minimum distribution/library support, desktop entry, icons, protocol handlers, Rust engine service/process ownership, dependencies, uninstall behavior, and preserved data locations. Sign APT or RPM repository metadata and packages with offline-controlled release keys. Serve them through TLS with immutable snapshots.

## Future updater expectations

Rust owns update eligibility and migration safety. The Linux adapter should prefer the system package manager for managed installations. It may invoke an approved PackageKit or distribution-specific flow after consent, but it cannot silently bypass administrator policy, replace repository trust, or choose a channel. Portable formats need a separately reviewed atomic replacement and rollback design.

Test Arabic RTL and mixed text on the selected native toolkit, supported display servers, accessibility stack, portals, keyring, desktop sessions, suspend/resume, and logout. Test engine termination when the user session ends.

## Rollback expectation

Pause or revoke the manifest and repository publication, then ship a higher-version recovery package when package-manager downgrade rules are uncertain. Keep the previous repository snapshot until the recovery window closes. Never reverse schema SQL by hand. Restore a compatible pre-migration backup only under the release-specific recovery procedure.

Related pages: [validate a release candidate](validate-release-candidate.md), [signed updates](../developer/subsystems/update-distribution.md), and [ADR-0013](../decisions/0013-platform-native-update-adapters.md).
