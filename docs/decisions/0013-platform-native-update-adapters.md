# ADR-0013: Use platform-native update adapters

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture and release maintainers
> - Last verified: 2026-07-11
> - Review trigger: A supported platform changes signing, distribution, sandbox, installer, or rollback requirements
> - Supersedes: None
> - Superseded by: None

## Context and problem

Update safety depends on product compatibility, rollout, and database state, while installation depends on operating-system signing, elevation, packaging, and lifecycle rules. Giving either side the whole responsibility would duplicate policy or bypass native platform controls.

## Decision drivers

- One compatibility and migration authority
- Native signing, installer, elevation, and recovery behavior
- Safe staged rollout, interruption recovery, and audit

## Considered options

- **Rust policy plus native installation adapters:** separates product decisions from OS mechanics.
- **Shell-owned updates:** native, but can bypass engine compatibility and migration policy.
- **Rust-only installer:** unified code, but conflicts with platform-native distribution and privilege requirements.
- **Manual updates only:** operationally unreliable and difficult to secure at scale.

## Decision

Rust MUST own update discovery policy, signed-manifest verification, eligibility, channels, rollout cohorts, compatibility, migration safety, and update state. Each platform-native adapter MUST own installation mechanics: approved delivery integration, prompts, elevation, installer invocation, restart coordination, and verified result reporting. Neither side may perform the other's authority independently.

## Architecture and contract impact

Versioned contracts expose update capabilities and a state machine covering availability, download, preflight, readiness, installation handoff, verification, failure, pause, revocation, and recovery. Engine, shell, contract, and data-format compatibility ranges are explicit.

## Storage and sync impact

Preflight coordinates sync, backup, active work, disk space, and forward migrations. Incompatible data formats cannot be casually downgraded; interrupted updates have deterministic recovery.

## Security impact

Artifacts and server-hosted manifests are signed and verified with key rotation and revocation. Update transport, adapter invocation, elevation, and result reporting resist substitution and rollback attacks.

## Arabic UX impact

Native adapters present localized, accessible RTL progress, scheduling, elevation context, failure, and recovery. Compatibility details remain structured and safely localized.

## Consequences and tradeoffs

### Positive

- Product safety policy remains consistent while each OS uses supported installation mechanisms.
- Update outcomes are explicit, recoverable, and auditable.

### Negative

- Every platform needs a maintained adapter and end-to-end release testing.

### Risks

- Policy and adapter state may diverge across crashes; mitigate with idempotent handoff, signed state, correlation, and startup reconciliation.

## Verification

Foundation review confirms the split of responsibility and signed-update requirements. Production verification requires signature, compatibility, staged rollout, interruption, disk-space, migration, rollback-protection, and localized UI tests per platform.

## Follow-up work

- Define the update state machine, signing hierarchy, compatibility matrix, and platform distribution assumptions.
- Threat-model update delivery and adapter elevation before implementation.

## Related decisions and documents

- [Target architecture: Update model](../architecture/target-architecture.md#update-model)
- [ADR-0002](0002-native-platform-ui-shells.md)
- [ADR-0005](0005-generated-or-validated-contracts.md)
- [ADR-0009](0009-zero-trust-security-model.md)
