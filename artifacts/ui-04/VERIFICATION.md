# UI-04 compact tables

Compact tables keep record identity, primary amounts, and row actions in the main columns. Secondary fields use always-visible WPF row details when the complete column set cannot fit. Detail fields retain their existing bindings, templates, numeric direction, and editing behavior. Sortable detail labels use the table's existing sort with a direction indicator. Widening the table restores the columns and preserves selection and sorting.

The affected views are Users, Raw materials, Parts, Furniture, Products, Pricing, Quotations, Orders, Work orders, composition editors, and customer history. Rust authority and calculations are unchanged. Feature-provided field availability remains effective in both layouts. The Parts usage-count placeholder has no sortable value and no longer offers a nonfunctional sort.

## Verification

- `dotnet build shells/windows/Eitmad.WindowsShell.csproj --configuration Release --nologo`: passed without warnings.
- Focused rendered run: 49 passed, 0 failed. See [test results](test-results/ui04-final.trx).
- Customer history at all requested/available window sizes: 4 passed, 0 failed. See [history results](test-results/history-sizes.trx).
- `python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs --files docs/developer/subsystems/windows-native-shell.md`: passed.
- `git diff --check`: passed.

The focused run selected `OperationsTableRenderedTests`, `PartsRenderedTests`, `UsersRenderedTests`, `CustomersRenderedTests.DetailEdit`, `FurnitureRenderedTests.CompleteEditor`, `PricingRenderedTests`, `QuotationsRenderedTests`, `OrdersRenderedTests`, `WorkOrdersRenderedTests`, and `ReceptionQuotationsRenderedTests` in `shells/windows/tests/Eitmad.WindowsShell.Tests.csproj`, using Release and `-m:1`.

Behavior checks cover viewport width, row-action position, detail sorting, editing with explicit LostFocus updates, selection and edited values across resizing, unavailable fields across resizing, material unit selectors, and existing row actions. Customer history checks cover both tables without horizontal scrolling. Synthetic Arabic fixtures and LTR numbers/identifiers were inspected in the captures. Existing simulated high-contrast and enlarged-control checks passed; OS accessibility certification is outside this issue.

## Rendered sizes and limitations

The display exposes 1536 × 864 logical pixels. WPF reports 125% scaling. Actual test window dimensions were 720 × 560, 1338.4 × 752.8, and 1536 × 864 DIPs. The existing 1920 × 1080 DIP cases also passed, but exceed the available logical display size.

The test host renders real WPF windows off-screen and saves their visuals at 96 DPI with RenderTargetBitmap. These are rendered UI captures, not desktop screen captures. Exact full-screen verification on a 1920 × 1080 display at 100% scaling was unavailable. No display settings were changed. The available large window is the nearest supported comparison. Native frame areas can appear blank in the bitmap.

## Selected evidence

| State | Minimum | Default | Available large |
| --- | --- | --- | --- |
| Users: actions and role | [720 × 560](captures/ui04-users-720x560.png) | [1338 × 753](captures/ui04-users-1338x753.png) | [1536 × 864](captures/ui04-users-1536x864.png) |
| Parts materials editor | [720 × 560](captures/part-materials-720x560.png) | [1338 × 753](captures/part-materials-1338x753.png) | [1536 × 864](captures/part-materials-1536x864.png) |
| Customer history | [720 × 560](captures/ui04-history-OrdersTable-720x560.png) | [1338 × 753](captures/ui04-history-OrdersTable-1338x753.png) | [1536 × 864](captures/ui04-history-OrdersTable-1536x864.png) |

[Material unit cost after vertical scrolling](captures/ui04-material-costs-720x560.png) remains readable without horizontal scrolling. Other selected compact captures cover manager lists and composition tables. Vertical scrolling remains available for long records and editor pages.
