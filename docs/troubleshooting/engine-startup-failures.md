---
title: "Resolve engine startup and authority failures"
description: "Diagnose failed engine startup, invalid supervision, unhealthy readiness checks, and duplicate authority errors safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust engine maintainers"
last_verified: "2026-07-18"
review_triggers:
  - "engine lifecycle error IDs, authority locking, readiness checks, CLI exit codes, or recovery behavior changes"
keywords:
  - "eitmad.error.engine-startup-failed.v1"
  - "eitmad.error.engine-already-running.v1"
  - "eitmad.error.engine-supervisor-invalid.v1"
  - "eitmad.error.engine-health-check-failed.v1"
  - "engine failed startup"
---

# Resolve engine startup and authority failures

A `failed` lifecycle snapshot means the engine did not become ready or could not drain safely. Startup rollback stops initialized components before releasing authority. Rust-owned SQLite storage may already contain durable configuration, authorization, audit, and idempotency state; do not delete lock files, databases, or WAL companions, and do not bypass health checks.

## Symptoms

- stdout contains `state: "failed"` and `ready: false`;
- stderr contains a `ContractError` with an `eitmad.error.engine-*.v1` code;
- the process exits `1`, or diagnostics exit `3`;
- a second engine reports `eitmad.error.engine-already-running.v1`;
- supervised mode reports `eitmad.error.engine-supervisor-invalid.v1`.

These identifiers apply to Windows, macOS, and Linux; supervised desktop and headless modes share the same Rust lifecycle. No user-facing Arabic message exists yet. A future shell must localize the returned `messageId` without exposing internal details.

## Fast checks

1. Run non-mutating diagnostics:

   ```powershell
   cargo run -q -p eitmad-engine-cli -- diagnose
   ```

2. Capture only the stable code, `messageId`, lifecycle stage, engine mode, product/protocol versions, timestamp, and correlation ID.
3. Confirm whether another engine launched with the same runtime directory is still active.
4. Recheck the mode arguments. Supervised mode requires a nonzero PID different from the engine PID; headless mode accepts no supervisor PID.

## Causes and resolutions

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| `eitmad.error.engine-already-running.v1` | Another process owns the runtime-directory OS lock | Confirm the intended shell or headless process is still running | Stop the owning process cleanly, wait for exit, then retry; do not delete the file |
| `eitmad.error.engine-supervisor-invalid.v1` | Missing, zero, self, or mode-incompatible supervisor PID | Review only the mode and numeric PID | Relaunch with `--mode supervised --supervisor-pid <shell PID>`, or remove the PID in headless mode |
| `eitmad.error.engine-health-check-failed.v1` with `readinessCheck` | A required check is degraded, unhealthy, or timed out | Run `diagnose` and inspect the check ID/status | Correct the environment represented by the stable check ID, then retry |
| `eitmad.error.engine-startup-failed.v1` with `authorityLock` | Runtime directory or lock metadata cannot be created safely | Verify the selected directory is user-local and available | Correct directory availability or permissions; never redirect it into customer data |
| `eitmad.error.engine-startup-failed.v1` with `componentStartup` | A component failed or the 30-second startup deadline elapsed | Reproduce with focused runtime tests and sanitized correlation data | Repair the owning component; startup rollback has already stopped earlier components |
| `eitmad.error.engine-startup-failed.v1` with `componentStartup` after storage was introduced | SQLite could not open or transactionally migrate | Run read-only diagnostics; preserve the complete runtime directory and storage version | Correct permissions/environment or restore an approved stopped-engine backup; never edit schema rows manually |
| `eitmad.error.engine-shutdown-failed.v1` | A component failed to stop or the 10-second deadline elapsed | Confirm the process exited and preserve the structured stage | Treat state as failed, investigate the owning component, and verify its cleanup before restart |

## Verify recovery

Run diagnostics, start the intended mode, observe `state: "ready"` with `ready: true`, then request clean shutdown and observe `stopped` with exit `0`. If the same failure returns, stop retrying automatically and escalate to the Rust engine maintainers.

Safe escalation evidence excludes paths, stdin contents, raw component errors, secrets, product payloads, customer data, authorization graphs, and copied lock-file contents. See [Run and diagnose the engine runtime](../operations/run-engine-runtime.md), [engine runtime ownership](../developer/subsystems/engine-runtime.md), and [Windows supervision recovery](windows-engine-supervision-failures.md).
