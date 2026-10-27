---
title: "Deploy staging and production server environments"
description: "Use repeatable server profiles, TLS boundaries, migrations, channel separation, and rollback controls."
audience: "operations"
page_type: "task"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-21"
review_triggers:
  - "server topology, reverse proxy, TLS, backup, migration, channel, or rollback changes"
keywords:
  - "server deployment profile"
  - "reverse proxy TLS"
  - "PostgreSQL restore"
  - "staging production"
---

# Deploy staging and production server environments

Deploy one immutable `eitmad-server` artifact per release. Keep staging and production in separate trust, data, credentials, update channels, and network boundaries. The combined binary composes control, sync, relay, update, and administration planes, but their privileges and future split points remain separate.

For a new single-workshop Windows LAN host, use [guided server and client installation](install-windows.md). Use the [installed service procedure](run-windows-host-services.md) for advanced import, unattended startup, service identities, readiness, power requirements, and preserved uninstall data. The environment deployment procedure below is a separate operator path.

## Profiles

| Profile | Purpose | Update channel | Data and trust | Availability |
| --- | --- | --- | --- | --- |
| Development | Local diagnostics with synthetic data | `canary` | Loopback plaintext allowed only with explicit flag | One process |
| Staging | Production-like migration and release proof | `beta` | Separate PostgreSQL, secrets, signing trust, DNS, and manifest directory | Same topology class as production where practical |
| Production | Customer operations | `stable` | Dedicated least-privilege roles, secret manager, immutable backups, production CA | Multiple instances only after one instance proves readiness |

Never copy production records or credentials into staging. Use synthetic Arabic and mixed-direction fixtures.

## Reverse proxy and TLS

Prefer TLS termination at a reviewed reverse proxy or load balancer. Bind the server to loopback or a private interface and set `EITMAD_SERVER_ALLOW_INSECURE_LOOPBACK=true` only when the plaintext hop is loopback. For a private network hop, configure TLS on the server too; the runtime rejects non-loopback plaintext.

The proxy must allow HTTP upgrade for WebSocket synchronization, preserve the request path and query, enforce bounded request/header sizes and timeouts, and send traffic only after `/readyz` succeeds. `/livez` proves that the process responds; it does not prove PostgreSQL readiness. Do not trust client-supplied forwarding headers for authorization. Network source data is diagnostic only; authenticated Rust contracts determine identity, tenant, scope, and permission.

Use TLS 1.2 or newer with reviewed ciphers, automatic certificate renewal, expiry alerts, HSTS at the public edge, and no direct public access to PostgreSQL or manifest storage. Redact URLs with credentials, tokens, device proofs, and payloads from proxy logs.

## Deploy and migrate

1. Build the server validation artifact with `python scripts/release/build_artifacts.py server --version VERSION --output dist` and verify its manifest digest.
2. Sign and publish the immutable production artifact through the release service. CI artifacts remain `productionEligible: false`.
3. Preserve PostgreSQL and the signed-manifest directory. Record migration version, binary digest, and protocol window. Current source has no backup or restore workflow.
4. Drain traffic. Run `eitmad-server migrate` once with the migration role. Migration files and checksums in `deploy/migrations.sha256` are immutable.
5. Start one new instance with the runtime role. Verify `/livez`, `/readyz`, authentication, authorization denial, tenant isolation, redaction, sync resume, update selection, and administration status.
6. Add instances and restore traffic in bounded steps. Observe error rate, database connections, sync backlog, and readiness before each step.

The runtime currently also applies migrations during `serve`; production must still run the explicit migration step first and remove schema-change privilege from the normal runtime role where PostgreSQL permits it.

## Backup support

Backup and restore support is deferred by [ADR-0033](../decisions/0033-defer-backup-support.md). No setup or upgrade backup gate remains. A separate accepted recovery design and deployment evidence are required before production acceptance.

## Roll back

Stop rollout when readiness, isolation, authorization, redaction, migration, or sync checks fail. Drain the new instances. If the prior binary supports the applied schema and protocol, redeploy it against the preserved database. Otherwise stop all writers, preserve the database and matching manifest directory, and correct the cause for forward repair with the same release. Never run reverse SQL by hand.

Start one prior-version instance and repeat the full smoke suite before adding replicas. Preserve the failed artifact, database recovery point, migration inventory, correlation IDs, and redacted logs. Update the release incident record and troubleshooting path before another attempt.

Related pages: [run the server](run-server-authority.md), [validate a release](validate-release-candidate.md), [server authority](../developer/subsystems/server-authority.md), and [server failures](../troubleshooting/server-plane-failures.md).
