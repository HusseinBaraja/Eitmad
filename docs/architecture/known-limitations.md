---
title: "Known application limitations"
description: "Lists current capability, platform, operations, Arabic, recovery, and evidence limits without overstating readiness."
audience: "architecture"
page_type: "reference"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-10-10"
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

- Password sign-in and durable desktop sessions exist, but the engine does not import accounts from the server control plane. Debug seeding provisions synthetic local accounts only.
- Customer synchronization libraries and the registered server handler exist. The desktop executable has no production connector or background reconciliation coordinator.
- Other product domains do not synchronize. LAN discovery and WAN relay payload routing are not implemented; relay routes coordinate metadata and lifecycle only.
- The server has no later-account invitation creation or delivery, license service, channel-assignment mutation, server-authoritative command submission, or history compaction entry point. Existing database rows and immutable migrations remain intact.

## Updates and platforms

- Signed manifests, rollout, revocation, compatibility policy, and update server routes exist. The desktop has no manifest retrieval coordinator, package download, native installation handoff, or interrupted-update recovery.
- Windows is the only runnable desktop shell. macOS has generated Swift binding conformance; Linux has no native desktop shell.
- Local IPC accepts the current beta protocol only. Engine and shell must change together. Storage uses its declared compatibility window; preserve validated recovery artifacts before changing versions.

## Deployment and recovery

- Build scripts produce unsigned validation bundles. Production signing, MSIX, notarization, native package installation, and signed promotion are not implemented.
- The combined server has no billing, MFA, email, package CDN, relay payload router, backup scheduler, or operator UI.
- SQLite backup, restore, migration artifacts, and tenant export are Rust APIs and runbooks. There is no recovery UI, scheduled retention, or remote backup destination.
- PostgreSQL backup, WAL retention, and restore depend on deployment infrastructure. Live role, RLS, migration, backup, and restore evidence is required before deployment.

## Evidence limits

- Automated WPF fixtures cover the current Arabic and RTL surfaces. Product-level screen-reader, keyboard, high-contrast, text-scaling, printing, and physical-device evidence must match each changed workflow.
- Saved quotation and confirmed order customer documents have an authorized Windows native preview and print path. Standalone PDF generation, reports and spreadsheet export are not implemented. See the [saved-document boundary and device evidence](../developer/subsystems/quotations.md#saved-customer-documents).
- No production load profile, long-session soak result, or measured server capacity baseline exists.
- Native secret lifecycle tests require an isolated platform test account. Branch protection and production secret-manager policy are external controls.

Use [release validation](../operations/validate-release-candidate.md) for the current checks and [storage recovery](../operations/recover-local-storage.md) before any database intervention.
