---
title: "ADR-0031: Distribute two self-contained Windows installers"
description: "Require separate offline Server and Client executables with embedded prerequisites and native guided setup."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "product developer"
last_verified: "2026-10-24"
review_triggers:
  - "installer delivery format, embedded payload trust, role selection, or offline acceptance changes"
keywords:
  - "click and run and follow in app guided setup"
  - "EitmadServerSetup.exe"
  - "EitmadClientSetup.exe"
  - "offline installer"
---

# ADR-0031: Distribute two self-contained Windows installers

> - Status: Accepted
> - Date: 2026-10-24
> - Decision owners: Product developer
> - Supersedes: ADR-0030, distribution format and installer role selection only
> - Superseded by: None

## Context

The user's requirement is **"click and run and follow in app guided setup"**. A folder of scripts, runtime installers, and manual certificate or firewall commands does not meet this requirement, including for a controlled LAN test.

## Decision

Distribute exactly two standalone offline installer entry points:

- `EitmadServerSetup.exe` contains the Server setup payload, required redistributable runtimes and tools, and the desktop app. Its Arabic wizard collects the workshop and first Owner details and performs the existing Server installation workflow.
- `EitmadClientSetup.exe` contains the Client setup payload, desktop app, Rust engine, and required redistributable runtimes. It installs the desktop, which guides the user through LAN discovery, Owner-approved pairing, and sign-in. PostgreSQL and host-service prerequisites are not Client payload requirements.

Each executable fixes its role. Build both from one shared setup implementation. Do not maintain separate copies of installation policy or business logic. Preserve all other [ADR-0030](0030-offline-guided-windows-setup.md) decisions, including Rust authority, independent devices, scoped firewall rules, verified TLS, unattended Server services, and protected backups.

The installer automatically extracts its embedded payload into a protected local location. Users do not extract an archive, arrange folders, run a shell, install prerequisites separately, or edit configuration files. Embedded payloads must be authenticated before execution and reverified across elevation. Never execute an unverified bundled runtime to establish its own trust. Refuse changed, missing, unexpected, unsafe, or wrong-role payload content.

Installation creates normal installed files, separate Rust and database processes, and fresh machine-specific state. Single-file distribution does not combine these runtime boundaries or embed a provisioned workshop, passwords, private keys, or device identities in the installer. Preserve accepted work and recovery state during interruption, repair, upgrade, and removal.

Server setup requests Windows administrator approval when required. Client setup runs for the intended ordinary Windows user. Required restarts, unsupported Windows prerequisites, trust failures, progress, and recoverable installation failures must have clear native Arabic guidance. Signing and OS trust checks remain enabled. A test-signed build must be identified as a test build; controlled testing does not permit replacing the wizard with developer commands.

## Consequences and alternatives

Offline executable sizes include their redistributable dependencies. Build and version Server and Client together, inventory their contents, preserve redistribution notices, and rebuild when bundled security updates require it. The release builder can use staged directories internally; these are not the end-user delivery format.

Supported Windows, disk space, network and publisher policy, server power, and protected backup media remain host requirements. Bundling cannot create those conditions. Setup must report an unmet requirement without bypassing it or claiming completion.

## Verification

Acceptance starts with only the appropriate EXE on a clean supported offline Windows PC. Require native Arabic guidance from launch to app use, automatic prerequisite handling, authenticated extraction, role enforcement, preserved interruption state, and no end-user terminal or manual runtime/certificate/firewall procedure. Verify two independent LAN devices, authorized customer changes in both directions, retained pairing after Client restart, and Server availability after reboot before Windows sign-in. Run the focused native rendering checks in `AGENTS.md`.

An accepted decision is a requirement, not a claim of working release artifacts. Record implementation and observed acceptance status in [Windows installation](../operations/install-windows.md) and [offline setup verification](../operations/verify-offline-windows-setup.md).

Return to the [decision index](index.md).
