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
- Layout direction MUST use logical start/end concepts rather than hard-coded left/right positioning.
- Mirroring MUST preserve meaning. Media controls, timelines, charts, physical diagrams, brand marks, numbers, and universally directional icons MUST be reviewed individually rather than mirrored blindly.
- Focus order, keyboard navigation, menus, breadcrumbs, tables, validation summaries, and screen-reader order MUST follow the visual and semantic RTL flow.
- Dense operational screens MUST remain scannable at common desktop sizes and supported text scaling.

## Bidirectional text

Real records will mix Arabic with Latin product codes, phone numbers, measurements, URLs, file paths, model names, and currency values. Every relevant component MUST be tested with mixed-direction data.

- Store clean Unicode text; never persist presentation control characters as a workaround for a rendering bug unless the domain value truly contains them.
- Isolate embedded directional runs at rendering boundaries so one value cannot reorder surrounding labels or punctuation.
- Keep identifiers, phone numbers, quantities, and measurements readable and copyable in their conventional order.
- Cursor movement, selection, deletion, copy/paste, truncation, and ellipsis behavior MUST be verified in mixed Arabic/Latin inputs.
- Logs and exported structured data MUST preserve canonical text even when visual display adds direction isolation.

Representative test values include `طلب ORD-2048`, `خزانة 120×60 cm`, `0555 123 456`, `خشب MDF أبيض`, and Arabic names containing Latin initials.

## Input and data entry

- Switching between Arabic and Latin keyboard layouts MUST not lose focus, text, selection, or validation state.
- Forms MUST accept Arabic and Latin digits where users reasonably enter either, normalize only with clear domain rules, and never silently change an identifier.
- Required fields, optional fields, examples, units, errors, and recovery actions MUST be explicit in Arabic.
- Validation belongs to Rust. The shell MAY provide immediate presentation hints, but the authoritative result comes from the engine and MUST be localized safely.
- Destructive and irreversible actions require clear Arabic consequences and appropriate confirmation; frequent safe actions should not be burdened with unnecessary dialogs.
- Data-entry flows for reception and workshop use MUST be efficient with keyboard-only interaction and tolerant of intermittent connectivity.

## Search and sorting

Arabic search behavior MUST be defined per domain and implemented by the Rust authority so every shell and sync mode agrees.

- Preserve original values for display and audit; use derived normalized forms only for matching and indexing.
- Define intentional handling for diacritics, tatweel, hamza/alef variants, alef maqsura/ya, and Arabic/Latin digits.
- Do not collapse distinct letters or names when doing so could return unsafe or misleading matches.
- Support mixed Arabic/Latin queries for codes, materials, phone numbers, and customer names.
- Explain and test tokenization, prefix behavior, typo tolerance, ranking, and exact-match priority for each searchable domain.
- Sorting MUST use an explicit locale and stable tie-breaker. Identifier and numeric sorting MUST not be accidentally treated as Arabic lexical sorting.
- Highlighting MUST map normalized matches back to the correct original characters.

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
