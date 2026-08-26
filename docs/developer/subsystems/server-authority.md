---
title: "Extend the modular server authority safely"
description: "Understand the combined server deployment, control and sync ownership, PostgreSQL isolation, authentication, compatibility, and extension points."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-08-27"
review_triggers:
  - "server identity, authorization, storage, synchronization, licensing, update assignment, or deployment boundaries change"
keywords:
  - "eitmad-server"
  - "PostgreSQL RLS"
  - "device proof"
  - "server sync"
  - "tenant code"
  - "protocol 1.5"
---

# Extend the modular server authority safely

`eitmad-server` is the initial combined deployment for the control, sync, relay, update, and administration planes. The Rust crates keep explicit ownership seams so a plane can move to a separate service later without moving product authority into a shell.

## Purpose and current scope

The foundation provides tenant and organization identity, accounts, registered devices, invitation activation, authentication tokens, session policy, relationship authorization, licensing hooks, update-channel assignment, sync coordination, snapshots, operation history, resumable subscriptions, conflict records, WAN relay coordination, signed update distribution, operational status, fleet visibility, audit access, support workflows, and client compatibility negotiation.

It does not provide a production business domain, billing provider, email provider, MFA challenge, package CDN, production relay payload router, admin UI, backup scheduler, or native client workflow. `DomainRegistry` is intentionally empty in the base executable. The server advertises no domain schemas and rejects domain sync traffic until a product registers a handler. Control, relay, update, and administration planes can still start and report their own readiness.

## Ownership and module boundaries

| Concern | Rust authority |
| --- | --- |
| External server, configuration, HTTPS routes, and WebSocket session | `server/host` (`eitmad-server`) |
| Tenant, user, account, organization, device, invitation, session, token, relationship, license, update assignment, and outbox state | `server/control-plane` |
| Canonical server audit envelope, append operation, and migration | `server/audit` |
| Registered domain handlers, operations, idempotency, conflicts, history, snapshots, checkpoints, and subscription events | `server/sync-plane` |
| PostgreSQL types without enabling SQLite-linked features | `server/postgres-support` |
| Relay lifecycle, routing hooks, reconnect, health, and failures | `server/relay-plane` |
| Signed manifest publication and durable file repository | `server/update-plane` |
| Update signature, compatibility, and staged-rollout policy | `crates/update-policy` |
| Diagnostics, status, visibility, audit, and support workflows | `server/admin-plane` |
| External wire types, identifiers, versions, capabilities, and generated bindings | `crates/contracts` |

Native shells and network adapters must not copy these rules, access PostgreSQL, inspect token hashes, assign update policy, interpret domain payloads, or bypass domain authorization.

## Contracts and compatibility

Protocol `1.5` adds relay, signed update, and administration contracts and generated bindings. The server WebSocket accepts protocol `1.4–1.5`, consumes `ServerClientMessage`, and emits `ServerMessage`. A client must first send `eitmad.server.hello.v1`; no sync or subscription traffic is valid before negotiation.

The server requires these capabilities:

- `eitmad.capability.sync.v1`
- `eitmad.capability.server-connection.v1`
- `eitmad.capability.server-device-proof.v1`
- `eitmad.capability.server-snapshot-chunks.v1`
- `eitmad.capability.server-subscription-resume.v1`
- `eitmad.capability.server-relay.v1`
- `eitmad.capability.server-update-distribution.v1`
- `eitmad.capability.server-administration.v1`

Negotiation selects an overlapping protocol and registered schema range. Missing capabilities, an unknown required schema, or no compatible version produces `eitmad.error.server-client-incompatible.v1` before normal traffic. Protocol `1.0–1.3` remains in the encoded compatibility window for existing local IPC behavior. Server sync needs at least `1.4`. Each relay, update-distribution, or administration HTTP request must send the base64url-encoded `PeerHello` JSON in `x-eitmad-peer-hello`; Rust requires protocol `1.5` and the route capability before it authenticates or dispatches the request.

## Identity, authentication, and sessions

The CLI creates the first tenant, organization, owner account, owner relationships, default license record, default update assignment, audit row, and activation invitation in one transaction. Later accounts use an owner-authorized invitation and activation flow.

Login uses a tenant code and username. Usernames accept Arabic Unicode text after whitespace normalization and case folding, but reject bidirectional control characters. Tenant, organization, and other display names apply the same bidirectional-control rejection so mixed Arabic and Latin reports cannot be reordered by hidden format characters. Login denials are intentionally non-specific: an absent account and an inactive or locked account return the same redacted failure. Bootstrap serializes its tenant-count check through a PostgreSQL advisory lock so two concurrent bootstrap calls cannot both observe an empty `control.tenants` and both commit. Device-proof clock-skew validation uses checked arithmetic and rejects extreme client-supplied timestamps instead of overflowing. Passwords use Argon2 hashes. Access and refresh tokens are opaque random values; PostgreSQL stores only keyed HMAC-SHA-256 hashes. The default policy is:

| Limit | Default |
| --- | --- |
| Access-token lifetime | 15 minutes |
| Refresh-token lifetime | 30 days |
| Session idle limit | 14 days |
| Device-proof clock skew | 5 minutes |

Every access-authenticated request must include the access token, device ID, timestamp, nonce, and an Ed25519 signature over the canonical proof bytes. A nonce can be used once in its validity window. A registered device ID cannot be rebound to another public key. Refresh-token reuse revokes the token family. MFA and invitation delivery are provider hooks only; no production provider ships in this checkpoint.

## Storage, scope, and audit invariants

Migrations `0001_control_foundation.sql`, `0002_sync_foundation.sql`, `0003_admin_foundation.sql`, and `0004_server_audit_envelope.sql` own the PostgreSQL schema. Every tenant-scoped table has a `tenant_id`, enables row-level security, and forces row-level security. Rust opens a transaction and sets the tenant context before scoped access. Application credentials must not have a PostgreSQL role that can bypass RLS.

Every accepted state change adds a redacted audit record in the same transaction. `server/audit` is the only PostgreSQL audit contract. It records actor kind, optional session and principal, tenant and optional workspace, exact scope, target kind and target ID, operation, outcome, correlation, optional causation and idempotency, stable redacted failure ID, and time. Each control-plane and sync-plane entry point receives a caller-supplied correlation identifier, so records from one request remain joinable. Invalid and denied sync boundaries are recorded in a separate mandatory transaction before the operation returns; if that append fails, the boundary fails closed as unavailable. Successful mutations, conflicts, snapshots, compaction, and acknowledgements append in the authoritative state transaction. Token plaintext, password input, device private keys, domain payloads, and customer content must not enter logs or audit metadata.

Invitation delivery is injected through `ControlPlane::with_notification_sink`. `create_invite` resolves the sink before it opens its transaction, so a missing provider fails with `eitmad.server.identity` delivery-unavailable semantics without committing identity, invitation, or directory state.

Migration files are append-only after release. Back up PostgreSQL before migration and restore the complete cluster or database by the approved PostgreSQL recovery process. Do not edit rows, RLS policies, checkpoints, operation history, or conflict records as a repair shortcut.

## Sync modes and flows

A registered domain declares one immutable mode:

- Local-first accepts an authorized local operation, assigns ordered server history, projects the record, and creates a conflict when the base revision is stale and the domain cannot resolve it safely.
- Server-authoritative accepts a typed command only through its registered handler. The handler authorizes and returns the authoritative change or denial. The server does not infer business truth from opaque payload bytes.

An idempotency key is stored with a deterministic request fingerprint and serialized result. An exact retry returns the first result, including the same conflict ID. Reusing the key for another intent returns `eitmad.error.server-idempotency-mismatch.v1`. Scope locking prevents concurrent writers from assigning the same next position.

Pull sessions return ordered history after a checkpoint. Clients acknowledge applied checkpoints separately through `eitmad.sync.acknowledge.v1`, which persists durable device checkpoints. The connection-level subscription acknowledgement message `eitmad.server.acknowledge.v1` is rejected with `eitmad.error.server-subscription-ack-unsupported.v1` until a durable subscription-checkpoint store exists; the server never reports success without changing cursor state.

Subscription resume cursors are scoped to their exact stream (tenant, scope kind, scope ID, schema ID, and event ID). A cursor from another stream returns `ResyncRequired` instead of silently skipping events. A stale base revision on a record the server has never stored — or removed by compaction — returns snapshot-required semantics instead of an availability error, so clients resynchronize rather than retry forever. Snapshot creation and history compaction write audit records inside their transactions, and compaction additionally requires write-level domain authorization before deleting anything.

Operation history has a 90-day retention floor. Compaction is safe only after a complete snapshot exists and retained client checkpoints no longer require the removed range; the covering-snapshot check excludes the checkpoint row itself so later compactions keep resolving. When a requested checkpoint is unavailable, the host sends a manifest, bounded chunks, and completion checksum instead of pretending that incremental history is complete.

## Licensing and update assignment

Licensing is a persisted enforcement seam, not billing. A provider adapter may record active, expired, suspended, or unavailable state and entitlements. Recording provider state requires an existing license row for the tenant; the update must match exactly one row or the operation fails without deleting entitlements or recording success. An expired license receives at most seven days of grace; suspension never becomes grace. Product domains call the license boundary before licensed actions.

Update assignment resolves in this order: device override, tenant default, then global `stable`. Assignment commands return the resolved effective assignment for the target device, including an existing device override. Only a tenant owner may change assignments or publish a signed manifest. The update check requires the authenticated device and assigned channel to match the client profile. Rust verifies Ed25519 signatures and owns compatibility, pause, revocation, staged rollout, and package selection. Platform adapters may install a selected update but must not calculate eligibility.

See [signed update distribution](update-distribution.md) for the manifest contract, host configuration, and current key-rotation limit.

## HTTP and WebSocket boundary

The combined host exposes:

| Route | Purpose |
| --- | --- |
| `GET /livez` | Process liveness only |
| `GET /readyz` | Database readiness |
| `POST /v1/auth/activate` | Invitation activation and initial token issue |
| `POST /v1/auth/login` | Password and device-proof authentication |
| `POST /v1/auth/refresh` | Refresh rotation with device proof |
| `GET /v1/update-assignment` | Authorized effective channel query |
| `POST /v1/updates/check` | Authorized signed-manifest eligibility and package selection |
| `POST /v1/admin/update-manifests` | Owner-authorized signed-manifest publication |
| `/v1/relay/*` | Authenticated relay lifecycle, reconnect, failure, and health routes |
| `/v1/admin/*` | Owner-authorized diagnostics, status, audit, visibility, and support routes |
| `GET /v1/connect` | Authenticated WebSocket upgrade |

TLS is mandatory unless the server binds to a loopback address and the operator explicitly enables insecure loopback for development. The WebSocket uses one ordered connection for negotiation, sync pull/acknowledgement, snapshot transfer, and resumable subscription traffic.

The host revalidates the authenticated session every 60 seconds for the lifetime of each WebSocket. When the access token expires, the session ends, or the device or account-device link is revoked, the socket closes instead of serving further sync traffic with stale credentials.

The process-wide PostgreSQL connection budget is split evenly across control, sync, and administration pools (`pool_connection_budget`), so `EITMAD_SERVER_MAX_CONNECTIONS` bounds total pool connections. An empty domain registry is a valid base-server state: the negotiated schema list is empty, and every unknown domain request fails closed. `/readyz` reports process readiness, not product-domain readiness. A product deployment must verify that its required schema appears in negotiation before it routes product sync traffic. A malformed `bootstrap` invocation prints usage and exits with a failure code instead of succeeding silently.

Relay actions require tenant membership and source-device ownership. Administrative close and all administration routes require tenant ownership. Manifest publication requires the configured operator tenant and its dedicated publish permission. Read [WAN relay coordination](wan-relay-coordination.md) and [server administration](server-administration.md) before extending these boundaries.

## Arabic UX impact

There is no server UI. The wire remains locale-independent UTF-8 JSON. Tenant codes are lowercase ASCII routing identifiers; display names and usernames may contain Arabic. The server rejects Unicode bidi controls in usernames instead of modifying visible text. Domain payloads remain opaque and keep mixed text such as `خزانة Wardrobe 120 cm` unchanged.

Future shells must localize stable message IDs, use the approved glossary terms, render identifiers in isolated LTR runs inside RTL layouts, and distinguish pending, synchronized, conflicted, stale, and denied states. The server must not return English prose for a shell to parse.

Arabic-first checklist evidence for this server-only checkpoint: terminology and mixed-direction handling pass at the contract boundary; username normalization and bidi rejection have tests; UI layout, fonts, keyboard flow, accessibility, search, documents, and reports are not applicable because no UI or business document was added.

## Failure modes and recovery

| Failure | Preserved state | Safe action |
| --- | --- | --- |
| Authentication, token expiry, or device proof failure | No protected operation runs | Correct credentials or device registration; do not weaken proof checks |
| Token reuse | The token family is revoked | Sign in again and investigate copied refresh-token use |
| Authorization denial or scope mismatch | No domain mutation commits; denial is audited | Repair the owning relationship or authenticated tenant context |
| Idempotency mismatch | The original result remains | Stop retrying the changed request; use a new key only for new intent |
| Open conflict | Both competing inputs and provenance remain | Use the registered domain resolution workflow; never edit history |
| Snapshot required | Incremental history is not reported as complete | Apply and verify the full snapshot, then resume from its checkpoint |
| Incompatible client | No normal WebSocket traffic starts | Upgrade the client or server as one compatible rollout |
| Database or migration failure | The process does not become ready | Preserve data, repair PostgreSQL, then rerun migrations |
| License required | The protected domain action does not run | Restore provider state or valid entitlement; do not change database rows manually |

Use [server troubleshooting](../../troubleshooting/server-authentication-and-sync.md) for symptom-led checks.

## Tests and verification

Focused unit and static migration tests cover token secrecy, password hashing, Arabic identifiers, overflow-safe device proof, forced RLS, the complete append-only server audit envelope, durable sync, relay lifecycle and denial, signed manifest changes, channels, incompatible clients, backup status, administration authorization, router authentication, generated bindings, and the three-pool connection budget. Existing `eitmad-sync` tests cover complete local-first and server-authoritative flows, duplicate delivery, conflicts, unauthorized remote changes, compatibility, and WAN relay fallback.

Run:

```powershell
cargo test -p eitmad-server-audit -p eitmad-control-plane -p eitmad-sync-plane -p eitmad-relay-plane -p eitmad-update-plane -p eitmad-admin-plane -p eitmad-server
cargo clippy --workspace --all-targets -- -D warnings
npm run contracts:verify --prefix crates/contracts/codegen
```

`eitmad-server check-config` requires synthetic loopback transport plus the manifest directory and trusted Ed25519 public-key configuration. A live PostgreSQL migration, bootstrap, login, RLS, snapshot, relay, administration, backup, restore, and readiness exercise remains a deployment-environment requirement.

## Tradeoffs and extension points

One process reduces initial deployment and operational cost. Separate crates, migrations, contracts, and ownership files preserve later service seams. PostgreSQL gives durable transactions, row locking, and defense-in-depth tenant RLS, but it adds an external operational dependency and does not remove the need for Rust scope checks.

To add a domain, implement `DomainHandler`, register one immutable `DomainDescriptor`, define its sync mode and schema range, authorize every action, define conflict and stale-data behavior, add Arabic/mixed-direction evidence, and add live PostgreSQL tests. Keep licensing, invitation delivery, MFA, relay routers, manifest repositories, and administration providers behind their named seams rather than adding product logic to HTTP handlers.

Related documents: [ADR-0025](../../decisions/0025-modular-server-authority-foundation.md), [ADR-0026](../../decisions/0026-compose-authorized-operational-server-planes.md), [server operations](../../operations/run-server-authority.md), [protocol 1.5 rollout](../../releases/protocol-1-5-operational-server-planes.md), [synchronization](synchronization.md), and [protocol contracts](../../api/index.md).
