# ADR-0012: Make observability privacy-preserving by construction

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-07-11
> - Decision owners: Architecture and security maintainers
> - Last verified: 2026-07-11
> - Review trigger: Data classification, support workflows, telemetry destinations, or regulatory obligations change
> - Supersedes: None
> - Superseded by: None

## Context and problem

The system needs enough operational evidence to diagnose failures across shells, engines, sync, updates, and background jobs. Raw payloads, customer records, secrets, permission graphs, and unredacted errors would turn logs and support bundles into a second sensitive database.

## Decision drivers

- Actionable diagnostics with minimal sensitive data
- Consistent correlation across processes and sync
- Bounded collection, retention, and access

## Considered options

- **Structured, privacy-preserving observability:** useful signals with explicit schemas and redaction.
- **Verbose payload logging:** easy debugging, but unacceptable privacy and breach risk.
- **Minimal or no observability:** reduces collection, but makes reliability and support unmanageable.

## Decision

Rust MUST own observability policy, schemas, classification, redaction, sampling, correlation, retention intent, and diagnostic behavior. Collection is off or minimal by default according to deployment policy and limited to necessary operational data. Shells MAY emit approved platform health signals through a narrow contract. Product payloads, customer data, secrets, credentials, authorization graphs, raw database values, and unredacted errors MUST NOT enter telemetry or routine logs.

## Architecture and contract impact

Events use stable identifiers, severity, monotonic and wall-clock context where needed, correlation and causation identifiers, component version, and privacy classification. Support bundles are explicit, inspectable, bounded exports rather than raw directory archives.

## Storage and sync impact

Diagnostic storage has separate quotas, retention, access, and deletion behavior. Operational correlation does not become authoritative business history; audit records remain distinct.

## Security impact

Observability endpoints, files, exports, and support actions are authenticated, authorized, encrypted where appropriate, scope-safe, and auditable. Redaction occurs before data leaves its trusted origin.

## Arabic UX impact

User-visible diagnostics and consent or export flows are localized and RTL-correct. Canonical diagnostic identifiers remain language-neutral; Arabic text is not collected merely to improve readability.

## Consequences and tradeoffs

### Positive

- Operators can correlate failures without duplicating sensitive product data.
- Privacy and storage bounds are testable.

### Negative

- Some defects are harder to diagnose without payloads; targeted diagnostics require deliberate, approved schemas.

### Risks

- Developers may add sensitive fields accidentally; mitigate with allowlisted schemas, safe wrappers, field review, automated secret/data scans, and adversarial tests.

## Verification

Foundation review confirms the sensitive-observability anti-pattern and Rust ownership. Production CI must test redaction, prohibited fields, bounds, access control, and support-bundle inspection.

## Follow-up work

- Define data classification, event schema, retention bounds, and support-bundle threat model.
- Select telemetry destinations only after privacy and deployment requirements are approved.

## Related decisions and documents

- [Target architecture: Security model](../architecture/target-architecture.md#security-model)
- [ADR-0009](0009-zero-trust-security-model.md)
- [ADR-0004](0004-headless-engine-mode.md)
