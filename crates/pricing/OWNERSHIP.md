# Pricing ownership

Rust owns whole-YER arithmetic, discount policy, cost and margin projections,
variant pricing, immutable confirmed price revisions, publication intents,
authorization, and atomic audit. Product and Furniture definitions remain in
their separate capabilities. Server confirmation is mandatory for publication.

See [Pricing](../../docs/developer/subsystems/pricing.md) for the public boundary
and verification commands.

Quotation evaluation and durable draft saves share the public evaluator and a
single SQLite transaction. `drafts.rs` owns branch draft commands, queries,
permissions, audit, and local-first publication. `draft_sync.rs` connects that
outbox to the shared schema-specific sync engine. Issuance, approvals, and
numbering are separate unimplemented authority boundaries. See
[durable quotation drafts](../../docs/developer/subsystems/quotations.md#durable-quotation-drafts).
