---
title: "ADR-0027: Require one mandatory release evidence chain"
description: "Record why every release uses always-running CI, immutable artifacts, separated channels, and restore-backed rollback evidence."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture and release infrastructure maintainers"
last_verified: "2026-08-26"
review_triggers:
  - "mandatory CI, artifact promotion, environment channels, or rollback policy changes"
keywords:
  - "ADR-0027"
  - "mandatory release validation"
---

# ADR-0027: Require one mandatory release evidence chain

- Status: Accepted
- Date: 2026-08-26
- Decision owners: Architecture and release infrastructure maintainers
- Supersedes: None

## Context and forces

Eitmad crosses Rust, generated contracts, native shells, PostgreSQL, SQLite, synchronization, authorization, updates, and Arabic UI. Path-filtered checks can miss cross-boundary effects. Rebuilding between staging and production can also invalidate tested evidence.

## Decision

Every pull request and push to `main` runs one mandatory validation workflow without path filters. The workflow produces a final aggregate status and explicit dependency checks. Branch protection requires all listed statuses.

Release engineering builds an immutable artifact once, records its SHA-256 digest, signs it outside ordinary CI, and promotes the same digest through `canary`, `beta`, and `stable`. Staging and production use separate data and trust boundaries. Schema rollback uses compatibility or a complete verified restore; operators do not reverse migration SQL by hand.

Repository policy fails on contract or generated-binding drift, changed released migration bytes, missing documentation impact, unsafe secret-bearing logging, likely committed secrets, missing ownership, Arabic root-direction loss, or direct shell access to database, configuration, and secret APIs.

## Consequences

Validation takes longer and uses Windows, macOS, and Linux runners. The evidence is easier to audit, and an unrelated path filter cannot skip a required boundary check. Production signing and restore drills remain controlled external gates because CI must not hold production keys or customer data.

macOS and Linux packaging stay disabled until their runnable shells, native update adapters, security tests, Arabic behavior, and rollback tests exist. Windows CI artifacts remain unsigned and not production-eligible until the MSIX signing path exists.

## Security, Arabic, compatibility, and operations

Rust stays authoritative for contracts, authorization, storage, sync, update eligibility, and migration safety. Native installers perform OS mechanics only. Each desktop gate preserves Arabic and mixed-direction values and verifies root RTL behavior. Contract compatibility covers the supported protocol window. Server promotion requires tenant isolation, redaction, backup, restore, and rollback evidence.

## Alternatives rejected

- Path-filtered workflows: they can omit cross-boundary checks.
- Rebuild per environment: it promotes different bytes from those tested.
- Automatic database downgrade: it can lose state and violate migration invariants.
- One shared staging and production trust boundary: it can leak data or credentials across environments.

## Follow-ups

Implement production MSIX, macOS, and Linux package adapters before enabling their production formats. Add artifact provenance and SBOM generation when the release signing service is selected.

Related pages: [validate a release candidate](../operations/validate-release-candidate.md) and [signed update authority](../developer/subsystems/update-distribution.md).
