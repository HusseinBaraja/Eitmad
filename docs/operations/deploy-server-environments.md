---
title: "Deploy staging and production server environments"
description: "Use repeatable server profiles, TLS boundaries, migrations, backups, restore drills, channel separation, and rollback controls."
audience: "operations"
page_type: "task"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-08-26"
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
3. Back up PostgreSQL and the signed-manifest directory. Record restore verification, migration version, binary digest, protocol window, and rollback decision point.
4. Drain traffic. Run `eitmad-server migrate` once with the migration role. Migration files and checksums in `deploy/migrations.sha256` are immutable.
5. Start one new instance with the runtime role. Verify `/livez`, `/readyz`, authentication, authorization denial, tenant isolation, redaction, sync resume, update selection, and administration status.
6. Add instances and restore traffic in bounded steps. Observe error rate, database connections, sync backlog, and readiness before each step.

The runtime currently also applies migrations during `serve`; production must still run the explicit migration step first and remove schema-change privilege from the normal runtime role where PostgreSQL permits it.

## Back up and prove restore

Use one consistent PostgreSQL recovery point for control, sync, audit, and administration state. Use encrypted physical backups plus WAL archiving for point-in-time recovery, or an approved logical backup when its recovery-time objective is sufficient. Back up the immutable signed-manifest directory and its object-version metadata separately. Protect backup keys and access logs as production secrets.

Run a staging restore drill on the defined schedule and before a migration that changes recovery risk:

1. Restore into an isolated database with no production egress.
2. Restore the matching manifest directory snapshot.
3. Start the exact matching server binary with staging-only secrets.
4. Verify migration version, tenant counts without exposing records, audit continuity, tenant isolation, authentication, sync checkpoints, snapshot fallback, and manifest signatures.
5. Record recovery point, duration, checks passed, and sanitized failure evidence. A completed backup without this restore proof is not a verified recovery path.

## Roll back

Stop rollout when readiness, isolation, authorization, redaction, migration, or sync checks fail. Drain the new instances. If the prior binary supports the applied schema and protocol, redeploy it against the preserved database. Otherwise stop all writers and restore the complete pre-migration PostgreSQL recovery point plus the matching manifest directory. Never run reverse SQL by hand.

Start one prior-version instance and repeat the full smoke suite before adding replicas. Preserve the failed artifact, database recovery point, migration inventory, correlation IDs, and redacted logs. Update the release incident record and troubleshooting path before another attempt.

Related pages: [run the server](run-server-authority.md), [validate a release](validate-release-candidate.md), [server authority](../developer/subsystems/server-authority.md), and [server failures](../troubleshooting/server-plane-failures.md).

