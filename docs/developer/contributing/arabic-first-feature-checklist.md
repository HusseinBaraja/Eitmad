---
title: "Verify Arabic-first feature readiness"
description: "Block shell work and feature completion until Arabic, RTL, bidi, locale, search, accessibility, and document evidence is recorded."
audience: "developer"
page_type: "reference"
status: "active"
owner: "product, architecture, and quality maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "an Arabic UX requirement, locale policy, shell platform, search profile, or generated document changes"
keywords:
  - "Arabic feature checklist"
  - "RTL acceptance criteria"
  - "mixed Arabic English tests"
  - "قائمة التحقق العربية"
---

# Verify Arabic-first feature readiness

Use this checklist before shell implementation and again before completing every feature. For each item record `Pass` with evidence, `Not applicable` with a reason, or `Blocked` with an owner and required decision. An applicable blocked or unassessed item prevents implementation or completion at the stated gate.

The canonical behavior is defined by [Arabic-first UX requirements](../../architecture/arabic-first-ux.md). Copy this checklist into the owning feature document or link a version-controlled test or review record that preserves each result.

## Pre-shell product decisions

- [ ] The default locale and fallback chain are approved.
- [ ] Calendar, time-zone, date precision, and relative-date policies are approved.
- [ ] Accepted input digits and displayed digit set are approved.
- [ ] Default and supported currencies, fraction digits, rounding, sign, and symbol or code policies are approved.
- [ ] UI font, fallback stack, supported weights, glyph coverage, licensing, and document-embedding strategy are approved.
- [ ] Localization contracts define stable message identifiers, typed parameters, fallback behavior, and missing-translation detection.
- [ ] Search normalization profiles are approved for each initial searchable domain.
- [ ] Shared Arabic, Latin, and bidirectional fixtures have stable expected results.
- [ ] Accessibility and screen-reader verification platforms are named.
- [ ] PDF, print, paper-size, and generated-document verification baselines are named.

## Feature design and ownership

- [ ] The feature was designed first in Arabic and RTL using approved terms from the [glossary](../../glossary.md).
- [ ] The owning Rust vertical defines domain meaning, validation, canonical values, search semantics, and structured outcomes.
- [ ] Native shells own only presentation, direction isolation, native input, focus, and accessibility behavior.
- [ ] The feature document records applicable Arabic behavior, test evidence, platform differences, and known gaps.

## RTL layout and interaction

- [ ] Layout uses logical inline/block and start/end properties without hidden LTR assumptions.
- [ ] Navigation, focus, keyboard order, dialogs, menus, breadcrumbs, and validation summaries follow semantic RTL order.
- [ ] Icons, charts, diagrams, media controls, and physical directions have an explicit mirror or no-mirror decision.
- [ ] Tables define leading and frozen columns, actions, sorting, resizing, numeric alignment, and RTL scrolling.
- [ ] Empty, loading, long-content, narrow-window, text-scaling, permission-denied, offline, retry, and conflict states render correctly where applicable.

## Bidirectional text and input

- [ ] Arabic labels containing Latin codes, numbers, paths, phone numbers, measurements, and currencies are isolated at rendering boundaries.
- [ ] User-controlled text cannot reorder adjacent labels, punctuation, actions, or security-relevant identifiers.
- [ ] Cursor movement, selection, deletion, undo/redo, wrapping, truncation, ellipsis, and copy/paste were checked with mixed-direction data.
- [ ] Switching Arabic and Latin keyboard layouts preserves focus, selection, entered text, and validation state.
- [ ] Canonical storage, IPC, audit, logs, and machine-readable exports exclude presentation-only direction controls.

## Typography and localization

- [ ] Arabic shaping, ligatures, diacritics, punctuation, baselines, line height, fallback, and supported weights render without missing glyphs or clipping.
- [ ] Arabic text has no artificial letter spacing and remains readable in dense, disabled, emphasized, high-contrast, and scaled states.
- [ ] Complete localized messages are used; translated fragments are not concatenated.
- [ ] Missing translations and malformed localization parameters fail automated checks or produce privacy-safe diagnostics.
- [ ] Accessible names, descriptions, states, relationships, and dynamic announcements use approved Arabic copy.

## Validation and errors

- [ ] Rust returns a stable message identifier, structured parameters, affected field or operation, and safe recovery metadata.
- [ ] Validation states what is wrong and how to correct it without losing entered data.
- [ ] Command failures state what happened, what was preserved, whether retry is safe, and the next action.
- [ ] Field errors and summaries remain programmatically associated and usable by keyboard and screen reader.
- [ ] Denial and security messages do not expose internal policies, secrets, cross-scope existence, or unsafe diagnostics.

## Dates, numbers, units, and currency

- [ ] Formatters and parsers use explicit locale, calendar, time zone, precision, digit, separator, sign, and rounding policies.
- [ ] Identifiers are never parsed or reformatted as numbers.
- [ ] Ambiguous date or number input is rejected or explicitly disambiguated; it is never silently reinterpreted.
- [ ] Currency includes an explicit code or unambiguous approved symbol and uses Rust-owned precision and rounding.
- [ ] Dimensions preserve number, multiplication sign, axis order, and unit readability in RTL and copy/paste.
- [ ] Machine-readable values remain canonical and include currency codes, units, and time-zone information where applicable.

## Search and sorting

- [ ] The owning domain documents Unicode normalization, diacritics, tatweel, letter and digit variants, punctuation, whitespace, tokenization, prefixes, ranking, and exact-match priority.
- [ ] Normalized indexes never replace the original display and audit value.
- [ ] Potentially distinct names or identifiers are not collapsed without an approved, tested domain rule.
- [ ] Highlighting maps normalized matches to the correct original grapheme clusters.
- [ ] Sorting uses an explicit locale and deterministic tie-breaker; numeric and identifier sorting do not use accidental lexical order.
- [ ] Authorization and scope filtering occur before counts, suggestions, results, or highlights are disclosed.

## Reports, PDF, print, and exports

- [ ] Arabic shaping, font embedding, mixed-direction values, tables, totals, headers, footers, page numbers, signatures, and page breaks were checked on rendered pages.
- [ ] Paper size, margins, orientation, scaling, repeated headers, row splitting, and continuation behavior are defined.
- [ ] Output remains usable in monochrome and does not rely on color alone.
- [ ] Platform print preview and physical or virtual-printer output were checked where printing is supported.
- [ ] CSV and spreadsheet encoding, direction expectations, canonical numbers, units, dates, and localized labels are documented.
- [ ] Accessibility tags or archival conformance are defined when required by the document's audience, retention, or legal role.

## Test evidence and completion

- [ ] Unit tests cover Rust-owned validation, parsing, formatting inputs, normalization, ranking, and stable sorting where applicable.
- [ ] Contract tests cover Unicode preservation, message identifiers, typed parameters, versions, capabilities, and mixed-language serialization.
- [ ] Shell-conformance tests cover RTL layout, bidi interaction, focus, keyboard input, accessibility, localization fallbacks, and platform differences.
- [ ] End-to-end tests cover the critical Arabic-first workflow, including denial and failure recovery.
- [ ] Generated documents are visually rendered and inspected; extracted text alone is not accepted as evidence.
- [ ] The app builds, runs, and completes focused checks without warnings or errors on every affected platform available to the change.
- [ ] Feature documentation links the evidence and records any unavailable platform verification with an owner and follow-up trigger.

## Shared scenarios

Begin with the canonical [mixed-language scenarios](../../architecture/arabic-first-ux.md#shared-mixed-language-scenarios), then add feature-specific values. Use synthetic people, organizations, records, scopes, and financial values only.

## Related work

- [Document a feature](templates/feature-documentation.md)
- [Review documentation quality](documentation-review-checklist.md)
- [Run foundation checks](../../operations/index.md)
