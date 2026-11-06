---
title: "Extend privacy-preserving observability safely"
description: "Trace metadata-only logs, correlation, and IPC error redaction without exposing product data or secrets."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "Rust reliability and security maintainers"
last_verified: "2026-11-01"
review_triggers:
  - "diagnostic fields, sinks, error contracts, or redaction changes"
keywords:
  - "eitmad-observability-audit"
  - "metadata-only logging"
  - "correlation ID"
---

# Extend privacy-preserving observability safely

`eitmad-observability-audit` applies Rust-owned field contracts before operational logs are serialized. Only declared metadata is included. Sensitive and secret fields are always redacted; there is no temporary mode that reveals them.

## Authority and boundary

`crates/contracts/src/observability.rs` owns event, component, field, severity, classification, and value-kind types. `crates/observability-audit/src/diagnostics.rs` checks the allowlist, field types, duplicate declarations, and duplicate emitted values. The engine CLI uses this path for operational failures.

`ObservationContract::redact` creates a `StructuredLog` with private fields, a stable event ID, occurrence time, component, severity, correlation ID, and checked values. Callers cannot bypass classification by filling a public output map.

| Classification | Serialized output |
| --- | --- |
| `Metadata` | Included after field and type validation |
| `Sensitive` | `redacted` |
| `Secret` | `redacted` |

Local IPC uses `ContractError::redacted_for_external_boundary` immediately before serialization. That projection retains stable codes, message IDs, retry policy, correlation, safe numeric details, and allowlisted metadata. It removes free text, mismatched parameter kinds, and compatibility reasons even if an internal dispatcher constructed an unsafe error.

## Security and correlation

Product payloads, customer text, credentials, authorization graphs, and raw causes must not enter logs or audit. A correlation ID links operational failures; it does not authorize cross-scope joins or product access.

Mutation audit remains separate from diagnostic output. State changes and their redacted audit records commit together in the owning storage boundary. The historical `SensitiveDebugMode` audit tag remains readable, but no diagnostic controller or permission enables that mode.

## Collect local support evidence

Automatic local diagnostics cover guided setup, desktop, engine, and server. The new [Inno preparation wizard](../../decisions/0034-inno-windows-installation.md) does not emit this typed stream or provide the report button; retain its error message when it stops before guided setup. Historical custom installers can have outer-installer events. `crates/contracts/src/local_diagnostics.rs` owns the version-one event/report schemas and policy. Code generation produces native bindings and `local-diagnostic-policy-v1.json`; the Windows file adapter consumes that policy. `crates/observability-audit/src/local_diagnostics.rs` owns the Rust sink. `platform-adapters/windows/Diagnostics/` provides native file, Downloads, ZIP, exception-symbol, and service-status mechanics. No upload destination or remote collection is configured.

Select **تنزيل السجلات** in the fixed footer of the desktop or Eitmad guided setup window. The same control remains available on sign-in, Manager and Receptionist pages, and the Owner pairing window. It saves `Eitmad-Logs-<UTC time>-<random suffix>.zip` directly to the current Windows user's Downloads known folder. **فتح التنزيلات** opens that folder after success. Collection does not require application sign-in or server availability: it reads only technical evidence available to the current Windows account, not product state. If the desktop cannot start, open **استكمال الإعداد** from the installed role folder and use its report button without starting workshop setup.

Separate quotation editor, conflict review and print-preview windows also contain this control. The ZIP contains only:

| Entry | Evidence |
| --- | --- |
| `manifest.json` | Export UTC time, fixed Eitmad service states and native query/exit codes, per-component collection status and record counts, rejected/truncated evidence, current-process capture failures and pending drops, and the timeline SHA-256 |
| `events.jsonl` | Events ordered by UTC milliseconds, process session and sequence; component, release/build, binary SHA-256, OS identifier, phase/result, optional correlation, approved operation/error ID, elapsed time, native/HTTP code, code symbols and dropped-event count |
| `READ-ME.txt` | How to inspect evidence, collection limits, clock differences and safe interpretation |

No exception message, panic payload, argument value, customer content, credential, authorization graph, raw IPC frame, database/config file, raw WinSW/PostgreSQL log, or setup journal is included. Export accepts only the typed event shape and generated allowlists; unknown fields, unknown phases/codes, malformed tails and oversized records are rejected and counted. Setup projects only native numeric codes, exception type, script basename/line and the safe failure category into the persistent stream. Detailed historical setup text remains in its existing manual log view/export and recovery journal; it is not automatically archived.

For each manifest source, `Ready` means matching files were found, `Missing` means the directory or matching files were absent, `Denied` means access was refused, and `Unavailable` means an I/O or reparse-path failure. `records` counts accepted events; `rejectedRecords` counts invalid records and unfinished tails; `truncatedFiles` counts omitted files and files read only to the byte limit. Records accepted before a source fails remain available. Service entries report only the two fixed Eitmad Windows services: their SCM state or `NotInstalled`, `Denied`, or `Unavailable`, with native query codes and the reported Win32 exit code when available. Manifest capture failures and pending drops apply to the exporting process; they are not totals for all source processes.

### Storage, access and bounds

| Scope | Directory | Access |
| --- | --- | --- |
| Current Windows user: desktop, engine, installer and setup | `%LOCALAPPDATA%\Eitmad\Logs` | Current user's inherited local application-data permissions |
| Installed server's metadata-only diagnostics | `%ProgramData%\EitmadHost\diagnostics` | Administrators/SYSTEM full control, server virtual account Modify, local Users read/execute |

The installed wrapper sets `EITMAD_SERVER_DIAGNOSTIC_DIRECTORY` to its own host root's `diagnostics` directory. A custom host root must be collected by an operator from that root; guided setup uses the fixed ProgramData path above. Raw service logs, keys, configuration and database permissions remain private. Local Users cannot write the server diagnostic stream. Technical process-session UUIDs are unrelated to authentication sessions; the report contains no principal, tenant, device identity or customer record identifier.

Each component retains at most four files of at most 2 MiB each. Each record is below 8 KiB, code locations are bounded, and the writer queue holds at most 256 events. Writers run off the UI/product request path and drop rather than block a full queue. Subsequent evidence records drops; the report also exposes available capture failures. Rotating files removes only diagnostic output. User-created ZIPs in Downloads are not deleted automatically. Reparse paths are refused for diagnostic storage and collection.

Invalid native metadata is counted and skipped without stopping the writer. A later valid event carries the dropped-event count. The Rust panic hook replaces the default hook to avoid printing a panic payload that may contain secrets. Reparse rejection also applies to Downloads and can prevent export when that folder is redirected through a cloud placeholder or link; use a plain local Downloads location for collection.

`EITMAD_DIAGNOSTICS=1` still enables the older, process-capped stderr troubleshooting stream; automatic file capture does not require it and continues past that stream's 4,096-event cap. Product audit remains separate and unchanged.

### Interpret the report without inventing a cause

Check source status and missing evidence before reading the timeline. Match binary/build identity, phase, correlation and operation. A desktop `IpcReply` success proves delivery of a response; the engine event with that correlation records the product outcome. Sequence gaps, drop counts, malformed tails and unavailable sources limit what can be concluded. Current-process failure counters are not a durable history for dead processes.

Collect a report on the affected client and on the server for server-side failures. Export never connects to another PC or copies its files. Clocks can differ between machines. A `Process/Started` event without `Stopped` can mean a crash, power loss, a running process, rotation, or unavailable storage; it does not identify which occurred. Managed/Rust panic handlers record safe code locations, but forced termination, native crashes before managed startup, and power loss cannot guarantee a final record. The SHA-256 detects copy damage; these files are not a tamper-proof audit. A signed release inventory maps a binary hash to its reviewed source.

Changes require rebuilt authenticated installers before test PCs receive this behavior. Local component/rendered tests do not replace clean-PC installation and service-account acceptance.

## Failure and recovery

| Failure | Behavior | Repair |
| --- | --- | --- |
| Unknown, duplicate, or wrong-kind field | Event construction fails | Fix the owning field contract |
| Raw internal error text | IPC strips it | Replace the raw cause with stable typed metadata |
| Serialization failure | CLI emits only a stable fallback event and correlation ID | Fix the schema without printing the value |
| Suspected sensitive output | Treat as a privacy incident | Follow [diagnostic leakage recovery](../../troubleshooting/privacy-and-secret-leakage.md) |

Rust identifiers are language-neutral. Future Arabic diagnostic UI must isolate correlation and error identifiers in LTR runs inside RTL layout. No customer text may be collected for readability.

## Verification

Focused tests cover sensitive and secret sentinels, field allowlists, duplicate and wrong-kind values, mandatory audit metadata, external error projection, IPC serialization, structured CLI failures, local rotation, bounded exports, damaged/unknown records, correlation, timeline checksums and native error projection. Run `cargo test -p eitmad-observability-audit` and the Windows adapter scenarios in `platform-adapters/windows/tests/`. The `DiagnosticReportRenderedTests` class checks the report control at the three baseline window sizes. Use the [focused check table](../index.md#choose-the-smallest-normal-proof) for cross-boundary changes.

Related authority: [ADR-0012](../../decisions/0012-privacy-preserving-observability.md), [typed local IPC](local-ipc.md), and [secret storage](secret-storage.md).
