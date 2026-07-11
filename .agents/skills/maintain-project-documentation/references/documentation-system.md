# Eitmad documentation system

## Contents

- Principles
- Audience collections
- Information architecture
- Page metadata
- Source-of-truth rules
- Change-impact rules
- Language and directionality

## Principles

Use Zulip's useful system properties: documentation-based onboarding, explicit audiences, deep subsystem coverage, strong navigation, previewable Markdown, and automated documentation tests. Adapt them to Eitmad instead of reproducing Zulip's tooling or page hierarchy.

Optimize for five outcomes:

1. A new contributor finds the correct starting point.
2. A maintainer identifies the authoritative module and invariant before editing.
3. An operator completes a procedure and knows how to recover from failure.
4. A support engineer searches a symptom or error and reaches a diagnostic path.
5. A user completes an Arabic-first workflow without understanding implementation details.

## Audience collections

Use this target structure when rebuilding `docs/`:

```text
docs/
  index.md                    # audience router and search entry point
  user/                       # Arabic-first product help
    index.md
  developer/                  # contributor onboarding and engineering practice
    index.md
    setup/
    contributing/
    subsystems/               # one page tree per vertical capability
    testing/
  operations/                 # deployment, backup, restore, update, monitoring
    index.md
  api/                        # generated and explanatory IPC/external contracts
    index.md
  architecture/               # system overview, boundaries, cross-cutting models
    index.md
  troubleshooting/            # symptom/error-led diagnosis and recovery
    index.md
  decisions/                  # immutable/superseded decision records
    index.md
  releases/                   # compatibility, migration, and upgrade notes
    index.md
  glossary.md                 # Arabic/English canonical product language
  _includes/                  # deliberately reused fragments
  _generated/                 # generated reference; never hand-edit
```

Do not create all directories in advance. Add a collection when it has real content and an accountable owner.

Audience boundaries:

| Collection | Primary reader | Main question |
| --- | --- | --- |
| `user` | Eitmad role using the app | How do I complete my work? |
| `developer` | Contributor or maintainer | How does this work, and how do I change it safely? |
| `operations` | Installer or operator | How do I run, observe, recover, and update it? |
| `api` | Shell/client/integration developer | What exact contract can I rely on? |
| `architecture` | Designer or senior maintainer | Why are the system boundaries this way? |
| `troubleshooting` | User, support, developer, or operator | What does this symptom mean, and how do I recover? |
| `decisions` | Future decision-maker | What durable choice was made and why? |
| `releases` | Operator, support, and maintainer | What changed, what is compatible, and what must migrate? |

## Information architecture

- Make `docs/index.md` a short audience router, not a flat file inventory.
- Give every collection an index that offers common tasks, concepts, and troubleshooting routes.
- Organize developer subsystem pages by product capability, not `models`, `services`, `handlers`, or other technical layers.
- Keep one canonical page per subject. Add contextual links from other collections.
- Prefer updating a discoverable existing page when a small feature does not justify a new page.
- Use stable lowercase hyphenated paths. Avoid dates in active page paths.
- Preserve old inbound links with redirects when the site tool supports them; otherwise leave a short moved-page stub during migration.
- End task pages with verification and related next steps. End reference pages with ownership and related concepts.

## Page metadata

New Markdown pages use YAML frontmatter:

```yaml
---
title: "Recover a stalled synchronization"
description: "Diagnose queued, conflicted, and failed synchronization without losing local work."
audience: "operations"
page_type: "troubleshooting"
status: "active"
owner: "sync maintainers"
last_verified: "YYYY-MM-DD"
review_triggers:
  - "sync state machine changes"
keywords:
  - "المزامنة"
  - "sync stalled"
  - "SYNC_CHECKPOINT_REJECTED"
---
```

Allowed `audience`: `user`, `developer`, `operations`, `api`, `architecture`, `support`.

Allowed `page_type`: `tutorial`, `task`, `reference`, `explanation`, `troubleshooting`, `decision`, `release`.

Allowed `status`: `draft`, `active`, `deprecated`, `historical`.

Templates and generated files may omit metadata. Index pages still need `title`, `description`, `audience`, `status`, `owner`, and `last_verified` once the new system is adopted.

## Source-of-truth rules

| Claim | Canonical evidence |
| --- | --- |
| Domain behavior and validation | Owning Rust vertical plus behavior tests |
| IPC shape and compatibility | Rust-owned typed contract and generated/validated binding |
| Storage schema and migration | Rust-owned migration/schema source and migration tests |
| Authorization | Rust policy implementation and denied-path tests |
| UI steps and Arabic copy | Current native shell plus UI tests or rendered verification |
| Configuration | Rust config schema/default source |
| Operational command | Current executable behavior in a safe environment |
| Architecture rationale | Accepted decision record |
| Known issue resolution | Reproduced evidence, fix tests, and verified recovery |

Never cite an old document as sole proof of current behavior. Mark discrepancies and repair the canonical page.

## Change-impact rules

Update documentation with any change to:

- user-visible workflow or Arabic terminology;
- public command, query, subscription, error, version, or capability;
- authorization, scope, audit, privacy, or redaction behavior;
- record lifecycle, schema, migration, backup, restore, or retention;
- sync semantics, offline behavior, retry, conflicts, or recovery;
- installation, update, compatibility, diagnostics, or rollback;
- module ownership, invariant, extension boundary, or durable architecture;
- a failure whose diagnosis would help the next investigator.

Documentation-only changes may stand alone. Behavior changes must not defer required docs to an unspecified later task.

## Language and directionality

- Make user help canonical in Arabic. Add English only when a real audience needs it.
- Keep Rust names, commands, paths, error IDs, and configuration keys exact and LTR.
- Define each product term once in the bilingual glossary. Include local workshop synonyms only as search aliases unless approved as canonical.
- Wrap mixed-direction UI examples using the documentation renderer's bidi isolation support. If none exists, prefer separate lines or tables over ambiguous inline mixing.
- Test Arabic shaping, numeral display, punctuation, filenames, dimensions, and copy/paste in the rendered output.
- Use synthetic names and records. Never put real customer or employee data in examples.
