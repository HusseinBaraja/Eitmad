# Eitmad UI audit — fix prompts

55 individual fix prompts, with screenshots, for the current Windows app. The app UI was inspected one screen at a time. No gallery or computer-use tools were used. Product source was not changed.

## Audit scope and evidence

- Date: 14 October 2026. Both Manager and Receptionist routes were reviewed, including all 18 feature views, editor steps, dialogs, state variants, and print preview.
- Requested environment only: maximized windows on the current 1920 × 1080 physical display at 125% scaling. Windows reports 1536 × 864 DIP for the display and 1536 × 816 DIP for its work area. The maximized window measures 1550.4 × 830.4 DIP because its non-client border extends outside the work area. Main-window captures are 1938 × 1038 pixels. No smaller application-window checks were used.
- The current WPF XAML was rendered with existing synthetic Arabic fixtures and fake engine replies. These are actual WPF renders, not mockups. This is a presentation audit; it does not establish live engine, database, synchronization or commercial calculation correctness.
- Screenshots below are shown separately beside their issue. Some screens are scrolled to the affected section. Small calendar, swatch and tooltip images are overlays of a maximized window, not separate viewport tests.
- The black bottom strip in window captures comes from the render target/native window boundary. It is not an app defect. Native caption chrome is outside this capture method.
- Product action-menu and category-dropdown render targets were transparent/black. Their source was inspected, but their rendered popup layout was not verified. The category manager/editor and archive dialogs were rendered and reviewed. No defect is inferred from those black images.
- A high-contrast capture emitted by an existing fixture was excluded from this audit. The findings use the normal current theme at 125%.

[Screen/state coverage](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/COVERAGE.md>) records each feature and print page. [Capture review log](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/CAPTURE-REVIEW.md>) records individual inspection and byte-identical duplicates. [Capture geometry](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/capture-manifest.tsv>) is the size/scaling authority; legacy filenames and copied fixture log messages are not.

## How to use the prompts

Send one complete prompt, with its attached screenshot, to the fixing agent. Each prompt states its own scope, source, observed defect and done state. Shared defects have one prompt with all affected screens named. Apply the shared-header and subtext changes before judging spacing changes that depend on them.

The user's subtext requirement covers UI-001, UI-004, UI-006, UI-008, UI-010, UI-012, UI-018, UI-021, UI-022, UI-027, UI-029, UI-032, UI-035, UI-037 and UI-039. Remove heading descriptions and instructional paragraphs throughout those scopes. Keep concise real states/errors, required labels, input units, record descriptions, customer notes and essential confirmation consequences. Replace a rule that is needed to operate a control with a clear label or explicit control state; do not leave the rule hidden after deleting its paragraph.

## Issue index

| ID | Screen | Issue |
| --- | --- | --- |
| [UI-001](#ui-001) | Sign-in | Remove sign-in explanatory text |
| [UI-002](#ui-002) | Manager and Receptionist shell | Reduce the oversized shared shell header |
| [UI-003](#ui-003) | Manager dashboard and Receptionist Home | Make home activity and ready-order rows easy to scan |
| [UI-004](#ui-004) | Manager dashboard, Receptionist Home and Home search | Remove home explanatory footnotes |
| [UI-005](#ui-005) | Raw materials and other operational lists | Improve shared list density and filter proportions |
| [UI-006](#ui-006) | Raw materials list, create/edit material and reference dialogs | Remove raw-material list and editor subtext |
| [UI-007](#ui-007) | Create/edit raw material | Remove the empty stretched material editor card |
| [UI-008](#ui-008) | Add/edit measurement unit | Redesign unit conversion so labels replace paragraphs |
| [UI-009](#ui-009) | Reference, category, picker and confirmation dialogs | Give dialogs a consistent whole-shell modal appearance |
| [UI-010](#ui-010) | Part information, materials, review and material picker | Remove Part wizard explanatory text |
| [UI-011](#ui-011) | Part information | Keep Part wizard navigation fully visible |
| [UI-012](#ui-012) | Furniture list and all six editor steps | Remove Furniture wizard subtext |
| [UI-013](#ui-013) | Furniture information | Reduce the Furniture image placeholder's visual weight |
| [UI-014](#ui-014) | Furniture options | Make Furniture options a compact comparison |
| [UI-015](#ui-015) | Furniture pricing step | Make Furniture size pricing easier to compare |
| [UI-016](#ui-016) | Furniture review | Show below-cost confirmation only in the relevant review context |
| [UI-017](#ui-017) | Furniture review | Make the Furniture review summary compact |
| [UI-018](#ui-018) | Products list, editor, categories and archive dialogs | Remove ready-made Product subtext |
| [UI-019](#ui-019) | Products — single purchase cost and variant purchase costs | Add clear currency to Product variant cost entry |
| [UI-020](#ui-020) | Pricing list | Restore the visible Pricing category selection after loading |
| [UI-021](#ui-021) | Pricing list and price editor | Replace Pricing prose with compact state and actions |
| [UI-022](#ui-022) | Reception Products catalog, Furniture selection and Product selection | Remove sales catalog and selection subtext |
| [UI-023](#ui-023) | Furniture and ready-made Product selection | Redesign the sales selection workspace |
| [UI-024](#ui-024) | New/edit quotation review | Use the current workflow name in the quotation window header |
| [UI-025](#ui-025) | Current quotation and saved draft editor | Make quotation lines compact and aligned |
| [UI-026](#ui-026) | Current quotation, draft and issue review | Balance quotation customer and totals sections |
| [UI-027](#ui-027) | Current quotation, customer selection and new customer form | Remove quotation editor instructional subtext |
| [UI-028](#ui-028) | Inline new customer within current quotation | Hide the New customer action while creating that customer |
| [UI-029](#ui-029) | Customer details, edit dialog and sync-conflict state | Remove Customer detail and editor technical subtext |
| [UI-030](#ui-030) | Customer details | Group Customer contact details coherently |
| [UI-031](#ui-031) | Reception Quotations and Orders | Remove duplicate headers in Reception list/detail routes |
| [UI-032](#ui-032) | Quotation lists, detail, issue/accept/convert/cancel and discount review | Remove Quotation list/detail explanatory subtext |
| [UI-033](#ui-033) | Issued quotation details | Put quotation information before oversized action forms |
| [UI-034](#ui-034) | Manager discount approval | Keep discount decision evidence beside the decision |
| [UI-035](#ui-035) | Order lists, details, original quotation, delivery and cancellation | Remove Order explanatory subtext and duplicate state text |
| [UI-036](#ui-036) | Order detail | Hide the idle pending-operation retry control |
| [UI-037](#ui-037) | Work-order list, detail and production actions | Remove Work-order subtext |
| [UI-038](#ui-038) | Work-order details | Fix Work-order section and table alignment |
| [UI-039](#ui-039) | Users list, create/edit and deactivation | Remove Users explanatory subtext |
| [UI-040](#ui-040) | Add/edit User | Unify Users editor page hierarchy |
| [UI-041](#ui-041) | Account switch and user deactivation confirmations and quotation conversion confirmation | Use consistent confirmation dialog typography |
| [UI-042](#ui-042) | Saved quotation/order/draft and multi-page print preview | Open print previews with the complete page visible |
| [UI-043](#ui-043) | Furniture — size editor | Make allowed Furniture options explicit |
| [UI-044](#ui-044) | Print preview — multi-page quotation and order | Keep each printed line together across pages |
| [UI-045](#ui-045) | Furniture — handle editor | Label Furniture option price adjustments with their unit |
| [UI-046](#ui-046) | Furniture — add part dialog | Show an explicit empty state in the Furniture part picker |
| [UI-047](#ui-047) | Products — create/edit, with and without variants | Use a compact Product editor hierarchy |
| [UI-048](#ui-048) | Receptionist — Products sales catalog | Make the sales catalog useful at full-screen width |
| [UI-049](#ui-049) | Work Orders — list and detail | Correct mixed-direction delivery dates |
| [UI-050](#ui-050) | Receptionist — ready Order detail and delivery | Align the Order delivery form with the record |
| [UI-051](#ui-051) | Sales Furniture selection — custom dimensions | Place custom-dimension validation beside the field |
| [UI-052](#ui-052) | Raw materials — edit validation | Identify the actual raw-material field error |
| [UI-053](#ui-053) | Furniture — list category filter | Restore the visible Furniture category filter after loading |
| [UI-054](#ui-054) | Parts — raw material picker | Add a visible close action to the Part material picker |
| [UI-055](#ui-055) | Orders — mixed Furniture and ready-made Product detail | Make Order line cards compact and scannable |

<a id="ui-001"></a>

## UI-001 — Remove sign-in explanatory text

**Page:** Sign-in.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove sign-in explanatory text**? It appears in **Sign-in** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The line under the sign-in heading explains entering account details. The image panel also has a marketing sentence under its slogan. Both add copy to a self-evident task.
>
> **Source to inspect:** [Features/Authentication/SignInView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Authentication/SignInView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove explanatory subtext under the form heading and image slogan. Rebalance margins around the logo, heading and inputs. Keep field labels, the brand lockup, and actual error/progress messages.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [sign-in-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/sign-in-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** sign-in-1920x1080.

![UI-001 — Remove sign-in explanatory text, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/sign-in-1920x1080.png>)

<a id="ui-002"></a>

## UI-002 — Reduce the oversized shared shell header

**Page:** Manager and Receptionist shell.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Reduce the oversized shared shell header**? It appears in **Manager and Receptionist shell** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The shared title/search/action band is approximately 135 DIP high, while its controls use only a narrow central strip. The large blank band pushes every home workflow down.
>
> **Source to inspect:** [Controls/ShellTitleBar.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/ShellTitleBar.xaml>), [Resources/OperationsControls.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsControls.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the title bar as a compact native toolbar. Align the title, search and actions on a deliberate baseline, reduce empty vertical padding, preserve comfortable targets and the visible Arabic account action.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [ui14-reception-top-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui14-reception-top-1920.png>). Inspect each image separately.

**Screenshot 1:** ui14-reception-top-1920.

![UI-002 — Reduce the oversized shared shell header, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui14-reception-top-1920.png>)

<a id="ui-003"></a>

## UI-003 — Make home activity and ready-order rows easy to scan

**Page:** Manager dashboard and Receptionist Home.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Make home activity and ready-order rows easy to scan**? It appears in **Manager dashboard and Receptionist Home** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Ready-order and activity cards span almost the entire workspace. Customer/title/status are at the far right while document number and date are at the far left, with a wide empty gap. The activity card is tall for one record and visually heavier than the actual daily work.
>
> **Source to inspect:** [Features/Home/HomeView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign these rows with the best restrained native layout. Group each record's identity, customer, status and date so it can be scanned as one item; use compact rows and clear hierarchy. Keep identifiers isolated LTR and preserve real counts and open actions.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [audit-home-manager-start.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-home-manager-start.png>). Inspect each image separately.

**Screenshot 1:** audit-home-manager-start.

![UI-003 — Make home activity and ready-order rows easy to scan, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-home-manager-start.png>)

<a id="ui-004"></a>

## UI-004 — Remove home explanatory footnotes

**Page:** Manager dashboard, Receptionist Home and Home search.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove home explanatory footnotes**? It appears in **Manager dashboard, Receptionist Home and Home search** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The home includes a data-scope explanation, an eight-record explanation and a long list of absent capabilities. Home search repeats the entered term below the heading with a separate introductory label. This reads as documentation inside daily work. Partial, denied and unavailable states also repeat long explanations in several cards; the partial reception state illustrates this repetition.
>
> **Source to inspect:** [Features/Home/HomeView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeView.xaml>), [Features/Home/HomeViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Home/HomeViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove instructional and repeated subtext. Retain concise truthful data availability, stale/denied/partial states and actionable retry controls. Represent a meaningful result limit or status compactly, and keep the active search term in a clear search control rather than a paragraph.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached images:** [ui13-home-search-manager-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui13-home-search-manager-1920.png>); [ui12-home-reception-partial-denied-1920-ready.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui12-home-reception-partial-denied-1920-ready.png>). Inspect each image separately.

**Screenshot 1:** ui13-home-search-manager-1920.

![UI-004 — Remove home explanatory footnotes, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui13-home-search-manager-1920.png>)

**Screenshot 2:** ui12-home-reception-partial-denied-1920-ready.

![UI-004 — Remove home explanatory footnotes, screenshot 2](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui12-home-reception-partial-denied-1920-ready.png>)

<a id="ui-005"></a>

## UI-005 — Improve shared list density and filter proportions

**Page:** Raw materials and other operational lists.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Improve shared list density and filter proportions**? It appears in **Raw materials and other operational lists** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** A single material appears inside a large table body with a substantial empty area below the row. All three filters stretch to equal oversized widths, including the small status choice. The sparse list occupies several hundred pixels but presents very little information.
>
> **Source to inspect:** [Controls/OperationsTable.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/OperationsTable.cs>), [Resources/OperationsControls.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsControls.xaml>), [Features/RawMaterials/RawMaterialsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Use a deliberate compact list layout, proportionate filter widths, and content-driven table body sizing for small lists. Preserve useful rows at larger record counts, counts/pagination and keyboard/native scrolling. Verify each list that uses the shared table; do not shrink text to hide overflow.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [material-durable-list-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-list-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** material-durable-list-1920x1080.

![UI-005 — Improve shared list density and filter proportions, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-list-1920x1080.png>)

<a id="ui-006"></a>

## UI-006 — Remove raw-material list and editor subtext

**Page:** Raw materials list, create/edit material and reference dialogs.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove raw-material list and editor subtext**? It appears in **Raw materials list, create/edit material and reference dialogs** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The page header describes the list; the editor repeats 'raw material data' below its title and asks users to review before saving. Material rows repeat their record type. Reference dialogs repeat the same generic subtitle.
>
> **Source to inspect:** [Features/RawMaterials/RawMaterialsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsView.xaml>), [Controls/PageHeader.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/PageHeader.cs>), [Controls/FormField.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/FormField.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove all explanatory subtitles and redundant record-type labels in this capability. Keep meaningful category/unit labels, actual validation, saved data and errors. Close the spacing left by removed text.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [material-durable-editor-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-editor-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** material-durable-editor-1920x1080.

![UI-006 — Remove raw-material list and editor subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-editor-1920x1080.png>)

<a id="ui-007"></a>

## UI-007 — Remove the empty stretched material editor card

**Page:** Create/edit raw material.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove the empty stretched material editor card**? It appears in **Create/edit raw material** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The editor has only a name, category, unit and cost, but its white panel fills nearly the entire available height. A large blank area remains under the cost field.
>
> **Source to inspect:** [Features/RawMaterials/RawMaterialsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the form with content-sized sections and balanced width. Keep Save/Cancel in a stable native footer and place the short form near the reading origin; avoid a large empty white container.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [material-durable-editor-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-editor-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** material-durable-editor-1920x1080.

![UI-007 — Remove the empty stretched material editor card, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-editor-1920x1080.png>)

<a id="ui-008"></a>

## UI-008 — Redesign unit conversion so labels replace paragraphs

**Page:** Add/edit measurement unit.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Redesign unit conversion so labels replace paragraphs**? It appears in **Add/edit measurement unit** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The unit dialog has a generic subtitle, an abbreviation example, a mathematical conversion explanation, raw numerator/denominator fields and another long example. The form relies on paragraphs to explain a central relationship.
>
> **Source to inspect:** [Features/RawMaterials/RawMaterialsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsView.xaml>), [Features/RawMaterials/RawMaterialsViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign with the clearest Arabic field labels and a compact explicit conversion relationship, so the user can enter an exact factor without instructional subtext. Keep exact numerator/denominator semantics and Rust validation. Do not silently round or alter saved units.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [material-durable-unit-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-unit-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** material-durable-unit-1920x1080.

![UI-008 — Redesign unit conversion so labels replace paragraphs, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-unit-1920x1080.png>)

<a id="ui-009"></a>

## UI-009 — Give dialogs a consistent whole-shell modal appearance

**Page:** Reference, category, picker and confirmation dialogs.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Give dialogs a consistent whole-shell modal appearance**? It appears in **Reference, category, picker and confirmation dialogs** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The material dialog dims the feature workspace but leaves the sidebar bright and visually active. The overlay ends exactly at the content/sidebar boundary. Many feature dialogs use this same partial treatment.
>
> **Source to inspect:** [Controls/DialogHost.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/DialogHost.cs>), [Resources/OperationsControls.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsControls.xaml>), [MainWindow.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/MainWindow.xaml>), [Features/Reception/ReceptionistHomeView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ReceptionistHomeView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Make modal coverage and interaction scope consistent across the shell. The complete blocked background, including navigation, must have one deliberate scrim; preserve focus containment, Escape/cancel and return focus. Confirm the actual navigation/input behavior instead of assuming it from the image.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [material-durable-unit-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-unit-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** material-durable-unit-1920x1080.

![UI-009 — Give dialogs a consistent whole-shell modal appearance, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-unit-1920x1080.png>)

<a id="ui-010"></a>

## UI-010 — Remove Part wizard explanatory text

**Page:** Part information, materials, review and material picker.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove Part wizard explanatory text**? It appears in **Part information, materials, review and material picker** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The header narrates the wizard order. Each step repeats a paragraph telling the user what to enter or which navigation button to use. The materials cost card also explains when calculation happens.
>
> **Source to inspect:** [Features/Parts/PartsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Parts/PartsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove explanatory subtext from all Part steps and picker surfaces. Keep step names, field labels, current calculation availability, returned costs and actual errors. Tighten the spacing after removal.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [part-information-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/part-information-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** part-information-1920x1080.

![UI-010 — Remove Part wizard explanatory text, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/part-information-1920x1080.png>)

<a id="ui-011"></a>

## UI-011 — Keep Part wizard navigation fully visible

**Page:** Part information.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Keep Part wizard navigation fully visible**? It appears in **Part information** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** At maximized 125%, the Next and Cancel buttons are partly below the visible client viewport, although this step contains only name, category, category creation and notes. The form uses excessive spacing and a permanently expanded category creation row.
>
> **Source to inspect:** [Features/Parts/PartsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Parts/PartsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Keep all step navigation in a stable footer outside the scrolling form. Redesign spacing and category creation so the short information form is balanced and its actions are fully visible at this exact maximized size. Preserve entered values and focus behavior.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [part-information-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/part-information-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** part-information-1920x1080.

![UI-011 — Keep Part wizard navigation fully visible, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/part-information-1920x1080.png>)

<a id="ui-012"></a>

## UI-012 — Remove Furniture wizard subtext

**Page:** Furniture list and all six editor steps.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove Furniture wizard subtext**? It appears in **Furniture list and all six editor steps** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Every step header narrates the step. Information, image, options, pricing and review sections add more instructional paragraphs. The pricing step adds a general cost explanation. The result is repeated explanation across an already named six-step flow.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>), [Features/Furniture/FurnitureViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove instructional subtext across information, parts, sizes, options, pricing, review and their dialogs. Keep actual descriptions/notes entered by users, dimensions, price labels, scope/preview markers and real error or availability states. Remove the empty margins left behind.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [furniture-information-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-information-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** furniture-information-1920x1080.

![UI-012 — Remove Furniture wizard subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-information-1920x1080.png>)

<a id="ui-013"></a>

## UI-013 — Reduce the Furniture image placeholder's visual weight

**Page:** Furniture information.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Reduce the Furniture image placeholder's visual weight**? It appears in **Furniture information** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The empty image panel takes about half of the form width and is very tall, with a small illustration in a large beige area. Internal notes and image actions require scrolling while the placeholder dominates the first viewport.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign with a compact image preview/upload group and balanced primary fields. Keep existing native image selection/removal behavior and real thumbnails. The form's important fields should dominate, and the empty image state should use a short label with no explanatory sentence.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [furniture-information-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-information-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** furniture-information-1920x1080.

![UI-013 — Reduce the Furniture image placeholder's visual weight, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-information-1920x1080.png>)

<a id="ui-014"></a>

## UI-014 — Make Furniture options a compact comparison

**Page:** Furniture options.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Make Furniture options a compact comparison**? It appears in **Furniture options** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Each simple color/handle option becomes a very wide standalone card. Name/visual, adjustment, state and action are separated by large empty gaps; four options nearly fill the viewport.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign using the best compact native comparison layout, with aligned option identity, swatch/handle visual, adjustment, state and action. Preserve separate color/handle groups, inactive options and price values. Several realistic options should be easy to compare without giant cards.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [ui22-options-top-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui22-options-top-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** ui22-options-top-1920x1080.

![UI-014 — Make Furniture options a compact comparison, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui22-options-top-1920x1080.png>)

<a id="ui-015"></a>

## UI-015 — Make Furniture size pricing easier to compare

**Page:** Furniture pricing step.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Make Furniture size pricing easier to compare**? It appears in **Furniture pricing step** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Every size repeats a large header and three broad cost/price/margin blocks. Only one complete size and part of a second are visible. Most block area is empty, so the manager must scroll to compare prices.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign with the best native layout for comparing size identity, dimensions, cost, editable price and returned margin. Keep long names readable, clear currency and Rust-owned review/validation. Reduce repeated labels and vertical card size without shrinking text.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [ui21-pricing-top-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui21-pricing-top-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** ui21-pricing-top-1920x1080.

![UI-015 — Make Furniture size pricing easier to compare, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui21-pricing-top-1920x1080.png>)

<a id="ui-016"></a>

## UI-016 — Show below-cost confirmation only in the relevant review context

**Page:** Furniture review.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Show below-cost confirmation only in the relevant review context**? It appears in **Furniture review** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** A below-cost acknowledgement sits alone above the review title even when the displayed size has a selling price of 200,000 and a cost of 18,900. The XAML shows this checkbox without a visibility condition. It looks unrelated to the reviewed record and is easy to miss.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml:846](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml:846>), [Features/Furniture/FurnitureViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Show and group the acknowledgement with the actual Rust-reported below-cost warning/affected sizes when it is required. Omit it from ordinary positive-margin reviews. Use the existing accessible checkbox style, preserve Rust authorization/validation and retain the confirmation requirement for a real loss.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [furniture-review-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-review-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** furniture-review-1920x1080.

![UI-016 — Show below-cost confirmation only in the relevant review context, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-review-1920x1080.png>)

<a id="ui-017"></a>

## UI-017 — Make the Furniture review summary compact

**Page:** Furniture review.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Make the Furniture review summary compact**? It appears in **Furniture review** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The product summary is a very tall card with identity at the right, category isolated near the center-left, and a large placeholder at the left. Only the top of the sizes/prices section is visible at full screen.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the review as a coherent compact record summary followed by aligned review sections. Group name, category, actual description and thumbnail. Show more substantive review data before scrolling and keep the save footer stable.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [furniture-review-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-review-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** furniture-review-1920x1080.

![UI-017 — Make the Furniture review summary compact, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/furniture-review-1920x1080.png>)

<a id="ui-018"></a>

## UI-018 — Remove ready-made Product subtext

**Page:** Products list, editor, categories and archive dialogs.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove ready-made Product subtext**? It appears in **Products list, editor, categories and archive dialogs** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The list explains local storage/pricing. The editor explains basic information and supplier variants, and repeats that selling prices are set elsewhere. Category management explains that the same categories occur in two places.
>
> **Source to inspect:** [Features/Products/ProductsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Products/ProductsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove all explanatory subtitles and instructional help text throughout Products. Retain real product descriptions, supplier variant identities, currency, actual errors and concise durable/availability state when relevant. Rebalance the remaining form.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [products-variants-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/products-variants-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** products-variants-1920x1080.

![UI-018 — Remove ready-made Product subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/products-variants-1920x1080.png>)

<a id="ui-019"></a>

## UI-019 — Add clear currency to Product variant cost entry

**Page:** Products — single purchase cost and variant purchase costs.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Add clear currency to Product variant cost entry**? It appears in **Products — single purchase cost and variant purchase costs** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The purchase-cost column contains plain values such as 55000 and 80000 with no currency in the column heading or input. Other money displays use ر.ي, so this form has a weaker unit cue. The product without variants also leaves its unit in a sentence beneath the cost input.
>
> **Source to inspect:** [Features/Products/ProductsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Products/ProductsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Identify YER with the cost label or a stable native input suffix. Isolate numeric entry LTR and preserve exact whole-YER input and Rust validation. Avoid explanatory helper paragraphs. Apply the same visible unit to the single purchase-cost field; its currency currently appears only in a helper sentence that must be removed.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached images:** [products-variants-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/products-variants-1920x1080.png>); [audit-product-without-variants.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-product-without-variants.png>). Inspect each image separately.

**Screenshot 1:** products-variants-1920x1080.

![UI-019 — Add clear currency to Product variant cost entry, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/products-variants-1920x1080.png>)

**Screenshot 2:** audit-product-without-variants.

![UI-019 — Add clear currency to Product variant cost entry, screenshot 2](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-product-without-variants.png>)

<a id="ui-020"></a>

## UI-020 — Restore the visible Pricing category selection after loading

**Page:** Pricing list.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Restore the visible Pricing category selection after loading**? It appears in **Pricing list** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The category filter renders as an empty field after data loads, although the model has AllCategories and the table shows prices. ApplyDurableData clears/repopulates Categories and resets SelectedCategory only when its string is absent; the ComboBox selection can remain blank.
>
> **Source to inspect:** [Features/Pricing/PricingView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Pricing/PricingView.xaml>), [Features/Pricing/PricingViewModel.cs:71](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Pricing/PricingViewModel.cs:71>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Ensure the visible selection and model agree after initial load and reload. Display كل الفئات for the default and preserve valid user selections. Verify the actual WPF ComboBox, category filtering and reload behavior with a focused regression check.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [pricing-1550x830-125percent.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/pricing-1550x830-125percent.png>). Inspect each image separately.

**Screenshot 1:** pricing-1550x830-125percent.

![UI-020 — Restore the visible Pricing category selection after loading, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/pricing-1550x830-125percent.png>)

<a id="ui-021"></a>

## UI-021 — Replace Pricing prose with compact state and actions

**Page:** Pricing list and price editor.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Replace Pricing prose with compact state and actions**? It appears in **Pricing list and price editor** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The Pricing heading carries a server-availability sentence. A long synchronization paragraph stretches above the filters and tells users to correct definitions and save another revision. The editor adds another server-confirmation explanation.
>
> **Source to inspect:** [Features/Pricing/PricingView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Pricing/PricingView.xaml>), [Features/Pricing/PricingViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Pricing/PricingViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove explanatory subtext. Retain essential truth as concise server/publication/sync states next to the relevant action or affected record, with an actionable review/retry route. Do not hide failed publication, rejected definitions, or stale prices. Tighten the layout and keep real below-cost warnings.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [pricing-1550x830-125percent.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/pricing-1550x830-125percent.png>). Inspect each image separately.

**Screenshot 1:** pricing-1550x830-125percent.

![UI-021 — Replace Pricing prose with compact state and actions, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/pricing-1550x830-125percent.png>)

<a id="ui-022"></a>

## UI-022 — Remove sales catalog and selection subtext

**Page:** Reception Products catalog, Furniture selection and Product selection.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove sales catalog and selection subtext**? It appears in **Reception Products catalog, Furniture selection and Product selection** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The catalog/selection surfaces add paragraphs about connection, confirmed catalogs, local selection, saving drafts and fixed dimensions. Similar text appears again under the selection footer. Users already have named options, amounts and an Add action.
>
> **Source to inspect:** [Features/Reception/SalesCatalogView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesCatalogView.xaml>), [Features/Reception/FurnitureSelectionView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/FurnitureSelectionView.xaml>), [Features/Reception/ProductSelectionView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ProductSelectionView.xaml>), [Features/Reception/SalesText.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesText.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove instructional subtext and redundant record-source labels. Keep concise, truthful stale/offline/unavailable states, actual variant identities, limits and returned amounts. Integrate meaningful dimension limits into labelled controls. Close the spacing left by removal.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [ui28-furniture-reception-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui28-furniture-reception-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** ui28-furniture-reception-1920x1080.

![UI-022 — Remove sales catalog and selection subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui28-furniture-reception-1920x1080.png>)

<a id="ui-023"></a>

## UI-023 — Redesign the sales selection workspace

**Page:** Furniture and ready-made Product selection.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Redesign the sales selection workspace**? It appears in **Furniture and ready-made Product selection** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** A single small option group is pushed to the far right of an almost empty full-width canvas. Furniture options stack down a narrow area while dimensions span very long fields. Quantity, total and Add are separated across the entire footer. Price-breakdown labels are on the far right while their amounts are on the far left, so a simple calculation also requires scanning the complete screen width.
>
> **Source to inspect:** [Features/Reception/FurnitureSelectionView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/FurnitureSelectionView.xaml>), [Features/Reception/ProductSelectionView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ProductSelectionView.xaml>), [Resources/SalesSelectionStyles.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/SalesSelectionStyles.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign using the best restrained native desktop layout. Group item identity, options and quantity with a compact purchase summary; use available width to compare options without a large empty canvas. Keep the existing scrolling/selection-footer behavior and all Arabic keyboard/accessible names.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached images:** [ui28-product-reception-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui28-product-reception-1920x1080.png>); [sales-furniture-total-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/sales-furniture-total-1920.png>). Inspect each image separately.

**Screenshot 1:** ui28-product-reception-1920x1080.

![UI-023 — Redesign the sales selection workspace, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui28-product-reception-1920x1080.png>)

**Screenshot 2:** sales-furniture-total-1920.

![UI-023 — Redesign the sales selection workspace, screenshot 2](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/sales-furniture-total-1920.png>)

<a id="ui-024"></a>

## UI-024 — Use the current workflow name in the quotation window header

**Page:** New/edit quotation review.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Use the current workflow name in the quotation window header**? It appears in **New/edit quotation review** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The standalone quotation editor still says Products in the large page header while the content says Quotation and New quotation. The header action also repeats the quotation item count, creating conflicting hierarchy.
>
> **Source to inspect:** [Features/Reception/SalesCatalogView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesCatalogView.xaml>), [Features/Reception/SalesCatalogView.xaml.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesCatalogView.xaml.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Make the title reflect browsing, item selection or quotation review as appropriate. Use one clear current task title with the relevant navigation/action. Preserve the item count where it helps and remove repeated headings.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [quotation-draft-lines-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-draft-lines-1920.png>). Inspect each image separately.

**Screenshot 1:** quotation-draft-lines-1920.

![UI-024 — Use the current workflow name in the quotation window header, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-draft-lines-1920.png>)

<a id="ui-025"></a>

## UI-025 — Make quotation lines compact and aligned

**Page:** Current quotation and saved draft editor.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Make quotation lines compact and aligned**? It appears in **Current quotation and saved draft editor** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Each line uses a tall region, a large title and total, three separate edit/duplicate/remove buttons and a thumbnail, clustered only on the right. Two lines consume most of the first viewport while the left half is empty.
>
> **Source to inspect:** [Features/Reception/CurrentQuotationView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotationView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign as a compact native line editor with aligned identity/options, quantity, unit price and total. Group row actions clearly. Keep real saved descriptions/options and Rust totals. Several lines must be readable in the first viewport without shrinking text or making actions ambiguous.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [quotation-draft-lines-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-draft-lines-1920.png>). Inspect each image separately.

**Screenshot 1:** quotation-draft-lines-1920.

![UI-025 — Make quotation lines compact and aligned, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-draft-lines-1920.png>)

<a id="ui-026"></a>

## UI-026 — Balance quotation customer and totals sections

**Page:** Current quotation, draft and issue review.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Balance quotation customer and totals sections**? It appears in **Current quotation, draft and issue review** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The customer form and totals stack in narrow right-aligned panels while most of the left side is empty. The user must scroll far to reach totals and save/issue actions. Explanatory and unsaved-state sentences accumulate beneath the actions.
>
> **Source to inspect:** [Features/Reception/CurrentQuotationView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotationView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the desktop workspace so customer selection, totals and the relevant save/issue action have a clear relationship and use the available width. Keep essential unsaved/pending/failure states concise and visible. Preserve read-only saved customer data and every Rust-owned availability rule.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [quotation-evaluation-summary-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-evaluation-summary-1920.png>). Inspect each image separately.

**Screenshot 1:** quotation-evaluation-summary-1920.

![UI-026 — Balance quotation customer and totals sections, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-evaluation-summary-1920.png>)

<a id="ui-027"></a>

## UI-027 — Remove quotation editor instructional subtext

**Page:** Current quotation, customer selection and new customer form.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove quotation editor instructional subtext**? It appears in **Current quotation, customer selection and new customer form** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The editor explains which fields are required, how to search/select/save a customer, where notes come from, when to save/issue, and why the draft is local. Much of this repeats labels or visible buttons.
>
> **Source to inspect:** [Features/Reception/CurrentQuotationView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotationView.xaml>), [Features/Reception/CurrentQuotation.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotation.cs>), [Features/Reception/SalesCatalogViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesCatalogViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove explanatory subtext from all customer modes, quotation lines, totals and footer. Use required markers and precise Arabic labels. Keep actionable validation, current unsaved/pending/conflict state and real server confirmation without explanatory paragraphs.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [ui30-customer-new-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui30-customer-new-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** ui30-customer-new-1920x1080.

![UI-027 — Remove quotation editor instructional subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui30-customer-new-1920x1080.png>)

<a id="ui-028"></a>

## UI-028 — Hide the New customer action while creating that customer

**Page:** Inline new customer within current quotation.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Hide the New customer action while creating that customer**? It appears in **Inline new customer within current quotation** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The form is already creating a new customer but still shows another New customer button inside it. The same generic customer panel and source-of-notes paragraph are reused, so the creation state is unclear.
>
> **Source to inspect:** [Features/Reception/CurrentQuotationView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/CurrentQuotationView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Use one clear creation state with labelled fields and Save/Cancel. Hide the redundant New customer trigger during creation and restore search/selection after cancel or a confirmed save. Keep existing typed data and authority behavior.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [ui30-customer-new-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui30-customer-new-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** ui30-customer-new-1920x1080.

![UI-028 — Hide the New customer action while creating that customer, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui30-customer-new-1920x1080.png>)

<a id="ui-029"></a>

## UI-029 — Remove Customer detail and editor technical subtext

**Page:** Customer details, edit dialog and sync-conflict state.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove Customer detail and editor technical subtext**? It appears in **Customer details, edit dialog and sync-conflict state** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The page and dialog explain that data is stored/validated in the engine. The conflict state includes an extra sentence explaining that the user can review the tables below or return.
>
> **Source to inspect:** [Features/Customers/CustomerDetailView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerDetailView.xaml>), [Features/Customers/CustomerEditorView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerEditorView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove implementation and navigation explanations. Keep actual contact fields, user notes, history, concise sync state and meaningful conflict/permission errors. Preserve clear inability to edit unresolved conflicts.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached images:** [customers-editor-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/customers-editor-1920x1080.png>); [ui38-conflict-OrdersTable-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui38-conflict-OrdersTable-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** customers-editor-1920x1080.

![UI-029 — Remove Customer detail and editor technical subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/customers-editor-1920x1080.png>)

**Screenshot 2:** ui38-conflict-OrdersTable-1920x1080.

![UI-029 — Remove Customer detail and editor technical subtext, screenshot 2](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui38-conflict-OrdersTable-1920x1080.png>)

<a id="ui-030"></a>

## UI-030 — Group Customer contact details coherently

**Page:** Customer details.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Group Customer contact details coherently**? It appears in **Customer details** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The customer card spans the whole window; phone sits on the far right while address is near the middle, and Edit is at the far left. The identity, sync state and contact values are spread across a large empty surface.
>
> **Source to inspect:** [Features/Customers/CustomerDetailView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Customers/CustomerDetailView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the contact summary with a compact, deliberate reading order for identity, sync state, phone, address, notes and Edit. Keep history tables and all real customer data. Use isolated LTR phone values without moving the surrounding RTL labels.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [customers-detail-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/customers-detail-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** customers-detail-1920x1080.

![UI-030 — Group Customer contact details coherently, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/customers-detail-1920x1080.png>)

<a id="ui-031"></a>

## UI-031 — Remove duplicate headers in Reception list/detail routes

**Page:** Reception Quotations and Orders.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove duplicate headers in Reception list/detail routes**? It appears in **Reception Quotations and Orders** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The reception shell title bar says Quotations, then a second tall page header repeats Quotations directly below it. Orders has the same duplicate structure. These two bands use about one-third of the usable screen before the filters.
>
> **Source to inspect:** [Features/Reception/ReceptionistHomeView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/ReceptionistHomeView.xaml>), [Features/Quotations/QuotationsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsView.xaml>), [Features/Orders/OrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrdersView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Give each hosted page one clear title and a compact place for its actions. Keep the shared search/account controls and the feature's task/navigation actions. Remove duplicate titles and redundant subtitle space in list and detail modes.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [quotation-list.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-list.png>). Inspect each image separately.

**Screenshot 1:** quotation-list.

![UI-031 — Remove duplicate headers in Reception list/detail routes, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-list.png>)

<a id="ui-032"></a>

## UI-032 — Remove Quotation list/detail explanatory subtext

**Page:** Quotation lists, detail, issue/accept/convert/cancel and discount review.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove Quotation list/detail explanatory subtext**? It appears in **Quotation lists, detail, issue/accept/convert/cancel and discount review** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Detail headings explain reviewing lines/totals. Action cards narrate issuing, reissuing and canceling. Approval blocks tell the manager to review before deciding and explain server confirmation. List and saved draft screens have more subtitles.
>
> **Source to inspect:** [Features/Quotations/QuotationsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsView.xaml>), [Features/Quotations/QuotationsViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove explanatory subtext from every quotation state and action surface. Keep accurate draft/issued/expired/converted/approval status, validity, actual rejection/cancellation reasons, warnings and unavailable actions. Preview-only fixtures must still be clearly labelled with a compact marker.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [quotation-issued-manager-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-issued-manager-1920.png>). Inspect each image separately.

**Screenshot 1:** quotation-issued-manager-1920.

![UI-032 — Remove Quotation list/detail explanatory subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-issued-manager-1920.png>)

<a id="ui-033"></a>

## UI-033 — Put quotation information before oversized action forms

**Page:** Issued quotation details.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Put quotation information before oversized action forms**? It appears in **Issued quotation details** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The first viewport is dominated by a full-width issue/print card and a full-width cancellation form. The quotation's actual items and financial summary are below them, so a user opening details sees action forms before the commercial record.
>
> **Source to inspect:** [Features/Quotations/QuotationsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the detail hierarchy: compact identity/state, readable items and totals, then appropriate task actions. Keep frequent actions clear and place cancellation in a restrained deliberate area. Preserve cancellation reasons, confirmation and all Rust-owned status rules.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [quotation-issued-manager-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-issued-manager-1920.png>). Inspect each image separately.

**Screenshot 1:** quotation-issued-manager-1920.

![UI-033 — Put quotation information before oversized action forms, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-issued-manager-1920.png>)

<a id="ui-034"></a>

## UI-034 — Keep discount decision evidence beside the decision

**Page:** Manager discount approval.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Keep discount decision evidence beside the decision**? It appears in **Manager discount approval** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The approval section fills most of the viewport with empty width, repeated headings and a very wide rejection textarea. Approve/Reject are visible while the actual quotation lines/totals are farther down.
>
> **Source to inspect:** [Features/Quotations/QuotationsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the approval workspace so requested percentage/value, relevant quotation summary and Approve/Reject can be understood together. Make rejection entry proportionate and show it when needed. Keep required reason, pending authorization, server-confirmed results and clear focus.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [discount-review-manager-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/discount-review-manager-1920.png>). Inspect each image separately.

**Screenshot 1:** discount-review-manager-1920.

![UI-034 — Keep discount decision evidence beside the decision, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/discount-review-manager-1920.png>)

<a id="ui-035"></a>

## UI-035 — Remove Order explanatory subtext and duplicate state text

**Page:** Order lists, details, original quotation, delivery and cancellation.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove Order explanatory subtext and duplicate state text**? It appears in **Order lists, details, original quotation, delivery and cancellation** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Order details explain that data is server-confirmed and prices/customer data are saved as accepted. The ready state repeats its badge in a green banner. Cancellation and delivery add explanatory copy, and the list adds a subtitle.
>
> **Source to inspect:** [Features/Orders/OrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrdersView.xaml>), [Features/Orders/OrdersViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrdersViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove instructional subtext and redundant repeated state text. Retain compact current state, customer/order identifiers, actual operational notes, authorization/availability errors and meaningful delivery/cancellation confirmation consequences.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [order-confirmed-manager-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/order-confirmed-manager-1920.png>). Inspect each image separately.

**Screenshot 1:** order-confirmed-manager-1920.

![UI-035 — Remove Order explanatory subtext and duplicate state text, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/order-confirmed-manager-1920.png>)

<a id="ui-036"></a>

## UI-036 — Hide the idle pending-operation retry control

**Page:** Order detail.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Hide the idle pending-operation retry control**? It appears in **Order detail** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** A disabled Confirm pending operation button is permanently displayed between notes and cancellation, even when no operation is pending. XAML binds only IsEnabled to CanRetry at this location. It adds an unexplained recovery control to an ordinary order.
>
> **Source to inspect:** [Features/Orders/OrdersView.xaml:443](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrdersView.xaml:443>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Show the retry/confirmation action only for the corresponding real unresolved-operation state. Group it with concise status/failure feedback and use a clear Arabic action name. Preserve recovery/idempotency and keep an unavailable real pending operation truthful.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [order-manager-actions-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/order-manager-actions-1920.png>). Inspect each image separately.

**Screenshot 1:** order-manager-actions-1920.

![UI-036 — Hide the idle pending-operation retry control, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/order-manager-actions-1920.png>)

<a id="ui-037"></a>

## UI-037 — Remove Work-order subtext

**Page:** Work-order list, detail and production actions.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove Work-order subtext**? It appears in **Work-order list, detail and production actions** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The detail header asks what must be manufactured. State controls describe the whole transition sequence. Preview/list sections add explanatory copy although the data and actions have labels.
>
> **Source to inspect:** [Features/WorkOrders/WorkOrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/WorkOrders/WorkOrdersView.xaml>), [Features/WorkOrders/WorkOrdersViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/WorkOrders/WorkOrdersViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove explanatory subtitles, helper paragraphs and repeated transition narration. Retain actual work-order/customer/assignment/date/production values, real notes, scope/preview markers and concise failure/availability states.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [work-confirmed-detail-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/work-confirmed-detail-1920.png>). Inspect each image separately.

**Screenshot 1:** work-confirmed-detail-1920.

![UI-037 — Remove Work-order subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/work-confirmed-detail-1920.png>)

<a id="ui-038"></a>

## UI-038 — Fix Work-order section and table alignment

**Page:** Work-order details.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Fix Work-order section and table alignment**? It appears in **Work-order details** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The required-furniture heading is physically left-aligned while the surrounding page and required-parts heading are RTL at the right. Its count badge is crowded against the title. In the Parts table, the part name is near the left edge of its very wide column while the header is centered much farther right, making the association hard to scan.
>
> **Source to inspect:** [Features/WorkOrders/WorkOrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/WorkOrders/WorkOrdersView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Apply a consistent deliberate RTL heading/count layout and align part values with their headers. Keep quantity LTR where appropriate and use proportionate column widths. Verify the rendered furniture and parts sections with mixed Arabic identifiers.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [work-confirmed-detail-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/work-confirmed-detail-1920.png>). Inspect each image separately.

**Screenshot 1:** work-confirmed-detail-1920.

![UI-038 — Fix Work-order section and table alignment, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/work-confirmed-detail-1920.png>)

<a id="ui-039"></a>

## UI-039 — Remove Users explanatory subtext

**Page:** Users list, create/edit and deactivation.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Remove Users explanatory subtext**? It appears in **Users list, create/edit and deactivation** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The list states that the engine manages accounts/permissions. The editor explains the effect on existing sessions. The password helper repeats the 12-character requirement already repeated by validation.
>
> **Source to inspect:** [Features/Users/UsersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Remove general explanatory text. Express any necessary password requirement in a concise label/control cue, retain actionable validation and essential consequences in deactivation confirmation. Keep explicit login identity and role as actual data, not expendable decoration.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [ui40-password-error-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui40-password-error-1920.png>). Inspect each image separately.

**Screenshot 1:** ui40-password-error-1920.

![UI-039 — Remove Users explanatory subtext, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui40-password-error-1920.png>)

<a id="ui-040"></a>

## UI-040 — Unify Users editor page hierarchy

**Page:** Add/edit User.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Unify Users editor page hierarchy**? It appears in **Add/edit User** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The Users editor uses an isolated heading inside a narrow centered layout, unlike the shared page headers elsewhere. It has a large gap between title and form, with inconsistent visual placement of the task and its actions.
>
> **Source to inspect:** [Features/Users/UsersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the editor using the established native page/form hierarchy and deliberate spacing. Give the task a clear heading, proportionate form and stable Save/Cancel. Keep read-only login identity, creation-only password and role controls.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [users-editor.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/users-editor.png>). Inspect each image separately.

**Screenshot 1:** users-editor.

![UI-040 — Unify Users editor page hierarchy, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/users-editor.png>)

<a id="ui-041"></a>

## UI-041 — Use consistent confirmation dialog typography

**Page:** Account switch and user deactivation confirmations and quotation conversion confirmation.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Use consistent confirmation dialog typography**? It appears in **Account switch and user deactivation confirmations and quotation conversion confirmation** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Confirmation body text is almost as large as the dialog title and spans several prominent lines. User deactivation has the same oversized body treatment. These dialogs read as another page rather than a short decision. The quotation-to-order confirmation also has several large explanatory paragraphs competing with its customer, item count and total.
>
> **Source to inspect:** [MainWindow.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/MainWindow.xaml>), [Features/Users/UsersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Users/UsersView.xaml>), [Resources/OperationsControls.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsControls.xaml>), [Features/Quotations/QuotationsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Quotations/QuotationsView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Use the existing theme's restrained title/body hierarchy, balanced spacing and concise consequences. Preserve warnings about unsaved fields/session ending, the exact affected account identity, default Cancel focus and the explicit action. Apply this hierarchy to quotation conversion: retain customer, total, essential consequence and a concise preview-only marker, while removing repeated process explanations.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached images:** [ui-09-dialog-manager-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui-09-dialog-manager-1920.png>); [ui39-users-deactivate-m.salem-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui39-users-deactivate-m.salem-1920.png>); [quotation-conversion-confirmation.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-conversion-confirmation.png>). Inspect each image separately.

**Screenshot 1:** ui-09-dialog-manager-1920.

![UI-041 — Use consistent confirmation dialog typography, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui-09-dialog-manager-1920.png>)

**Screenshot 2:** ui39-users-deactivate-m.salem-1920.

![UI-041 — Use consistent confirmation dialog typography, screenshot 2](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui39-users-deactivate-m.salem-1920.png>)

**Screenshot 3:** quotation-conversion-confirmation.

![UI-041 — Use consistent confirmation dialog typography, screenshot 3](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/quotation-conversion-confirmation.png>)

<a id="ui-042"></a>

## UI-042 — Open print previews with the complete page visible

**Page:** Saved quotation/order/draft and multi-page print preview.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Open print previews with the complete page visible**? It appears in **Saved quotation/order/draft and multi-page print preview** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The default fit-width mode opens at roughly 190% zoom on this full-screen display. Only the document heading and contact data are visible; item rows and totals require substantial vertical scrolling even for a one-page document.
>
> **Source to inspect:** [Controls/PrintPreview.xaml.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/PrintPreview.xaml.cs>), [Controls/PrintPreview.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/PrintPreview.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Use a deliberate initial whole-page view at this maximized 125% environment so users can inspect the paper, items and totals. Keep native fit-width/manual zoom commands and preserve a user-chosen zoom across refresh. Do not alter print scaling or the saved commercial document to compensate.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [saved-document-quotation-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/saved-document-quotation-1920.png>). Inspect each image separately.

**Screenshot 1:** saved-document-quotation-1920.

![UI-042 — Open print previews with the complete page visible, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/saved-document-quotation-1920.png>)

<a id="ui-043"></a>

## UI-043 — Make allowed Furniture options explicit

**Page:** Furniture — size editor.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Make allowed Furniture options explicit**? It appears in **Furniture — size editor** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The allowed-colors and allowed-handles section shows unchecked options, but the explanatory paragraph says that no selection allows every option. The visual state means the opposite of the actual rule. Removing subtext would make this ambiguity worse.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>), [Features/Furniture/FurnitureViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Show an explicit, labeled choice for all options versus selected options. Make the allowed set clear from the controls themselves, without a paragraph. Preserve the Rust contract meaning for an unrestricted set and for a restricted set; do not invent validation in the shell. Use the existing contract and focused editor tests to verify both cases.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [audit-furniture-size-edit.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-furniture-size-edit.png>). Inspect each image separately.

**Screenshot 1:** audit-furniture-size-edit.

![UI-043 — Make allowed Furniture options explicit, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-furniture-size-edit.png>)

<a id="ui-044"></a>

## UI-044 — Keep each printed line together across pages

**Page:** Print preview — multi-page quotation and order.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Keep each printed line together across pages**? It appears in **Print preview — multi-page quotation and order** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** In the 36-line synthetic quotation, item A-8 starts at the bottom of page 1 with its name and monetary cells. Its description and dimensions continue alone at the top of page 2. The repeated column header cannot identify which item that fragment belongs to. Pages 3 and 4 and the final total were checked separately.
>
> **Source to inspect:** [Controls/CustomerDocumentPages.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/CustomerDocumentPages.cs>), [Controls/CustomerDocumentLayout.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/CustomerDocumentLayout.cs>), [Features/Reception/QuotationCustomerDocument.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/QuotationCustomerDocument.cs>), [Features/Orders/OrderCustomerDocument.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrderCustomerDocument.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Keep normal-sized line rows, including their descriptions, selected options, dimensions, quantity and prices, on one page. Move a row to the next page when it will fit there. Handle an exceptional row taller than a page explicitly without losing text or causing a pagination loop. Retain repeated column headings, page numbering, Arabic shaping, LTR identifiers and unchanged Rust totals. Add a focused pagination regression for the observed A-8 split and verify every page at full-screen 125%.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached images:** [audit-print-multipage-page-2.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-multipage-page-2.png>); [audit-print-multipage-page-1.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-multipage-page-1.png>). Inspect each image separately.

**Screenshot 1:** audit-print-multipage-page-2.

![UI-044 — Keep each printed line together across pages, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-multipage-page-2.png>)

**Screenshot 2:** audit-print-multipage-page-1.

![UI-044 — Keep each printed line together across pages, screenshot 2](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-print-multipage-page-1.png>)

<a id="ui-045"></a>

## UI-045 — Label Furniture option price adjustments with their unit

**Page:** Furniture — handle editor.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Label Furniture option price adjustments with their unit**? It appears in **Furniture — handle editor** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The handle dialog labels its numeric input only as price adjustment and shows 0. The color dialog labels the equivalent adjustment with ر.ي. The handle field does not show the currency or clearly communicate that it is an adjustment rather than a full price.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Use the same concise adjustment label and visible YER unit in the handle and color editors. Keep signed numeric input isolated LTR within RTL layout, and preserve the contract's existing adjustment semantics. Remove the illustration explanation below the input, close its gap, and retain the actual preview visual.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [dialog-furniture-handle-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/dialog-furniture-handle-1920.png>). Inspect each image separately.

**Screenshot 1:** dialog-furniture-handle-1920.

![UI-045 — Label Furniture option price adjustments with their unit, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/dialog-furniture-handle-1920.png>)

<a id="ui-046"></a>

## UI-046 — Show an explicit empty state in the Furniture part picker

**Page:** Furniture — add part dialog.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Show an explicit empty state in the Furniture part picker**? It appears in **Furniture — add part dialog** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The part picker contains a search box and a blank results area with no status. In the captured fixture the only seeded part is already in the composition, so no addable row is shown. The user cannot tell whether there are no available parts, no search matches, or data has not loaded. The material picker already gives a concise no-match state.
>
> **Source to inspect:** [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>), [Features/Furniture/FurnitureViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Show one concise Arabic state for an empty loaded set and a different concise state for an unmatched search. Use actual authority loading/unavailable state where supported; do not turn unavailable data into a successful empty result. Preserve the addable-part filtering and close action. Remove the instructional paragraph and size the dialog around its real contents.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [dialog-furniture-part-picker-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/dialog-furniture-part-picker-1920.png>). Inspect each image separately.

**Screenshot 1:** dialog-furniture-part-picker-1920.

![UI-046 — Show an explicit empty state in the Furniture part picker, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/dialog-furniture-part-picker-1920.png>)

<a id="ui-047"></a>

## UI-047 — Use a compact Product editor hierarchy

**Page:** Products — create/edit, with and without variants.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Use a compact Product editor hierarchy**? It appears in **Products — create/edit, with and without variants** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The basic information card uses most of the viewport for a name, category, description and small image. The variant-mode choice begins below the fold even on this full-screen display. Large card padding, vertical gaps and helper copy make a simple product form feel long; the no-variant cost and notes require a separate scroll.
>
> **Source to inspect:** [Features/Products/ProductsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Products/ProductsView.xaml>), [Resources/OperationsTheme.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/OperationsTheme.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the form with the best native layout for this task. Keep the product identity, variant-mode choice and main cost path visible together when practical at full-screen 125%. Use restrained spacing and one clear hierarchy; keep optional description, image and notes available without giving them more visual weight than required fields. Retain the fixed save/cancel footer, entered text, all variant data and actual errors. Do not scale the page or force every large variant list into the initial viewport.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [audit-product-add.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-product-add.png>). Inspect each image separately.

**Screenshot 1:** audit-product-add.

![UI-047 — Use a compact Product editor hierarchy, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-product-add.png>)

<a id="ui-048"></a>

## UI-048 — Make the sales catalog useful at full-screen width

**Page:** Receptionist — Products sales catalog.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Make the sales catalog useful at full-screen width**? It appears in **Receptionist — Products sales catalog** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Each catalog entry occupies almost the full content width and about 150 physical pixels of height. The image and product name sit at the far right while price and selection action sit at the far left, leaving most of each card empty. Only about three products fit in the initial viewport. The small fallback Furniture drawing is dwarfed by its large gray tile.
>
> **Source to inspect:** [Features/Reception/SalesCatalogView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/SalesCatalogView.xaml>), [Resources/SalesCatalogVisuals.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Resources/SalesCatalogVisuals.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the catalog using the best native list or card layout for quick product comparison. Keep each product's visual, identity, variant count, starting price and select action together. Use available full-screen width to show more useful choices without making labels small. Give real images and fallback vectors a deliberate scale. Preserve actual product descriptions, category filters, search, unavailable/stale status and keyboard selection; do not remove record descriptions as explanatory subtext.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [audit-reception-catalog.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-reception-catalog.png>). Inspect each image separately.

**Screenshot 1:** audit-reception-catalog.

![UI-048 — Make the sales catalog useful at full-screen width, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-reception-catalog.png>)

<a id="ui-049"></a>

## UI-049 — Correct mixed-direction delivery dates

**Page:** Work Orders — list and detail.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Correct mixed-direction delivery dates**? It appears in **Work Orders — list and detail** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The delivery date is constructed as day, Arabic month, year, but both list and detail force the entire mixed string to LeftToRight. The rendered date reads as '21 2026 أكتوبر', separating the month from its intended sequence. This is visible in the list and the detail metadata.
>
> **Source to inspect:** [Features/WorkOrders/WorkOrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/WorkOrders/WorkOrdersView.xaml>), [Features/WorkOrders/WorkOrderModels.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/WorkOrders/WorkOrderModels.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Render the localized date in a clear stable Arabic sequence, for example '21 أكتوبر 2026', using direction isolation for numeric segments where required. Keep physical alignment distinct from reading direction. Do not change the stored date, timezone or authority rules. Verify the list, detail and changed date field at full-screen 125% with a focused mixed-direction formatting check if presentation logic changes.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [audit-workorders-shell-detail.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-workorders-shell-detail.png>). Inspect each image separately.

**Screenshot 1:** audit-workorders-shell-detail.

![UI-049 — Correct mixed-direction delivery dates, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-workorders-shell-detail.png>)

<a id="ui-050"></a>

## UI-050 — Align the Order delivery form with the record

**Page:** Receptionist — ready Order detail and delivery.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Align the Order delivery form with the record**? It appears in **Receptionist — ready Order detail and delivery** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The order total is a narrow card at the right. The delivery form is a wider card centered farther left, separated by a large vertical blank gap. The register-delivery action sits at the bottom edge even after scrolling. This breaks the page's alignment and makes the next daily action harder to find.
>
> **Source to inspect:** [Features/Orders/OrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrdersView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the detail and delivery area with the best native layout. Use one consistent content grid, a compact total and a clearly placed delivery section. Keep receiver, acceptance method, delivery notes and register action together with no arbitrary empty spacer. Keep relevant lines and authority state available and preserve actual permission, validation and delivery behavior. Verify the ready reception state at full-screen 125%.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [order-reception-delivery-1920.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/order-reception-delivery-1920.png>). Inspect each image separately.

**Screenshot 1:** order-reception-delivery-1920.

![UI-050 — Align the Order delivery form with the record, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/order-reception-delivery-1920.png>)

<a id="ui-051"></a>

## UI-051 — Place custom-dimension validation beside the field

**Page:** Sales Furniture selection — custom dimensions.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Place custom-dimension validation beside the field**? It appears in **Sales Furniture selection — custom dimensions** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** An invalid width is shown as plain 'invalid' in a normal input. The only correction message appears in black near the bottom of the page, far from the width field, beside a separate process explanation. The add action is disabled and total is a dash, but the invalid field has no clear visible error association.
>
> **Source to inspect:** [Features/Reception/FurnitureSelectionView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/FurnitureSelectionView.xaml>), [Features/Reception/FurnitureSelectionViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Reception/FurnitureSelectionViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Show a concise Arabic validation message and an accessible error state at the relevant dimension input, with a visible indication beyond color. Keep allowed bounds and cm units as compact field information. Keep an actual pending-authority status near the total while evaluation runs, without a permanent process paragraph. Preserve Rust evaluation, unavailable totals and disabled add until a valid reply; do not locally invent a price.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [ui29-dimensions-invalid-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui29-dimensions-invalid-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** ui29-dimensions-invalid-1920x1080.

![UI-051 — Place custom-dimension validation beside the field, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/ui29-dimensions-invalid-1920x1080.png>)

<a id="ui-052"></a>

## UI-052 — Identify the actual raw-material field error

**Page:** Raw materials — edit validation.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Identify the actual raw-material field error**? It appears in **Raw materials — edit validation** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The captured form has a selected category and unit, but a purchase cost of 8.5 is rejected with a combined message that tells the user to select a category, unit and valid cost. It does not identify what needs correction and can lead the user to change valid fields.
>
> **Source to inspect:** [Features/RawMaterials/RawMaterialsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsView.xaml>), [Features/RawMaterials/RawMaterialsViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/RawMaterials/RawMaterialsViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Display a concise error beside the field that is actually invalid, with a clear visible association and accessible name. Keep a form summary for authority failures that cannot be mapped safely. Use the existing Rust-owned validation contract and accepted numeric rules; do not duplicate domain validation or loosen precision in C#. Preserve entered values and keep legitimate errors after explanatory subtext is removed.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [material-durable-editor-error-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-editor-error-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** material-durable-editor-error-1920x1080.

![UI-052 — Identify the actual raw-material field error, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/material-durable-editor-error-1920x1080.png>)

<a id="ui-053"></a>

## UI-053 — Restore the visible Furniture category filter after loading

**Page:** Furniture — list category filter.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Restore the visible Furniture category filter after loading**? It appears in **Furniture — list category filter** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The category filter is blank while the list contains a loaded Furniture item. ApplyDurableData clears and rebuilds CategoryOptions, which includes the all-categories label, but does not restore or notify the selected category afterward. The control therefore does not tell the user which scope is active.
>
> **Source to inspect:** [Features/Furniture/FurnitureDurableState.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureDurableState.cs>), [Features/Furniture/FurnitureView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureView.xaml>), [Features/Furniture/FurnitureViewModel.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Furniture/FurnitureViewModel.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Keep a valid selected category visibly selected when the category collection is refreshed. Preserve a still-valid choice and fall back to the existing all-categories choice if it is removed. Do not insert a cosmetic placeholder that hides a stale selection. Use a focused binding/state regression and render the loaded list at full-screen 125%.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [audit-furniture-list.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-furniture-list.png>). Inspect each image separately.

**Screenshot 1:** audit-furniture-list.

![UI-053 — Restore the visible Furniture category filter after loading, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/audit-furniture-list.png>)

<a id="ui-054"></a>

## UI-054 — Add a visible close action to the Part material picker

**Page:** Parts — raw material picker.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Add a visible close action to the Part material picker**? It appears in **Parts — raw material picker** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** The loaded material picker shows a search box and Choose buttons, but no visible Close or Cancel action. Its no-match state also lacks an exit control. The Furniture part picker has an explicit Close footer, so the two otherwise similar dialogs use different exit patterns.
>
> **Source to inspect:** [Features/Parts/PartsView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Parts/PartsView.xaml>), [Controls/DialogHost.cs](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Controls/DialogHost.cs>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Give this picker a visible concise Arabic Close/Cancel action in the existing dialog footer, without adding an explanatory sentence. Preserve Escape, focus return and selection behavior. Closing must not add a material or discard existing composition rows. Verify the loaded and no-match states at full-screen 125%, using the existing dialog interaction tests where applicable.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [part-picker-1920x1080.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/part-picker-1920x1080.png>). Inspect each image separately.

**Screenshot 1:** part-picker-1920x1080.

![UI-054 — Add a visible close action to the Part material picker, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/part-picker-1920x1080.png>)

<a id="ui-055"></a>

## UI-055 — Make Order line cards compact and scannable

**Page:** Orders — mixed Furniture and ready-made Product detail.

**Prompt:**

> [$frontend-design](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/.agents/skills/frontend-design/SKILL.md>) Can you fix this UI issue: **Make Order line cards compact and scannable**? It appears in **Orders — mixed Furniture and ready-made Product detail** in Eitmad, the Arabic-first native Windows furniture operations app.
>
> **Observed issue:** Each order line stretches across the workspace while its name, variant, options, quantity and price occupy a small cluster at the right. A large empty area remains in every card, and only one complete line fits below the duplicated reception headers. This slows comparison of a mixed order.
>
> **Source to inspect:** [Features/Orders/OrdersView.xaml](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/shells/windows/Features/Orders/OrdersView.xaml>). Inspect the current source and direct callers before editing; the screenshots show the audited revision.
>
> **Done state:** Redesign the order lines as compact native rows or a purposeful details grid. Keep identity, variant, dimensions, options, quantity and unit price together with clear labels and a deliberate image scale. Preserve all stored line data and Rust totals; do not delete variant details as subtext. Show more useful lines at full-screen 125% without shrinking typography or scaling the page.
>
> **Scope and verification:** Make the smallest complete native WPF change for this issue. Reuse the established theme, approved Arabic terms and RTL layout. Keep business rules, authorization, calculations and saved data under Rust authority. Preserve record text and concise real errors/states. Build the affected shell and verify the changed screen at maximized full screen on the current 125% scaling only, using synthetic Arabic data. Do not use computer use or test other window dimensions. Use an existing focused behavior check if this change affects consequential presentation state. Do not claim an unperformed rendered or interaction check.
>
> **Attached image:** [reception-orders-mixed-items.png](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/reception-orders-mixed-items.png>). Inspect each image separately.

**Screenshot 1:** reception-orders-mixed-items.

![UI-055 — Make Order line cards compact and scannable, screenshot 1](<C:/Users/Hussein/Desktop/SWProjects/Eitmad/artifacts/ui-audit-2026-10-14-fullscreen-125/captures/reception-orders-mixed-items.png>)

## Verification limits

The audit harness compiled in Release against .NET 8 and ran 37 existing focused rendered fixture methods; all 37 reported PASS. Supplemental renders covered missing list/editor/dialog surfaces and complete print pages. This was not the complete Windows test suite, a Rust test run, or a live business workflow test. NuGet reported NU1900 because vulnerability metadata could not be reached; no package-security result is claimed.

Keyboard and UI Automation assertions executed inside selected existing fixtures. This audit did not perform a complete keyboard, physical focus, high-contrast, accessibility or OS popup-placement review. Prompts specify the focused verification required when the corresponding interaction is changed.

Fixture prices and returned totals can come from independent fake replies. Apparent numerical inconsistencies in those fixtures are not reported as product calculation bugs. The prompts preserve Rust-owned commercial values.

There is no canonical product-documentation change in this audit: it records proposed fixes and evidence only.
