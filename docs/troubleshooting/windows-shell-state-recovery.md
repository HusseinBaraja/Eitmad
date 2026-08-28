---
title: "Recover a disconnected or stale Windows operations shell"
description: "Diagnose Windows caption, Arabic unavailable-engine, restart-exhaustion, unsupported-state, and subscription-resync symptoms without bypassing Rust authority."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Windows UI and platform maintainers"
last_verified: "2026-08-28"
review_triggers:
  - "Windows shell frame, availability copy, reconnect, resync, query support, or shutdown behavior changes"
keywords:
  - "missing close minimize maximize icons"
  - "Windows caption buttons"
  - "المحرك غير متاح الآن"
  - "نعيد الاتصال بالمحرك"
  - "نحدّث الحالة من المصدر"
  - "توقفت محاولات إعادة تشغيل المحرك"
  - "ReconnectExhausted"
  - "RestartExhausted"
  - "eitmad.error.contract-invalid.v1"
  - "blank square dashboard icons"
  - "Arabic sidebar text alignment"
---

# Recover a disconnected or stale Windows operations shell

The Windows shell can become temporarily unavailable without losing Rust-owned data. Stop new state-changing work until the engine card says **سليم** and **جاهز لاستقبال الطلبات**. Never open the database, edit a config file, start an uncontained engine, or bypass the retry ceiling.

## Symptoms

- **المحرك غير متاح الآن** or **نعيد الاتصال بالمحرك…** remains visible;
- **تعذر استعادة الاتصال بالمحرك** maps `EngineIpcHealthState.ReconnectExhausted`;
- **توقفت محاولات إعادة تشغيل المحرك** maps `EngineSupervisionState.RestartExhausted`;
- **نحدّث الحالة من المصدر…** remains visible after subscription resynchronization;
- a sync or update card says **غير متاحة** with `eitmad.error.ipc-subscription-unsupported.v1`;
- a configuration revision says **غير متاح** or the patch action remains disabled.
- minimize, maximize, or close icons are missing, use custom colors, or appear on the wrong RTL/LTR side.
- dashboard, toolbar, or sidebar icons appear as blank squares; Arabic labels drift away from their icons; **آخر عروض الأسعار** overlaps **عرض الكل**; or the toolbar and footer controls touch adjacent edges.

Sync and update **غير متاحة** currently means that the engine did not advertise those optional runtime capabilities. The shell sends no query or subscription for them. It does not mean that the shell calculated an offline, current, or failed state. Configuration remains usable when its typed query succeeds.

## Fast checks

1. Record the Arabic message, engine health/readiness text, `EngineSupervisionState`, `EngineIpcHealthState`, generation, restart count, app and engine versions, and stable error code.
2. Wait for the bounded reconnect or restart sequence to finish. Do not click the restart action repeatedly.
3. Confirm the configuration card has a revision and the engine card says **جاهز لاستقبال الطلبات**.
4. Run the non-mutating engine diagnostic from [engine startup recovery](engine-startup-failures.md).
5. Reproduce with the focused shell and real-engine commands in the [Windows shell guide](../developer/subsystems/windows-native-shell.md#tests-and-safe-extension-points).

## Causes and resolutions

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| **نعيد الاتصال بالمحرك…**, same generation | Named-pipe session ended while the engine stayed ready | Observe whether `IpcHealth` returns to `Connected` inside the bounded attempts | Wait for automatic reconnect; if it exhausts, exit through the tray and start one new normal session |
| `ReconnectExhausted`, lifecycle still `Ready` | Three same-generation reconnect attempts failed | Run the real-engine boundary suite and capture only typed failure kinds | Restart through normal supervision; escalate recurring failures to Windows platform maintainers |
| `RestartExhausted`, restart count `3` | Four unexpected engine exits occurred inside 60 seconds | Match the last typed Rust error with engine diagnostics | Correct the engine failure, then use **إعادة المحاولة** once to start a new supervision session |
| **نحدّث الحالة من المصدر…** after engine replacement | The old in-memory cursor cannot prove continuity | Check whether each negotiated query returns | Wait for the bounded refresh attempt; an absent optional capability remains unavailable without request traffic |
| Sync/update **غير متاحة**, `eitmad.error.ipc-subscription-unsupported.v1` | The current engine did not negotiate that optional capability | Confirm engine health is still **سليم** and configuration has a revision | No recovery is needed; do not infer product state. Implement and advertise the Rust vertical before enabling the panel |
| Configuration revision says **غير متاح** | Configuration query was denied, unavailable, or the IPC session is unusable | Verify synthetic development scope coherence or production identity and ReBAC without copying the relationship graph | Correct identity/scope provisioning or Rust authority; never add a shell-side permission decision |
| Patch rejected after another client changed configuration | `ExpectedRevision` is stale | Read the newest configuration snapshot | Review the new value and submit a new typed patch with the new revision and a new user intent |
| Caption icons are missing or do not follow RTL/LTR placement | The shell replaced the Windows non-client frame with custom controls | Check `MainWindow.xaml` for `WindowStyle="None"`, `WindowChrome`, or a custom caption-button style | Restore `WindowStyle="SingleBorderWindow"` with `ResizeMode="CanResize"`; remove custom caption controls and handlers, then run the shell tests |
| Dashboard icons are blank squares or use inconsistent shapes and colors | The dashboard uses private-use font code points or per-card raw colors | Search `MainWindow.xaml` for `Segoe Fluent Icons`, `&#xE`, and raw icon foreground values | Use geometry from `Resources/OperationsIcons.xaml` and semantic brushes from `OperationsTheme.xaml`; then run the shell tests and inspect the rendered window |
| Arabic sidebar or notification labels are not beside their icons, or **آخر عروض الأسعار** overlaps **عرض الكل** | An inherited LTR grid or implicit column placement controls the Arabic row | Inspect the affected grid for explicit `FlowDirection="RightToLeft"`, icon and label columns, and `TextAlignment="Right"` | Restore the explicit RTL column layout and the named header grid; do not use margins as a direction substitute |
| **عرض سعر جديد** touches the search field, or the sidebar footer touches the window edge | The fixed design surface lost its explicit separator or bottom inset | Check `ToolbarGap` and `SidebarFooterCard` in `MainWindow.xaml` | Restore the `16`-unit toolbar separator and `20`-unit footer bottom margin, then inspect at `1326×746` logical pixels |

## Verify recovery

The title bar must expose native **Minimize**, **Maximize**, and **Close** buttons to Windows UI Automation. The buttons must appear on the left for the Arabic RTL window and on the right for an LTR localized window. Dashboard icons must render as vectors in the shared walnut and ink theme, and Arabic labels must remain right-aligned beside their icon columns. The **آخر عروض الأسعار** title and **عرض الكل** control must occupy separate columns. The toolbar and footer insets must remain visible. The engine card must show **سليم** and **جاهز لاستقبال الطلبات**. The resynchronization banner must disappear after the refresh attempt finishes. A successful configuration query must show a non-negative revision; a failed query must clear old entries, show **غير متاح**, and keep patch submission disabled. A clean tray exit must produce `Stopping → Stopped`, exit `0`, and `Forced: false` in the real-engine test.

## Escalate safely

Send Windows UI and platform maintainers only the states, generation, restart count, stable error/message IDs, negotiated protocol, timestamps, and correlation IDs needed for the failing interval. Do not include bearer tokens, raw IPC frames, command lines, local paths, database content, authorization graphs, secrets, or customer records.

Return to [Troubleshoot Eitmad](index.md), inspect [Windows process supervision failures](windows-engine-supervision-failures.md), or review [local IPC failures](local-ipc-failures.md).
