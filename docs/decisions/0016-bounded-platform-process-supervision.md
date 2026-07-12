---
title: "ADR-0016: Bound platform engine process supervision"
description: "Defines containment, restart, stale-event, and shutdown rules for native engine supervisors."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "desktop process ownership, restart policy, engine shutdown, or platform containment support changes"
keywords:
  - "engine process supervision"
  - "Windows Job Object"
  - "RestartExhausted"
---

# ADR-0016: Bound platform engine process supervision

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-12
> - Decision owners: Architecture maintainers and native platform maintainers
> - Last verified: 2026-07-12
> - Review trigger: Desktop process ownership, restart policy, engine shutdown, or platform containment support changes
> - Supersedes: None
> - Superseded by: None

## Context and problem

[ADR-0003](0003-separate-supervised-engine-process.md) requires each desktop shell to supervise its separate Rust engine. A process launcher alone does not prevent orphaned engines, crash loops, stale exit callbacks, or abrupt shutdown that skips the engine drain deadline. These rules must be consistent across platforms without moving engine authority into a native shell.

## Decision drivers

- One visible application must not leave an owned engine process tree behind.
- Recovery must remain bounded and must respect Rust-owned retry safety.
- PID reuse and asynchronous callbacks must not let an old process affect its replacement.
- Normal exit must allow the Rust engine to drain before platform termination.

## Considered options

- **Bounded native supervision:** use typed Rust lifecycle evidence plus platform containment. This adds adapter state but gives deterministic recovery.
- **Rely only on engine stdin EOF:** supports graceful abandonment detection but cannot contain descendants or force a hung process tree to exit.
- **Restart every failure indefinitely:** maximizes attempted availability but creates crash loops and ignores `RetryDisposition::Never`.
- **Never restart automatically:** is simple but discards safe recovery from isolated process failure.

## Decision

Every desktop platform adapter MUST own one supervision session for an engine it launches. It MUST:

- group the engine process tree with the application when the operating system provides a reliable containment primitive;
- identify each launch with a monotonic supervisor generation and the Rust `EngineInstanceId`;
- ignore output and exit observations from any stale generation or mismatched instance;
- restart only unexpected, retry-safe exits under a bounded rolling policy;
- cancel pending restart when stop is requested and require explicit action after exhaustion;
- close the engine control pipe first, wait beyond the Rust drain deadline, then terminate the contained process tree if exit does not complete;
- keep process arguments and diagnostics free of secrets and product data.

The first Windows adapter uses a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. It allows three replacement launches within 60 seconds with delays of one, two, and four seconds. Five continuous minutes in Rust `Ready` clears the budget. Shutdown waits 15 seconds before `TerminateJobObject`.

## Architecture and contract impact

Rust remains authoritative for `LifecycleSnapshot`, `EngineProcessIdentity`, `ContractError`, and `RetryDisposition`. Native `EngineSupervisionState` describes adapter activity only; it is not a product contract and must not replace engine lifecycle state. PID and inherited pipes coordinate lifetime but do not authenticate IPC.

Other desktop platforms must implement the same observable invariants using native containment. They may choose different OS primitives, but a policy difference requires an explicit platform decision and conformance evidence.

## Storage and sync impact

The adapter reads and writes no product state. Graceful shutdown gives Rust-owned storage, audit, and sync components time to drain. Forced termination remains crash recovery; those Rust components must preserve committed-state safety independently of the shell.

## Security impact

The Job Object contains descendants and closes on shell termination. The adapter passes only mode, supervisor PID, and an optional platform runtime-directory override. It parses generated Rust contract types and does not log raw stdout, stderr, paths, secrets, customer records, or authorization data.

## Arabic UX impact

This foundation has no visual surface, Arabic label, input, search, report, or accessibility tree. A future shell must localize typed lifecycle and recovery states, render them RTL, and avoid exposing internal process details. Machine identifiers remain directionally isolated LTR values.

## Consequences and tradeoffs

### Positive

- Normal shutdown drains cleanly while shell crashes still release the full process tree.
- Isolated crashes recover automatically without an infinite loop.
- Stale asynchronous events cannot terminate or replace the current engine.

### Negative

- Each platform needs native containment code and platform-specific integration tests.
- Windows launches before Job Object assignment, leaving a short assignment interval; inherited stdin still closes if the shell dies during that interval.

### Risks

- A containment API failure could leave an ungrouped child; the launcher kills the child and enters a fault instead of continuing unsupervised.
- A hung process may exceed the graceful deadline; the adapter records forced termination and operators investigate before retrying repeatedly.

## Verification

Dependency-free C# scenarios cover intentional stop, unexpected death, restart exhaustion, stale exits, clean shutdown, and forced timeout. A Windows smoke scenario launches the real Rust CLI, observes `Starting → Ready → Stopping → Stopped`, and verifies exit code `0` without Job Object termination.

## Follow-up work

- Integrate the adapter into the future C# shell bootstrap after the Arabic-first pre-shell gate is approved.
- Define equivalent containment implementations when macOS and Linux shells begin.
- Integrate authenticated local IPC without treating process supervision as authentication.

## Related decisions and documents

- [ADR-0003](0003-separate-supervised-engine-process.md)
- [ADR-0009](0009-zero-trust-security-model.md)
- [Windows process supervision guide](../developer/subsystems/windows-process-supervision.md)
- [Run and diagnose the engine](../operations/run-engine-runtime.md)
