# UI-06: Reception title hierarchy

The reception catalog and manager dashboard now use their shell title as the page title. The catalog listing starts with its status, search, and category filters. The manager dashboard starts with its quick actions. Global search and shell actions keep their existing positions. The standalone quotation editor retains its PageHeader because it has no ShellTitleBar. The receptionist home greeting provides distinct context and remains unchanged.

This is a presentation-only change. No canonical documentation impact, new Arabic terms, authority changes, or accessibility changes were required. The existing edit to `artifacts/ui-audit-2026-10-12/UI-AUDIT.md` is excluded.

## Verification

- .NET SDK 8.0.422.
- `dotnet build shells/windows/Eitmad.WindowsShell.csproj --configuration Release --nologo`: passed, zero warnings and errors.
- Focused tests: 27 passed, zero failed or skipped. [Results](results/ui06-focused.trx).
- `git diff --check`: passed.

```powershell
$env:EITMAD_UI_CAPTURE_DIR = Join-Path $PWD 'artifacts/ui-06/captures'
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter 'FullyQualifiedName~SalesCatalogPresentationTests|FullyQualifiedName~SalesCatalogRenderedTests|FullyQualifiedName~HomeRenderedTests|FullyQualifiedName~MainWindowRenderedTests' --logger 'trx;LogFileName=ui06-focused.trx' --results-directory artifacts/ui-06/results
```

Existing tests cover manager and receptionist destinations, catalog navigation, search, filter clearing, selection, quotation actions, and return focus. The catalog navigation test now runs at minimum, default, and available large sizes. No assertions were added for literal heading text or spacing.

## Rendered inspection

The display was checked before capture: 1536 × 864 logical pixels, with a 1536 × 816 working area. WPF reported 125% scaling. The existing WpfTestHost rendered real WPF windows offscreen with synthetic Arabic fixtures and saved client visuals using RenderTargetBitmap at 96 DPI. These images are not desktop screenshots. Native frame regions can appear blank.

All six linked captures were inspected. Each affected screen has one page title. At minimum width, the shell retains its two-row toolbar and the content starts directly with useful information or actions. The catalog status and its separate product search remain visible. The manager quick actions remain in two columns. Role-specific navigation passed the existing checks.

| State | Requested window | Actual outer window (DIP) | Capture |
| --- | --- | --- | --- |
| Catalog, minimum | 720 × 560 | 720 × 560 | [Image](captures/catalog-720x560.png) |
| Catalog, default | 1338 × 753 | 1338.4 × 752.8 | [Image](captures/catalog-1338x753.png) |
| Catalog, available large | 1536 × 864 | 1536 × 864 | [Image](captures/catalog-1536x864.png) |
| Manager, minimum | 720 × 560 | 720 × 560 | [Image](captures/home-manager-top-720.png) |
| Manager, default | 1338 × 753 | 1338.4 × 752.8 | [Image](captures/home-manager-top-1338.png) |
| Manager, available maximized | 1920 × 1080, then maximized | 1550.4 × 830.4 | [Image](captures/home-manager-top-1920.png) |

Exact 100% baseline verification, including full-screen 1920 × 1080, was unavailable. Windows constrained the maximized manager window to the available display and working area; the filename records the requested baseline. No display settings were changed. Broader accessibility verification was outside this issue.
