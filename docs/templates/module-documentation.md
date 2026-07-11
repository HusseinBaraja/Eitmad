# Module: {product capability or infrastructure boundary}

> - Category: Reference
> - Status: Proposed | Active | Deprecated | Historical
> - Owner: {team or role}
> - Last verified: YYYY-MM-DD
> - Review trigger: {public API, ownership, invariant, dependency, or lifecycle changes}

## Purpose and ownership

State the product capability or infrastructure boundary this module owns, why it exists, and what it MUST NOT own. Prefer a vertical product concept over a technical-layer bucket.

## Location and architecture placement

- Rust crate/module: `{path}`
- Native adapters: `{paths or None}`
- Server planes: `{names or None}`
- Authority: `{authoritative component}`

## Public surface

List deliberate exports, commands, queries, subscriptions, events, errors, capabilities, and versioning rules. Keep internals private or `pub(crate)` unless another vertical has a legitimate dependency.

## Invariants

List rules callers and maintainers MUST preserve, including scope, authorization, audit, consistency, and Arabic behavior.

## Data and storage

List owned records, migrations, indexes, retention, transaction boundaries, and audit behavior, or state `Not applicable` with a reason.

## Sync and lifecycle

Describe sync mode, offline behavior, startup, shutdown, background work, cancellation, retry, and recovery, or state `Not applicable` with a reason.

## Security boundary

List trusted inputs, untrusted inputs, required authorization, secrets, redaction, and denied operations.

## Arabic UX boundary

List owned message identifiers, terminology, normalization rules, RTL or bidirectional constraints, and shell responsibilities.

## Dependencies

| Dependency | Direction | Reason | Contract |
| --- | --- | --- | --- |
| `{module or system}` | Inbound | `{why it calls this module}` | `{public API}` |
| `{module or system}` | Outbound | `{why this module calls it}` | `{public API}` |

## Failure modes

List failures exposed to callers, retry safety, degraded behavior, diagnostics, and recovery.

## Tests

List colocated unit tests, cross-module integration tests, contract tests, Arabic cases, and clean verification commands.

## Tradeoffs and extension points

Record deliberate limitations, rejected abstractions, stable extension seams, and rules for adding exports or dependencies.

## Related documentation

Link the owning feature documents, ADRs, how-to guides, and exact contract reference.
