# ADR-0003: Run the engine as a separate supervised process

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: Process isolation prevents a required lifecycle, performance, security, or deployment capability
> - Supersedes: None
> - Superseded by: None

## Context and problem

Embedding the engine into every shell would couple authoritative state and lifecycle to UI technology. The engine must also support unattended and diagnostic use while preserving state when a shell crashes.

## Decision drivers

- Fault and lifecycle isolation
- Independent headless and diagnostic operation
- Explicit, securable client-authority boundary

## Considered options

- **Separate supervised process:** clear isolation and reuse, with IPC and lifecycle cost.
- **In-process library:** simpler calls, but couples engine lifetime and trust to each shell.
- **Remote-only service:** central control, but prevents reliable local-first operation.

## Decision

The Rust engine MUST run as a separate process. The shell supervises an engine instance it launches: discovery, authenticated startup, compatibility negotiation, health observation, reconnection, and graceful shutdown. The engine MUST detect abandoned sessions and preserve storage, audit, migration, and sync safety without a living shell. Accidental duplicate authorities are prohibited.

## Architecture and contract impact

Communication uses authenticated, typed local IPC with version and capability negotiation, deadlines, cancellation, backpressure, and reconnect semantics.

## Storage and sync impact

Engine shutdown and crash recovery must preserve committed state, audit outcomes, checkpoints, and migration integrity.

## Security impact

Same-device processes are not implicitly trusted. Both peers authenticate the channel, limit privileges, validate input, and avoid exposing secrets in process arguments or diagnostics.

## Arabic UX impact

Shells must present localized startup, compatibility, recovery, and connection states without exposing internal diagnostics.

## Consequences and tradeoffs

### Positive

- UI failures do not automatically corrupt or redefine authoritative work.
- One engine supports native shells, diagnostics, and headless workflows.

### Negative

- Packaging, supervision, IPC, and cross-process debugging add complexity.

### Risks

- Restart loops or duplicate instances could damage availability; mitigate with ownership locks, bounded restart policy, health states, and explicit recovery.

## Verification

Documentation review confirmed lifecycle and failure responsibilities are explicit. Implementation requires process crash, abandoned-session, compatibility, and clean-shutdown tests.

## Follow-up work

- Threat-model local IPC and engine discovery.
- Define supervision states, restart bounds, and single-authority enforcement.

## Related decisions and documents

- [Target architecture: Process model](../architecture/target-architecture.md#process-model)
- [ADR-0004](0004-headless-engine-mode.md)
- [ADR-0009](0009-zero-trust-security-model.md)
