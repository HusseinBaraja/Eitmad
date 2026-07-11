# Synchronization ownership

**Owner:** Rust synchronization maintainers.

This crate owns the single versioned sync protocol used across LAN and WAN transports, including checkpoints, idempotency, ordering, tombstones, conflict outcomes, retry safety, and backpressure.

It does not own transport credentials, domain-specific merge rules, or UI conflict presentation. Domain verticals supply explicit conflict policy through narrow seams.

Changes require offline, resume, duplicate-delivery, partial-transfer, authorization, scope, and incompatible-version tests.

