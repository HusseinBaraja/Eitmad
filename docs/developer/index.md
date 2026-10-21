---
title: "Start developing Eitmad"
description: "Find each component's authority, choose the smallest valid check, and document changes in the correct vertical."
audience: "developer"
page_type: "tutorial"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-10-21"
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

This path leads to the correct change location and the smallest proof that must pass before handoff.

## Before you start

Read `AGENTS.md` at the repository root, then review:

- [Repository layout and ownership](repository-layout.md)
- [Target architecture](../architecture/target-architecture.md)
- [Feature documentation standard](contributing/documentation-standard.md)
- [Arabic-first feature checklist](contributing/arabic-first-feature-checklist.md)
- [Domain glossary](../glossary.md)

Then read only the subsystem page that owns the change.

For customer, catalog, pricing, quotation, order, work-order, or delivery implementation, use the [Manager and Receptionist workflow specification](subsystems/manager-receptionist-workflows.md) as the accepted product behavior. Follow each subsystem’s documented Rust or preview boundary; [Pricing](subsystems/pricing.md) now uses Rust calculations and server-confirmed publication.

The implemented Rust customer contact boundary is documented in [Maintain customer contact records](subsystems/customers.md).
The durable image boundary is documented in [Import and transfer durable catalog images](subsystems/catalog-images.md).
The untrusted LAN candidate boundary is documented in [Server discovery](subsystems/server-discovery.md).
The durable manager raw-material boundary is documented in [Maintain raw material definitions](subsystems/raw-materials.md).

## 1. Run the local Windows app

To install a workshop or employee PC without a source checkout, use [guided Windows installation](../operations/install-windows.md). The steps here are for source development.

Install the .NET 8 SDK and Rust 1.85.1 with rustup. From the repository root, build the Rust engine and start the Windows shell with one command:

```powershell
.\run.ps1
```

The shell opens sign-in and supervises the Rust engine. The launcher does not provision a server or create accounts. Pair a new device with a running workshop through [Owner-code pairing](../operations/install-windows.md#3-install-and-pair-each-employee-pc), or provision synthetic data with the isolated workflow environment linked below. Use **إنهاء الاعتماد** in the system tray menu to stop both processes.

For an existing connected installation, pass `-RuntimeDirectory <installation-directory>` to this same launcher. It preserves accounts and saved work. Rust loads the installation's connection references; the server must be running. See [connected workflow test setup](../operations/run-workflow-test-environment.md) for provisioning and test limits. Use `-SkipBuild` only for unchanged source and dependencies.

## 2. Name the product capability

Put behavior in a vertical module or crate named for what the product does. Do not create generic containers such as `utils`, `services`, or `handlers`. Keep Rust authoritative for domain rules, contracts, storage, authorization, and synchronization.

## 3. Define boundaries before implementation

Define commands, queries, subscriptions, errors, versions, and capabilities, followed by scope, ReBAC permissions, audit, storage and sync modes, and Arabic UI behavior. Complete the [Arabic-first pre-shell gate](contributing/arabic-first-feature-checklist.md#pre-shell-product-decisions) before shell implementation. The native shell remains a thin presentation adapter.

## 4. Develop with focused tests

Keep unit tests near the capability they verify. Use `tests/` only for cross-boundary flows. Cover relevant success, denial, and failure paths.

### Choose the smallest normal proof

The commands below exist in this checkout and show one concrete focused example. Replace the example crate, test class, or documentation path with the owner of the change. Run broader gates only for a release, CI or workspace-wide change, or evidence of wider impact.

| Changed boundary | Smallest normal proof | Additional proof only when needed |
| --- | --- | --- |
| One preview page | Build the shell, then run its affected presentation and rendered test classes. For example: `dotnet build shells/windows/Eitmad.WindowsShell.csproj --configuration Release --nologo` and `dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo --filter "FullyQualifiedName~RawMaterialsPresentationTests\|FullyQualifiedName~RawMaterialsRenderedTests"` | Inspect the affected synthetic capture; check keyboard and OS behavior when interactions changed |
| Rust capability behavior | Format and test the affected crate. For example: `cargo fmt --package eitmad-material -- --check` and `cargo test -p eitmad-material` | Test the direct dependent integration boundary when public behavior changed |
| Rust contracts or generated bindings | Generate intended outputs with `npm run contracts:generate --prefix crates/contracts/codegen`, then run `npm run contracts:verify --prefix crates/contracts/codegen` | Run the affected consuming runtime or shell path; leave all-platform gates to CI |
| One documentation page | Run `python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs --files docs/developer/subsystems/raw-materials.md`, then compare its behavior claims with the named source owner | Run the full documentation audit only for shared navigation or documentation-system changes |

## 5. Update the knowledge graph

Follow `.agents/skills/maintain-project-documentation/SKILL.md` after feature behavior is complete and before considering the feature done. Update the canonical page, index, glossary, ADR, and troubleshooting knowledge where applicable.

## 6. Verify

Run the selected focused proof and inspect the final diff. Use the [release guide](../operations/validate-release-candidate.md) only for release checks.

## What you learned

Rust owns the truth, and each vertical capability owns its behavior, tests, and documentation. Next, [choose the correct page for a change](contributing/documentation-standard.md).
