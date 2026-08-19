# Secret storage ownership

**Owner:** Rust security and platform-integration maintainers.

This crate owns typed secret lifecycle operations, OS-native credential-store adapters, encrypted fallback persistence, memory redaction, and sanitized failures.

Native shells may pass non-secret references through approved contracts. They may not read, cache, serialize, log, or persist secret material. Product command boundaries remain responsible for authorization and audit before calling this internal capability.

Changes require lifecycle, invalid-identifier, native/fallback selection, encryption, file-permission, corruption, recovery, redaction, and accidental-leakage tests on every supported desktop platform.
