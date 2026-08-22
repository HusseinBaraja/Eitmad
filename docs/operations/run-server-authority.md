---
title: "Run and recover the modular server authority"
description: "Configure, migrate, bootstrap, start, verify, back up, and recover the combined Eitmad server safely."
audience: "operations"
page_type: "task"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-08-22"
review_triggers:
  - "server configuration, CLI, migrations, health routes, TLS, backup, or recovery changes"
keywords:
  - "eitmad-server migrate"
  - "eitmad-server bootstrap"
  - "EITMAD_SERVER_DATABASE_URL"
  - "readyz"
---

# Run and recover the modular server authority

Run the combined control and sync server only with a dedicated PostgreSQL database, a protected token key, and TLS. Development plaintext is permitted only on an explicit loopback bind.

## Before you start

- Use a PostgreSQL role that can create the owned schema during migration but cannot bypass row-level security during normal operation.
- Back up the database before each migration or binary rollback.
- Generate a random 32-byte token key and encode it with URL-safe base64 without padding. Store it in the deployment secret manager. Do not rotate it without a token invalidation plan.
- Provide a certificate and private key for every non-loopback listener.
- Do not use customer credentials or data in command transcripts.

## Configure the process

| Environment variable | Required | Meaning |
| --- | --- | --- |
| `EITMAD_SERVER_DATABASE_URL` | Yes | PostgreSQL connection URL; secret |
| `EITMAD_SERVER_TOKEN_KEY` | Yes | URL-safe base64 32-byte HMAC key; secret |
| `EITMAD_SERVER_LISTEN` | No | Listener, default `127.0.0.1:8443` |
| `EITMAD_SERVER_TLS_CERTIFICATE` | For TLS | PEM certificate path |
| `EITMAD_SERVER_TLS_PRIVATE_KEY` | For TLS | PEM private-key path; secret path |
| `EITMAD_SERVER_ALLOW_INSECURE_LOOPBACK` | Development only | Must be `true` to serve plaintext on loopback |
| `EITMAD_SERVER_MAX_CONNECTIONS` | No | PostgreSQL pool size from 2 through 128; default 16 |

Validate without connecting to PostgreSQL:

```powershell
cargo run -q -p eitmad-server -- check-config
```

Expected output is `server configuration is valid`. Configuration debug output redacts the database URL, token key, and private-key path.

## Migrate before bootstrap or serve

Take a PostgreSQL backup, then run:

```powershell
cargo run -q -p eitmad-server -- migrate
```

Expected output is `server migrations are current`. The command applies both control and sync migrations before it exits. Migration files are immutable after release. Stop if the process emits `eitmad.error.server-database-unavailable.v1` or `eitmad.error.server-migration-failed.v1`; preserve the database and diagnose the PostgreSQL service, permissions, and schema history.

## Bootstrap the first owner

Run this once for a new tenant. Quote display names that contain spaces:

```powershell
cargo run -q -p eitmad-server -- bootstrap al-eitmad "الاعتماد" "مصنع الاعتماد" owner
```

The command prints tenant, organization, and account IDs, one activation token, and its expiry. Capture the activation token through an approved secret channel. It cannot be recovered from PostgreSQL because only its keyed hash is stored. Activate the account through `POST /v1/auth/activate` before the invitation expires.

Do not repeat bootstrap after an uncertain result. First check the tenant and audit state through an approved administrative query or database operator under a documented incident procedure. A repeated tenant code is rejected.

## Start and verify

Start the foreground server:

```powershell
cargo run -q -p eitmad-server -- serve
```

The process applies migrations before it starts listening. Keep migration privileges separate in a hardened deployment even though the combined development binary supports this convenience.

Verify:

1. `GET /livez` returns success when the process can serve requests.
2. `GET /readyz` returns success only when PostgreSQL is reachable.
3. A protocol `1.4` synthetic client can authenticate, send `eitmad.server.hello.v1`, negotiate every required server capability and schema, and then close cleanly.
4. A synthetic tenant cannot read another tenant's scoped records.
5. Logs contain stable error identifiers and no URL credentials, token values, passwords, private keys, signatures, or domain payloads.

Do not route traffic from the load balancer until readiness and tenant-isolation checks pass.

## Backup, restore, and rollback

Use the approved PostgreSQL physical or logical backup procedure and verify the backup can be restored. Include both control and sync schemas in one recovery point so identity, audit, operation positions, snapshots, and checkpoints remain consistent.

For application rollback:

1. Stop inbound traffic and the server process.
2. Confirm whether the older binary supports the applied migration set and protocol `1.4` data.
3. If it does not, restore the complete pre-migration backup. Do not reverse migration SQL by hand.
4. Start one server replica, wait for readiness, then verify authentication, tenant isolation, update assignment, sync pull, snapshot fallback, and subscription resume.
5. Add replicas only after the first replica is healthy.

Do not delete operation history until a complete snapshot covers it and retained client checkpoints no longer need it. Keep at least the configured 90-day history floor.

## Current verification limit

On 2026-08-22, `check-config` passed with synthetic loopback configuration. Rust tests, strict Clippy, minimum-version compilation, and generated contract verification passed. Live `migrate`, `bootstrap`, `serve`, RLS, and HTTP/WebSocket integration could not run because the verification workstation had no PostgreSQL, Docker, or Podman runtime. Complete those checks in the deployment environment before production approval.

For failures, use [Resolve server authentication, tenant, and sync failures](../troubleshooting/server-authentication-and-sync.md). For design ownership, use [the modular server authority](../developer/subsystems/server-authority.md).
