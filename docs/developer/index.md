---
title: "Start developing Eitmad"
description: "Find each component's authority, choose the smallest valid check, and document changes in the correct vertical."
audience: "developer"
page_type: "tutorial"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-09-19"
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

Then read only the subsystem page that owns the change. Use [Build the first real product](build-first-product.md) only when adding a complete product vertical.

For customer, catalog, pricing, quotation, order, work-order, or delivery implementation, use the [Manager and Receptionist workflow specification](subsystems/manager-receptionist-workflows.md) as the accepted product behavior. The existing Windows feature pages describe preview presentation only.

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

### Choose the smallest normal proof

The commands below exist in this checkout and show one concrete focused example. Replace the example crate, test class, or documentation path with the owner of the change. Run broader gates only for a release, CI or workspace-wide change, or evidence of wider impact.

| Changed boundary | Smallest normal proof | Additional proof only when needed |
| --- | --- | --- |
| One preview page | Build the shell, then run its affected presentation and rendered test classes. For example: `dotnet build shells/windows/Eitmad.WindowsShell.csproj --configuration Release --nologo` and `dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo --filter "FullyQualifiedName~RawMaterialsPresentationTests\|FullyQualifiedName~RawMaterialsRenderedTests"` | Inspect the affected synthetic capture; check keyboard and OS behavior when interactions changed |
| Rust capability behavior | Format and test the affected crate. For example: `cargo fmt --package eitmad-reference-marker -- --check` and `cargo test -p eitmad-reference-marker` | Test the direct dependent integration boundary when public behavior changed |
| Rust contracts or generated bindings | Generate intended outputs with `npm run contracts:generate --prefix crates/contracts/codegen`, then run `npm run contracts:verify --prefix crates/contracts/codegen` | Run the affected consuming runtime or shell path; leave all-platform gates to CI |
| One documentation page | Run `python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs --files docs/developer/subsystems/reference-marker.md`, then compare its behavior claims with the named source owner | Run the full documentation audit only for shared navigation or documentation-system changes |

## 5. Update the knowledge graph

Follow `.agents/skills/maintain-project-documentation/SKILL.md` after feature behavior is complete and before considering the feature done. Update the canonical page, index, glossary, ADR, and troubleshooting knowledge where applicable.

## 6. Verify

Run the selected focused proof and inspect the final diff. Use the [release guide](../operations/validate-release-candidate.md) only for release checks.

## What you learned

Rust owns the truth, and each vertical capability owns its behavior, tests, and documentation. Next, [choose the correct page for a change](contributing/documentation-standard.md).
