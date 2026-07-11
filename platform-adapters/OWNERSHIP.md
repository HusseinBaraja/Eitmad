# Platform adapter ownership

**Owner:** Native platform maintainers.

Platform adapters own replaceable OS mechanics requested through typed capabilities: engine process supervision, authenticated local transport, notifications, file pickers, credential prompts, printing, deep links, installers, and lifecycle signals.

They do not own domain rules, product schemas, storage, sync, authorization, external APIs, secrets, or update eligibility. Each adapter is tested against Rust-owned contracts and shell conformance suites.

