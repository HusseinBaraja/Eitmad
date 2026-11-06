---
title: "ADR-0034: Use Inno Setup for Windows installation and removal"
description: "Replace the custom extraction bootstrap with signed Inno packages and clear installation blockers while retaining user data."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "product developer"
last_verified: "2026-11-05"
review_triggers:
  - "installer ownership, removal targets, retained data, publisher trust, or MSIX delivery changes"
keywords:
  - "Inno Setup"
  - "uninstall"
  - "fresh installation"
  - "إزالة الاعتماد"
---

# ADR-0034: Use Inno Setup for Windows installation and removal

> - Status: Accepted
> - Date: 2026-11-05
> - Decision owner: Product developer
> - Supersedes: ADR-0031, custom outer bootstrap and extracted-payload lifecycle only

## Context

The developer requests Inno Setup wherever it can replace custom Windows installation code. Removal must clear components that can block the next installation, including services and Eitmad registry entries. User data can remain. Existing setup preserves active host directories and journals, which prevents a new workshop installation after removal.

## Decision

Build `EitmadServerSetup.exe` and `EitmadClientSetup.exe` from one Inno Setup 7.1.1+ template. Inno owns the standard Arabic wizard, extraction, installed payload files, setup shortcuts, uninstall registration, signed uninstaller, and removal entry point. Keep release trees separate by Windows version and an authenticated inventory digest prefix so a different release cannot overwrite an interrupted setup's files.

Rust continues to own workshop provisioning, validation, credentials, migrations, enrollment, release policy, and recovery. The existing thin WPF wizard renders that workflow. Closing or failing this wizard must not report installation success to Inno. Keep MSIX for desktop deployment; changing package identity or replacing MSIX is a separate decision.

Keep [ADR-0032](0032-self-signed-windows-publisher.md) publisher policy. Inno presents the public certificate fingerprint, requires explicit confirmation for missing trust, invokes Windows `certutil.exe` for each missing machine store, and verifies trusted signatures before running payload code. Sign Inno's temporary setup copies and uninstaller as well as the final EXE. Initial download authenticity still depends on the independent handover.

The original ordinary Windows user owns desktop installation and removal. Server removal requests separate elevation for machine mechanics. It validates ownership before stopping or removing services, the TLS scheduled task, firewall rules, and Event Log sources. SCM owns removal of its service registry keys. Inno owns removal of its exact registry keys; do not search for and delete registry entries by substring.

Before completing removal, rename local records, machine databases, settings, private material, and journals into separate private `EitmadRetained/<unique-id>` directories under their existing LocalAppData or ProgramData parent. Remove the active names that prevent fresh setup. Remove installed executable trees and authenticated installer caches. Retain credentials referenced by retained data. Leave shared Windows prerequisites and publisher trust installed. Reject foreign registrations, unsafe paths, junctions, and active applications. A failed cleanup must stop Inno before it deletes the tools needed for retry.

## Consequences

Inno removes custom extraction code and provides a normal installed uninstaller. Some Eitmad-specific WPF and native adapter code remains because a generic installer does not own workshop behavior or safe data recovery.

Reinstallation starts fresh. Retained records are not automatically reactivated, copied into a new workshop, or presented as a supported backup. Keep them for deliberate recovery investigation. Removal of the Server interrupts all employee connections; Client removal does not remove machine services or other Windows users' installations. Earlier published custom installers retain their old removal behavior; this decision does not silently migrate or alter an installed PC.

## Verification

Require compilation and trusted signing of both Inno roles, focused WPF completion checks, isolated removal tests with synthetic records, rejection of foreign registrations and junctions, and the [offline Windows acceptance drill](../operations/verify-offline-windows-setup.md). Real Inno rendering, clean-PC installation, service removal, reboot, and fresh reinstallation require observed evidence before production acceptance.

Return to the [decision index](index.md).
