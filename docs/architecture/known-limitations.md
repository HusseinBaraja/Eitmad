---
title: "Known application limitations"
description: "Lists current capability, platform, operations, Arabic, recovery, and evidence limits without overstating readiness."
audience: "architecture"
page_type: "reference"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-10-24"
review_triggers:
  - "a listed limitation is implemented, removed, split, or becomes release-critical"
keywords:
  - "known limitations"
  - "not implemented"
  - "production gaps"
---

# Known application limitations

The Windows app has Rust-owned Customers, Materials, Parts, Furniture, Products, account administration, quotations and confirmed orders. Isolated preview fixtures remain available for UI development, including dashboard surfaces. Saved customer documents require authorized Rust reads. See the [workflow authority](../developer/subsystems/manager-receptionist-workflows.md) before treating a preview as persisted product behavior.

## Identity and synchronization

- Desktop password sign-in uses [live server authority](../decisions/0029-require-live-server-authority-for-protected-access.md). Retained local records and credentials do not authorize disconnected access. Debug seeding is a synthetic development path, not workshop installation.
- The [direct server connection](../developer/subsystems/server-authority.md#retained-desktop-connection) supports enrolled desktops and synchronized customer work. Isolated engine tests do not establish clean-PC or physical-LAN deployment acceptance.
- [LAN discovery](../developer/subsystems/server-discovery.md) and [Owner-code device pairing](../operations/install-windows.md#3-install-and-pair-each-employee-pc) are implemented. Clean-machine, multi-PC, effective firewall, and physical DHCP acceptance remain required. WAN relay payload routing is not implemented; relay routes coordinate metadata and lifecycle only. Consult each product subsystem for its synchronization boundary.
- Shared account administration supports invitation creation and activation through authenticated Rust routes; email delivery remains unimplemented. The server has no license service, channel-assignment mutation, generic synchronization command-submission route, or history compaction entry point. Pricing and commercial transitions use their own server-confirmed routes. Existing database rows and immutable migrations remain intact.

## Updates and platforms

- Signed manifests, rollout, revocation, compatibility policy, and update server routes exist. The desktop has no manifest retrieval coordinator, package download, native installation handoff, or interrupted-update recovery.
- Windows is the only runnable desktop shell. macOS has generated Swift binding conformance; Linux has no native desktop shell.
- Local IPC accepts the current beta protocol only. Engine and shell must change together. Storage uses its declared compatibility window; preserve validated recovery artifacts before changing versions.

## Deployment and recovery

- [Windows package authoring and manual lifecycle tools](../operations/package-windows-desktop.md) produce versioned server ZIPs and self-contained desktop MSIX bundles. Azure Artifact Signing integration is supplied, but the verified publisher/profile, signing-node authorization, signed clean-host install/repair/removal/upgrade, and coordinated restore acceptance remain external prerequisites. Unsigned artifacts are validation-only. Notarization and signed automatic promotion remain unimplemented.
- [Guided Windows setup](../operations/install-windows.md) composes server preparation, services, first-desktop enrollment, backups, renewal, and per-user MSIX registration. The complete trusted signed offline bundle and clean-PC Server/Client installation remain **Needs verification**. The combined server has no billing, MFA, email, package CDN, or relay payload router.
- [Windows server backup administration](../operations/backup-lan-installation.md#manage-backups-after-installation) supplies native configuration/status, scheduled local capture, retention, and an optional separate encrypted disk copy. Whole-installation recovery still requires Windows profile/credential dependencies and replacement-machine acceptance. SQLite backup, restore, migration artifacts, and tenant export remain Rust APIs and runbooks; they have no general recovery UI.
- PostgreSQL backup, WAL retention, and restore depend on deployment infrastructure. Live role, RLS, migration, backup, and restore evidence is required before deployment.

## Evidence limits

- Automated WPF fixtures cover the current Arabic and RTL surfaces. Product-level screen-reader, keyboard, high-contrast, text-scaling, printing, and physical-device evidence must match each changed workflow.
- Saved quotation and confirmed order customer documents have an authorized Windows native preview and print path. Standalone PDF generation, reports and spreadsheet export are not implemented. See the [saved-document boundary and device evidence](../developer/subsystems/quotations.md#saved-customer-documents).
- No production load profile, long-session soak result, or measured server capacity baseline exists.
- Native secret lifecycle tests require an isolated platform test account. Branch protection and production secret-manager policy are external controls.

Use [release validation](../operations/validate-release-candidate.md) for the current checks and [storage recovery](../operations/recover-local-storage.md) before any database intervention.
