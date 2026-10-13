# UI audit follow-up evidence

The [main prompt file](../ui-audit-2026-10-12/UI-AUDIT.md) now contains **45 individual visual/UX fix prompts**, each with an issue, reproduction steps, source context, done state, and image attachment. This follow-up closes the initial visual verification gaps. Accessibility changes and accessibility testing are excluded at the user's request. No product UI was changed.

## Coverage and results

| Requirement | Evidence and result |
| --- | --- |
| All implemented pages and dialogs | The [18-view coverage inventory](../ui-audit-2026-10-12/coverage.json) still matches every feature XAML view. The original report covers manager/reception home, all management lists/editors, sales selection, quotation/approval, orders/work orders, customers, users, shared shell/dialogs, and sign-in. |
| All three display baselines at 100% | [1920 × 1080](./scale-100/geometry-1920x1080.txt), [1338 × 753](./scale-100/geometry-1338x753.txt), and [720 × 560](./scale-100/geometry-720x560.txt) outer-window geometry was measured at 100%. All 404 new fixture PNGs were visually inspected through contact sheets and relevant full-size details. They confirm the existing issues; no additional distinct layout defect was found. |
| Real engine/server presentation | Twelve screens were captured and inspected: manager home, materials, parts, furniture, pricing, products, quotations, orders, work orders, users; reception home and catalog. The existing integrated account-switch/retained-work test and the temporary capture flow both passed. This supplements, rather than replaces, the richer synthetic state coverage. |
| Dropdown placement | Material category and unit dropdowns were opened in the interactive Windows desktop. The unit dropdown opens above its field near the bottom edge and remains visible. See [parent](./native/material-unit-bottom-0.png), [popup](./native/material-unit-bottom-1.png), and [geometry](./native/material-unit-bottom-geometry.json). |
| Calendar placement | The work-order calendar was opened at default position and near the bottom edge. It opens above the date field and remains on screen. See [parent](./native/calendar-bottom-0.png), [calendar](./native/calendar-bottom-1.png), and [geometry](./native/calendar-bottom-geometry.json). The existing calendar styling concern remains UI-36. |
| Product action menu | The actual WPF row-action menu was opened at default position and near the bottom edge. It opens above its button when space below is limited. See [parent](./native/product-actions-bottom-0.png), [menu](./native/product-actions-bottom-1.png), and [geometry](./native/product-actions-bottom-geometry.json). |
| System-tray menu | The actual `Platform/TrayIcon.cs` menu was rendered on the interactive desktop at the bottom-right screen edge. Both Arabic menu entries fit. See [menu](./native/tray-menu.png) and [geometry](./native/tray-menu-geometry.json). The host uses no-op callbacks and opens the real menu from a test button; notification-area icon hit-testing and process exit were not asserted. |
| Windows print dialog | The actual Windows print dialog opened from the application's Arabic print button and was cancelled. See [dialog](./native/windows-print-dialog.png). The OS uses English, so its English labels are not an app localization defect. No physical print job was sent. The isolated preview fixture authorizes printing locally only; this is not evidence of real server print authorization. |
| Printed documents and reports | Saved quotation, order, draft, and four-page quotation pages were re-rendered and inspected at 100%. Reports, tasks, settings, and messages remain disabled/unimplemented. There is no report page or report template to inspect in this source snapshot. |
| New finding | UI-45 records the sign-in helper text that exposes the Rust engine detail. Its [screenshot](./native/live-sign-in-0.png) and source were verified. This is separate from compact clipping in UI-01. |
| Environment restoration | Display scaling was restored to the original 125% and verified in Windows Settings. Contrast themes remained None. The isolated workflow environment was stopped with its existing script; accepted synthetic work was retained. |

## Reproduction

Product source remains at `234bce32642e92f10e1070b5e8a86cfa6b70f20b`. The follow-up was performed on branch `ui-audit-prompts` with .NET SDK 8.0.422 and the repository's existing Windows rendering harness. The display resolution was 1920 × 1080; scaling was temporarily set to 100% through Windows Settings. Captures in `scale-100` and `scale-100-supplement` are WPF client visuals at 96 DPI. Native screenshots are bounded computer-use captures; popup geometry is recorded separately. A partly offscreen parent in an edge-placement test is intentional, not a layout finding.

The rendered suite passed **153 cases**, and the supplemental capture driver passed **14 cases**, both with zero failures/skips. See [suite result](./results/scale-100.trx) and [supplement result](./results/scale-100-supplement.trx). Build output contained no new warnings.

```powershell
$env:EITMAD_UI_CAPTURE_DIR = Join-Path (Get-Location) 'artifacts/ui-audit-2026-10-13/scale-100'
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter 'FullyQualifiedName~Rendered&FullyQualifiedName!~WorkflowEnvironmentTests&FullyQualifiedName!~UiAuditCaptureTests' --logger 'trx;LogFileName=scale-100.trx' --results-directory artifacts/ui-audit-2026-10-13/results
```

For supplemental capture, temporarily restore [UiAuditCaptureTests.cs.txt](./provenance/UiAuditCaptureTests.cs.txt) into `shells/windows/tests/Rendered/UiAuditCaptureTests.cs`, use `scale-100-supplement` as the capture directory, and filter `FullyQualifiedName~UiAuditCaptureTests`. The driver is presentation evidence, not a product regression suite. Its bottom-scroll helper scrolls nested controls; do not infer amount truncation from its synthetic scroll position.

The real-engine flow used the existing isolated workflow environment with synthetic accounts/data. It passed **2 tests, 0 failed, 0 skipped** in 2 m 7 s. See the [sanitized result](./results/live-summary.txt) and [capture driver](./provenance/UiGapLiveCaptureTests.cs.txt). Raw environment logs and provisioned credentials remain in the ignored, access-controlled test state and are not included in this deliverable. An initial standalone capture attempt failed because the desktop already owned its runtime database; the final capture used the environment's intended test workflow and passed.

```powershell
$env:UI_AUDIT_CAPTURE_DIR = Join-Path (Get-Location) 'artifacts/ui-audit-2026-10-13/live'
& tools/workflow-environment.ps1 -Action Start -StateDirectory (Join-Path (Get-Location) 'tests/engine-integration/artifacts/ui-audit-20261013') -DatabasePort 55449 -ServerPort 58449 -SkipBuild -VerifySignIn
```

Native hosts used [UiGapNativePreviewTests.cs.txt](./provenance/UiGapNativePreviewTests.cs.txt), with `EITMAD_AUDIT_ROLE` set to `print`, `calendar`, `products`, or `tray`. The default role supplied the materials editor in the manager preview. Each interactive host completed successfully: [materials](./results/native-interactive.trx), [print](./results/native-print-interactive.trx), [calendar](./results/native-calendar.trx), [products](./results/native-products.trx), and [tray](./results/native-tray.trx). Native interactions used the computer-use tool. Fixture setup and local print permission do not claim real account authority.

All temporary `.cs` drivers were removed from the product test directory after capture. No canonical workflow, contract, domain term, or product behavior changed. No canonical documentation change or Rust workspace test was required. No remote publication was performed.

## Artifact checks and review limits

The [capture manifest](./capture-manifest.json) inventories all retained follow-up PNGs. The [contact-sheet index](./review-sheets/index.md) maps the complete image set. All original 404 images, the follow-up 404 fixture images, all 12 real-engine images, and each retained native image were inspected. Repeated captures are supporting evidence, not extra findings.

Run both artifact checkers to validate prompt counts, source and image paths, page inventory, image dimensions, and the retained test results:

```powershell
& 'C:/Users/Hussein/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' artifacts/ui-audit-2026-10-12/provenance/check_artifact.py
& 'C:/Users/Hussein/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' artifacts/ui-audit-2026-10-13/provenance/check_artifact.py
git diff --check
```

These checks validate artifact integrity; rendered inspection is the evidence for visual findings. The audit covers implemented pages and inspected states. It cannot guarantee that every future data combination or OS configuration has no other defect. Recheck each prompt against current code before applying it.
