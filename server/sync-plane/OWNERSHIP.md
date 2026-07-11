# Sync plane ownership

**Owner:** Server synchronization maintainers.

This plane owns the server-side sync protocol, authorized scoped exchange, durable checkpoints, ordering where required, deduplication, and server-authoritative coordination.

It does not invent a second sync semantic or silently resolve domain conflicts. Changes require protocol compatibility, partial-transfer, replay, scope, authorization, load, and disaster-recovery tests.

