# Issue search and troubleshooting

## Contents

- Goal
- Search vocabulary
- Investigation workflow
- Troubleshooting page contract
- From defect to durable knowledge
- Issue report contract

## Goal

Make a future investigator able to search what they observe and reach the owning capability, exact diagnostic evidence, safe recovery, and relevant tests without scanning the whole repository.

## Search vocabulary

Index useful pages with:

- exact Arabic UI text and common Arabic symptom wording when the product exposes them;
- English engineering term and accepted synonyms;
- exact error ID, event name, command/query/subscription name, config key, and executable name;
- affected role, platform, product mode, and lifecycle state;
- authoritative vertical module/crate and server plane;
- safe log field names or diagnostic correlation identifiers;
- legacy term only when people will realistically search it.

Do not add vague keyword stuffing. Put the strongest terms in the title, description, headings, frontmatter `keywords`, and natural text.

## Investigation workflow

1. Capture expected behavior, actual behavior, exact steps, version/capabilities, platform, product mode, scope type, and timing.
2. Search exact UI text and error IDs with `rg`. Then search contract variants, state enum names, and Arabic/English glossary terms.
3. Locate the owning vertical through subsystem indexes and public module boundaries.
4. Trace the command/query/subscription across Rust authority, storage, sync/server, and thin shell adapters.
5. Identify the earliest state divergence. Separate root cause from later symptoms.
6. Reproduce with synthetic scoped data. Preserve privacy and never copy production secrets.
7. Find or add a focused test that fails for the observed behavior.
8. Verify the fix and recovery path, including denial, retry, partial failure, offline, and Arabic cases that apply.
9. Update the canonical subsystem/reference page and add a troubleshooting entry when the diagnostic path is reusable.
10. Cross-link the issue, tests, error reference, and troubleshooting page when repository policy permits.

Useful local searches:

```powershell
rg -n --hidden --glob '!target/**' 'EXACT_ERROR_ID|exact Arabic message'
rg -n --hidden --glob '!target/**' 'CommandName|QueryName|SubscriptionName'
rg -n --hidden --glob '!target/**' 'state_variant|config_key|capability_name'
```

## Troubleshooting page contract

Every troubleshooting page states:

- observable symptoms and exact identifiers;
- affected versions, platforms, modes, roles, and scope;
- data-safety impact and whether work may continue;
- ordered non-destructive checks;
- a decision table mapping evidence to likely cause;
- safe resolution, verification, rollback, and retry behavior for each cause;
- diagnostic fields safe to collect;
- secrets and personal/cross-scope data that must be redacted;
- escalation boundary and owning capability;
- related reference, subsystem, operations, and issue links.

Prefer this decision table:

| Evidence | Likely cause | Next safe check | Resolution |
| --- | --- | --- | --- |
| Exact observable fact | Narrow cause | Non-destructive discriminator | Linked steps |

Never advise destructive repair before backup/rollback requirements and authoritative ownership are clear.

## From defect to durable knowledge

After resolving a defect, document only transferable knowledge:

- the invariant that was misunderstood or violated;
- the stable symptom/error and diagnostic discriminator;
- the safe recovery or prevention step;
- the regression test and owning capability;
- any new observability or error contract.

Do not paste issue discussion chronology into active documentation. Preserve durable rationale in a decision record and historical detail in the issue.

## Issue report contract

A high-quality report contains:

- concise task/symptom title with exact error ID when available;
- expected and actual behavior;
- minimal reproducible steps using synthetic data;
- app/engine/shell versions and negotiated capabilities;
- OS and shell version;
- local-first/server-authoritative mode and offline state;
- affected role and scope type without identifying real people or customers;
- timestamp/timezone and correlation ID when safe;
- screenshots or short recordings when visual evidence matters;
- sanitized logs limited to the relevant interval;
- attempted checks and their results;
- regression or frequency information;
- explicit security/private-data handling route when public reporting is unsafe.

An issue template should link directly to the relevant troubleshooting and privacy guidance.
