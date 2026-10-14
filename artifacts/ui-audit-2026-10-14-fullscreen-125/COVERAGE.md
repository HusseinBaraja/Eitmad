# UI audit coverage

All 18 feature view files in the current Windows shell were inventoried and reviewed. Each usable screen image was inspected individually at maximized full screen and 125% scaling. This is a visual audit with synthetic Arabic fixtures; states in this table are reviewed presentation states, not proof of live product completion.

## Feature pages

| Source | Page | Reviewed presentation states | Example capture | Prompt IDs |
| --- | --- | --- | --- | --- |
| [Authentication/SignInView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Authentication/SignInView.xaml>) | Sign-in | Form; connection failure | [sign-in-1920x1080](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/sign-in-1920x1080.png>) | 001 |
| [Home/HomeView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeView.xaml>) | Manager Home and shared Home/search content | Loaded; ready orders/activity; search; empty; denied; partial; unavailable | [audit-home-manager-start](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-home-manager-start.png>) | 003,004 |
| [Reception/ReceptionistHomeView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ReceptionistHomeView.xaml>) | Receptionist shell/Home | Daily work; search; loaded; empty; denied; partial; unavailable; hosted lists | [ui14-reception-top-1920](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui14-reception-top-1920.png>) | 002,004,031 |
| [RawMaterials/RawMaterialsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsView.xaml>) | Raw materials | List; add/edit; validation; category manager/add/edit; unit manager/add/edit; count/length/area conversions | [material-durable-editor-1920x1080](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-editor-1920x1080.png>) | 005–009,052 |
| [Parts/PartsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Parts/PartsView.xaml>) | Parts | List; information; materials; review; loaded material picker; no-match picker; cost section | [part-materials-1920x1080](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/part-materials-1920x1080.png>) | 010,011,054 |
| [Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>) | Furniture | List; create/edit information and image; parts; sizes; size editor; allowed options; color/handle editor; swatch choices; options top/bottom; all three price sections; review | [furniture-review-1920x1080](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-review-1920x1080.png>) | 012–017,043,045,046,053 |
| [Products/ProductsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Products/ProductsView.xaml>) | Ready-made Products | List; add/edit; image; supplier variants; no-variant cost; variant/archive state; category manager/editor; archive confirmation | [products-variants-1920x1080](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/products-variants-1920x1080.png>) | 018,019,047 |
| [Pricing/PricingView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Pricing/PricingView.xaml>) | Pricing | Loaded list/category; price editor; below-cost warning; invalid numeric input | [pricing-1550x830-125percent](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/pricing-1550x830-125percent.png>) | 020,021 |
| [Reception/SalesCatalogView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesCatalogView.xaml>) | Reception sales catalog and quotation window | Catalog; search/filters; new quotation starts in catalog; review navigation | [audit-reception-catalog](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-reception-catalog.png>) | 022,024,048 |
| [Reception/FurnitureSelectionView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/FurnitureSelectionView.xaml>) | Sales Furniture selection | Standard size; color/handle choices; custom dimensions; valid/invalid/pending state; returned price breakdown; footer | [sales-furniture-total-1920](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/sales-furniture-total-1920.png>) | 022,023,051 |
| [Reception/ProductSelectionView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ProductSelectionView.xaml>) | Sales Product selection | Loaded choices; supplier variants; selection review/footer | [ui28-product-reception-1920x1080](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui28-product-reception-1920x1080.png>) | 022,023 |
| [Reception/CurrentQuotationView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotationView.xaml>) | Current quotation | Mixed lines; duplicate/edit/remove; evaluation totals/invalid; customer search/selection/new; draft save/reopen; failed save; saved values; required fields; final actions | [quotation-draft-lines-1920](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-draft-lines-1920.png>) | 025–028 |
| [Customers/CustomerDetailView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerDetailView.xaml>) | Customer details | Identity/contact; quotations history; orders history; conflict blocks editing | [customers-detail-1920x1080](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/customers-detail-1920x1080.png>) | 029,030 |
| [Customers/CustomerEditorView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerEditorView.xaml>) | Customer editor | Name/contact/address/notes; save/cancel; real authority presentation | [customers-editor-1920x1080](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/customers-editor-1920x1080.png>) | 029 |
| [Quotations/QuotationsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsView.xaml>) | Quotations and approvals | Both roles: list/filter; draft; issued/active; converted; detail; issue; acceptance; cancellation; conversion confirmation; manager inbox/review; reception pending/rejected approval; saved draft | [quotation-issued-manager-1920](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-issued-manager-1920.png>) | 031–034,041 |
| [Orders/OrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrdersView.xaml>) | Orders | Both roles: list/filter; ready state; mixed-item detail; production/state; original quotation; cancellation; idle recovery control; delivery | [reception-orders-mixed-items](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/reception-orders-mixed-items.png>) | 031,035,036,050,055 |
| [WorkOrders/WorkOrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/WorkOrders/WorkOrdersView.xaml>) | Work Orders | Shell list/detail; production metadata; required Furniture/parts; empty and long notes; transition action; calendar/date | [audit-workorders-shell-detail](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-workorders-shell-detail.png>) | 037,038,049 |
| [Users/UsersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml>) | Users | List/filter; create/edit; password validation; authority failure; retained stale rows; retry; distinct login identities with same display name; deactivation | [ui39-users-list-1920](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui39-users-list-1920.png>) | 039–041 |

## Shared shell and dialogs

| Surface | Reviewed | Prompt IDs |
| --- | --- | --- |
| Title/search/actions and sidebar | Manager and reception shell; account action; account tooltip; account-switch confirmation | 002,009,041 |
| Page headers, form fields and operational tables | Screens listed above, including shared headers, empty space and filter proportions | 005,006,031 |
| Feature dialog host | Material references, Furniture part/size/color/handle, Product categories/archive and Part material picker | 008,009,043,045,046,054 |
| Customer history tables | Quotations and Orders, ordinary and conflict state | 029,030 |
| Overlay controls | Calendar; color palette; account tooltip | 045,049; no additional defect inferred |

The Product action context-menu and category-dropdown captures were transparent/black. Their source was inspected. Their popup geometry, physical focus and OS placement remain unverified. Category manager/editor and archive confirmation images were usable. The audit does not treat the black captures as app defects.

## Print preview — pages inspected separately

Initial fit-width saved quotation, order, draft and multi-page previews were inspected separately. The supplemental whole-page view exposed the document body and pagination without changing application window size. All seven whole-page captures below were opened one at a time.

| Document | Page | Capture | Finding |
| --- | --- | --- | --- |
| Saved quotation | 1 of 1 | [audit-print-quotation-page-1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-quotation-page-1.png>) | Initial zoom: UI-042 |
| Saved order | 1 of 1 | [audit-print-order-page-1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-order-page-1.png>) | Initial zoom: UI-042 |
| Draft preview | 1 of 1 | [audit-print-draft-page-1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-draft-page-1.png>) | Initial zoom: UI-042; preserve draft marker/print availability |
| 36-line quotation | 1 of 4 | [audit-print-multipage-page-1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-multipage-page-1.png>) | A-8 row starts at page bottom: UI-044 |
| 36-line quotation | 2 of 4 | [audit-print-multipage-page-2](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-multipage-page-2.png>) | A-8 description continues alone: UI-044 |
| 36-line quotation | 3 of 4 | [audit-print-multipage-page-3](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-multipage-page-3.png>) | Continued rows and repeated header inspected |
| 36-line quotation | 4 of 4 | [audit-print-multipage-page-4](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-multipage-page-4.png>) | Last rows and total inspected |

No paper was printed and no external printer state was changed. The shared pagination finding names both quotation and order document paths; the reproduced split is from the synthetic 36-line quotation.

## Method and result

The task-local [Audit.csproj](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/harness/Audit.csproj>) links the existing shell and rendered fixtures. [WpfTestHost.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/harness/WpfTestHost.cs>) maximizes each window, checks its actual DPI against 125%, and renders the current visual tree at 120 DPI. [Program.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/harness/Program.cs>) lists the 37 focused fixture methods. [Supplement.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/harness/Supplement.cs>) covers missing list/editor states and whole print pages.

The focused runner produced 37 PASS entries and no INCOMPLETE entries in [render-run.log](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/render-run.log>). Supplemental execution finished and produced its final quotation-actions capture after all print-page captures. The runner catches exceptions, so process exit code alone is not a pass gate: the logged fixture results and capture completeness were inspected.

The harness was built with:

```powershell
dotnet build artifacts/ui-audit-2026-10-14-fullscreen-125/harness/Audit.csproj --configuration Release --nologo -m:1
```

To reproduce the captures from the repository root on a 125% Windows display:

```powershell
$env:EITMAD_UI_CAPTURE_DIR = Join-Path (Get-Location) 'artifacts/ui-audit-2026-10-14-fullscreen-125/captures'
dotnet artifacts/ui-audit-2026-10-14-fullscreen-125/harness/bin/Release/net8.0-windows/Eitmad.WindowsShell.Tests.dll
dotnet artifacts/ui-audit-2026-10-14-fullscreen-125/harness/bin/Release/net8.0-windows/Eitmad.WindowsShell.Tests.dll Supplement
Remove-Item Env:EITMAD_UI_CAPTURE_DIR
```

The build completed with NU1900 because the NuGet vulnerability service was unreachable. No package audit is claimed. The complete Windows suite and Rust checks were not required for a report-only task and were not run. Existing fixture log prose about 96 DPI/offscreen rendering was copied from fixture methods; this task's DPI check and manifest establish actual 125%/120-DPI captures.

The individual inspection and duplicate record is in [CAPTURE-REVIEW.md](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/CAPTURE-REVIEW.md>). High-contrast output from an existing fixture was excluded. Product source and canonical documentation were not edited.
