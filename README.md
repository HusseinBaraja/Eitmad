# الاعتماد | Eitmad

Arabic-first operations system for الاعتماد furniture manufacturing.

This repository is a cross-platform native desktop workspace built around a separate, authoritative Rust engine. Native shells stay thin; Rust owns contracts, product behavior, storage, synchronization, authorization, update policy, observability, audit, and external boundaries.

Start with:

- [Repository policy](AGENTS.md)
- [Target architecture](docs/architecture/target-architecture.md)
- [Repository layout and ownership](docs/reference/repository-layout.md)
- [Documentation index](docs/README.md)

Foundation verification:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -q -p eitmad-engine-cli
```

Production features must be organized by bounded product capability. Do not add generic `utils`, `common`, `shared`, `handlers`, or `services` buckets.
