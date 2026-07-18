---
title: "Eitmad documentation"
description: "Choose the relevant task or audience to reach product, engineering, operations, and contract documentation."
audience: "developer"
page_type: "reference"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "a documentation collection or canonical entry point changes"
keywords:
  - "documentation index"
  - "developer guide"
---

# Eitmad documentation

Choose the route that matches your task. Protocol `1.2`, the engine lifecycle, SQLite authority storage, Rust-owned configuration, scoped ReBAC, and Windows process supervision are implemented foundations; no native settings UI or production business workflow exists yet.

## Use the product

No user workflow is ready to document. When the first product flow exists, English help for its Arabic-first UI and UX will appear in `user/`.

## Develop the system

- [Start developing Eitmad](developer/index.md): contribution setup, ownership boundaries, tests, and feature documentation.
- [Extend Rust-owned configuration](developer/subsystems/configuration.md): defaults, validation, revisions, redaction, import/export, and events.
- [Extend scoped ReBAC authorization](developer/subsystems/authorization.md): direct relationships, policy revisions, owner protection, and revocation.
- [Understand the target architecture](architecture/index.md): Rust authority, trust boundaries, synchronization, security, and updates.
- [Use protocol v1 contracts](api/index.md): exact Rust-owned contracts, compatibility, generation, and native bindings.
- [Review architectural decisions](decisions/index.md): durable choices, reasons, and consequences.
- [Review releases and compatibility](releases/index.md): protocol rollout, rollback, and migration notes.
- [Use approved terminology](glossary.md): English definitions and canonical Arabic UI terms.

## Operate or deploy the system

- [Run foundation checks](operations/index.md): currently safe commands and operational-readiness limits.
- [Run and diagnose the engine](operations/run-engine-runtime.md): headless and supervised modes, readiness JSON, exit codes, and recovery.

## Search by symptom or identifier

- [Troubleshoot Eitmad](troubleshooting/index.md): diagnose contract drift and future stable system failures.

## Documentation authority

Current code, tests, contracts, schemas, and configuration are evidence of behavior. Documentation explains these sources; it does not replace them. When they conflict, repair the canonical page in the same change.
