# UI-03 checkbox text verification

The furniture variant choices and pricing below-cost acknowledgement now use a keyed native WPF checkbox style. Labels use the existing Arabic form font at 14 DIP, normal weight, and wrap within the available width. Each row has a transparent full-width hit area with a minimum height of 36 DIP. The native checkbox template and RTL direction remain in use. Variant compatibility instructions and customization captions also use the form body size instead of inheriting the dialog title size.

Arabic copy, choices, bindings, acknowledgements, permissions, and Rust authority are unchanged. No canonical documentation change is required. The existing edit to `artifacts/ui-audit-2026-10-12/UI-AUDIT.md` is excluded from this commit.

## Focused checks

.NET SDK: 8.0.422. The shell build passed with zero warnings and errors:

```powershell
dotnet build shells/windows/Eitmad.WindowsShell.csproj --configuration Release --nologo
```

The [focused results](results/ui-03-focused.trx) contain 30 passing tests across the affected furniture and pricing presentation/rendered classes:

```powershell
$env:EITMAD_UI_CAPTURE_DIR = Join-Path $PWD 'artifacts/ui-03-checkboxes/captures'
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo --filter 'FullyQualifiedName~FurniturePresentationTests|FullyQualifiedName~FurnitureRenderedTests|FullyQualifiedName~PricingPresentationTests|FullyQualifiedName~PricingRenderedTests' --logger 'trx;LogFileName=ui-03-focused.trx' --results-directory artifacts/ui-03-checkboxes/results
```

After adding the wrapping assertion and enlarged-text captures, all six affected baseline cases passed again. See the [wrapping and text-size results](results/ui-03-wrap-and-text-size.trx).

```powershell
dotnet test shells/windows/tests/Eitmad.WindowsShell.Tests.csproj --configuration Release --nologo --filter 'FullyQualifiedName~VariantCustomizationDialogRendersNativeAccessibleBoundsAndChoices|FullyQualifiedName~PricingListEditorAndBelowCostWarningRenderAtBaselineSizes' --logger 'trx;LogFileName=ui-03-wrap-and-text-size.trx' --results-directory artifacts/ui-03-checkboxes/results
```

The existing variant rendered test now checks a long synthetic Arabic option at all three sizes. It verifies wrapping, label bounds, hit testing at the far edge of the row, selection binding, and the selected option ID in the save input. Existing customization bounds and pricing keyboard-focus checks passed. No accessibility behavior was changed.

## Rendered inspection

Native WPF windows were rendered with the existing offscreen WpfTestHost and synthetic Rust projections. The variant and below-cost dialogs were inspected at each available size. Checkbox labels are subordinate to dialog titles, align on the RTL side, and remain within the content viewport. At minimum size, scrolling reaches the handle choice and the dialog footer remains available.

Before capture, the display reported 1536 × 864 DIP at 125% scaling (1920 × 1080 physical pixels), with a 1536 × 816 DIP working area. Windows constrained the requested large window. The test results and geometry files record actual outer window dimensions; PNGs contain the client visuals at 96 DPI.

| Requested outer size | Actual outer size (DIP) | Display scaling | Furniture variant | Below-cost pricing |
| --- | --- | --- | --- | --- |
| 720 × 560 | 720 × 560 | 125% | [Image](captures/furniture-variant-720x560-125percent.png) | [Image](captures/pricing-warning-720x560-125percent.png) |
| 1338 × 753 | 1338.4 × 752.8 | 125% | [Image](captures/furniture-variant-1338x753-125percent.png) | [Image](captures/pricing-warning-1338x753-125percent.png) |
| 1920 × 1080 | 1553.6 × 881.6 | 125% | [Image](captures/furniture-variant-1554x882-125percent.png) | [Image](captures/pricing-warning-1554x882-125percent.png) |

Additional inspected minimum-size evidence:

- [Selected long option](captures/furniture-variant-choices-720x560-125percent.png).
- [Handle choice reached by scrolling](captures/furniture-variant-choices-bottom-720x560.png).
- [Variant choices with checkbox font enlarged to 150%](captures/furniture-variant-enlarged-text-bottom-720x560.png).
- [Pricing acknowledgement with checkbox font enlarged to 150%](captures/pricing-warning-enlarged-text-720x560.png).

The enlarged-text captures are local font-size stress checks at the same 125% display scaling, not an OS text-scaling test. Long labels still wrap and the controls remain reachable. Exact 100% baseline captures, including full-screen 1920 × 1080, were unavailable. High-contrast OS behavior and broader accessibility verification were not performed.
