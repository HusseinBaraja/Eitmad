---
title: "Recover a disconnected or stale Windows operations shell"
description: "Diagnose Windows layout, caption, Arabic unavailable-engine, restart-exhaustion, unsupported-state, and subscription-resync symptoms without bypassing Rust authority."
audience: "support"
page_type: "troubleshooting"
status: "active"
owner: "Windows UI and platform maintainers"
last_verified: "2026-08-29"
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
  - "selected sidebar icon stays dark"
  - "dashboard does not resize"
  - "fixed Viewbox"
  - "window ratio leaves empty space"
  - "الواجهة لا تتجاوب مع حجم النافذة"
  - "Win98 dropdown"
  - "القائمة المنسدلة قديمة"
  - "raw materials popup"
  - "dark button text"
  - "missing rounded button border"
  - "clipped text box content"
  - "dropdown arrow shifts"
  - "brand text left aligned"
  - "brand locale label"
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
- dashboard, toolbar, or sidebar icons appear as blank squares; a selected sidebar icon stays dark on the walnut background; Arabic labels drift away from their icons; **آخر عروض الأسعار** overlaps **عرض الكل**; or the toolbar and footer controls touch adjacent edges.
- the dashboard scales as one frozen canvas, leaves large empty bands, clips cards, or does not reflow when the window width, height, or ratio changes.
- the **المواد الخام** category, status, unit, or row-action dropdown uses square gray platform chrome, loses copper focus states, or places Arabic actions outside the expected popup surface.
- the **حفظ في المعاينة** text is dark on the brown button, or the **إلغاء** button loses its right border at a rounded corner.
- Arabic or numeric values in the **اسم المادة** or **التكلفة الحالية** fields are clipped at the lower edge.
- the category or status chevron flips upward but shifts away from the center of its circular control.
- brand descriptor lines share a left edge instead of one physical right boundary, or a locale label appears below the wordmark.

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
| The dashboard stays at one aspect ratio, shrinks as a single image, or leaves unused bands | `MainWindow.xaml` wraps a fixed-size root grid in a uniform `Viewbox`, or a page bypasses `ResponsiveLayout` | Search for a root `Viewbox`, fixed root `Width`/`Height`, and missing `layout:ResponsiveLayout.IsEnabled="True"`; then run `DashboardReflowsInsteadOfScalingAFixedCanvas` | Remove root uniform scaling. Use star/auto layout, overflow scrolling, and inherited `ResponsiveLayoutMode` triggers. Keep breakpoint-overridable base values in styles, not local XAML values |
| A compact rule exists but search, logo, or cards keep their desktop position or width | A local XAML value overrides the responsive style trigger because local values have higher WPF precedence | Inspect the affected property for both an inline value and a responsive style setter | Move the base value into the same style as the breakpoint trigger, rebuild, and inspect compact and standard widths |
| Caption icons are missing or do not follow RTL/LTR placement | The shell replaced the Windows non-client frame with custom controls | Check `MainWindow.xaml` for `WindowStyle="None"`, `WindowChrome`, or a custom caption-button style | Restore `WindowStyle="SingleBorderWindow"` with `ResizeMode="CanResize"`; remove custom caption controls and handlers, then run the shell tests |
| Dashboard icons are blank squares or use inconsistent shapes and colors | The dashboard uses private-use font code points or per-card raw colors | Search `MainWindow.xaml` for `Segoe Fluent Icons`, `&#xE`, and raw icon foreground values | Use geometry from `Resources/OperationsIcons.xaml` and semantic brushes from `OperationsTheme.xaml`; then run the shell tests and inspect the rendered window |
| A **المواد الخام** dropdown looks like an old square Windows control, or its row-action popup opens on the wrong RTL side | A raw-material selector uses the platform-default template, or popup placement and Arabic text share one direction boundary | Inspect `RawMaterialsView.xaml` for `RawMaterialsComboBox`, `PART_Popup`, `RawMaterialsContextMenu`, and RTL `RawMaterialsMenuItem`; verify the popup placement boundary is LTR while Arabic items remain RTL | Restore the raw-material control templates and direction isolation. Run `RawMaterialsPageUsesTheDashboardVisualSystem`, then inspect an open category selector and three-dot action popup in the rendered WPF window |
| The **حفظ في المعاينة** label is dark, or the **إلغاء** button has a missing right border | `PrimaryButton` does not forward `Foreground` to its `ContentPresenter`, or the secondary button border is not pixel-snapped and is too faint at the rounded edge | Inspect `OperationsTheme.xaml` for a `TextElement.Foreground` binding to the templated button foreground; inspect `RawMaterialsSecondaryButton` for its explicit border brush and `SnapsToDevicePixels` | Restore the shared content foreground binding and the pixel-snapped secondary border. Run `RawMaterialsPageUsesTheDashboardVisualSystem`, then inspect both editor actions in the rendered WPF window |
| Raw-material text appears clipped at the bottom of an editor field | `RawMaterialsTextInput` leaves the content host at its default vertical alignment or allows an internal scrollbar to constrain the line box | Inspect `RawMaterialsView.xaml` for `VerticalContentAlignment="Center"` on the style and the matching template binding on `PART_ContentHost` | Keep the content host vertically centered with hidden horizontal and vertical scrollbars, rebuild, and inspect Arabic and numeric values in both editor fields |
| A category or status chevron shifts when its combo opens | The open-state trigger rotates an asymmetric path around the wrong pivot | Inspect `RawMaterialsComboBox` for separate centered closed and open `Data` geometries instead of a `RotateTransform` | Use the centered up-chevron geometry for `IsDropDownOpen`, rebuild, and inspect both closed and open selectors |
| Brand lines share a left edge, or a locale label appears below the wordmark | The RTL coordinate frame is combined with content-sized brand text, so alignment follows the wrong physical edge and the old locale label remains in the header | Inspect the brand header grid for its fixed logo column, full-width text column, explicit physical anchor, and absence of a locale `TextBlock` | Keep the header geometry explicit, anchor RTL brand lines at the physical right edge, remove the locale label, rebuild, and run `RtlLayoutIncludesMixedDirectionFixtures` |
| A selected sidebar label is white but its icon stays dark | `SetNavigationTone` updates only `TextBlock.Foreground`, or deselection leaves a local `Path.Fill` value | Check `MainWindow.xaml.cs` for the `VisualDescendants<System.Windows.Shapes.Path>` update and `Shape.FillProperty` reset | Set the selected icon fill to white and clear the local fill on deselection so `NavVectorIcon` restores the ink theme brush; then run the shell tests and select **عروض الأسعار** in the rendered app |
| Arabic sidebar or notification labels sit at the left edge of their cell, touch an icon, or leave an oversized gap | The RTL `NavText` style does not use `TextAlignment="Right"` with physical `HorizontalAlignment="Right"`, or a fixed label-to-icon spacer is missing from the affected component | Check for an LTR row with a flexible RTL text column, the shared no-wrap `NavText` style, a fixed `12`-unit spacer, and a fixed icon column; sidebar icons use `34` units, notification icons use `34` units, and notification status dots use `18` units | Restore the component’s physical LTR columns and the shared right-aligned, right-anchored RTL label style; do not use per-label margins or physical alignment guesses |
| Arabic notification labels are not beside their icons, or **آخر عروض الأسعار** overlaps **عرض الكل** | The component lost its explicit local direction boundary or named columns | Inspect the affected component for its fixed icon and label columns and inspect `LatestQuotesHeader` for separate title and action columns | Restore the component-specific direction boundary and named columns; do not use margins as a direction substitute |
| **آخر عروض الأسعار** is left-aligned | `LatestQuotesHeader` uses logical `TextAlignment="Right"` inside its RTL boundary | Check the title TextBlock for `FlowDirection="RightToLeft"` and logical `TextAlignment="Left"` | Restore logical-left alignment so the title ends at the physical right edge while **عرض الكل** stays in its action column |
| Work-distribution title or labels align at the left edge, or progress bars touch their icons | The header/row label lost its local RTL logical-left alignment, or the fixed text-to-icon spacer/right icon column is missing | Inspect the named `WorkDistributionHeader` and each row for RTL logical-left alignment, an explicitly LTR progress bar, percentage `38`, flexible label/progress, spacer `12`, and icon `38` columns | Restore the local RTL header and four-column LTR rows; do not use physical-right alignment or per-label margins |
| Work-distribution footer link is right-aligned while notification footer link is left-aligned | The shared `LinkButton` style defaults to right-aligned content and the work footer has no local override | Check the work footer button for `HorizontalContentAlignment="Left"` | Keep the local left-content override; do not change the shared link style and move unrelated footer links |
| Arabic search hint or query text sits at the left edge | `SearchBox` uses logical `TextAlignment="Right"` inside its RTL boundary | Check `SearchBox` for `FlowDirection="RightToLeft"` with logical `TextAlignment="Left"` | Restore logical-left alignment so WPF renders Arabic text at the physical right edge beside the search icon |
| Search bar appears before the action buttons or does not fill the toolbar space | The responsive toolbar columns or compact row trigger are missing, or a local `Grid.Column` blocks the trigger | Check `ToolbarLayout` for new quotation, status actions, flexible search, and content-sized title columns; compact search must move to row `1` and span four columns | Restore the four-column physical order and compact row trigger; keep the search text’s local RTL boundary |
| **عرض سعر جديد** touches the status actions or compact search | The status-action inset or compact search margin is missing | Check the status action stack for its `16`-unit inset and the compact search trigger for its top margin | Restore the responsive insets, then inspect standard and compact logical widths |

## Verify recovery

The title bar must expose native **Minimize**, **Maximize**, and **Close** buttons to Windows UI Automation. The buttons must appear on the left for the Arabic RTL window and on the right for an LTR localized window. At compact width, confirm the icon rail, full-width second-row search field, two-column metrics and quick actions, full-width detail cards, readable Arabic text, and vertical scrolling. At standard width, confirm the full sidebar, two-column metrics, three-column quick actions, full-width quotation table, and two balanced lower cards. Dashboard icons must render as vectors in the shared walnut and ink theme, and Arabic labels must remain right-aligned beside their icon columns. Select **عروض الأسعار**: its label and icon must be white, and the old **الرئيسية** icon must use the ink theme color. The **آخر عروض الأسعار** title and **عرض الكل** control must occupy separate columns. The engine card must show **سليم** and **جاهز لاستقبال الطلبات**. The resynchronization banner must disappear after the refresh attempt finishes. A successful configuration query must show a non-negative revision; a failed query must clear old entries, show **غير متاح**, and keep patch submission disabled. A clean tray exit must produce `Stopping → Stopped`, exit `0`, and `Forced: false` in the real-engine test.

## Escalate safely

Send Windows UI and platform maintainers only the states, generation, restart count, stable error/message IDs, negotiated protocol, timestamps, and correlation IDs needed for the failing interval. Do not include bearer tokens, raw IPC frames, command lines, local paths, database content, authorization graphs, secrets, or customer records.

Return to [Troubleshoot Eitmad](index.md), inspect [Windows process supervision failures](windows-engine-supervision-failures.md), or review [local IPC failures](local-ipc-failures.md).
