# Storage ownership

**Owner:** Rust storage maintainers.

This crate owns database access, migrations, transactions, scope isolation, backup/restore primitives, and persistence health. It exposes narrow persistence capabilities to the owning product verticals.

It does not own domain policy, synchronization conflict policy, or shell caches. No shell, plugin, report, or external adapter may access product storage directly.

Changes require migration, rollback/recovery, corruption handling, scope-isolation, and representative Arabic data tests.

