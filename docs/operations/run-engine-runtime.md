---
title: "Run and diagnose the engine runtime"
description: "Start the engine in headless or supervised mode, interpret readiness JSON, and run non-mutating diagnostics."
audience: "operations"
page_type: "task"
status: "active"
owner: "Rust engine maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "engine CLI arguments, process modes, lifecycle output, exit codes, or runtime directory behavior changes"
keywords:
  - "eitmad-engine-cli"
  - "run --mode headless"
  - "run --mode supervised"
  - "diagnose"
  - "readyToStart"
---

# Run and diagnose the engine runtime

The engine CLI runs one foreground authority or performs one non-mutating diagnostic pass. Stdout contains JSON only. Stderr contains sanitized structured failures only.

## Before you start

- Run commands from the repository root.
- Do not point `--runtime-directory` at customer documents, a shared network path, or another application's data.
- Treat the supervisor PID and lock metadata as correlation data, not credentials.
- Authenticated IPC and product storage are not implemented in this foundation.

## Run non-mutating diagnostics

```powershell
cargo run -q -p eitmad-engine-cli -- diagnose
```

Expected result: one `DiagnosticReport` JSON line with `status: "healthy"`, `readyToStart: true`, and exit code `0`. Diagnostics do not acquire the authority lock or start components, so they may run while an engine is active.

## Run independent headless mode

```powershell
cargo run -q -p eitmad-engine-cli -- run --mode headless
```

Expected result: `starting` then `ready` lifecycle lines. Press Ctrl+C once. The engine emits `stopping` then `stopped` and exits `0` after bounded draining.

## Run supervised desktop mode

A shell must launch the child with stdin and stdout pipes, keep stdin open for the owned lifetime, and read stdout as newline-delimited JSON. Use its real PID for `--supervisor-pid`; this value does not authenticate the shell.

```powershell
'' | cargo run -q -p eitmad-engine-cli -- run --mode supervised --supervisor-pid $PID
```

The example sends one empty pipeline record, then closes the input pipe. Expected result: `starting`, `ready`, `stopping`, and `stopped`, followed by exit code `0`. A production shell closes the pipe on requested shutdown; an unexpected shell exit produces the same EOF signal and graceful engine shutdown.

## Interpret status and exit codes

| Signal | Meaning | Operator action |
| --- | --- | --- |
| `live: true`, `ready: false` | Starting or draining; do not send work | Wait for a later lifecycle event |
| `state: "ready"`, `ready: true` | Components and required health checks are ready | Establish authenticated IPC when that transport exists |
| `health: "degraded"`, `ready: true` | Advisory check is non-healthy | Continue only while investigating the advisory check |
| `state: "failed"` | Startup or shutdown failed | Use the stable error code and troubleshooting guide |
| Exit `0` | Clean shutdown or healthy diagnostics | No recovery needed |
| Exit `1` | Runtime lifecycle failure | Inspect sanitized stderr and failed snapshot |
| Exit `2` | Invalid CLI syntax | Correct the arguments; no engine authority started |
| Exit `3` | Diagnostic check is unhealthy | Correct the named check before starting an authority |

## Recover safely

Do not delete the lock file to resolve a conflict. Confirm that the other engine process has stopped, then retry; OS lock release makes stale contents harmless. Do not include runtime paths, raw process arguments, customer records, secrets, or authorization data in support evidence.

For exact failure checks, see [Resolve engine startup and authority failures](../troubleshooting/engine-startup-failures.md). For implementation ownership, see [Extend the engine runtime lifecycle safely](../developer/subsystems/engine-runtime.md).
