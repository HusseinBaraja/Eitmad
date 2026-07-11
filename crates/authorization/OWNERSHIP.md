# Security and authorization ownership

**Owner:** Rust security maintainers.

This crate owns authenticated principals, device and service identity, explicit scopes, relationship tuples, policy evaluation, capability grants, and deny-by-default authorization decisions.

It does not own login UI, OS credential prompts, or business workflows. Every authoritative command and query is checked here or through an equally explicit Rust-owned domain guard.

Changes require threat-model review plus denial, revocation, cross-scope isolation, confused-deputy, and audit-correlation tests.

