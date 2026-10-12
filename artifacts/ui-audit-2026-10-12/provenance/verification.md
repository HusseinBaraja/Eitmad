# Audit verification and reproduction

The audit reviewed the Windows shell at Git revision `234bce32642e92f10e1070b5e8a86cfa6b70f20b` on 2026-10-12. The starting branch was `main` and the worktree was clean. This deliverable changes only dated audit artifacts.

## Rendering evidence

The existing WPF capture harness is `shells/windows/tests/Rendered/WpfTestHost.cs`. It opens native WPF windows offscreen, pumps layout, and captures client visuals with RenderTargetBitmap at 96 DPI. Tests use synthetic fixtures and fake engine responses. Capture filenames can record requested sizes rather than actual window geometry.

The first command built the Windows shell and tests with .NET SDK 8.0.422 and passed **153 tests**, with **0 failed, 0 skipped, and no build warnings**:

```powershell
$env:EITMAD_UI_CAPTURE_DIR = Join-Path (Get-Location) 'artifacts/ui-audit-2026-10-12/captures'
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter 'FullyQualifiedName~Rendered&FullyQualifiedName!~WorkflowEnvironmentTests' --logger 'trx;LogFileName=ui-audit-rendered.trx' --results-directory artifacts/ui-audit-2026-10-12/results
```

[Existing rendered suite result](../results/ui-audit-rendered.trx)

The integrated WorkflowEnvironment test was excluded because it requires an external engine/server environment and credentials. No credentials were read for this audit. This exclusion does not remove a UI page from the rendered screen inventory.

A temporary capture driver filled gaps in sign-in busy/error, home search/partial/unavailable, all six furniture editor steps, user list/create/empty/failure/deactivation, and native calendar content. It passed **14 cases**. A later focused run of its three user cases added password-validation images and passed **3 cases**. These three cases repeat part of the 14-case run; they are not three additional unique workflows.

[Supplemental result](../results/ui-audit-supplement.trx) · [Validation capture result](../results/ui-audit-user-validation.trx)

The temporary source is retained as [UiAuditCaptureTests.cs.txt](./UiAuditCaptureTests.cs.txt). It was removed from the product test directory after capture. To reproduce the supplemental evidence, temporarily copy it into `shells/windows/tests/Rendered/UiAuditCaptureTests.cs`, set the capture directory as above, and run:

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter 'FullyQualifiedName~UiAuditCaptureTests' --logger 'trx;LogFileName=ui-audit-supplement.trx' --results-directory artifacts/ui-audit-2026-10-12/results
```

The supplemental driver is a capture tool, not a product regression suite. Its Bottom helper scrolls all visible descendant ScrollViewers, including inner text editors. Do not infer numeric-text defects or starting scroll positions from these bottom captures. Use the original screen-specific tests and targeted reproduction when fixing a finding.

## Display and documents

Measured WPF outer-window values were 1338.4 × 752.8, 1553.6 × 881.6, and 720 × 560 DIP at 125% scaling. The requested large window was desktop-constrained. The geometry records are [default](../captures/geometry-1338x753.txt), [large](../captures/geometry-1920x1080.txt), and [minimum](../captures/geometry-720x560.txt). Exact full-screen 1920 × 1080 at 100% was not available for this audit.

The captures include saved quotation, order, and draft previews. Native XPS exports are retained: [quotation](../captures/saved-quotation.xps), [order](../captures/saved-order.xps), and [four-page quotation](../captures/saved-quotation-multipage.xps). The corresponding PNG page renders were inspected, including all four pages. Physical printing was not tested.

There are 404 PNGs: 316 from the existing suite and 88 supplemental captures. The [manifest](../capture-manifest.json) records dimensions and contact-sheet positions. All initial contact sheets and all supplemental contact sheets were inspected; affected details were opened at full size. The final contact sheets regroup the same images into one alphabetical inventory.

Native OS high contrast, screen-reader output, all manual keyboard/focus paths, system-tray rendering, Windows print-dialog rendering, and physical popup placement remain unverified. System-color/large-text fixtures and separately captured calendar content provide limited rendering evidence only. These limits do not imply a confirmed defect.

## Scope and completion check

The [coverage inventory](../coverage.json) maps all 18 feature XAML views to rendered evidence. The main report also covers the manager home composition, shared shell/dialogs, and print surfaces. Reports, tasks, settings, and messages are disabled or absent capabilities in this revision. No report view or report template was found; this is stated explicitly rather than treated as a rendered report.

The final artifact validation checks every finding's source paths and images, all Markdown file/image links, the PNG manifest dimensions, all 18 feature-view mappings, the 44 prompt sections, and the three passing TRX results. This is an artifact integrity check. It does not substitute for visual inspection.

```powershell
& 'C:/Users/Hussein/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe' artifacts/ui-audit-2026-10-12/provenance/check_artifact.py
git diff --check
```

No canonical workflow, contract, domain term, or product behavior changed. No canonical documentation update or repository-wide Rust validation was required. No remote publication was performed.
