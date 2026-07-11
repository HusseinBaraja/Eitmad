# Native shell ownership

**Owner:** Native UI maintainers.

Shells own Arabic-first presentation, navigation, accessibility, input, ephemeral view state, and translation of user intent into typed engine requests. Each shell consumes generated or mechanically validated Rust contracts.

Shells never own business logic, domain validation, product storage or config, sync, authorization, updates policy, external service access, secrets, or authoritative background jobs.

