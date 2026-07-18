---
title: "Run and diagnose the engine runtime"
description: "Start the engine in headless or supervised mode, interpret readiness JSON, and run non-mutating diagnostics."
audience: "operations"
page_type: "task"
status: "active"
owner: "Rust engine maintainers"
last_verified: "2026-07-18"
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
- Windows typed IPC is implemented with development-only authentication. Rust-owned SQLite storage is active; trusted production identity provisioning remains blocked.

## Run non-mutating diagnostics

```powershell
cargo run -q -p eitmad-engine-cli -- diagnose
```

Expected result: one `DiagnosticReport` JSON line with `status: "healthy"`, `readyToStart: true`, and exit code `0`. Diagnostics do not acquire the authority lock, create a missing database, run migrations, or start components. If `eitmad.sqlite3` exists, the authority-store check opens it read-only and rejects corruption or a newer storage version.

## Run independent headless mode

```powershell
cargo run -q -p eitmad-engine-cli -- run --mode headless
```

Expected result: `starting` then `ready` lifecycle lines. Press Ctrl+C once. The engine emits `stopping` then `stopped` and exits `0` after bounded draining.

On first authoritative start, the storage component creates and migrates `eitmad.sqlite3` under the selected runtime directory after acquiring the authority lock and before emitting `ready`. Keep the runtime directory user-private. Do not edit, move, or copy an open database as a recovery shortcut.

## Run supervised desktop mode

A shell launches the child with stdin/stdout lifecycle pipes and a unique `--ipc-pipe-name`. The Windows adapter may add `--allow-insecure-development-auth` only for synthetic development sessions and supplies its random token through `EITMAD_DEVELOPMENT_IPC_TOKEN`; never record or reuse that value. The supervisor PID remains correlation data, not authentication.

The following lifecycle-only example omits `--ipc-pipe-name`, so typed IPC is not available:

```powershell
'' | cargo run -q -p eitmad-engine-cli -- run --mode supervised --supervisor-pid $PID
```

The example sends one empty pipeline record, then closes the input pipe. Expected result: `starting`, `ready`, `stopping`, and `stopped`, followed by exit code `0`. A production shell closes the pipe on requested shutdown; an unexpected shell exit produces the same EOF signal and graceful engine shutdown.

The implemented Windows adapter additionally assigns the child to a kill-on-close Job Object. It restarts only unexpected retry-safe exits, stops after three replacements within 60 seconds, ignores stale generations, and waits 15 seconds after closing stdin before terminating the process group. Run its fake and real-engine checks from the [Windows supervision guide](../developer/subsystems/windows-process-supervision.md#tests-and-safe-extension-points).

## Interpret status and exit codes

| Signal | Meaning | Operator action |
| --- | --- | --- |
| `live: true`, `ready: false` | Starting or draining; do not send work | Wait for a later lifecycle event |
| `state: "ready"`, `ready: true` | Components and required health checks are ready | Negotiate the typed local IPC session before sending work |
| `health: "degraded"`, `ready: true` | Advisory check is non-healthy | Continue only while investigating the advisory check |
| `state: "failed"` | Startup or shutdown failed | Use the stable error code and troubleshooting guide |
| Exit `0` | Clean shutdown or healthy diagnostics | No recovery needed |
| Exit `1` | Runtime lifecycle failure | Inspect sanitized stderr and failed snapshot |
| Exit `2` | Invalid CLI syntax | Correct the arguments; no engine authority started |
| Exit `3` | Diagnostic check is unhealthy | Correct the named check before starting an authority |

## Recover safely

Do not delete the lock file to resolve a conflict. Confirm that the other engine process has stopped, then retry; OS lock release makes stale contents harmless. Do not include runtime paths, raw process arguments, customer records, secrets, or authorization data in support evidence.

For exact IPC checks, see [Resolve local IPC failures](../troubleshooting/local-ipc-failures.md). For storage/configuration/permission failures, see [configuration and authorization recovery](../troubleshooting/configuration-authorization-failures.md). For engine failures, see [Resolve engine startup and authority failures](../troubleshooting/engine-startup-failures.md). Implementation ownership is split between [typed local IPC](../developer/subsystems/local-ipc.md), the [engine lifecycle](../developer/subsystems/engine-runtime.md), and [Windows supervision](../developer/subsystems/windows-process-supervision.md).
