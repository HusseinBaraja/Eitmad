# Configuration ownership

**Owner:** Rust configuration maintainers.

This crate owns the setting registry, defaults, validation, schema version, redaction, optimistic patch orchestration, and deterministic import/export. It composes narrow authorization and storage capabilities but does not expose database handles.

Native shells may consume generated snapshots, typed patch contracts, and change events. They may not copy defaults or validators, persist product configuration, import/export values directly, or handle raw secrets.

Changes require focused default, validation, revision, no-op, idempotency, redaction, import/export, UTF-8, authorization, audit, persistence, and event tests plus the canonical configuration documentation workflow.
