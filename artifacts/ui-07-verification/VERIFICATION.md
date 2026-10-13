# UI-07: Remove unavailable sidebar destinations

Removed the disabled Tasks, Reports, and Settings buttons from `ShellSidebar.xaml`. The ten implemented Manager routes keep their order, handlers, labels, icons, and responsive layout. No feature, Rust boundary, or authorization behavior was added. The shell guide now states the sidebar availability rule.

## Verification

- `dotnet build shells/windows/Eitmad.WindowsShell.csproj --configuration Release --nologo`: passed, zero warnings and errors.
- `dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo --filter 'FullyQualifiedName~MainWindowRenderedTests|FullyQualifiedName~Ui07CaptureTests' --logger 'console;verbosity=normal' --logger 'trx;LogFileName=ui-07-rendered.trx' --results-directory artifacts/ui-07-verification/results`: passed all 10 cases with no new warnings.
- `python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs --files docs/developer/subsystems/windows-native-shell.md`: passed.
- `git diff --check`: passed.

The temporary capture driver used the existing WPF host and synthetic Arabic Products fixture. Its source is saved in [provenance](provenance/Ui07CaptureTests.cs.txt); it was removed from the test project after verification. No permanent string or static-layout test was added for this button removal. The [test results](results/ui-07-rendered.trx) record the actual window size, WPF DPI, scroll range, and visible navigation routes.

## Rendered evidence and limits

A DPI-aware Windows query confirmed one 1920 × 1080 physical display, a 1920 × 1020 work area, and 120 DPI (125% scaling). The first DPI-unaware query returned virtualized 1536 × 864 coordinates and 96 DPI; the WPF results corrected that reading. The display scaling was not changed. Exact baseline verification at 100% scaling remains unavailable.

| Requested window (DIP) | Actual window (DIP) | Scaling | Result |
| --- | --- | --- | --- |
| 1338 × 753 | 1338.4 × 752.8 | 125% | All ten routes visible; no sidebar scrolling |
| 720 × 560 | 720 × 560 | 125% | Compact icon rail; Users remains reachable at the bottom |
| 1536 × 816 | 1536 × 816 | 125% | Largest window that fits the available work area; all ten routes visible |

Inspected the actual WPF captures at full size:

- [Default window](captures/ui-07-products-1338x753.png)
- [Minimum window](captures/ui-07-products-720x560.png)
- [Minimum sidebar scrolled to Users](captures/ui-07-sidebar-bottom-720x560.png)
- [Available large window](captures/ui-07-products-1536x816.png)

The unavailable items and their gaps are absent. Users immediately follows Work Orders. Selection, Arabic labels, and icons keep the existing visual system. The capture helper renders WPF window content and does not capture native caption chrome. No interaction, color, typography, size, or control-template changes were introduced; no additional accessibility, high-contrast, or text-scaling verification was performed.

The existing edit to `artifacts/ui-audit-2026-10-12/UI-AUDIT.md` was preserved and excluded from the issue commit.
