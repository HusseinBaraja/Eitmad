# Server ownership

**Owner:** Server platform maintainers, with Rust security review.

Server infrastructure is split into control, sync, relay, update, admin, and audit authorities. They may share a deployment initially, but contracts, credentials, privileges, data ownership, and failure boundaries stay separable.

No plane may become a shortcut around Rust-owned contracts, authorization, scope isolation, or audit. Cross-plane calls are authenticated, versioned, bounded, and observable without exposing sensitive payloads.

`server/audit` owns the canonical PostgreSQL audit envelope and append operation. Other planes must not define weaker local audit records or write `control.audit_log` directly.
