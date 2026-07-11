# Engine CLI and headless ownership

**Owner:** Rust engine maintainers.

This binary owns process entry for headless workflows, diagnostics, health checks, compatibility inspection, and support tooling. It delegates authoritative behavior to the engine runtime and focused product verticals.

It does not create alternate business logic, direct storage shortcuts, or unaudited administrative paths. Interactive output must be localizable where user-facing and safe for support logs.

Changes require exit-code, signal, permission, redaction, non-interactive, and clean-start/stop tests.

