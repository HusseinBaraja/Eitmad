# Observability and audit ownership

**Owner:** Rust reliability and security maintainers.

This crate owns structured diagnostics, redaction policy, correlation identifiers, health signals, and tamper-evident audit outcomes for every state-changing command.

It does not accept raw domain payload logging or make audit optional. Product verticals define safe event fields; this crate enforces common envelopes and sinks.

Changes require redaction, secret-leak, audit-completeness, ordering, failure-path, retention, and Arabic-data tests.

