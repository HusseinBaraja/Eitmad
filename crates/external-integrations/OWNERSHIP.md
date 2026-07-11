# External service adapter ownership

**Owner:** Rust integration and security maintainers.

This crate is the composition boundary for approved external providers. Each provider must live in a separately named vertical module containing its contract mapping, authentication, retry, rate-limit, redaction, and tests.

It must not become a generic `services` bucket. Shells and plugins cannot call providers or hold provider secrets directly; domain verticals consume narrow provider capabilities.

Each adapter requires failure, timeout, retry-safety, credential, redaction, scope, sandbox, and contract tests.

