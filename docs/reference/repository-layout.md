# Repository layout and ownership

> - Category: Reference
> - Status: Active
> - Owner: Engineering maintainers
> - Last verified: 2026-07-11
> - Review trigger: Workspace area, authority boundary, or ownership changes

## Purpose

This repository starts with explicit authority and change boundaries. Focused product verticals will be added beside these foundation areas; broad `utils`, `common`, `shared`, `handlers`, or `services` buckets are prohibited.

## Layout

| Area | Authority |
| --- | --- |
| `crates/contracts/` | Canonical Rust-owned external contracts and binding inputs |
| `crates/engine-runtime/` | Authoritative engine lifecycle and orchestration |
| `crates/engine-cli/` | Headless and diagnostic process entry |
| `crates/storage/` | Database, migrations, transactions, backup boundaries |
| `crates/sync/` | One transport-independent synchronization protocol |
| `crates/authorization/` | Identity, scope, and relationship authorization |
| `crates/update-policy/` | Update eligibility, compatibility, and migration safety |
| `crates/observability-audit/` | Redacted diagnostics and mutation audit |
| `crates/external-integrations/` | Named, engine-owned external provider adapters |
| `crates/extensions/` | Permission-gated plugin and extension host seams |
| `platform-adapters/` | Replaceable native OS mechanics |
| `shells/windows/` | Thin C# Windows UI shell |
| `shells/macos/` | Thin native macOS UI shell |
| `shells/linux/` | Thin native Linux UI shell |
| `server/*-plane/` | Separable control, sync, relay, update, and admin planes |
| `tests/` | Cross-boundary suites; unit tests stay with their vertical |
| `deploy/` | Executable packaging and deployment assets |
| `docs/deployment/` | Operator deployment and recovery procedures |
| `docs/product/` | Product intent, workflows, terminology, and acceptance criteria |

Every area has an `OWNERSHIP.md` or an owning index. New verticals must document purpose, owned behavior, forbidden responsibilities, tests, tradeoffs, and extension seams before production implementation is complete.

