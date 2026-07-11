# Cross-boundary test ownership

**Owner:** Quality engineering and the maintainers of each affected boundary.

This area owns tests that cross crate, process, platform, or deployment boundaries. Unit and feature tests stay beside their owning vertical. Cross-boundary fixtures must use representative Arabic, Latin, mixed-direction, scope, permission, offline, and failure data.

Suites are grouped by behavior: contract compatibility, engine integration, shell conformance, security boundaries, sync protocol, and end-to-end workflows. This directory must not become a mirror of implementation layers.

All suites start from the canonical [mixed-language scenarios](../docs/architecture/arabic-first-ux.md#shared-mixed-language-scenarios). Each owning feature adds only the fixtures needed for its domain and preserves original values so normalization, rendering, copy/paste, audit, and serialization assertions can be compared without presentation controls contaminating canonical data.
