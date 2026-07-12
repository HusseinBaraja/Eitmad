# Windows shell ownership

**Owner:** Windows UI maintainers.

The C# Windows shell owns native Windows presentation, accessibility, RTL layout, bidirectional input, navigation, focus, and ephemeral view state. It delegates authoritative work to the Rust engine and OS mechanics to the Windows platform adapter.

When implemented, shell bootstrap consumes `Eitmad.Platform.Windows.ProcessSupervision.EngineSupervisor`; it must not duplicate restart, Job Object, or engine lifecycle policy.

Bindings are generated or mechanically validated from Rust contracts. Tests cover Arabic-first UI behavior, capability negotiation, reconnection, localized failures, accessibility, and prevention of direct database/config access.
