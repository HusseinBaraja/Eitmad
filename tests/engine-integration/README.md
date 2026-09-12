# Engine integration tests

Own process lifecycle, authenticated IPC, command/query/subscription, persistence/audit atomicity, cancellation, and recovery coverage.

Implemented lifecycle process coverage lives in `crates/engine-cli/tests/process_modes.rs`. Authenticated command, query, and subscription IPC coverage lives beside `eitmad-engine-runtime`; durable storage, audit, migration, and recovery coverage lives beside `eitmad-storage`. This directory remains available for a cross-capability scenario that needs one integration owner instead of focused crate coverage.

Arabic-first coverage verifies that validation and command failures preserve entered and durable state, return stable structured outcomes, and keep Arabic or mixed-direction values unchanged across IPC, storage, audit, restart, retry, and recovery. Domain integration tests own normalization, parsing, ranking, stable sorting, scope filtering, and locale-policy inputs when those behaviors cross Rust modules.
