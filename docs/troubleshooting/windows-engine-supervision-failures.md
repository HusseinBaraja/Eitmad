---
title: "Resolve Windows engine supervision failures"
description: "Diagnose restart exhaustion, forced shutdown, stale process events, and Job Object setup failures safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Windows platform maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "Windows supervision states, retry bounds, process containment, or shutdown recovery changes"
keywords:
  - "RestartExhausted"
  - "EngineSupervisionState"
  - "Windows Job Object"
  - "forced engine shutdown"
---

# Resolve Windows engine supervision failures

`RestartExhausted`, `Faulted`, or a forced exit means the Windows adapter could not keep one healthy supervised engine running or could not drain it within 15 seconds. Product work must not continue until the engine is `Ready`; do not bypass containment or start an untracked replacement manually.

## Symptoms

- `EngineSupervisionState.RestartExhausted` after four consecutive unexpected exits;
- `EngineSupervisionState.Faulted` when Rust reports `RetryDisposition::Never` or launch/containment setup fails;
- `EngineExitOutcome.Forced` is `true` after shutdown;
- the engine never reaches Rust `LifecycleState.Ready`;
- a replacement process exists but an older exit callback also arrives.

No Arabic UI message exists yet. These are engineering state names, not approved user-facing copy. A future shell message must use localized Arabic labels in an RTL layout, isolate LTR identifiers such as `RestartExhausted` and correlation IDs, and preserve mixed Arabic/English diagnostic text without reordering it. Arabic search must cover the approved label and common terms such as `تعذر تشغيل المحرك` and `توقف المحرك`. Shell tests and support-report rendering must verify Arabic shaping, bidi isolation, numerals, identifier copy/paste, and redaction before that message ships; the Rust state and error identifiers remain the diagnostic authority.

## Fast checks

1. Capture only supervision state, generation, restart count, exit code, Rust engine instance ID, stable error code, message ID, retry disposition, lifecycle stage, versions, timestamp, and correlation ID.
2. Run non-mutating diagnostics with `cargo run -q -p eitmad-engine-cli -- diagnose`.
3. Confirm the packaged engine path exists and the process is launched by the intended shell build.
4. Confirm no other engine owns the same runtime directory; do not delete the authority lock file.
5. Reproduce with the Windows supervision scenario command in the [subsystem guide](../developer/subsystems/windows-process-supervision.md#tests-and-safe-extension-points).

## Causes and resolutions

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| `RestartExhausted`, restart count `3` | The engine failed four times inside 60 seconds | Inspect the last typed Rust error and diagnostic report | Correct the named engine/environment failure, then explicitly start a new supervision session |
| `Faulted` with retry `Never` | Rust says repeating the same launch is unsafe | Match the stable error in [engine startup troubleshooting](engine-startup-failures.md) | Correct the non-retryable cause; do not loop or override retry metadata |
| `Faulted` without a Rust lifecycle snapshot | Executable launch or Job Object creation/assignment failed | Verify Windows support, packaging, and executable availability | Repair packaging or platform containment; never continue with an ungrouped child |
| `Forced: true` | Engine did not exit within 15 seconds | Preserve the last lifecycle state and component-shutdown error | Treat the exit as crash recovery and repair the Rust component that failed to drain |
| Old generation reports exit after replacement starts | An asynchronous callback completed late | Compare its generation and `EngineInstanceId` with the current snapshot | No process action is required; the adapter ignores stale observations by design |

## Verify recovery

The real-engine scenario must observe `Starting → Ready → Stopping → Stopped`, exit `0`, and report `Forced: false`. A repeated failure must stop at `RestartExhausted` rather than launch a fifth process.

## Escalate safely

Escalate to Windows platform maintainers with the sanitized fields from the fast checks and the exact scenario that failed. Never include raw stdout/stderr, executable or runtime paths, process command lines, secrets, customer records, database content, authorization graphs, or copied lock-file contents.

Return to [Troubleshoot Eitmad](index.md) or review [ADR-0016](../decisions/0016-bounded-platform-process-supervision.md).
