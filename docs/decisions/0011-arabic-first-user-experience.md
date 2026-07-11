# ADR-0011: Make Arabic-first UX a system requirement

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Product and architecture maintainers
> - Last verified: 2026-07-11
> - Review trigger: Supported languages, user populations, platform text behavior, or document requirements materially change
> - Supersedes: None
> - Superseded by: None

## Context and problem

The primary users work in Arabic while records routinely mix Arabic with codes, measurements, phone numbers, and Latin product terms. Retrofitting RTL and Arabic semantics after LTR implementation creates structural layout, search, input, accessibility, and reporting defects.

## Decision drivers

- Natural, efficient workflows for primary users
- Correct mixed-direction data and Arabic search
- Accessible native UI and Arabic-ready documents

## Considered options

- **Arabic-first from design:** validates the hardest language and direction constraints early.
- **English-first with later localization:** faster prototypes, but embeds unsafe LTR and Latin assumptions.
- **Arabic-only:** reduces scope, but cannot represent mixed operational data or future locales well.

## Decision

Arabic MUST be the default design and verification language. Every feature and contract MUST account for RTL, bidirectional text, Unicode preservation, Arabic and Latin input and digits, localization, Arabic-aware search and sorting, native accessibility, and Arabic-ready print and exports. English and other supported locales remain first-class compatibility cases, not the source of layout assumptions.

## Architecture and contract impact

Rust owns domain terms, canonical values, search semantics, structured validation, and stable message identifiers. Shells own native RTL rendering, input, focus, accessibility, and direction isolation. Layout uses logical start/end rather than hard-coded left/right.

## Storage and sync impact

Canonical Unicode data is preserved. Search indexes store derived normalized forms without changing display or audit values, and sync never introduces presentation controls into canonical records.

## Security impact

Localized errors remain safe and do not expose internal keys, secrets, or policy details. Mixed-direction rendering must not obscure identifiers or destructive actions.

## Arabic UX impact

This ADR establishes the complete baseline in [Arabic-first UX requirements](../product/arabic-first-ux.md), including representative mixed text, keyboard use, tables, status, documents, and accessibility.

## Consequences and tradeoffs

### Positive

- Primary workflows are designed around actual language and direction needs.
- Cross-platform text defects appear before feature completion.

### Negative

- Design, fixtures, fonts, search rules, accessibility, and visual verification require early investment.

### Risks

- Platform-specific bidi behavior can diverge; mitigate with shared fixtures and native per-platform acceptance tests.

## Verification

The foundation contains normative Arabic UX acceptance criteria and synthetic representative values. Every production feature must record and pass applicable Arabic, LTR, bidi, input, search, accessibility, and output checks.

## Follow-up work

- Approve first-domain Arabic terminology and representative user workflows.
- Establish automated localization completeness and native visual test coverage.

## Related decisions and documents

- [Arabic-first UX requirements](../product/arabic-first-ux.md)
- [Domain glossary](../domain-glossary.md)
- [ADR-0002](0002-native-platform-ui-shells.md)
