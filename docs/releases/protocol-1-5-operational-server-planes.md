---
title: "Upgrade to protocol 1.5 operational server planes"
description: "Roll out WAN relay metadata, signed update manifests, administration contracts, migration 3, generated bindings, and new host configuration."
audience: "support"
page_type: "release"
status: "active"
owner: "release maintainers"
last_verified: "2026-08-24"
review_triggers:
  - "protocol 1.5 contracts, server migration 3, update key configuration, or rollback policy changes"
keywords:
  - "protocol 1.5"
  - "server.admin-foundation.v1"
  - "signed update manifest"
  - "WAN relay"
---

# Upgrade to protocol 1.5 operational server planes

Protocol `1.5` adds typed relay, signed update distribution, and administration contracts while keeping protocol `1.4` sync peers compatible. The server protocol negotiation range is `1.4–1.5`; new clients use generated `1.5` types and capabilities for the new HTTP surfaces.

## Added contracts and capabilities

The Rust catalog adds:

- `eitmad.capability.server-relay.v1`;
- `eitmad.capability.server-update-distribution.v1`;
- `eitmad.capability.server-administration.v1`;
- relay session, route, health, reconnect, and failure contracts;
- signed manifest, rollout, compatibility, package, client-profile, and outcome contracts;
- diagnostic, backup, migration, visibility, audit, and support-workflow contracts;
- narrow relay, publication, and administration permission and error identifiers.

Generated JSON Schema, protocol catalogs, conformance fixture, C# bindings, Swift bindings, and generated reference move with the Rust source.

## Storage and configuration changes

Server migration `3`, `server.admin-foundation.v1`, creates forced-RLS `operations.backup_status` and `operations.support_workflows`. It requires migrations `1` and `2`.

The combined host now requires `EITMAD_SERVER_UPDATE_MANIFEST_DIRECTORY`, `EITMAD_SERVER_UPDATE_KEY_ID`, and `EITMAD_SERVER_UPDATE_PUBLIC_KEY`. The public key is base64-encoded Ed25519 verification material. The server does not accept a signing private key.

The process-wide PostgreSQL budget is divided across control, sync, and administration pools. `EITMAD_SERVER_MAX_CONNECTIONS` now accepts `3–192`; each pool receives `max_connections / 3`, with a floor of one.

## Rollout

1. Back up PostgreSQL and verify restore in a safe environment.
2. Deploy generated `1.5` bindings to clients that will use the new planes.
3. Create a dedicated manifest directory with service-account write access and no shell-user write access.
4. Configure the reviewed key ID and Ed25519 public key.
5. Run `eitmad-server migrate`. Verify migration versions `1`, `2`, and `3` and forced RLS.
6. Start the host. Verify authentication denial on relay and admin routes, tenant-owner administration, signed manifest rejection after a synthetic byte change, and exact tenant device visibility.
7. Publish first to `canary`, then `beta`, then `stable` with explicit staged-rollout metadata.

## Compatibility and rollback

Protocol `1.4` remains valid for the existing server WebSocket sync boundary. A `1.4` client cannot rely on the new `1.5` types or routes. Do not translate relay, administration, or manifest records in a platform adapter.

Migration `3` is append-only. Rollback may stop using the new routes and return to a compatible server binary only if that binary tolerates the existing migration registry. Do not drop `operations` tables or edit the migration checksum. Preserve signed manifest files and PostgreSQL together for investigation. A client that installed a newer package follows native platform rollback policy; deleting a manifest does not uninstall it.

## Verify

Run:

```powershell
cargo test -p eitmad-relay-plane -p eitmad-update-policy -p eitmad-update-plane -p eitmad-admin-plane -p eitmad-control-plane -p eitmad-server
```

```powershell
npm run contracts:verify --prefix crates/contracts/codegen
```

Then run the full workspace, strict Clippy, server configuration check, and documentation audit. Live PostgreSQL backup, restore, RLS, and route exercises remain deployment-environment requirements.

Related pages: [server operations](../operations/run-server-authority.md), [server-plane troubleshooting](../troubleshooting/server-plane-failures.md), [ADR-0026](../decisions/0026-compose-authorized-operational-server-planes.md), and [contract evolution](../api/evolve-contracts-compatibly.md).
