# Engine runtime ownership

**Owner:** Rust engine maintainers.

This crate owns engine startup, shutdown, authenticated IPC lifecycle, command/query/subscription dispatch, capability negotiation, background-job supervision, and composition of product verticals.

It coordinates focused crates but does not absorb their business rules. Native shells may supervise this process; they may not bypass it or become an authority.

Runtime changes require lifecycle, compatibility, cancellation, failure-recovery, and clean-shutdown tests.

