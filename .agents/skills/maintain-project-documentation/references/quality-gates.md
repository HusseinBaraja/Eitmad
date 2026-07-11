# Documentation quality gates

## Contents

- Evidence gate
- Structure gate
- Search gate
- Safety and architecture gate
- Arabic gate
- Rendering and automation gate
- Review evidence
- Migration strategy

## Evidence gate

- Every current-behavior claim is supported by code, contract, schema, test, configuration, or verified runtime evidence.
- Every runnable command was executed safely or labeled unverified with a reason.
- Generated reference names its source and regeneration/check command.
- `last_verified` changes only after evidence review.

## Structure gate

- The page has one primary audience and reader task.
- Title, description, metadata, headings, and filename agree.
- The closest collection index links to it.
- It links to prerequisites, exact reference, troubleshooting, and next steps as applicable.
- It updates an existing canonical page instead of duplicating facts.
- Removed or renamed pages preserve discoverability with redirects or migration stubs.

## Search gate

- Search succeeds using likely Arabic symptom text.
- Search succeeds using the English engineering term.
- Exact error IDs, contract names, configuration keys, and state terms are present where relevant.
- Results lead from symptom to owning capability and recovery.
- Page descriptions distinguish near-duplicate search results.

## Safety and architecture gate

- Rust is named as authority for domain, contracts, storage, authorization, sync, update policy, and validation.
- Native shells remain thin adapters and do not gain duplicated schemas or business rules through documentation examples.
- Every state-changing flow covers identity, ReBAC, scope, audit, denial, idempotency, partial failure, and recovery as applicable.
- Every record example has an explicit synthetic scope.
- No secrets, real records, unsafe logs, or destructive commands without safeguards appear.
- Cross-boundary examples cover version/capability negotiation and compatibility.

## Arabic gate

- User help is Arabic-first and uses approved glossary terms.
- UI labels match the current native shell.
- RTL, mixed-direction text, numerals, dimensions, identifiers, and copy/paste were rendered and checked.
- Arabic search variants and normalization behavior are documented and tested where relevant.
- Images have useful Arabic alt text and no personal data.

## Rendering and automation gate

At minimum automate:

- YAML metadata shape and allowed values;
- missing files, broken relative links, and broken anchors;
- duplicate headings/anchors;
- unreachable active pages;
- stale `last_verified` dates based on risk;
- forbidden secrets or obvious real-data patterns;
- placeholder markers in active pages;
- generated-reference drift;
- documented command/contract/error names against authoritative sources where practical;
- full site build and search index generation.

Keep local checks fast and deterministic. Run external-link checks separately so network failures do not make core documentation validation flaky. Publish a rendered preview for documentation changes when hosting supports it.

## Review evidence

Record:

- authoritative sources inspected;
- focused code and documentation checks run;
- rendered pages/platforms checked;
- Arabic/RTL and accessibility checks performed;
- commands actually exercised;
- unresolved gaps and owner;
- next review trigger.

Screenshots help reviewers evaluate visual documentation changes but do not replace rendered checks.

## Migration strategy

Existing project documentation is legacy. Rebuild without a flag day:

1. Establish the new `docs/index.md`, metadata contract, and collection indexes.
2. Select one high-value vertical capability and migrate its user, developer, reference, and troubleshooting paths end to end.
3. Add validation for newly migrated pages first.
4. Mark legacy pages clearly and map each to keep, rewrite, merge, generate, supersede, or delete.
5. Preserve useful inbound links during moves.
6. Expand checks as each collection becomes compliant.
7. Remove legacy exemptions only when their pages are migrated.

Do not mechanically reformat inaccurate legacy text. Re-establish claims from authoritative evidence.
