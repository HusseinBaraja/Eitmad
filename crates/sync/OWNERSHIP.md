# Synchronization ownership

**Owner:** Rust synchronization maintainers.

This crate owns the single versioned sync protocol and shared simulation/LAN/WAN transport interface, including connection authentication/encryption expectations, negotiation, streaming, cancellation, health, reconnect/backoff, checkpoints, idempotency, ordering, tombstones, conflict outcomes, retry safety, and backpressure.

It owns typed non-secret credential references and route policy. It does not own credential material, production socket/TLS implementations, domain-specific merge rules, or UI conflict presentation. Rust secret storage and production connector verticals supply credential and network mechanics; domain verticals supply explicit conflict policy through narrow seams.

Changes require offline, disconnect/reconnect, authentication/encryption, streaming/cancellation, duplicate-delivery, partial-network/relay, authorization, scope, and incompatible-version tests.
