---
title: "Start developing Eitmad"
description: "Find each component's authority, run workspace checks, and document changes in the correct vertical."
audience: "developer"
page_type: "tutorial"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "workspace layout, contributor checks, or ownership rules change"
keywords:
  - "developer guide"
  - "developer onboarding"
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

## 1. Name the product capability

Put behavior in a vertical module or crate named for what the product does. Do not create generic containers such as `utils`, `services`, or `handlers`. Keep Rust authoritative for domain rules, contracts, storage, authorization, and synchronization.

## 2. Define boundaries before implementation

Define commands, queries, subscriptions, errors, versions, and capabilities, followed by scope, ReBAC permissions, audit, storage and sync modes, and Arabic UI behavior. Complete the [Arabic-first pre-shell gate](contributing/arabic-first-feature-checklist.md#pre-shell-product-decisions) before shell implementation. The native shell remains a thin presentation adapter.

## 3. Develop with focused tests

Keep unit tests near the capability they verify. Use `tests/` only for cross-boundary flows. Cover relevant success, denial, and failure paths.

## 4. Update the knowledge graph

Follow `.agents/skills/maintain-project-documentation/SKILL.md` after feature behavior is complete and before considering the feature done. Update the canonical page, index, glossary, ADR, and troubleshooting knowledge where applicable.

## 5. Verify

Run the [foundation checks](../operations/index.md). Expected result: formatting, checks, builds, and tests complete without warnings; `eitmad-engine-cli` runs cleanly; and the documentation audit passes.

## What you learned

Rust owns the truth, and each vertical capability owns its behavior, tests, and documentation. Next, [choose the correct page for a change](contributing/documentation-standard.md).
