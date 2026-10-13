# UI-01 sign-in layout verification

The sign-in form now uses the available width up to 420 DIP. The existing compact layout mode collapses the decorative photo and reduces the form margins. Default and larger windows retain the two-panel design. Guidance, progress, and error text wrap within the form. Authentication behavior and Arabic copy are unchanged.

## Focused checks

With .NET SDK 8.0.422, both commands passed with no warnings:

```powershell
dotnet build shells/windows/Eitmad.WindowsShell.csproj --configuration Release --nologo
$env:EITMAD_UI_CAPTURE_DIR = Join-Path (Get-Location) 'artifacts/ui-01-sign-in/captures'
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo -m:1 --filter 'FullyQualifiedName~SignInRenderedTests' --logger 'trx;LogFileName=sign-in.trx' --results-directory artifacts/ui-01-sign-in/results
```

All four tests passed. The rendered regression checks visible form content against the horizontal viewport and confirms that vertical scrolling can reach the sign-in action. The existing Arabic/Latin username line test also passed. The [test result](results/sign-in.trx) contains the measured geometry.

## Rendered inspection

Native WPF windows were rendered with the existing offscreen WpfTestHost and synthetic credentials. Both normal and connection-failure client visuals were inspected at each available size. The logo, heading, field labels, complete credential inputs, action, and guidance fit horizontally. The minimum-size form fits vertically in both captured states.

| Requested outer window | Measured outer window (DIP) | Scaling | Normal state | Connection failure |
| --- | --- | --- | --- | --- |
| 720 × 560 | 720 × 560 | 125% | [Image](captures/sign-in-720x560.png) | [Image](captures/sign-in-failure-720x560.png) |
| 1338 × 753 | 1338.4 × 752.8 | 125% | [Image](captures/sign-in-1338x753.png) | [Image](captures/sign-in-failure-1338x753.png) |
| 1920 × 1080 | 1553.6 × 881.6 | 125% | [Image](captures/sign-in-1920x1080.png) | [Image](captures/sign-in-failure-1920x1080.png) |

Capture names identify the requested sizes. The display reported 1536 × 864 DIP at 125% (1920 × 1080 physical pixels). The large window was constrained by the desktop. Exact 100% baseline captures, including full-screen 1920 × 1080, were not performed in this run. Geometry files beside the images record actual values. Accessibility changes and checks were outside the request.

No Rust boundary, contract, domain term, or workflow changed. No canonical documentation update was required. The existing edit to `artifacts/ui-audit-2026-10-12/UI-AUDIT.md` was preserved and excluded from this change.
