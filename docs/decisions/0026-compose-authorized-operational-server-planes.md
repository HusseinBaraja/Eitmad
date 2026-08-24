---
title: "ADR-0026: Compose authorized relay, update, and administration planes"
description: "Records how the first combined server deploys relay metadata, signed manifest distribution, and least-privilege administration."
audience: "architecture"
page_type: "decision"
status: "active"
owner: "architecture maintainers"
last_verified: "2026-08-24"
review_triggers:
  - "relay, update, or administration planes separate; signing storage changes; or operational authorization changes"
---

# ADR-0026: Compose authorized relay, update, and administration planes

> - Category: Explanation
> - Status: Accepted
> - Date: 2026-08-24
> - Decision owners: Architecture, server platform, connectivity, release, and operations maintainers
> - Last verified: 2026-08-24
> - Supersedes: None
> - Superseded by: None

## Context

ADR-0025 established separate control, sync, relay, update, and administration crate boundaries in one initial server process. Relay, update artifact distribution, and administration remained placeholders. Implementing them must not create unaudited operational shortcuts, duplicate update policy in a shell, or expose cross-tenant fleet state.

## Decision

Compose all five planes into `eitmad-server` while keeping relay, update, and administration as independent Rust library crates with narrow traits and contracts.

- Keep relay session metadata in bounded process memory. Relay state is connection coordination, not business truth. Use `RelayRouter` for peer and server transport hooks, and preserve opaque sync frames.
- Store signed update manifests as immutable JSON files in a dedicated durable directory. Keep private signing keys outside the server. Configure one trusted Ed25519 public key and key ID in the first deployment. Keep verification and eligibility in `crates/update-policy`.
- Store backup and support-workflow status in PostgreSQL migration `3` under forced tenant RLS. Read control-plane tenant, device, session, migration, and audit evidence through the administration data source.
- Use control-plane relationships as the production authorization source. Tenant members may use normal relay actions. Tenant owners may publish manifests, use administration interfaces, close another device's relay session, and revoke device sessions.
- Append a redacted audit result for every relay and administration action and manifest publication. Keep accepted device-session revocation and its audit in one transaction. Treat cross-plane support workflow plus audit as a reconcilable partial-failure boundary.
- Advance external contracts to protocol `1.5` and generate C# and Swift bindings from Rust.

## Alternatives

### Deploy three new services immediately

This improves process isolation but requires authenticated internal APIs, more credentials, distributed availability, and deployment sequencing before operating scale requires them.

### Store relay sessions durably

This would make expired connection coordination look like product truth and add cleanup and multi-replica complexity. A later multi-replica relay may add a bounded shared presence store without turning relay state into sync history.

### Store signing private keys in the update host

This makes a distribution compromise a signing compromise. The server therefore verifies and hosts externally signed manifests only.

### Add a general administrator role or database console

This creates a superuser bypass. The first interface uses existing tenant ownership and typed, bounded workflows. A future cross-tenant operations role requires a separate relationship and approval decision.

## Consequences

The combined deployment remains practical and gains explicit health, failure, backup, migration, audit, visibility, update, and support surfaces. Generated contracts and stable errors make clients compatible. Tenant isolation has both Rust checks and PostgreSQL RLS for durable administration records.

The combined process has one fault boundary. Relay metadata is lost on restart and clients must reconnect. The first key configuration supports one trusted update key, so safe key rotation needs a reviewed multi-key configuration extension. Support workflow state and the final cross-plane audit are not one distributed transaction and require correlation-based reconciliation after partial failure.

## Security, Arabic UX, and operations

Device proof authenticates the caller but does not grant authorization. Peer relay routes verify target-device tenancy. Administrative data never returns secrets or cross-tenant records. Audit and failure records use stable identifiers without payload text.

No new UI is included. Future Arabic UI localizes stable states and errors, uses `مرحّل المزامنة` only after terminology review, and isolates LTR versions, hashes, UUIDs, and error IDs in RTL layouts.

Operators must configure the manifest directory and trusted key, apply migrations `1`, `2`, and `3`, preserve PostgreSQL backups, and use startup migration commands rather than remote migration retries.

## Verification and follow-up

Focused tests cover relay lifecycle and denial, signature changes, channel and compatibility rules, tenant isolation, backup status, administration authorization, forced RLS, generated bindings, and unauthenticated routes. Full workspace tests, strict Clippy, contract verification, and documentation audit are the release gates.

Follow-up owners:

- Connectivity maintainers: implement a production payload router and multi-replica presence when deployment needs it.
- Release maintainers: add a multi-key trust-ring configuration before key rotation.
- Operations maintainers: connect the approved backup reporter and exercise PostgreSQL backup/restore in deployment CI.
- Security maintainers: design an explicit cross-tenant operator relationship if operational needs justify it.

Related pages: [server authority](../developer/subsystems/server-authority.md), [WAN relay](../developer/subsystems/wan-relay-coordination.md), [update distribution](../developer/subsystems/update-distribution.md), [server administration](../developer/subsystems/server-administration.md), and [ADR-0025](0025-modular-server-authority-foundation.md).
