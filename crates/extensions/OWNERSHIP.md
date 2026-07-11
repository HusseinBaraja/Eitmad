# Extensions ownership

**Owner:** Rust extension-host and security maintainers.

This crate owns versioned extension capabilities, lifecycle, isolation boundaries, resource limits, permission checks, and host-mediated access to product functions.

Extensions never receive direct database, config, secret, network, or shell authority. Provider- or feature-specific extension implementations belong in named vertical modules, not in this seam.

Changes require threat-model review, capability-denial, isolation, compatibility, resource-limit, and audit tests.

