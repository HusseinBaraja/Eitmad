---
title: "Keep documentation current"
description: "Review documentation when its evidence or triggers change, and mark stale content before it becomes unsafe guidance."
audience: "developer"
page_type: "reference"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "documentation governance, language policy, risk tiers, or release process changes"
keywords:
  - "documentation freshness"
  - "last_verified"
  - "review trigger"
---

# Keep documentation current

Documentation changes with the behavior it describes. Update the canonical page in the same logical change; a date alone is not evidence of review.

## Page metadata

Every non-generated Markdown page uses YAML frontmatter with `title`, `description`, `audience`, `page_type`, `status`, `owner`, `last_verified`, and explicit review triggers. Use the allowed metadata values in `.agents/skills/maintain-project-documentation/SKILL.md` when working locally. Do not create a Markdown link that escapes the `docs/` root.

## Review timing

- when any declared `review_trigger` occurs;
- when code, contract, schema, configuration, or test evidence changes;
- before a release depends on a documented procedure or compatibility claim;
- every 180 days for active reference, task, and explanation pages;
- every 90 days for security, recovery, update, migration, and critical operations pages.

Rerun tutorials before every supported major release and when prerequisites or UI flows change. Do not rewrite an accepted ADR; create a new decision that supersedes it.

## Review method

Compare the page with current sources, run safe commands, and inspect links, anchors, diagrams, and terminology. Documentation prose must remain English. Where the page contains Arabic UI evidence, verify exact labels, Arabic shaping, RTL, and bidirectional cases. Change `last_verified` only after inspecting evidence.

## Stale content

Mark an obsolete operational procedure `deprecated` immediately and link its replacement. Withdraw incorrect security, migration, or recovery guidance instead of leaving it usable. Keep content `historical` only when it preserves useful context.

Next, [review the quality gate](documentation-review-checklist.md).
