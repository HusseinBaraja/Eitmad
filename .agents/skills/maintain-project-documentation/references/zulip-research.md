# Zulip research basis

## Scope

Research checked official Zulip documentation and the public Zulip repository on 2026-07-11. Use these sources to understand system properties, not to copy Zulip text, branding, or framework choices.

## Findings adopted

1. **Documentation-based onboarding.** Zulip explicitly expects contributors to learn through extensive written guides and routes newcomers through a staged contribution journey.
2. **Audience-specific systems.** Zulip separates developer/sysadmin, user help, integrations, and API documentation because each audience has different tasks and vocabulary.
3. **Markdown-backed, previewable sources.** Developer documentation lives with code, builds locally, and receives pull-request previews.
4. **Deep subsystem documentation.** Navigation exposes architecture, directory structure, testing, operations, subsystem internals, contribution practice, and troubleshooting rather than only API summaries.
5. **Automation.** Zulip builds documentation and tests internal/external links; other suites verify API documentation and help content.
6. **Discoverability over page count.** Zulip recommends adding small refinements to an existing discoverable article and treats primary navigation space as limited.
7. **Task-oriented user writing.** Help-center guidance emphasizes clear, concise, correct writing for nontechnical readers, consistent structure, exact UI language, and maintained images/components.
8. **Actionable issue reports.** Bug guidance asks for expected versus actual behavior, reproduction steps, versions/platform context, and appropriate private/security reporting paths.
9. **Questions become shared knowledge.** Contributors are asked to research first, identify the precise blocker, give enough context, and summarize resolutions where others can learn.
10. **Documentation participates in review.** Documentation changes are rendered, tested, and presented for efficient review alongside code.

## Eitmad adaptations

- Replace Zulip's web-stack organization with vertical Rust capability ownership and typed IPC boundaries.
- Make user help Arabic-first and make bilingual search terminology a first-class requirement.
- Add explicit ReBAC, scope, audit, zero-trust, sync mode, offline, update, and ledger-grade requirements.
- Keep docs-engine selection open. The skill specifies content and quality behavior, not Sphinx, Astro, or Read the Docs.
- Add issue-search metadata and symptom-to-owner troubleshooting paths because Eitmad must support future maintainers and operational staff.

## Primary sources

- [Documentation systems](https://zulip.readthedocs.io/en/stable/documentation/overview.html)
- [Contributing guide](https://zulip.readthedocs.io/en/stable/contributing/contributing.html)
- [Contributor documentation index](https://zulip.readthedocs.io/en/stable/contributing/index.html)
- [Writing help center articles](https://zulip.readthedocs.io/en/stable/documentation/helpcenter.html)
- [Testing overview](https://zulip.readthedocs.io/en/stable/testing/testing.html)
- [Reporting bugs](https://zulip.readthedocs.io/en/stable/contributing/reporting-bugs.html)
- [Asking great questions](https://zulip.readthedocs.io/en/stable/contributing/asking-great-questions.html)
- [Zulip documentation sources](https://github.com/zulip/zulip/tree/main/docs)
