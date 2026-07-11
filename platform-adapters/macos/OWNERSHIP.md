# macOS platform adapter ownership

**Owner:** macOS platform maintainers.

Owns macOS-specific process, IPC, notification, picker, print, keychain-prompt, deep-link, updater, entitlement, sandbox, and lifecycle mechanics. The native shell consumes these adapters without gaining product authority.

Tests cover supported macOS versions, RTL and mixed input surfaces, accessibility integration, secure local IPC, notarized update flow, sandboxing, and process recovery.

