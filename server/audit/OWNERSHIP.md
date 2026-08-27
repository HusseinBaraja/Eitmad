# Server audit ownership

`eitmad-server-audit` owns the canonical server audit envelope, append interface, append-only PostgreSQL schema upgrade, and migration checksum. Control, sync, relay, update, and administration modules supply operation-specific scope and target metadata through this interface. They must not define another server audit record shape or write `audit.server_records` directly.
