# macOS shell ownership

**Owner:** macOS UI maintainers.

The macOS shell owns native macOS presentation, accessibility, RTL layout, bidirectional input, navigation, focus, and ephemeral view state. Its UI technology requires a separate recorded decision before implementation.

It delegates authoritative work to the Rust engine and OS mechanics to the macOS platform adapter. Tests cover Arabic-first UI behavior, contract conformance, reconnection, localized failures, and forbidden direct authority.

