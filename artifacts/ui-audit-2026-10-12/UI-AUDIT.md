# Eitmad UI audit and individual fix prompts

Audit date: 2026-10-12. Repository snapshot: recorded in [provenance](./provenance/verification.md).

This file contains **44 individual fix prompts** from a rendered audit of the implemented Windows app. It covers all **18 feature XAML views**, the manager home composition, shared shell and dialogs, and all three customer document types. Product source was not changed.

## Use this file

Copy one complete finding, including its prompt and screenshots, into a new agent request. Each prompt identifies its own source files and done state. The screenshot paths are absolute so an agent can inspect the original files. Shared findings apply to several screens; fix that concern once at the shared owner rather than creating separate competing styles. Findings are independent tasks, not authorization to implement missing business capabilities.

P1 means a blocked essential workflow. P2 means a defect or material usability problem. P3 means a visual/design improvement. “Defect” identifies an observed incorrect state or clipped content. “Design improvement” identifies a rendered usability or visual-quality problem and leaves the detailed solution to the fixing agent.

## Audit evidence and limits

The audit used native WPF rendering with synthetic Arabic fixtures, including authority-shaped fake-engine responses. It inspected 404 PNG captures through contact sheets and full-size checks of the affected details. This is a rendered UI audit, not a live server or physical-printer end-to-end test. Successful tests do not mean visual quality passed: the screenshots show issues that the existing assertions do not detect.

The display reported **125% scaling**. Requests for a 1920 × 1080 window were constrained by the available desktop. Captures are WPF client visuals rendered at 96 DPI, not physical desktop screenshots. Representative measurements are below; exact PNG dimensions are in [the capture manifest](./capture-manifest.json). Some focused tests render a standalone view, use 780-DIP width, or maximize the client area; their image names describe the test request, not a promise of exact outer-window geometry.

| Requested baseline | Actual WPF window measurement | Scaling |
| --- | --- | --- |
| 1920 × 1080 | 1553.6 × 881.6 DIP, desktop-constrained | 125% |
| 1338 × 753 | 1338.4 × 752.8 DIP | 125% |
| 720 × 560 | 720 × 560 DIP | 125% |

Exact 1920 × 1080 full-screen rendering at 100% scaling was **not verified**. The audit did not change the user's display configuration. Existing system-color and large-text fixture captures were inspected, but a native OS high-contrast session, screen-reader use, and manual keyboard/focus testing of every workflow were not performed. Popup content was inspected where captures exist; separately rendered popup images do not prove physical screen placement.

Black strips at the bottom/right of some images are capture boundaries, not findings. Two all-black product-menu captures were excluded as evidence; the usable default-size menu capture was inspected. Some tests deliberately scroll a control into view, so an absent title in a scrolled capture is not itself a bug. The supplemental bottom-scroll driver also scrolls nested controls; the shortened furniture price in UI-21 is not reported as a numeric defect.

The **reports destination is disabled and has no implemented page**. Its absence is recorded, and the implemented quotation/order/draft print documents were audited separately. A global customer list is also not implemented; customer detail/editor and quotation customer selection are covered. Tasks, settings, and messages have no active page. The system-tray menu was inspected in source only; its native OS rendering and the Windows print dialog were not captured.

## Screen coverage

| Screen or surface | States/surfaces inspected | Findings | Representative evidence |
| --- | --- | --- | --- |
| Sign-in | Default, busy, connection failure; compact clipping | 01 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-sign-in-720.png) |
| Manager home + shared HomeView | Metrics, ready orders, activity, search, denied/partial/unavailable | 06, 10–13 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-manager-home-top-1338.png) |
| Receptionist home | Default/unavailable, metrics, navigation cards, compact | 11, 12, 14 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/home-reception-720.png) |
| Raw materials | List, editor, category/unit dropdowns, reference manager, reference editor | 02, 04, 15, 16 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/material-durable-list-1338x753.png) |
| Parts | List; information, materials, review; material picker, inline category | 02, 04, 17 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/part-review-720x560.png) |
| Furniture | List; all six editor steps; part picker, size, color, handle dialogs, image states | 02–04, 18–23 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-step-6-720.png) |
| Ready-made products (manager) | List, editor, image states, variants, categories, category editor, row actions, archive confirmation | 02, 04, 24, 25 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/products-editor-1338x753.png) |
| Pricing | List, editor, below-cost confirmation, invalid input, restricted-role projection | 02, 03, 26 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/pricing-warning-720x560-125percent.png) |
| Sales catalog (reception) | Furniture/retail browsing, category popup/chips, filters, empty/unavailable, image/no-image | 06, 27 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/sales-catalog-live-720.png) |
| Furniture selection | Variant, custom dimensions, colors, handles, quantity, total, added feedback | 28, 29 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/sales-furniture-live-720.png) |
| Product selection | Variant, quantity, total, local selection notice | 28 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/sales-product-live-720.png) |
| Current quotation | Empty/populated lines, customer matching/new customer, evaluation, discount, saved draft, unsaved edit, failed save | 30–32 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/quotation-draft-failed-save-720.png) |
| Quotation lists/detail | Manager/reception lists; draft, active, issued, accepted, converted; issue/reissue/cancel, conversion confirmation | 02, 04, 33 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/quotation-issued-manager-720.png) |
| Discount approvals | Manager inbox/review, receptionist pending/rejected | 34 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/discount-review-manager-720.png) |
| Orders | Manager/reception lists/detail; ready, mixed item types, delivery, production links, notes/cancel | 02, 04, 35 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/reception-orders-mixed-items.png) |
| Work orders | List/detail, furniture/parts, planned/in-progress/complete action areas, assignee, date and native calendar | 02, 04, 36, 37 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/work-confirmed-action-720.png) |
| Customer detail | Manager/reception contact/history, pending sync and conflict | 02, 04, 38 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/customers-sync-conflict-720x560.png) |
| Customer editor | Name, phone, address, notes; compact dialog and footer | Shared findings only; no separate defect confirmed | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/customers-editor-720x560.png) |
| Users | Populated/filtered-empty/unavailable; create/edit/validation/deactivation | 02, 04, 05, 39–41 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-users-validation-720.png) |
| Shared shell/controls | Toolbar, sidebar, account switch, page header, dialogs, tables, selectors, floating feedback | 02–09 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/account-switch-720x560.png) |
| Print preview and printable documents | Saved quotation, order, draft; A4 export and four-page quotation including repeated headers/page numbers | 42–44 | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/saved-quotation-multipage-2.png) |
| Reports / التقارير | Source and sidebar inspected: disabled; no report view, report route, or report template exists | 07; unavailable capability, not a rendered report | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/products-list-1338x753.png) |
| Other unavailable destinations | Tasks, settings, messages are disabled; no implemented page to inspect | 07 where applicable | [Image](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/shared-chrome-manager.png) |

## Finding index

| ID | Priority | Kind | Issue |
| --- | --- | --- | --- |
| UI-01 | P1 | Defect | [Sign-in form is clipped at the minimum window size](#ui-01) |
| UI-02 | P2 | Design improvement | [Operational text is too small compared with the page chrome](#ui-02) |
| UI-03 | P2 | Design improvement | [Checkbox text overwhelms dialog labels and controls](#ui-03) |
| UI-04 | P2 | Design improvement | [Compact tables hide key values and row actions](#ui-04) |
| UI-05 | P2 | Defect | [Users empty state is vertically clipped](#ui-05) |
| UI-06 | P3 | Design improvement | [Repeated page titles consume space in reception flows](#ui-06) |
| UI-07 | P3 | Design improvement | [Unavailable sidebar destinations add noise to daily navigation](#ui-07) |
| UI-08 | P2 | Design improvement | [The compact manager primary action uses a misleading plus icon](#ui-08) |
| UI-09 | P3 | Design improvement | [Account switching is not clear from the toolbar icon](#ui-09) |
| UI-10 | P3 | Design improvement | [Manager dashboard lacks a clear operational hierarchy](#ui-10) |
| UI-11 | P3 | Design improvement | [Metric values and labels sit on opposite edges of each card](#ui-11) |
| UI-12 | P2 | Defect | [Unavailable home data also shows empty-result messages](#ui-12) |
| UI-13 | P2 | Design improvement | [Home search has no clear query context or reset action](#ui-13) |
| UI-14 | P3 | Design improvement | [Reception home pushes daily status below oversized action cards](#ui-14) |
| UI-15 | P2 | Design improvement | [Raw-material editor actions fall below the minimum viewport](#ui-15) |
| UI-16 | P2 | Design improvement | [Unit conversion inputs lack clear visible meaning](#ui-16) |
| UI-17 | P3 | Design improvement | [Pending part costs look like ordinary amount fields](#ui-17) |
| UI-18 | P2 | Design improvement | [Furniture image dominates the first wizard step at minimum size](#ui-18) |
| UI-19 | P3 | Design improvement | [Furniture progress strip is too dense for the compact editor](#ui-19) |
| UI-20 | P3 | Design improvement | [Variant compatibility choices need clearer grouping](#ui-20) |
| UI-21 | P2 | Design improvement | [Furniture price controls appear before the variant identity](#ui-21) |
| UI-22 | P3 | Design improvement | [Furniture color and handle sections use inconsistent layouts](#ui-22) |
| UI-23 | P3 | Design improvement | [Color selection exposes raw hex codes instead of a useful swatch](#ui-23) |
| UI-24 | P3 | Design improvement | [Product image area becomes a large blank section](#ui-24) |
| UI-25 | P3 | Design improvement | [Variant archive controls dominate product editing rows](#ui-25) |
| UI-26 | P2 | Defect | [Invalid price input leaves a stale margin and confirmation visible](#ui-26) |
| UI-27 | P3 | Design improvement | [Catalog cards waste compact width and reveal little useful content](#ui-27) |
| UI-28 | P2 | Design improvement | [Sales selection pages put essential actions below oversized previews](#ui-28) |
| UI-29 | P2 | Design improvement | [Custom furniture dimensions use unstyled narrow inputs](#ui-29) |
| UI-30 | P2 | Design improvement | [Customer search results appear far from the customer inputs](#ui-30) |
| UI-31 | P2 | Design improvement | [Quotation summary separates labels, values, and discount input](#ui-31) |
| UI-32 | P2 | Design improvement | [Draft-save failure has weak visual priority and mixed status messages](#ui-32) |
| UI-33 | P3 | Design improvement | [Quotation lifecycle actions appear as a flat form above the record](#ui-33) |
| UI-34 | P2 | Design improvement | [Discount approval panel does not support a clear decision review](#ui-34) |
| UI-35 | P3 | Design improvement | [Order detail places action forms before the ordered items](#ui-35) |
| UI-36 | P2 | Design improvement | [Work-order date picker is much smaller than its neighboring input](#ui-36) |
| UI-37 | P3 | Design improvement | [An empty work-order note is shown as a warning card](#ui-37) |
| UI-38 | P2 | Design improvement | [Customer sync conflict is too easy to miss and gives no next step](#ui-38) |
| UI-39 | P2 | Design improvement | [Users list and deactivation dialog omit the login identity](#ui-39) |
| UI-40 | P2 | Defect | [Password validation is displayed under the user's name](#ui-40) |
| UI-41 | P2 | Design improvement | [Users refresh failure leaves normal-looking stale rows with no retry action](#ui-41) |
| UI-42 | P2 | Defect | [Print preview opens wider than the available viewport](#ui-42) |
| UI-43 | P2 | Design improvement | [Print viewer search and chrome are not fully Arabic](#ui-43) |
| UI-44 | P3 | Design improvement | [Customer print documents need a stronger professional hierarchy](#ui-44) |

## Fix prompts

All prompts below target the native WPF shell in this repository. The fixing agent must read AGENTS.md and the named skill. Keep Rust-owned calculations, authorization, audit, persistence, schemas, and workflow state in Rust. Use existing controls and Arabic terms. Verify the changed screen with synthetic data at the three repository baselines when the rendering environment permits, and report the actual dimensions/scaling. Run only the focused checks appropriate to the change.

<a id="ui-01"></a>

### UI-01 — Sign-in form is clipped at the minimum window size

**P1 · Defect · Sign-in / تسجيل الدخول**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Sign-in form is clipped at the minimum window size**? It appears in **Sign-in / تسجيل الدخول** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The two-column layout keeps the photo panel at a minimum width of 285 DIP and the form panel at 390 DIP. The form itself has Width=420 plus 48-DIP side margins. The screenshot cuts off the logo, heading, field labels, and the right side of the credential inputs. Vertical scrolling cannot recover the missing horizontal content.

Reproduce: Open the sign-in screen at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Authentication/SignInView.xaml` and their direct callers/view models.

Done state: Replace the fixed-width form arrangement with an adaptive native WPF layout. At the minimum size, show the complete form with readable labels and reachable controls. Reduce or collapse the decorative photo when needed. Preserve the spacious two-panel design at larger sizes. Wrap connection errors and guidance within the form.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-01: audit-sign-in-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-sign-in-720.png)

![UI-01: audit-sign-in-failure-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-sign-in-failure-720.png)


<a id="ui-02"></a>

### UI-02 — Operational text is too small compared with the page chrome

**P2 · Design improvement · All manager lists and shared form controls**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Operational text is too small compared with the page chrome**? It appears in **All manager lists and shared form controls** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Large headings and the sidebar dominate the view, while table values, headers, metadata, help, and errors are much smaller and often pale. TableBody is 11 DIP; several headers, metadata labels, and actions are 10 DIP. Important operational data is harder to scan than navigation.

Reproduce: Open the materials, parts, products, pricing, orders, and quotations lists at the default size. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsControls.xaml` and their direct callers/view models.

Done state: Redesign the shared text hierarchy using the existing Arabic font and visual system. Make primary row data, meaningful labels, and errors comfortably readable. Use secondary styling only for secondary information. Adjust row heights and spacing with the text sizes. Check affected tables and forms for clipping. Do not shrink the whole page.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-02: material-durable-list-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/material-durable-list-1338x753.png)

![UI-02: products-list-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/products-list-1338x753.png)


<a id="ui-03"></a>

### UI-03 — Checkbox text overwhelms dialog labels and controls

**P2 · Design improvement · Furniture variant dialog and pricing editor**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Checkbox text overwhelms dialog labels and controls**? It appears in **Furniture variant dialog and pricing editor** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Checkbox labels render much larger than nearby field labels and help text. In the furniture dialog, long compatibility instructions become dominant multi-line headings with tiny checkbox squares. The below-cost acknowledgement similarly competes with the dialog title.

Reproduce: Edit a furniture size, then open a below-cost price confirmation. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Pricing/PricingView.xaml` and their direct callers/view models.

Done state: Apply a suitable native checkbox style to the affected choices. Match the form text hierarchy, keep the checkbox and label aligned in RTL, and wrap long labels without reducing the hit area. Keep all choices, acknowledgements, and Rust-owned permission rules intact.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-03: audit-furniture-variant-dialog-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-variant-dialog-720.png)

![UI-03: pricing-warning-720x560-125percent.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/pricing-warning-720x560-125percent.png)


<a id="ui-04"></a>

### UI-04 — Compact tables hide key values and row actions

**P2 · Design improvement · Manager lists, composition editors, and customer history tables**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Compact tables hide key values and row actions**? It appears in **Manager lists, composition editors, and customer history tables** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The sum of fixed column minimum widths exceeds the available content width. Users need horizontal scrolling to reach row actions; the parts materials editor cuts off cost columns. The page can also have an outer vertical scrollbar. Native horizontal scrolling is available, but the everyday compact workflow is difficult to scan and operate.

Reproduce: Open the users list or parts materials step at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/OperationsTable.cs`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsControls.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Parts/PartsView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerDetailView.xaml` and their direct callers/view models.

Done state: Redesign the affected compact table layouts using native WPF behavior. Keep record identity and the main action discoverable. Move secondary fields into an appropriate detail presentation or choose justified compact column widths. Keep numeric values readable and preserve native sorting, selection, editing, and keyboard behavior. Do not remove information or reduce all text to make it fit.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-04: audit-users-list-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-users-list-720.png)

![UI-04: part-materials-720x560.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/part-materials-720x560.png)


<a id="ui-05"></a>

### UI-05 — Users empty state is vertically clipped

**P2 · Defect · Users / المستخدمون**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Users empty state is vertically clipped**? It appears in **Users / المستخدمون** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Only fragments of the empty-state text appear below the table header. The filter area consumes height, and the empty table does not give its EmptyContent enough usable space. The message is not readable in the captured minimum-size view.

Reproduce: At 720 × 560, search for لا يوجد in a populated users list. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsControls.xaml` and their direct callers/view models.

Done state: Ensure the complete heading and guidance remain readable when the result set is empty. Let the appropriate parent scroll or reserve enough measured height for the empty state. Verify this specific filtered-empty state with the filters visible. Preserve populated-row scrolling.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-05: audit-users-empty-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-users-empty-720.png)


<a id="ui-06"></a>

### UI-06 — Repeated page titles consume space in reception flows

**P3 · Design improvement · Reception catalog and manager home**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Repeated page titles consume space in reception flows**? It appears in **Reception catalog and manager home** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The shell title and the inner page title repeat المنتجات or لوحة التحكم in two separate horizontal bands. The extra heading adds height without more context. At minimum width, the toolbar already uses two rows.

Reproduce: Open the receptionist products page or manager dashboard. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesCatalogView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ReceptionistHomeView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/MainWindow.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/ShellTitleBar.xaml` and their direct callers/view models.

Done state: Give each affected screen one clear title hierarchy. Keep global search and shell actions where they belong, and use the content heading only when it adds distinct context. Redesign this in the existing native visual system. Check role-specific navigation and compact height.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-06: catalog-normal.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/catalog-normal.png)

![UI-06: audit-manager-home-top-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-manager-home-top-720.png)


<a id="ui-07"></a>

### UI-07 — Unavailable sidebar destinations add noise to daily navigation

**P3 · Design improvement · Manager sidebar, including reports / التقارير**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Unavailable sidebar destinations add noise to daily navigation**? It appears in **Manager sidebar, including reports / التقارير** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

المهام, التقارير, and الإعدادات are disabled destinations mixed with active destinations. They consume vertical space and look similar to usable navigation, although their tooltips explain that they are unavailable. The audit found no implemented reports page to open.

Reproduce: Open the manager shell and inspect the lower sidebar. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/ShellSidebar.xaml` and their direct callers/view models.

Done state: Redesign the presentation of unavailable destinations so active daily routes are easy to scan. You may hide, group, or clearly separate unavailable items using the best native solution. Keep the current capability boundaries truthful. This prompt does not authorize implementing reports, tasks, or settings, or making disabled routes appear functional.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-07: products-list-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/products-list-1338x753.png)


<a id="ui-08"></a>

### UI-08 — The compact manager primary action uses a misleading plus icon

**P2 · Design improvement · Manager shell toolbar**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **The compact manager primary action uses a misleading plus icon**? It appears in **Manager shell toolbar** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The manager action is approvals, but the button always uses IconPlus. In compact mode its text is hidden, leaving a plus with no visible indication that it opens approvals. The same shared action has a different meaning for the receptionist quotation flow.

Reproduce: Resize the manager dashboard to 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/ShellTitleBar.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/ShellTitleBar.xaml.cs`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/MainWindow.xaml.cs` and their direct callers/view models.

Done state: Use an icon and compact presentation that match the current action. Bind an explicit Arabic accessible name to PrimaryActionButton. Keep the receptionist quotation action and its item count understandable in compact mode. Preserve the existing click routes and permission rules.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-08: audit-manager-home-top-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-manager-home-top-720.png)

![UI-08: shared-chrome-manager.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/shared-chrome-manager.png)


<a id="ui-09"></a>

### UI-09 — Account switching is not clear from the toolbar icon

**P3 · Design improvement · Shared shell toolbar and account-switch dialog**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Account switching is not clear from the toolbar icon**? It appears in **Shared shell toolbar and account-switch dialog** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The two-person icon can read as a users-management action, while it actually opens account switching and then sign-out. SwitchAccountHint is bound to Tag and the automation name, but the button has no explicit visible label or tooltip. The next dialog is the first clear explanation of the action.

Reproduce: Inspect the two-person toolbar button, then open تبديل الحساب. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/ShellTitleBar.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/ShellTitleBar.xaml.cs` and their direct callers/view models.

Done state: Make account switching understandable before activation. Use an appropriate icon, an Arabic tooltip, and a visible label where space allows. Keep the existing warning about unsaved changes and the sign-out behavior. Do not add account policy or authentication logic to the shell.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-09: shared-chrome-manager.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/shared-chrome-manager.png)

![UI-09: account-switch-720x560.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/account-switch-720x560.png)


<a id="ui-10"></a>

### UI-10 — Manager dashboard lacks a clear operational hierarchy

**P3 · Design improvement · Manager home / لوحة التحكم**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Manager dashboard lacks a clear operational hierarchy**? It appears in **Manager home / لوحة التحكم** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Six small quick links, a long availability paragraph, refresh text, metric cards, ready orders, and activity form a mostly flat vertical stack. The small links and large empty regions do not help the manager identify what needs action first.

Reproduce: Open the manager dashboard at the default and minimum sizes. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/MainWindow.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeView.xaml` and their direct callers/view models.

Done state: Redesign the dashboard using the best layout for the existing data. Give pending approvals and ready orders a clear place, make quick routes compact and consistent, and separate data availability from operational content. Use only the existing Rust-provided state. Keep all supported destinations and partial-data explanations.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-10: audit-manager-home-top-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-manager-home-top-1338.png)

![UI-10: audit-manager-home-top-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-manager-home-top-720.png)


<a id="ui-11"></a>

### UI-11 — Metric values and labels sit on opposite edges of each card

**P3 · Design improvement · Manager and receptionist home metric cards**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Metric values and labels sit on opposite edges of each card**? It appears in **Manager and receptionist home metric cards** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The Arabic label is on the right of each wide card, while its count is on the far left. HomeCount changes FlowDirection to LeftToRight and sets HorizontalAlignment=Right; the resulting physical placement separates the value from its label. The association becomes weak in the two-column layout.

Reproduce: Open either home screen with populated metrics. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeView.xaml` and their direct callers/view models.

Done state: Keep each count visually close to its label with a clear Arabic reading order. Isolate the numeric text as LTR without moving it to an unrelated physical edge. Choose a restrained native metric-card layout and keep unavailable dashes and partial counts distinct.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-11: audit-manager-home-top-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-manager-home-top-1338.png)

![UI-11: home-reception-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/home-reception-1338.png)


<a id="ui-12"></a>

### UI-12 — Unavailable home data also shows empty-result messages

**P2 · Defect · Manager and receptionist home failure states**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Unavailable home data also shows empty-result messages**? It appears in **Manager and receptionist home failure states** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The top notice says data is unavailable, but the ready-orders and activity sections also say that there are no records. IsEmpty and HasNoReadyOrders depend only on collection counts. This makes an unavailable query look like a confirmed empty result.

Reproduce: Load home data successfully, then make the fake engine query fail; also inspect a partially denied response. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeViewModel.cs` and their direct callers/view models.

Done state: Render distinct available-empty, denied, unavailable, and partial-result section states from the existing authority result. Keep a clear retry action. Do not show a confirmed-empty message for a section whose data could not be obtained. Add a focused presentation-state regression check; Rust remains the authority.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-12: audit-home-unavailable-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-home-unavailable-1338.png)

![UI-12: audit-home-partial-denied-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-home-partial-denied-1338.png)


<a id="ui-13"></a>

### UI-13 — Home search has no clear query context or reset action

**P2 · Design improvement · Global search results in home**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Home search has no clear query context or reset action**? It appears in **Global search results in home** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The activity heading changes to نتائج البحث, but the result area does not show the active term or a clear return-to-all action. The capture has filtered data beneath dashboard metrics and quick links. The user can lose the distinction between a filtered view and normal activity.

Reproduce: Use global search for عميل and inspect the resulting home section. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeViewModel.cs`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/ShellTitleBar.xaml.cs` and their direct callers/view models.

Done state: Show the active search term and a clear Arabic action to clear it and return to normal activity. Reuse the existing search behavior and query authority. Make the result state understandable after scrolling and after refresh. Keep identifiers directionally isolated.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-13: audit-home-search-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-home-search-1338.png)


<a id="ui-14"></a>

### UI-14 — Reception home pushes daily status below oversized action cards

**P3 · Design improvement · Receptionist home / الرئيسية**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Reception home pushes daily status below oversized action cards**? It appears in **Receptionist home / الرئيسية** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The 176-DIP new-quotation hero is followed by three 142-DIP navigation cards that become one column in compact mode. Status metrics and ready orders appear far below these navigation choices. The default view also gives a large share of the page to decorative action surfaces.

Reproduce: Open receptionist home at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ReceptionistHomeView.xaml` and their direct callers/view models.

Done state: Redesign reception home to support fast daily work. Keep new quotation prominent, make secondary routes compact, and bring relevant status and ready orders nearer the first viewport. Preserve Arabic-first native layout and the available home data. Choose the best design within the existing visual system.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-14: home-reception-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/home-reception-720.png)

![UI-14: catalog-home.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/catalog-home.png)


<a id="ui-15"></a>

### UI-15 — Raw-material editor actions fall below the minimum viewport

**P2 · Design improvement · Raw-material add/edit screen / الخامات**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Raw-material editor actions fall below the minimum viewport**? It appears in **Raw-material add/edit screen / الخامات** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The form shows name, category, unit, and cost, but Save and Cancel are below the initial viewport. This differs from the visible fixed action area in the furniture and products editors. The user must scroll past the last field just to find the main action.

Reproduce: Edit a material at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsView.xaml` and their direct callers/view models.

Done state: Keep Save and Cancel clearly reachable in the minimum-size editor. Choose a native action area or a more compact measured form. Keep the cost field and its validation visible when an error occurs, and preserve the existing persistence commands.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-15: material-durable-editor-720x560.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/material-durable-editor-720x560.png)


<a id="ui-16"></a>

### UI-16 — Unit conversion inputs lack clear visible meaning

**P2 · Design improvement · Raw-material unit editor / إضافة وحدة جديدة**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Unit conversion inputs lack clear visible meaning**? It appears in **Raw-material unit editor / إضافة وحدة جديدة** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The exact conversion is presented as two bare numbers separated by a slash. Only the whole group has a small caption; numerator and denominator names exist as automation names, not visible labels. The form does not make the direction of conversion easy to understand.

Reproduce: Open the unit editor from the material unit selector. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsView.xaml` and their direct callers/view models.

Done state: Give both values clear visible Arabic labels and explain the conversion direction in a short, accurate example based on the selected dimension and existing unit authority. Keep exact rational input; do not replace it with shell-side floating-point calculation. Make the full group usable when scrolled at minimum size.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-16: material-durable-unit-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/material-durable-unit-1338x753.png)

![UI-16: material-durable-unit-720x560.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/material-durable-unit-720x560.png)


<a id="ui-17"></a>

### UI-17 — Pending part costs look like ordinary amount fields

**P3 · Design improvement · Parts wizard, materials step / القطع**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Pending part costs look like ordinary amount fields**? It appears in **Parts wizard, materials step / القطع** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The UI explains in small help text that cost is calculated during review, but row totals and the prominent part-cost area still display amount-style placeholders. A dash beside ر.ي is visually similar to a missing financial value rather than a deliberate pending evaluation.

Reproduce: Add materials and quantities in the second wizard step before entering review. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Parts/PartsView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Parts/PartsViewModel.cs` and their direct callers/view models.

Done state: Show a clear Arabic pending-evaluation state where no authority-calculated cost exists. Keep the guidance near the totals and keep confirmed amounts visually distinct after review. Reuse Rust calculation and existing evaluation timing; do not calculate domain costs in C#.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-17: part-materials-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/part-materials-1338x753.png)

![UI-17: part-review-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/part-review-1338x753.png)


<a id="ui-18"></a>

### UI-18 — Furniture image dominates the first wizard step at minimum size

**P2 · Design improvement · Furniture wizard, information step / الأثاث**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Furniture image dominates the first wizard step at minimum size**? It appears in **Furniture wizard, information step / الأثاث** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The first viewport below the header and step strip is occupied by the image section. Name and category fields are below it, so the user cannot see the main product information while starting the form.

Reproduce: Open furniture add/edit at 720 × 560 before scrolling. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml` and their direct callers/view models.

Done state: Redesign the information step so the main identifying fields are visible first at minimum size. Reduce, move, or collapse the decorative image area while keeping image selection and preview easy to use. Preserve the larger-size composition and the pinned wizard actions.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-18: audit-furniture-step-1-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-step-1-720.png)

![UI-18: furniture-information-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/furniture-information-1338x753.png)


<a id="ui-19"></a>

### UI-19 — Furniture progress strip is too dense for the compact editor

**P3 · Design improvement · Furniture six-step wizard**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Furniture progress strip is too dense for the compact editor**? It appears in **Furniture six-step wizard** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

All six step labels and number badges occupy one narrow row. The labels are much smaller than the page title and compete with similarly styled pill controls. Raising the general text size alone will make this strip harder to fit.

Reproduce: Open any furniture editor step at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/StepIndicator.cs`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsControls.xaml` and their direct callers/view models.

Done state: Redesign the compact progress presentation with clear current-step context and a readable position such as current step plus total, or another suitable native pattern. Preserve all six steps and the existing forward/back rules. Keep the full progress view at wider sizes if it remains useful.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-19: audit-furniture-step-1-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-step-1-720.png)

![UI-19: audit-furniture-step-5-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-step-5-720.png)


<a id="ui-20"></a>

### UI-20 — Variant compatibility choices need clearer grouping

**P3 · Design improvement · Furniture size/variant editor dialog**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Variant compatibility choices need clearer grouping**? It appears in **Furniture size/variant editor dialog** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Name, three dimensions, custom-size permission, compatible colors, and compatible handles form one unstructured stack. Long instructions are repeated in the section labels. The lower choices require scrolling at minimum size, while the grouping does not make their relationship clear.

Reproduce: Edit a size that has color and handle compatibility choices. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml` and their direct callers/view models.

Done state: Redesign this dialog with clear groups for identification/dimensions, custom sizing, and compatible options. Put short instructions in help text and keep the actual choice labels short. Preserve empty-selection semantics and all values. Coordinate checkbox styling with UI-03 without requiring that fix first.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-20: audit-furniture-variant-dialog-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-variant-dialog-1338.png)

![UI-20: audit-furniture-variant-dialog-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-variant-dialog-720.png)


<a id="ui-21"></a>

### UI-21 — Furniture price controls appear before the variant identity

**P2 · Design improvement · Furniture wizard, pricing step**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Furniture price controls appear before the variant identity**? It appears in **Furniture wizard, pricing step** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

In the compact card, cost, selling-price input, and margin appear above the size name and dimensions. The financial fields have stronger visual weight than the identity below them. With several variants, it is easy to edit the wrong size's price.

Reproduce: Go to the fifth furniture wizard step at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml` and their direct callers/view models.

Done state: Place the size name and dimensions at the start of each pricing group, followed by its cost, editable price, and margin. Keep each group visually contained in RTL. Verify several variants and long names. Do not infer a numeric truncation defect from this screenshot: the supplemental driver scrolled inner controls.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-21: audit-furniture-step-5-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-step-5-720.png)

![UI-21: audit-furniture-step-5-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-furniture-step-5-1338.png)


<a id="ui-22"></a>

### UI-22 — Furniture color and handle sections use inconsistent layouts

**P3 · Design improvement · Furniture wizard, options step**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Furniture color and handle sections use inconsistent layouts**? It appears in **Furniture wizard, options step** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Colors appear in a table with one set of alignment rules, while handles use a different card-like presentation and action placement. The two similar option collections have different visual density and make price adjustments harder to compare.

Reproduce: Open the fourth furniture wizard step with colors and handles. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml` and their direct callers/view models.

Done state: Redesign the two collections as related option groups with consistent identity, preview, adjustment amount, and edit/remove action placement. Choose the best native layout for wide and compact widths. Preserve their distinct data and supported behavior; this is a presentation change.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-22: furniture-options-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/furniture-options-1338x753.png)

![UI-22: furniture-options-720x560.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/furniture-options-720x560.png)


<a id="ui-23"></a>

### UI-23 — Color selection exposes raw hex codes instead of a useful swatch

**P3 · Design improvement · Furniture color editor / إضافة لون**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Color selection exposes raw hex codes instead of a useful swatch**? It appears in **Furniture color editor / إضافة لون** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The color choice displays a technical hex string such as #F7F4EF in a generic selector. A workshop user cannot easily judge the displayed color from the code. The neighboring price-adjustment input also lacks a visible currency annotation.

Reproduce: Open إضافة لون and inspect اللون المعروض. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml` and their direct callers/view models.

Done state: Redesign the color choice around a visible swatch with an Arabic name or clear label. Keep technical codes secondary if useful. Mark the adjustment amount as ر.ي and clarify that it is a price difference. Preserve the existing allowed values and Rust-owned price behavior; use native vector resources.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-23: dialog-furniture-color-780.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/dialog-furniture-color-780.png)


<a id="ui-24"></a>

### UI-24 — Product image area becomes a large blank section

**P3 · Design improvement · Manager product editor / تعديل المنتج**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Product image area becomes a large blank section**? It appears in **Manager product editor / تعديل المنتج** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The image section shows a large white region when there is no image. Its choose/remove buttons are much farther down the form, so the blank area looks incomplete rather than an intentional empty state.

Reproduce: Edit a product without an image and scroll to الصورة. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Products/ProductsView.xaml` and their direct callers/view models.

Done state: Show a compact, clear empty-image state with image selection next to it. Give an existing image a bounded preview and show removal only when useful. Keep the name/category form efficient at minimum width and preserve the existing image authority and upload path.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-24: products-editor-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/products-editor-1338x753.png)

![UI-24: products-variants-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/products-variants-1338x753.png)


<a id="ui-25"></a>

### UI-25 — Variant archive controls dominate product editing rows

**P3 · Design improvement · Manager product editor, variant table**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Variant archive controls dominate product editing rows**? It appears in **Manager product editor, variant table** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Each row shows a large outlined أرشفة button beside the name and purchase-cost fields. The destructive actions have more visual weight than the editable values, and repeated borders make the section look like separate forms rather than one coherent table.

Reproduce: Edit a product with several ready-made size/type variants. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Products/ProductsView.xaml` and their direct callers/view models.

Done state: Redesign variant rows so identity and cost are primary and archive is a secondary, clearly named action. Align field heights, row spacing, and status. Keep keyboard access and the current archive behavior; do not silently delete variants or change persistence rules.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-25: products-variants-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/products-variants-1338x753.png)

![UI-25: table-product-variants-780.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/table-product-variants-780.png)


<a id="ui-26"></a>

### UI-26 — Invalid price input leaves a stale margin and confirmation visible

**P2 · Defect · Pricing editor / تعديل سعر البيع**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Invalid price input leaves a stale margin and confirmation visible**? It appears in **Pricing editor / تعديل سعر البيع** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The input error is visible, but the calculated margin still shows -10,000 ر.ي from the earlier valid input, and the below-cost confirmation remains displayed. Those values no longer describe the current input.

Reproduce: Obtain a below-cost evaluation, then change the price to an invalid fractional value such as 220000.5. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Pricing/PricingView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Pricing/PricingViewModel.cs` and their direct callers/view models.

Done state: Invalidate or clearly mark the old presentation evaluation when the price text is invalid or differs from the evaluated value. Do not show the old margin as current, and do not offer a stale below-cost acknowledgement. Keep save availability and authority evaluation consistent. Add a focused state regression test.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-26: pricing-warning-720x560-125percent.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/pricing-warning-720x560-125percent.png)


<a id="ui-27"></a>

### UI-27 — Catalog cards waste compact width and reveal little useful content

**P3 · Design improvement · Reception sales catalog / المنتجات**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Catalog cards waste compact width and reveal little useful content**? It appears in **Reception sales catalog / المنتجات** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Cards have a fixed 290-DIP width. In the minimum-size capture, one card sits on the right with a large empty area to its left, and its 188-DIP image area takes most of the list viewport. Product details and the select action fall below the visible portion.

Reproduce: Open the catalog at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesCatalogView.xaml` and their direct callers/view models.

Done state: Redesign the compact catalog to use its available width and show useful identity, price, availability context, and selection affordance sooner. Choose a native list/card arrangement suited to frequent lookup. Keep category filters, paging, image fallback, and authority-backed availability truthful.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-27: sales-catalog-live-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/sales-catalog-live-720.png)

![UI-27: catalog-compact.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/catalog-compact.png)


<a id="ui-28"></a>

### UI-28 — Sales selection pages put essential actions below oversized previews

**P2 · Design improvement · Reception furniture and ready-made product selection**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Sales selection pages put essential actions below oversized previews**? It appears in **Reception furniture and ready-made product selection** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

A fixed 250-DIP image panel, large title, separate persistence notice, back button, and refresh action consume the top of the page. Quantity, total, and Add require further scrolling. The user cannot readily review the current selection and commit the local line.

Reproduce: Select a furniture item and a ready-made product at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/FurnitureSelectionView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ProductSelectionView.xaml` and their direct callers/view models.

Done state: Redesign both selection pages for efficient native input. Keep item identity, current options, quantity, total, and Add easy to find together. Reduce the compact preview and group supporting actions. Preserve explicit local-only state and later save confirmation. Do not imply that adding a line saves it.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-28: sales-furniture-live-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/sales-furniture-live-720.png)

![UI-28: sales-product-live-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/sales-product-live-720.png)


<a id="ui-29"></a>

### UI-29 — Custom furniture dimensions use unstyled narrow inputs

**P2 · Design improvement · Reception furniture selection, custom dimensions**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Custom furniture dimensions use unstyled narrow inputs**? It appears in **Reception furniture selection, custom dimensions** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The custom width/height/depth fields use small native text boxes that look different from the rounded application inputs. Labels and numeric text sit tightly on the field edges. The inputs have less visual affordance than the large option cards immediately above.

Reproduce: Select a furniture variant that allows custom dimensions at minimum size. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/FurnitureSelectionView.xaml` and their direct callers/view models.

Done state: Apply suitable existing native field styling and lay out the three dimensions with clear Arabic labels and سم units. Keep LTR numeric editing and ensure readable invalid/pending states. Preserve the actual allowed ranges and Rust evaluation; do not move dimension validation or price rules into the shell.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-29: sales-furniture-live-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/sales-furniture-live-720.png)

![UI-29: catalog-selection-compact.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/catalog-selection-compact.png)


<a id="ui-30"></a>

### UI-30 — Customer search results appear far from the customer inputs

**P2 · Design improvement · Current quotation, customer selection / بيانات العميل**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Customer search results appear far from the customer inputs**? It appears in **Current quotation, customer selection / بيانات العميل** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Name and phone fields sit on the right, while the matching-customer button and + عميل جديد sit on the far left of the wide form. The gap makes the result look unrelated to the search inputs. The full-width form also gives short data fields excessive horizontal travel.

Reproduce: Start a quotation and search for الاعتماد as the customer name at the default size. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotationView.xaml` and their direct callers/view models.

Done state: Keep matching customers and the new-customer action next to the customer search fields in a clear bounded section. Show selected versus typed customer state clearly, including phone. Use native RTL layout with LTR phone text. Preserve customer identity, existing-record selection, and save behavior.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-30: customers-selection-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/customers-selection-1338x753.png)

![UI-30: customers-selection-720x560.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/customers-selection-720x560.png)


<a id="ui-31"></a>

### UI-31 — Quotation summary separates labels, values, and discount input

**P2 · Design improvement · Current quotation, totals and discount section**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Quotation summary separates labels, values, and discount input**? It appears in **Current quotation, totals and discount section** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Subtotal and discount labels sit at the far right, their amounts at the far left, and the discount field also sits left. The final total returns to the right. A large empty middle makes the financial summary difficult to scan and compare.

Reproduce: Scroll to the totals section of a populated quotation at the default size. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotationView.xaml` and their direct callers/view models.

Done state: Redesign this as a clear bounded financial summary with aligned label/value pairs, a nearby discount input, and one prominent final total. Make it work at minimum width without long eye travel. Keep the displayed amounts from Rust evaluation and preserve Arabic labels, LTR amounts, and rounding behavior.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-31: quotation-evaluation-summary-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/quotation-evaluation-summary-1338.png)

![UI-31: quotation-evaluation-summary-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/quotation-evaluation-summary-720.png)


<a id="ui-32"></a>

### UI-32 — Draft-save failure has weak visual priority and mixed status messages

**P2 · Design improvement · Current quotation, failed save / حفظ كمسودة**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Draft-save failure has weak visual priority and mixed status messages**? It appears in **Current quotation, failed save / حفظ كمسودة** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The bottom area contains several plain paragraphs: a saved-local/pending-sync message, workflow guidance, and a failure telling the user to retry. The failure is not visually distinct from routine guidance, and the distinction between the saved version and current unsaved edits is easy to miss.

Reproduce: Load a saved draft, edit it, then simulate save failure. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotationView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/QuotationDraftState.cs` and their direct callers/view models.

Done state: Give the active save failure a clear Arabic error presentation and nearby retry action using existing UI controls. Clearly distinguish the previously saved draft from current unsaved edits. Preserve all edits on failure and make document/issue action availability truthful. Do not claim synchronization or persistence before authority confirms it.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-32: quotation-draft-failed-save-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/quotation-draft-failed-save-720.png)

![UI-32: quotation-draft-failed-save-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/quotation-draft-failed-save-1338.png)


<a id="ui-33"></a>

### UI-33 — Quotation lifecycle actions appear as a flat form above the record

**P3 · Design improvement · Saved quotation detail, issue/reissue/cancel/accept states**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Quotation lifecycle actions appear as a flat form above the record**? It appears in **Saved quotation detail, issue/reissue/cancel/accept states** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

Validity days, cancellation reason, issue-new-version, cancel, and print appear above the main record metadata. They are presented as a flat form, so a high-impact action is not clearly associated with its purpose or prerequisites.

Reproduce: Open an issued quotation and a receptionist acceptance view. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsView.xaml` and their direct callers/view models.

Done state: Redesign lifecycle actions into clear context-specific native groups. Put the current record identity and state before the relevant action form. Separate cancellation from ordinary issue/print actions and explain disabled actions locally. Preserve Rust-owned lifecycle rules, role permissions, and confirmation behavior.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-33: quotation-issued-manager-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/quotation-issued-manager-720.png)

![UI-33: quotation-accept-reception-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/quotation-accept-reception-720.png)


<a id="ui-34"></a>

### UI-34 — Discount approval panel does not support a clear decision review

**P2 · Design improvement · Manager quotation discount approval**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Discount approval panel does not support a clear decision review**? It appears in **Manager quotation discount approval** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The requested percentage and amount sit on the left of a largely right-aligned panel. The explanation is shown in a thin text field, followed by small rejection guidance and approve/reject buttons. It is difficult to identify the request, reason, and required response as one decision.

Reproduce: Open a quotation from the discount approval inbox at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsView.xaml` and their direct callers/view models.

Done state: Redesign the approval panel with a clear request summary, readable reason, and distinct approve/reject action area. Label the rejection reason explicitly and show its requirement only when relevant. Keep amounts and authority state unchanged, and keep the quotation details available for review.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-34: discount-review-manager-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/discount-review-manager-720.png)

![UI-34: discount-review-manager-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/discount-review-manager-1338.png)


<a id="ui-35"></a>

### UI-35 — Order detail places action forms before the ordered items

**P3 · Design improvement · Manager and receptionist order detail / تفاصيل الطلب**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Order detail places action forms before the ordered items**? It appears in **Manager and receptionist order detail / تفاصيل الطلب** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The status banner and metadata are followed by full-width note/cancellation or delivery forms. Ordered items and production context are farther down. Ordinary notes, delivery, and cancellation have similar visual weight despite different purposes.

Reproduce: Open a manager order and a ready receptionist order. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrdersView.xaml` and their direct callers/view models.

Done state: Redesign order detail around a readable identity/status summary, ordered items, production context, and a focused action area for the current role and state. Give cancellation a clearly separate treatment. Keep all current data, fields, authority gates, and confirmation paths.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-35: order-confirmed-manager-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/order-confirmed-manager-1338.png)

![UI-35: order-ready-reception-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/order-ready-reception-720.png)

![UI-35: reception-orders-mixed-items.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/reception-orders-mixed-items.png)


<a id="ui-36"></a>

### UI-36 — Work-order date picker is much smaller than its neighboring input

**P2 · Design improvement · Work-order detail, assignment and due date**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Work-order date picker is much smaller than its neighboring input**? It appears in **Work-order detail, assignment and due date** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The assignee input uses the application's 44-DIP rounded field, while the adjacent DatePicker appears as a thin system text field with a tiny calendar button. Their baselines, height, border, and input affordance do not match.

Reproduce: Open a planned work order and scroll to مسند إلى and موعد التسليم. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/WorkOrders/WorkOrdersView.xaml` and their direct callers/view models.

Done state: Style the native WPF DatePicker to fit the existing form field system while keeping its native date entry, calendar, keyboard behavior, and Arabic metadata. Keep the date value directionally safe. Verify the popup near screen edges and at minimum size. Do not replace it with a web-style calendar.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-36: work-confirmed-action-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/work-confirmed-action-720.png)

![UI-36: audit-work-calendar-popup-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-work-calendar-popup-720.png)


<a id="ui-37"></a>

### UI-37 — An empty work-order note is shown as a warning card

**P3 · Design improvement · Work-order detail / ملاحظات الطلب**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **An empty work-order note is shown as a warning card**? It appears in **Work-order detail / ملاحظات الطلب** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

A large yellow callout with a gold vertical stripe displays only an em dash. The visual treatment suggests a warning even though there is no note, and it uses substantial vertical space before the assignment fields.

Reproduce: Open a work order with no order notes. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/WorkOrders/WorkOrdersView.xaml` and their direct callers/view models.

Done state: Use a neutral, compact no-notes state or omit the empty card with a suitable nearby label. Reserve warning treatment for a real actionable warning. Keep non-empty notes clearly readable and wrapped, and preserve the note text supplied by the authority.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-37: work-confirmed-action-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/work-confirmed-action-720.png)


<a id="ui-38"></a>

### UI-38 — Customer sync conflict is too easy to miss and gives no next step

**P2 · Design improvement · Customer detail / تفاصيل العميل**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Customer sync conflict is too easy to miss and gives no next step**? It appears in **Customer detail / تفاصيل العميل** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The conflict appears only as a small gray تعارض يحتاج مراجعة subtitle. The Edit button is disabled, but the page gives no clear explanation of that restriction or next available action. It looks close to an ordinary muted sync status.

Reproduce: Open the existing synthetic customer conflict state. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerDetailView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerDetailView.xaml.cs`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerPreview.cs` and their direct callers/view models.

Done state: Show conflict as a clear text-and-icon status with an accurate explanation for the disabled editor. Provide only a recovery action that the current authority supports; if no resolution workflow exists, state that limitation clearly rather than inventing one. Keep the last known contact and history data readable.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-38: customers-sync-conflict-720x560.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/customers-sync-conflict-720x560.png)

![UI-38: customers-sync-conflict-1338x753.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/customers-sync-conflict-1338x753.png)


<a id="ui-39"></a>

### UI-39 — Users list and deactivation dialog omit the login identity

**P2 · Design improvement · Users list and deactivate confirmation**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Users list and deactivation dialog omit the login identity**? It appears in **Users list and deactivate confirmation** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The list displays only Name, role, and status. Username is present in UserPreview and searchable but not visible. The deactivation dialog also identifies the account only by display name. Similar names are difficult to distinguish before a high-impact action.

Reproduce: Open users with a display name that could be shared by another account, then choose deactivate. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersViewModel.cs` and their direct callers/view models.

Done state: Show the username as secondary LTR identity in each user row and in deactivation confirmation. Keep the Arabic display name primary. Verify two accounts with the same display name and different usernames. Preserve existing account IDs, roles, authorization, and deactivation behavior.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-39: audit-users-list-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-users-list-1338.png)

![UI-39: audit-users-deactivate-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-users-deactivate-720.png)


<a id="ui-40"></a>

### UI-40 — Password validation is displayed under the user's name

**P2 · Defect · Create-user form / إضافة مستخدم**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Password validation is displayed under the user's name**? It appears in **Create-user form / إضافة مستخدم** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The combined error tells the user to enter name, username, role, and a password of at least 12 characters. It appears under the valid name because EditorError is bound only to that FormField. The actual invalid password field has no local error or prior length guidance.

Reproduce: Fill a valid display name and username, leave the temporary password empty or too short, and save. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersViewModel.cs` and their direct callers/view models.

Done state: Present input guidance and errors at the relevant fields, with a suitable form-level summary for authority failures. For the reproduced short-password case, identify the temporary password field without falsely marking the valid name. Preserve entered values and Rust-owned validation; keep any shell checks limited to existing input presentation behavior.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-40: audit-users-validation-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-users-validation-720.png)


<a id="ui-41"></a>

### UI-41 — Users refresh failure leaves normal-looking stale rows with no retry action

**P2 · Design improvement · Users list, load failure**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Users refresh failure leaves normal-looking stale rows with no retry action**? It appears in **Users list, load failure** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The list keeps its previous rows while a small red error appears above the filters. Nothing near the table marks the rows as last known data, and the view has no direct reload button despite asking the user to try again.

Reproduce: Load users successfully, then make the next list query fail. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersViewModel.cs` and their direct callers/view models.

Done state: Make the failed load and retained data state explicit, and add a clear retry using the existing LoadAsync path. Keep last known rows if useful, but label them accurately. Ensure action availability respects existing session and authority results. Do not infer that the stale account status is current.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-41: audit-users-unavailable-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-users-unavailable-1338.png)

![UI-41: audit-users-unavailable-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/audit-users-unavailable-720.png)


<a id="ui-42"></a>

### UI-42 — Print preview opens wider than the available viewport

**P2 · Defect · Saved quotation, order, and draft print preview**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Print preview opens wider than the available viewport**? It appears in **Saved quotation, order, and draft print preview** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The A4 page opens at a zoom that exceeds the viewport. The left header identifier is clipped and the document requires horizontal scrolling before the user can inspect it. The page does not initially fit the usable width.

Reproduce: Open a saved quotation preview at 720 × 560. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/PrintPreview.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/PrintPreview.xaml.cs` and their direct callers/view models.

Done state: Choose a suitable initial fit-width or fit-page mode after layout. Keep native zoom controls and allow the user to change zoom. Handle viewport resize without repeatedly overriding a user's chosen zoom. Verify quotation, order, and draft previews, including a multipage document. Preserve print authorization and pagination.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-42: saved-document-quotation-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/saved-document-quotation-720.png)

![UI-42: saved-document-order-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/saved-document-order-720.png)


<a id="ui-43"></a>

### UI-43 — Print viewer search and chrome are not fully Arabic

**P2 · Design improvement · Shared print preview**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Print viewer search and chrome are not fully Arabic**? It appears in **Shared print preview** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The embedded DocumentViewer displays the English placeholder Type text to find inside the Arabic-first screen. The built-in icon toolbar is visually separate from the application's Arabic Print/Back actions.

Reproduce: Inspect the bottom search bar of a saved-document preview. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/PrintPreview.xaml`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/PrintPreview.xaml.cs` and their direct callers/view models.

Done state: Localize the exposed preview search and commands using an appropriate native WPF solution. Give relevant icon controls Arabic accessible names and tooltips. Preserve native text selection, search, page navigation, zoom, and the existing intercepted print authorization path. Do not duplicate an unrestricted print route.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-43: saved-document-quotation-720.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/saved-document-quotation-720.png)

![UI-43: saved-document-quotation-1338.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/saved-document-quotation-1338.png)


<a id="ui-44"></a>

### UI-44 — Customer print documents need a stronger professional hierarchy

**P3 · Design improvement · Printed quotation, order, and draft documents**

[$frontend-design](C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md) Can you fix this UI issue: **Customer print documents need a stronger professional hierarchy**? It appears in **Printed quotation, order, and draft documents** in the Eitmad Windows app at `C:/Users/Hussein/Desktop/SWProjects/Eitmad`.

The document has a small repeated header, a large title, and a long right-aligned stack of metadata before the items table. Much of the upper-left page is unused. The result is readable but visually plain, with weak grouping between document identity, customer information, and commercial terms.

Reproduce: Inspect the saved quotation and order pages at full page size. Inspect the attached screenshots at full size. Start with `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/QuotationCustomerDocument.cs`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrderCustomerDocument.cs`, `C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/CustomerDocumentPages.cs` and their direct callers/view models.

Done state: Redesign the customer document layout using the existing brand and native FlowDocument/vector facilities. Create clear document/customer/terms groups and a balanced item table with aligned financial totals. Use only existing printable fields; do not invent legal text, addresses, or contact details. Preserve draft labeling, snapshots, Arabic shaping, LTR identifiers, pagination, repeated table headers, and page numbers. Check the existing four-page quotation after the redesign.

Keep the change within this UI concern and follow AGENTS.md. Use native WPF controls, the existing Eitmad visual system, Arabic labels, and LTR isolation for technical values. Keep Rust as the product authority. Build the affected shell and perform focused rendered checks of the changed state at default, minimum, and available large size. Check keyboard/focus and Arabic accessible names for changed interactions; check system colors/text scaling if styling changes. Report any unavailable verification.

Evidence to attach:

![UI-44: saved-quotation-1.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/saved-quotation-1.png)

![UI-44: saved-order-1.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/saved-order-1.png)

![UI-44: saved-quotation-multipage-2.png](C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-12/captures/saved-quotation-multipage-2.png)

## Reproduction and retained evidence

See [verification and capture instructions](./provenance/verification.md) for the exact commands, results, fixture limitations, and temporary capture-driver source. See [capture manifest](./capture-manifest.json) for all image dimensions and [contact-sheet index](./review-sheets/index.md) for the complete visual inventory. Original XPS documents are retained next to their rendered pages.

This is a dated review artifact. It does not replace [the repository ownership map](C:/Users/Hussein/Desktop/SWProjects/Eitmad/docs/developer/repository-layout.md), [the target architecture](C:/Users/Hussein/Desktop/SWProjects/Eitmad/docs/architecture/target-architecture.md), or [the glossary](C:/Users/Hussein/Desktop/SWProjects/Eitmad/docs/glossary.md). Recheck an issue against current code before applying its prompt.
