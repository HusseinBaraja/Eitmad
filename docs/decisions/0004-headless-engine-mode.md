---
title: "ADR-0004: Require a headless engine mode"
description: "Records why authoritative behavior must run for diagnostics and automation without a desktop shell."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "supported unattended workflows or deployment assumptions materially change"
---

# ADR-0004: Require a headless engine mode

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: Supported unattended workflows or deployment assumptions materially change
> - Supersedes: None
> - Superseded by: None

## Context and problem

Authoritative behavior must be testable and operable without a desktop UI. Diagnostics, automation, recovery, and future service deployments cannot depend on a graphical shell.

## Decision drivers

- Independent engine verification and supportability
- Unattended workflows with the same domain semantics
- No hidden dependency on UI state or lifecycle

## Considered options

- **First-class headless mode:** one authority in interactive and unattended contexts.
- **GUI-only engine:** simpler initial entry point, but blocks automation and diagnostic isolation.
- **Separate automation implementation:** duplicates rules and increases security drift.

## Decision

The Rust engine MUST provide documented headless and diagnostic modes. They MUST use the same domain services, authorization, storage, audit, sync, and contracts as interactive operation. Headless operation MUST expose bounded lifecycle controls and machine-readable outcomes; it MUST NOT bypass authorization or become an unrestricted administrative path.

## Architecture and contract impact

Engine modules cannot depend on a shell. Headless adapters remain thin entry points into the same vertical capabilities.

## Storage and sync impact

Headless work uses normal transactions, scopes, idempotency, conflicts, and sync checkpoints. Concurrent authority rules remain unchanged.

## Security impact

Credentials, actor identity, scopes, and capabilities must be explicit. Diagnostic output follows normal redaction and access policy.

## Arabic UX impact

User-facing headless output supports stable localized messages where humans consume it; machine-readable fields remain locale-independent and preserve Unicode Arabic data.

## Consequences and tradeoffs

### Positive

- Core behavior can be tested and operated without UI automation.
- Automation cannot justify a second domain implementation.

### Negative

- Lifecycle, credential, output, and cancellation contracts require deliberate design.

### Risks

- Headless mode could become a privileged bypass; mitigate with identical policy enforcement and audit.

## Verification

Foundation structure review confirms no production shell dependency exists yet. The first vertical slice must run its principal behavior through both IPC and headless entry points.

## Follow-up work

- Define supported headless commands, authentication, exit codes, and diagnostic schema.

## Related decisions and documents

- [ADR-0001](0001-rust-core-product-authority.md)
- [ADR-0003](0003-separate-supervised-engine-process.md)
- [ADR-0012](0012-privacy-preserving-observability.md)
