---
title: "Back up and recover the Windows LAN installation"
description: "Configure automatic Rust-owned local server backups, an optional external disk, and coordinated Windows recovery generations."
audience: "operations"
page_type: "task"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-27"
review_triggers:
  - "backup cadence, retention, participant inventory, Windows credentials, PostgreSQL version, or measured recovery changes"
keywords:
  - "scheduled backup"
  - "bare metal recovery"
  - "DPAPI"
  - "pg_basebackup"
  - "recovery point objective"
  - "UNC path"
  - "Windows backup account"
---

# Understand backup availability

Backup support is deferred. Current Eitmad source has no backup setup fields, management screen, local or external capture, scheduling, retention, restore tools, backup administration routes, or manual engine backup/restore commands. Setup and upgrades require no backup destination or recovery generation. [ADR-0033](../decisions/0033-defer-backup-support.md) records this scope.

Existing databases, saved copies, installation journals, and files created by older builds are preserved. This source change does not remove an installed task or erase saved media. Do not run old bundled tools against current state as a substitute for a supported recovery procedure.

Synchronization and tenant export do not provide whole-installation recovery. PostgreSQL migration checksums and existing migration bytes remain unchanged, including the historical unused backup-status table. Secret-file replacement keeps its interrupted-write protection; that mechanism is part of credential integrity, not a scheduled backup facility.

For installation failure, use the copy or save log actions described in [Windows setup](install-windows.md#if-setup-or-pairing-stops). For unhealthy local storage, follow [storage inspection](recover-local-storage.md). Return to the [operations index](index.md).
