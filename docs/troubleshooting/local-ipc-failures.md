---
title: "Resolve local IPC connection and request failures"
description: "Diagnose unavailable engines, subscription capacity, resync, backpressure, version mismatch, deadlines, and payload limits safely."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Rust engine and Windows platform maintainers"
last_verified: "2026-10-17"
review_triggers:
  - "local IPC errors, timeouts, payload bounds, negotiation, or recovery changes"
keywords:
  - "EngineUnavailable"
  - "CommandOutcomeUnknown"
  - "blocked IPC write"
  - "SessionChanged"
  - "projectionInvalidated"
  - "eitmad.error.ipc-session-invalid.v1"
  - "eitmad.error.ipc-deadline-exceeded.v1"
  - "eitmad.error.ipc-payload-too-large.v1"
  - "eitmad.error.ipc-subscription-resync-required.v1"
  - "eitmad.error.ipc-subscription-unsupported.v1"
  - "eitmad.error.ipc-subscription-capacity-exceeded.v1"
---

# Resolve local IPC connection and request failures

Local IPC failures do not by themselves imply lost committed data. Stop new work, preserve the correlation ID and negotiated version, and use non-destructive checks before restarting the supervised engine.

## Symptoms and fast checks

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| `EngineUnavailable` | Engine absent, starting, or endpoint stale | Check the latest `LifecycleSnapshot` and supervisor generation | Wait for `Ready`; retry connection once against the current generation |
| Local `Timeout` or `OperationCanceledException` | A request expired or its caller cancelled; the request budget includes the write-lock wait, transmission, flush, and reply | Check operation type and elapsed time without recording frame contents | Retry a needed query after transport recovery; a command that never started transmission can be submitted again |
| `CommandOutcomeUnknown` | A command started transmission but no response was confirmed before cancellation, timeout, session invalidation, or connection loss | Preserve the existing retry payload and idempotency key in the owning feature; do not log them | Reconnect if needed and retry only the unchanged command with the same key; never infer that it failed or create a replacement command |
| A blocked write ends with `Timeout` or `CommandOutcomeUnknown`, and queued requests report `ConnectionLost` | The reader did not consume a frame before its request deadline | Run the synthetic adapter `IpcCancellationScenario` checks; inspect sanitized operation type and timing | Let supervision reconnect the closed transport; do not increase timeouts or reuse a stream containing an incomplete frame. This signature alone does not prove the cause of an earlier incident |
| Handshake authentication rejection | The inherited bootstrap token is absent, malformed, or does not match this engine generation | Confirm that the supervisor launched the current engine and connected with its retained token | Stop the failed generation and let the supervisor start a new paired process; do not copy or override the token |
| `eitmad.error.protocol-incompatible.v1` | No common protocol/capability | Capture both product versions and `PeerHello` ranges | Install a compatible shell/engine pair; do not bypass negotiation |
| `eitmad.error.ipc-session-invalid.v1` | Reused/stale session or changed context | Compare engine generation and request session ID | Reconnect and rebuild envelopes from the new negotiated session |
| `eitmad.error.ipc-deadline-exceeded.v1` | Deadline passed before query completion | Compare deadline, current time, and operation type | Retry a query if still needed; retry a command only with the same idempotency key because outcome may be unknown |
| `eitmad.error.ipc-payload-too-large.v1` | Frame exceeds 8 MiB | Measure encoded frame size without logging content | Page or stream through a domain contract; do not raise the cap ad hoc |
| `eitmad.error.ipc-subscription-unsupported.v1` | Peer omitted the subscription or policy-event capability | Capture negotiated protocol and capability IDs | Use a compatible engine/shell pair and required capability; do not replace subscriptions with UI polling |
| Handshake rejects missing `eitmad.capability.authorization-scopes.v1` | Peer cannot preserve the required tenant/workspace context | Compare generated binding and offered/required capability sets | Upgrade the shell/adapter with the engine; never disable scoped authorization |
| `eitmad.error.ipc-subscription-capacity-exceeded.v1` | One client connection already owns 64 active subscriptions | Count active feature subscriptions without recording payloads or scope data | Reuse or explicitly unsubscribe obsolete streams; do not raise the ceiling without a resource review |
| `eitmad.error.ipc-subscription-resync-required.v1` | Cursor expired, came from another generation, stream, or scope | Compare engine generation and subscription kind without exposing cursor ownership | Subscribe fresh, query the authoritative scoped projection, then apply buffered live events |
| `SubscriptionClosed` with `backpressure` | A discrete record, notification, or error gap exceeded replay | Preserve the last processed cursor and sanitized event kind | Allow supervision to reconnect/resubscribe; if resync is required, query current state before continuing |
| `SubscriptionClosed` with `authorizationRevoked` | Policy reauthorization denied the stream | Preserve only principal/scope identifiers approved for support | Discard later buffered state and resubscribe only after an owner restores access |
| `SubscriptionClosed` with `projectionInvalidated`, surfaced as `SessionChanged` | A Product policy change invalidated cached protected data even though read access may remain | Check the typed failure kind; IPC can still be `Connected` | Clear the old projection, subscribe fresh, and query Rust before showing protected data again; do not wait for transport recovery |
| Repeated `SubscriptionBackpressure` | Shell consumer is slower than event production | Measure queue depth/event rate without recording payloads | Fix consumer work or reduce producer frequency; do not enlarge bounds without a resource review |
| Connection closes after a large response | Encoded response exceeded 8 MiB or response serialization failed | Check the stable request type and sanitized frame-size metric | Reduce or page the response, reconnect, and retry only when the operation's retry contract permits it |
| `local IPC server failed` or `local IPC server task failed` on stderr | The accept loop returned an I/O error or the task panicked | Preserve the sanitized message and latest lifecycle state | Stop new work, restart through the supervisor, and escalate to the Rust IPC owner if it recurs |
| Forced shutdown | IPC and stdin drain did not exit in 15 seconds | Preserve lifecycle state and stable error IDs | Treat as crash recovery and diagnose the Rust component that failed to drain |

Do not delete runtime files, expose bootstrap or bearer tokens, copy raw frames, or include customer records, paths, authorization graphs, or process environment values in support evidence.

## Correlate activation, account switching, and server confirmation

Use this path for Windows supervised development when activation is slow, an account switch fails, or Home returns cached data with `ServerAvailable=false`. It uses a new synthetic installation. Do not point it at a customer installation or copy saved databases, credentials, tokens, or old incident logs into it.

Install the prerequisites in [connected workflow setup](../operations/run-workflow-test-environment.md). Use Rust 1.85.1, .NET 8, PowerShell 7.4 or later, and PostgreSQL 17. From the repository root:

```powershell
$env:RUSTC = (rustup which --toolchain 1.85.1 rustc)
$env:RUSTDOC = (rustup which --toolchain 1.85.1 rustdoc)
$env:CARGO_TARGET_DIR = "$PWD/target/toolchain-1.85.1"
$env:CARGO_INCREMENTAL = '0'
$diagnosticState = "$PWD/tests/engine-integration/artifacts/activation-$([Guid]::NewGuid().ToString('N'))"
pwsh -File ./tools/workflow-environment.ps1 Start -DiagnosticActivation -StateDirectory $diagnosticState -PostgresBin '<PostgreSQL-17-bin>' -DatabasePort 55449 -ServerPort 58449
```

Choose two unused ports. The script builds the owners, provisions synthetic accounts and sample work, and runs `WorkflowEnvironmentTests.DiagnosticAccountSwitchAndConfirmation`. The test signs in as each role, awaits real WPF activation, runs a synthetic Arabic Home search to include catalog, confirms an order-list response from the TLS server, and switches accounts without restarting the engine. It changes session state through Rust; the diagnostic test does not create or edit business records. Existing environment provisioning rules still apply. Success requires a passing TRX result; a cached response alone is insufficient.

Every launched process gets separate timestamp-and-random-suffix stdout/stderr files. PostgreSQL gets a separate server log per start, and each test gets its own `launch-check-*` TRX directory. Older logs and state remain intact. After success or failure, stop the services owned by this new installation:

```powershell
pwsh -File ./tools/workflow-environment.ps1 Stop -StateDirectory $diagnosticState
```

Retain the diagnostic stdout/stderr, TRX, and PostgreSQL log together. No reset or deletion is required. Correct the measured cause, then use a new run for verification. For code changes, omit `-SkipBuild`. That switch is valid only for unchanged verified outputs.

### Read the bounded diagnostic events

`EITMAD_DIAGNOSTICS=1` enables `eitmad.operation.v1` JSON events. The script sets it only for the diagnostic test and its child engine. It restores the caller's setting. Normal workflow verification is unchanged.

- `operation` joins C# sign-in, activation stages, account-switch work, and IPC phases. Each IPC event also records the existing request `correlation`; the wire value is unchanged.
- Rust workers explicitly receive that correlation before blocking work. Synchronous thread-local context is restored before the worker exits, including on panic. It is never kept across an async await.
- Rust `source` is a fixed label: `Quotations`, `Orders`, `WorkOrders`, `Approvals`, `Catalog`, `Pricing`, or `Images`. Home can use one correlation for several sources; filter by both fields. `Unspecified` means this boundary has no propagated source and must not be attributed to a Home source by timing alone.
- `phase` identifies IPC lock/send/reply, worker execution, driver/credential admission, credential work, DNS, TCP connect, TLS, HTTP send/receive, or confirmation. `elapsedMs` is monotonic duration within that process. Pair `Started` with its terminal event. There is no global timestamp or ordering guarantee across processes.

| Terminal result or phase | Evidence and next safe action |
| --- | --- |
| `CredentialAdmission/Succeeded` | Admission completed within the existing operation deadline, then the caller reloaded the persisted pair. Elapsed time includes any ownership wait and scheduling; compare overlapping phases before attributing contention. This alone is not a server failure. |
| `CredentialAdmission/Deadline` | Credential ownership did not become available within the remaining operation budget. No new refresh or HTTP request starts for that waiter. Compare overlapping refresh/connect timings; do not add a retry loop. |
| `Dns/Succeeded` | Admission and system resolution completed within the caller's existing deadline. Elapsed time includes waiting for one of two resolver slots. Short overlaps wait without starting more resolver work. |
| `Dns/Deadline` | The budget expired during admission or resolution. A waiter starts no resolver after expiry. An already-running system resolver retains its slot until it exits because it cannot be cancelled; at most two run at once. Compare overlapping DNS completions and deadlines before attributing a stuck resolver. Do not raise the cap or add blind retries. |
| Historical `GateBusy` at `Dns` | Earlier builds rejected a third caller immediately, including healthy short overlaps. Current builds use deadline-bounded admission and wake all waiters on release so an expiring waiter cannot strand a free slot. Keep the build identifier with retained traces. |
| Historical `GateBusy` at `DriverAdmission` or `CredentialAdmission` | Earlier builds used immediate mutex claims. Current HTTP requests have independent sockets and no shared driver lock; credential ownership uses deadline-bounded admission. Keep the build identifier with retained traces. |
| `Poisoned` | A local mutex was poisoned. Preserve this cause and inspect the initiating worker failure. |
| `IoTimeout` at connect/TLS/send/receive | A socket returned `TimedOut` or `WouldBlock` before the total request budget expired. Check the peer and network; do not label this as total deadline exhaustion. |
| `Deadline` | The request budget expired at the measured boundary. Compare prior phases to locate where the budget was spent. |
| `Cancelled` on IPC | Caller cancellation was observed. An unconfirmed command still has its existing unknown-outcome rules. |
| `Network`, `Denied`, `Server`, or `Invalid` | A safe transport, security, HTTP-status, or response-validation class. No raw error message or response body is collected. |
| `Confirmation/Succeeded` | An authenticated HTTP success body decoded at the client. The diagnostic test separately checks the typed `ServerAvailable` result. This does not prove any unrelated mutation committed. |
| `Worker/Succeeded` or `Activation/Succeeded` | The work returned normally. Capability results may still contain cached or unavailable state; inspect confirmation and admission events. |
| A terminal `Failed` | The enclosing boundary failed; inspect the specific earlier phase. It is not a fabricated network diagnosis. |

The internal HTTP error retains the original typed transport failure through diagnostic classification before the owning capability maps it to its existing public result. Public IPC contracts do not expose these internal details. Diagnostic/test cleanup preserves a single original error's type and stack. If both work and cleanup fail, an aggregate retains the initiating typed error first and the cleanup error second; do not report only the last exception.

### Measurement and privacy limits

The allowlist contains fixed enums, opaque diagnostic IDs, and duration only. It excludes payloads, routes, terms, account names, customer identifiers/values, authorization, tokens, certificates, raw frames, and underlying exception messages. The supervisor validates and re-emits only allowed engine fields; it never forwards arbitrary stderr as diagnostics.

Each process emits at most 4096 diagnostic events through a 256-entry non-blocking queue. The queue and background writer start only when an enabled event is emitted. A full queue drops events. The writer drains it; activation and shutdown do not wait for stderr. The supervisor already drains engine stderr concurrently with lifecycle output. A blocked sink, saturation, process exit, or disabled diagnostics can leave missing events. Absence is not evidence that work did not occur. Start a new bounded run if the relevant phase is missing; do not raise limits or timeouts to hide a cause.

HTTP receive time includes TLS/network I/O, remote queueing, server work, and response transfer. There is no separate SQL/lock or server-handler timer, and server-local audit correlation can differ. This trace cannot distinguish those parts. Local desktop sign-in uses Rust's stored account state and has no network stage. Background subscription/sync work without an explicit context is not attributed to activation. No trace produced now establishes the source of an old 27-second read. Empty stderr or PostgreSQL logs without errors do not rule out slow queries.

The focused regression proof is in `DiagnosticScenario.cs`, runtime blocking-worker tests, server-connection admission/loopback timeout tests, and observability output/redaction tests. Escalate only the relevant metadata interval and test result to the owner named below. Preserve original evidence privately; do not paste raw environment or TRX contents into support messages without checking them.

## Verify recovery

Confirm a new engine generation reaches `Ready`, the handshake reports the expected version `1.13`, and `eitmad.capability.authorization-scopes.v1` plus operation-specific capabilities are present. Confirm that Rust returns the tenant, scope, principal, and session; the shell must not send these fields. Verify one synthetic scoped subscription receives an ordered event and a revoked synthetic member receives no later configuration event. Intentional shutdown must reach `Stopping → Stopped` with exit code `0`.

Escalate with engine/shell versions, Windows version, generation, `EngineInstanceId`, stable error code, message ID, retry disposition, timestamp, and correlation ID. See [typed local IPC](../developer/subsystems/local-ipc.md), [Windows supervision](../developer/subsystems/windows-process-supervision.md), and the [threat model](../architecture/local-ipc-threat-model.md).
