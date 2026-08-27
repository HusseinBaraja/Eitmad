---
title: "Extend Windows engine process supervision safely"
description: "Understand Windows Job Object containment, bounded engine restart, stale-event rejection, and clean shutdown."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Windows platform maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "Windows engine launch, process containment, restart, lifecycle parsing, or shutdown behavior changes"
keywords:
  - "EngineSupervisor"
  - "EngineSupervisionState"
  - "EngineIpcHealthState"
  - "Windows Job Object"
  - "RestartExhausted"
---

# Extend Windows engine process supervision safely

`Eitmad.Platform.Windows.ProcessSupervision` groups the Rust engine with the Windows application, recovers from bounded unexpected exits, rejects stale process observations, and drains the engine on intentional shutdown. It is platform infrastructure; it does not create a Windows UI or own product behavior.

## Ownership and boundaries

| Concern | Authority |
| --- | --- |
| Engine lifecycle, identity, structured failures, and retry safety | Rust contracts and `eitmad-engine-runtime` |
| CLI process arguments and stdin-EOF shutdown | `eitmad-engine-cli` |
| Engine path, local runtime directory, development launch identity, redirected pipes, Job Object containment, restart budget, and forced termination | Windows platform adapter |
| Typed named-pipe client, handshake, and unavailable-engine mapping | `Eitmad.Platform.Windows.LocalIpc` |
| Localized recovery UI and accessibility | `Eitmad.WindowsShell` |

The adapter links the generated `Eitmad.Contracts` binding. It does not handwrite DTOs, read product configuration or databases, authorize requests, perform synchronization, or treat the supervisor PID as authentication.

## Supervision flow

```mermaid
sequenceDiagram
    participant Shell as "Windows operations shell"
    participant Bridge as "WindowsEngineBridge"
    participant Supervisor as "EngineSupervisor"
    participant Job as "Windows Job Object"
    participant Engine as "Rust engine CLI"
    Shell->>Bridge: Create(command-line arguments)
    Bridge->>Bridge: Select engine, runtime, launch identity
    Shell->>Bridge: StartAsync()
    Bridge->>Supervisor: StartAsync(authorized launch request)
    Supervisor->>Job: Create kill-on-close group
    Supervisor->>Engine: run --mode supervised
    Supervisor->>Job: Assign engine process
    Engine-->>Supervisor: Starting, Ready
    Shell->>Supervisor: StopAsync()
    Supervisor->>Engine: Typed shutdown request
    Engine-->>Supervisor: Shutdown accepted
    Supervisor->>Engine: Close stdin lifetime pipe
    Engine-->>Supervisor: Stopping, Stopped, exit 0
    Supervisor->>Job: Close empty group
```

`EngineSupervisor` serializes session state and exposes immutable `EngineSupervisionSnapshot` values. `Starting` and `Running` project the active process; `RestartDelay` and `RestartExhausted` belong only to native process supervision. The shell-local `IpcHealth` distinguishes `Unavailable`, `Connecting`, `Connected`, and `ReconnectExhausted`; it reports transport availability without replacing the Rust `LifecycleSnapshot` as engine readiness and health authority.

## Restart and stale-event invariants

An exit is intentional only after `StopAsync` marks the current generation for shutdown. Every other exit follows Rust retry metadata:

- `Never` enters `Faulted` without replacement;
- `SafeImmediately` uses the native bounded delay;
- `SafeAfterDelay` uses the greater of the Rust delay and native delay;
- no structured error uses the native delay.

The rolling window allows three replacements in 60 seconds at one, two, and four seconds. A fourth failure enters `RestartExhausted`. Five continuous minutes in `Ready` clears the history. Only a new `StartAsync` begins another session after exhaustion.

Every process launch increments `Generation`. Output is accepted only from that generation and, after the first lifecycle snapshot, from the same `EngineInstanceId`. PID is correlation metadata and is never used as stable identity.

The supervisor also owns IPC subscription continuity. It advertises protocol `1.0–1.6` and offers and requires `eitmad.capability.authorization-scopes.v1`. `WindowsEngineBridge` is the platform bootstrap boundary. It resolves only the packaged or explicitly supplied engine path and selects the local runtime directory. The launcher passes a random bootstrap token through inherited standard input. It never constructs an identity or permission assertion. Rust returns the verified installation authorization context. The supervisor retains generated subscription descriptors and only the cursor acknowledged after UI processing. Connection loss makes `IpcHealth` `Connecting` and permits at most the restart policy's three default reconnect attempts after 100 ms, 500 ms, and two seconds while the current generation remains `Ready`. Exhaustion sets `ReconnectExhausted`, so callers can distinguish a live process from a usable IPC channel. Same-generation reconnect resumes replay; engine replacement raises `ResyncRequired`, opens a fresh stream, and leaves the owning feature responsible for an authoritative query before applying buffered events.

## Shutdown and containment

Normal shutdown cancels pending restart, requests typed IPC shutdown when a negotiated session exists, closes stdin to preserve abandonment semantics and release the Windows reader, continues reading lifecycle output, and waits 15 seconds. A clean engine reaches `Stopped` and exits `0`; the adapter then closes the empty Job Object. If the deadline expires, `TerminateJobObject` ends the contained process tree and records `Forced: true`.

`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` is the crash safety net. If the shell exits without cleanup, Windows closes its Job Object handle and terminates the engine tree. If initial assignment fails, the launcher kills the child and does not continue with an uncontained engine.

## Security, privacy, and Arabic behavior

Process supervision is not human authentication. The named-pipe client proves possession of the supervised launch token, then uses the Rust-returned session. The adapter exposes parsed contract errors and exit metadata; it does not expose or log raw streams, paths, customer data, secrets, or authorization graphs.

The implemented Windows operations shell maps typed states to Arabic recovery text, sets the window and primary navigation to RTL, and isolates process, error, protocol, configuration, and mixed workshop identifiers in LTR child containers. Process supervision still does not own localization, layout, search, documents, or accessibility policy.

## Tests and safe extension points

The dependency-free scenario harness beside the adapter covers intentional stop, unexpected death, exhaustion, stale exit, subscription queue overflow and reattachment, graceful shutdown, and timeout termination. Passing `--engine target/debug/eitmad-engine-cli.exe` adds the real Windows Job Object and Rust lifecycle smoke flow.

Run:

```powershell
cargo build -p eitmad-engine-cli
```

```powershell
dotnet run --project platform-adapters/windows/tests/Eitmad.Platform.Windows.Tests.csproj -- --engine target/debug/eitmad-engine-cli.exe
```

Extend `Shell/WindowsEngineBridge.cs` for Windows launch selection and authorized bootstrap. Extend `ProcessSupervision` only for Windows lifecycle mechanics. Preserve generated Rust contract use, generation and instance checks, bounded retry, kill-on-close containment, and graceful-first shutdown. Add product behavior to its Rust vertical and presentation to the [Windows operations shell](windows-native-shell.md) instead.

For IPC authority and failures, see [typed local IPC](local-ipc.md) and [Resolve local IPC failures](../../troubleshooting/local-ipc-failures.md). For process recovery, use [Resolve Windows engine supervision failures](../../troubleshooting/windows-engine-supervision-failures.md).
