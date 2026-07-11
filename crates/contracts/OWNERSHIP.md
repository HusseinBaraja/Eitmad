# Contracts ownership

**Owner:** Rust core maintainers.

This crate owns canonical commands, queries, subscriptions, errors, identity, permissions, sync, update, version, and capability contracts. It also owns binding generation or mechanical validation inputs.

It does not execute domain behavior, persist data, or contain shell-specific presentation models. Consumers depend on its deliberate public surface; they must not copy schemas by hand.

Changes require compatibility tests, Arabic and mixed-direction text fixtures where user data is present, and a documented versioning decision.

