---
title: "Design an Arabic-first user experience"
description: "Treat RTL, bidirectional text, search, input, documents, and accessibility as system requirements."
audience: "architecture"
page_type: "reference"
status: "active"
owner: "product and architecture maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "a user workflow, Arabic term, search rule, native shell, or generated document changes"
keywords:
  - "العربية أولًا"
  - "Arabic-first UX"
  - "RTL"
  - "bidirectional text"
---

# Arabic-First UX Requirements

## Principle

Arabic is Eitmad's default UI and UX design language. Arabic-first means product flows are conceived, written, tested, and reviewed in Arabic and RTL from the beginning. English and other locales remain supported, but they do not define the layout assumptions.

These requirements apply to native shells, Rust-owned messages and contracts, search, imports and exports, documents, reports, diagnostics, and support tools.

## Language and content

- User-facing domain language MUST use the approved terms in the [domain glossary](../glossary.md).
- Arabic copy MUST be natural operational language, not literal machine-like translation.
- Stable message identifiers and structured parameters MUST cross IPC; shells MUST NOT build meaning by concatenating translated fragments.
- Dates, times, numbers, units, currency, plural forms, and names MUST use locale-aware formatting while preserving the underlying canonical value.
- The user MUST be able to distinguish similarly named customers, orders, products, and materials without relying on transliteration.
- Missing translations MUST be detectable in development and CI. Production fallbacks MUST remain comprehensible and MUST NOT expose internal keys.

## Direction and layout

- Arabic screens default to RTL at the window, navigation, reading, and component levels.
- Layout direction MUST use logical inline/block and start/end concepts rather than hard-coded left/right positioning. Margins, padding, alignment, borders, placement, animation origins, and resize affordances MUST follow the same rule.
- Mirroring MUST preserve meaning. Media controls, timelines, charts, physical diagrams, brand marks, numbers, and universally directional icons MUST be reviewed individually rather than mirrored blindly.
- Focus order, keyboard navigation, menus, breadcrumbs, tables, validation summaries, and screen-reader order MUST follow the visual and semantic RTL flow.
- Primary navigation begins at the logical start edge. Dialog actions, back/forward actions, disclosure indicators, and split views MUST follow the target platform's native RTL convention unless doing so changes their meaning.
- Tables MUST define the leading identity column, frozen columns, row actions, sort indicators, resize handles, and horizontal-scroll origin for RTL. Numeric values remain aligned for comparison without changing their stored value or reading order.
- Direction MUST be applied at the smallest reliable container boundary. A child displaying an identifier, code sample, phone number, path, or other conventionally LTR value MUST explicitly isolate that value instead of switching the entire screen to LTR.
- Dense operational screens MUST remain scannable at common desktop sizes and supported text scaling.

## Bidirectional text

Real records will mix Arabic with Latin product codes, phone numbers, measurements, URLs, file paths, model names, and currency values. Every relevant component MUST be tested with mixed-direction data.

- Store clean Unicode text; never persist presentation control characters as a workaround for a rendering bug unless the domain value truly contains them.
- Isolate embedded directional runs at rendering boundaries so one value cannot reorder surrounding labels or punctuation.
- Treat all user-controlled text as capable of affecting bidirectional display. Isolation MUST prevent one field from reordering adjacent labels, actions, delimiters, or security-relevant identifiers.
- Keep identifiers, phone numbers, quantities, and measurements readable and copyable in their conventional order.
- Cursor movement, selection, deletion, copy/paste, undo/redo, wrapping, truncation, and ellipsis behavior MUST be verified in mixed Arabic/Latin inputs. Truncation MUST preserve the part needed to distinguish the value and MUST NOT visually detach a sign, unit, or identifier suffix.
- Logs and exported structured data MUST preserve canonical text even when visual display adds direction isolation.

Representative test values include `طلب ORD-2048`, `خزانة 120×60 cm`, `0555 123 456`, `خشب MDF أبيض`, and Arabic names containing Latin initials.

## Arabic typography

- The selected UI font stack MUST shape Arabic correctly, cover all required Arabic and Latin glyphs, provide the weights used by the design, and render Arabic and Latin runs with compatible visual size.
- Fallback fonts MUST preserve shaping, baseline, line height, punctuation, and numeral readability. A missing glyph MUST be detected during development rather than accepted as a production fallback.
- Arabic text MUST NOT use artificial letter spacing. Kashida, ligatures, justification, and line breaking MUST be left to a tested shaping engine unless an approved document design specifies otherwise.
- Controls MUST allow enough vertical space for Arabic ascenders, descenders, diacritics, validation text, and supported text scaling without clipping.
- Typography MUST be tested at normal, dense, emphasized, disabled, and high-contrast states. Weight alone MUST NOT communicate status.
- UI and document fonts MUST have licenses that permit distribution. Generated PDF files MUST embed or package the required Arabic fonts when the receiving system cannot be trusted to provide them.

## Input and data entry

- Switching between Arabic and Latin keyboard layouts MUST not lose focus, text, selection, or validation state.
- Forms MUST accept Arabic and Latin digits where users reasonably enter either, normalize only with clear domain rules, and never silently change an identifier.
- Required fields, optional fields, examples, units, errors, and recovery actions MUST be explicit in Arabic.
- Validation belongs to Rust. The shell MAY provide immediate presentation hints, but the authoritative result comes from the engine and MUST be localized safely.
- Destructive and irreversible actions require clear Arabic consequences and appropriate confirmation; frequent safe actions should not be burdened with unnecessary dialogs.
- Data-entry flows for reception and workshop use MUST be efficient with keyboard-only interaction and tolerant of intermittent connectivity.

## Validation and localized errors

- Rust MUST return a stable message identifier, structured parameters, affected field or operation, and safe recovery metadata. It MUST NOT return an English sentence for a shell to parse or require a shell to reproduce domain validation.
- Shells MUST resolve complete localized messages. They MUST NOT construct Arabic meaning by concatenating translated fragments whose order, gender, plurality, or punctuation can change by locale.
- A validation message MUST explain what is invalid and how to correct it. A command failure MUST explain what happened, what state was preserved, whether retry is safe, and the next available action.
- Field errors MUST remain programmatically associated with their control. Error summaries MUST link or move focus to the affected field in semantic RTL order without discarding entered data.
- Parameters such as names, codes, quantities, paths, and currency values MUST be formatted and directionally isolated according to their type. User-controlled content MUST never become localization markup.
- Permission-denied and security messages MUST be clear in Arabic without exposing relationship graphs, internal policy names, secrets, record existence outside the caller's scope, or raw diagnostic details.
- Missing translations, unknown message identifiers, malformed parameters, and fallback-locale use MUST be detectable in automated tests and privacy-safe diagnostics. Production fallback text MUST remain actionable and MUST NOT expose an internal localization key.

## Locale-aware dates, numbers, units, and currency

- Canonical timestamps, decimal quantities, currency codes, identifiers, and units remain locale-independent in Rust-owned storage and contracts. Formatting is presentation; parsing never changes the authoritative meaning silently.
- Every formatted date or time MUST use an explicit locale, calendar, time zone, and precision. Relative dates MUST expose an unambiguous absolute value when needed for audit or coordination.
- Every number formatter and parser MUST define accepted digit sets, decimal and grouping separators, sign placement, rounding, precision, and treatment of pasted whitespace. Identifiers MUST NOT be parsed or reformatted as numbers.
- Currency display MUST use an explicit ISO currency code, approved symbol or label, fraction digits, rounding rule, sign placement, and spacing. A bare amount is invalid where more than one currency can occur.
- Units and dimensions MUST keep the number, multiplication sign, unit, and axis order readable in mixed-direction text. Conversion and rounding remain Rust-owned domain behavior.
- Human-readable reports MAY localize dates and numeric display. Machine-readable exports MUST retain canonical values, stable field identifiers, explicit currency codes, time-zone information, and documented units.
- The default locale, fallback chain, calendar, time zone behavior, displayed digit set, accepted input digits, default currency, and rounding policy MUST be approved before shell implementation. Until then, examples are fixtures, not product defaults.

## Search and sorting

Arabic search behavior MUST be defined per domain and implemented by the Rust authority so every shell and sync mode agrees.

- Preserve original values for display and audit; use derived normalized forms only for matching and indexing.
- Define intentional handling for diacritics, tatweel, hamza/alef variants, alef maqsura/ya, and Arabic/Latin digits.
- Start from a documented Unicode normalization form. Define treatment of Arabic and Persian letter variants, punctuation, whitespace, zero-width characters, and presentation forms; never let platform-default normalization decide product behavior.
- Do not collapse distinct letters or names when doing so could return unsafe or misleading matches.
- Support mixed Arabic/Latin queries for codes, materials, phone numbers, and customer names.
- Explain and test tokenization, prefix behavior, typo tolerance, ranking, and exact-match priority for each searchable domain.
- Sorting MUST use an explicit locale and stable tie-breaker. Identifier and numeric sorting MUST not be accidentally treated as Arabic lexical sorting.
- Highlighting MUST map normalized matches back to the correct original grapheme clusters without corrupting shaping or selecting unrelated characters.
- Each searchable domain MUST publish a normalization profile. Customer names, product codes, phone numbers, materials, and free text MAY use different equivalences and ranking because false matches have different operational risk.
- Exact original-form matches MUST rank ahead of broader normalized matches unless a documented domain rule says otherwise. Authorization and scope filtering MUST occur before results, counts, suggestions, or highlights are disclosed.

## Tables, status, and operational feedback

- Table reading order, frozen columns, row actions, sort indicators, resize handles, and horizontal scrolling MUST be usable in RTL.
- Status MUST never rely on color alone. Use localized text and an accessible semantic indicator.
- Offline, syncing, conflicted, queued, failed, and up-to-date states MUST be distinguishable and explain what the user can safely do next.
- Long-running commands MUST expose progress, cancellation when safe, and a localized completion or failure outcome.
- Error messages MUST state what happened, what was preserved, and the next action without leaking sensitive internals.

## Documents, printing, and exports

- Quotes, orders, invoices, work instructions, labels, and reports MUST be designed for Arabic typography and RTL pagination.
- Mixed-direction values, tables, totals, page numbers, headers, footers, and signatures MUST render correctly in print and PDF output.
- Fonts MUST contain required Arabic glyphs and be distributable for the target platform or embedded output.
- Line breaking and shaping MUST be visually verified; generated documents MUST not be approved solely from extracted text.
- Page size, margins, orientation, scaling, page order, repeated table headers, row splitting, totals, signatures, and continuation markers MUST be defined for every document type.
- Documents MUST remain legible in monochrome and MUST not communicate state through color alone. Print preview and physical or virtual-printer output MUST be checked on every supported platform that owns a print adapter.
- PDF verification MUST inspect rendered pages for missing glyphs, clipped diacritics, reordered values, broken table borders, orphaned totals, incorrect pagination, and font substitution. Accessibility tags and archival conformance MUST be defined when the document's audience, retention, or legal role requires them.
- CSV and spreadsheet exports MUST document encoding, direction expectations, canonical numeric values, and localized display behavior.
- Machine-readable exports retain stable field identifiers and canonical values independently from localized labels.

## Accessibility

- Controls MUST expose localized accessible names, roles, states, validation, and relationships through native accessibility APIs.
- Screen-reader output MUST be tested in Arabic, including mixed-direction values and dynamic subscription updates.
- Keyboard navigation, visible focus, contrast, scaling, reduced-motion preferences, and non-color status cues are required.
- Announcements for updates, sync, and errors MUST be useful without becoming repetitive during high-frequency work.

## Acceptance criteria for every feature

A feature is not complete until it has been verified with:

1. Arabic as the active locale and RTL layout;
2. an English or LTR locale where supported;
3. representative Arabic-only and mixed-direction records;
4. Arabic and Latin keyboard input and digits where relevant;
5. long labels, empty states, validation errors, and permission-denied states;
6. offline, retry, and sync/conflict states where relevant;
7. keyboard-only use, text scaling, and accessible-name inspection;
8. printed or exported output when the feature produces documents;
9. search normalization and stable sort tests when the feature is searchable.

Feature documentation MUST record the Arabic terms chosen, normalization behavior, representative test cases, and any known platform-specific rendering tradeoffs.

Use the mandatory [Arabic-first feature checklist](../developer/contributing/arabic-first-feature-checklist.md). Every item MUST be marked `Pass`, `Not applicable` with a reason, or `Blocked`. A blocked applicable item prevents feature completion.

## Shared mixed-language scenarios

Every relevant test layer reuses synthetic scenarios with the original value preserved for assertions:

| Scenario | Representative value | Required checks |
| --- | --- | --- |
| Arabic label with Latin identifier | `طلب ORD-2048` | isolation, punctuation, copy/paste, exact search |
| Material and dimensions | `خزانة MDF 120×60 cm` | shaping, axis order, unit placement, wrapping |
| Arabic and Latin digits | `١٢ قطعة / 12 قطعة` | accepted input, canonical value, display policy |
| Phone number | `+967 777 123 456` | LTR order, selection, normalization, masking |
| Mixed personal name | `أحمد A. القيسي` | cursor movement, sorting, safe truncation |
| Search variants | `إعتماد`, `اعتماد`, `اِعتماد`, `اعـتماد` | documented equivalence, ranking, highlighting |
| Currency and negative amount | `-1,250.50 YER` with an Arabic label | sign, separators, currency code, rounding |
| Ambiguous date input | `03/04/2026` | rejection or explicit interpretation; no silent ambiguity |
| Long validation outcome | synthetic Arabic field and permission errors | association, wrapping, focus, recovery, privacy |
| Multi-page quotation | Arabic customer and Latin codes across page breaks | font embedding, headers, totals, signatures, print |

Feature-specific scenarios MUST add empty, long, duplicate, denied, offline, retry, conflict, and text-scaling states where applicable.
