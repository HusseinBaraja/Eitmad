# Windows platform adapter ownership

**Owner:** Windows platform maintainers.

Owns Windows-specific process, IPC, notification, picker, print, credential-prompt, protocol-registration, installer, elevation, and lifecycle mechanics. `ProcessSupervision` implements Job Object containment, bounded retry, stale-event rejection, and graceful-first engine shutdown. The C# shell consumes these adapters without gaining product authority.

Tests cover supported Windows versions, RTL and mixed input surfaces, accessibility integration, secure local IPC, interrupted installation, intentional stop, unexpected engine death, restart exhaustion, stale exit events, clean shutdown, and forced process-tree recovery.
