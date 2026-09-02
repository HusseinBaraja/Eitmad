---
title: "Start developing Eitmad"
description: "Find each component's authority, run workspace checks, and document changes in the correct vertical."
audience: "developer"
page_type: "tutorial"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-09-02"
review_triggers:
  - "workspace layout, local app startup, contributor checks, or ownership rules change"
keywords:
  - "developer guide"
  - "developer onboarding"
  - "run Windows app"
  - "cargo check workspace"
  - "vertical capability"
---

# Start developing Eitmad

This path leads to the correct change location and the foundation checks that must pass before handoff.

## Before you start

Read `AGENTS.md` at the repository root, then review:

- [Repository layout and ownership](repository-layout.md)
- [Target architecture](../architecture/target-architecture.md)
- [Feature documentation standard](contributing/documentation-standard.md)
- [Arabic-first feature checklist](contributing/arabic-first-feature-checklist.md)
- [Domain glossary](../glossary.md)
- [Authoritative contract layer](subsystems/contract-layer.md)
- [Engine runtime lifecycle](subsystems/engine-runtime.md)
- [Typed local IPC](subsystems/local-ipc.md)
- [Rust-owned configuration](subsystems/configuration.md)
- [Scoped authorization and audit](subsystems/authorization.md)
- [Privacy-preserving observability](subsystems/privacy-preserving-observability.md)
- [Rust-owned secret storage](subsystems/secret-storage.md)
- [Rust-owned local storage](subsystems/local-storage.md)
- [Dual-mode synchronization and shared transports](subsystems/synchronization.md)
- [Modular PostgreSQL server authority](subsystems/server-authority.md)
- [WAN relay coordination](subsystems/wan-relay-coordination.md)
- [Signed update distribution](subsystems/update-distribution.md)
- [Least-privilege server administration](subsystems/server-administration.md)
- [Persistent tenant identity](subsystems/identity-foundation.md)
- [Windows engine process supervision](subsystems/windows-process-supervision.md)
- [Arabic-first Windows operations shell](subsystems/windows-native-shell.md)
- [Parts list vertical](subsystems/parts.md)
- [Furniture manager flow](subsystems/furniture.md)
- [Reference marker complete vertical](subsystems/reference-marker.md)
- [Build the first real product](build-first-product.md)

## 1. Run the local Windows app

Install the .NET 8 SDK and the stable Rust toolchain. From the repository root, build the Rust engine and start the Windows shell with one command:

```powershell
.\run.ps1
```

The **لوحة التحكم** window opens, and the shell supervises the Rust engine. Use **إنهاء الاعتماد** in the system tray menu to stop both processes.

## 2. Name the product capability

Put behavior in a vertical module or crate named for what the product does. Do not create generic containers such as `utils`, `services`, or `handlers`. Keep Rust authoritative for domain rules, contracts, storage, authorization, and synchronization.

## 3. Define boundaries before implementation

Define commands, queries, subscriptions, errors, versions, and capabilities, followed by scope, ReBAC permissions, audit, storage and sync modes, and Arabic UI behavior. Complete the [Arabic-first pre-shell gate](contributing/arabic-first-feature-checklist.md#pre-shell-product-decisions) before shell implementation. The native shell remains a thin presentation adapter.

## 4. Develop with focused tests

Keep unit tests near the capability they verify. Use `tests/` only for cross-boundary flows. Cover relevant success, denial, and failure paths.

## 5. Update the knowledge graph

Follow `.agents/skills/maintain-project-documentation/SKILL.md` after feature behavior is complete and before considering the feature done. Update the canonical page, index, glossary, ADR, and troubleshooting knowledge where applicable.

## 6. Verify

Run the [foundation checks](../operations/index.md). Expected result: formatting, checks, builds, and tests complete without warnings; `eitmad-engine-cli` and the Windows supervisor run cleanly; and the documentation audit passes.

## What you learned

Rust owns the truth, and each vertical capability owns its behavior, tests, and documentation. Next, [choose the correct page for a change](contributing/documentation-standard.md).
