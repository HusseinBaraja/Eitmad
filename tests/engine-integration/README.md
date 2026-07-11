# Engine integration tests

Own process lifecycle, authenticated IPC, command/query/subscription, persistence/audit atomicity, cancellation, and recovery coverage.

Arabic-first coverage verifies that validation and command failures preserve entered and durable state, return stable structured outcomes, and keep Arabic or mixed-direction values unchanged across IPC, storage, audit, restart, retry, and recovery. Domain integration tests own normalization, parsing, ranking, stable sorting, scope filtering, and locale-policy inputs when those behaviors cross Rust modules.
