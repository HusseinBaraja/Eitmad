---
title: "Extend the modular server authority safely"
description: "Understand the combined server deployment, control and sync ownership, PostgreSQL isolation, authentication, compatibility, and extension points."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-05"
review_triggers:
  - "server identity, authorization, storage, synchronization, licensing, update assignment, or deployment boundaries change"
keywords:
  - "eitmad-server"
  - "PostgreSQL RLS"
  - "device proof"
  - "server sync"
  - "tenant code"
  - "protocol 1.6"
---

# Extend the modular server authority safely

`eitmad-server` is the initial combined deployment for the control, sync, relay, update, and administration planes. The five planes are library crates; `server/host` owns the server executable. The Rust crates keep explicit ownership seams so a plane can move to a separate service later without moving product authority into a shell.

## Purpose and current scope

The foundation provides tenant and organization identity, accounts, registered devices, invitation activation, authentication tokens, session policy, relationship authorization, update-channel assignment, sync coordination, snapshots, operation history, resumable subscriptions, conflict records, WAN relay coordination, signed update distribution, operational status, fleet visibility, audit access, support workflows, and client compatibility negotiation.

The server registers the branch-scoped Customer contact schema and handler as its first product domain. It does not provide billing, email, MFA challenge, package CDN, production relay payload routing, admin UI, backup scheduling, or customer conflict resolution. Other general sync domain schemas remain unregistered. The pricing boundary also stores immutable catalog dependencies for server cost validation; see [Pricing authority](pricing.md#authorization-and-durable-publication).

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

Protocol `1.5` adds relay, signed update, and administration contracts and generated bindings. Protocol `1.6` changes only the local IPC handshake. The server WebSocket accepts protocol `1.4–1.17`, consumes `ServerClientMessage`, and emits `ServerMessage`. A client must first send `eitmad.server.hello.v1`; no sync or subscription traffic is valid before negotiation.

The server requires these capabilities:

- `eitmad.capability.sync.v1`
- `eitmad.capability.server-connection.v1`
- `eitmad.capability.server-device-proof.v1`
- `eitmad.capability.server-snapshot-chunks.v1`
- `eitmad.capability.server-subscription-resume.v1`
- `eitmad.capability.server-relay.v1`
- `eitmad.capability.server-update-distribution.v1`
- `eitmad.capability.server-administration.v1`

Negotiation selects an overlapping protocol and registered schema range. Missing capabilities, an unknown required schema, or no compatible version produces `eitmad.error.server-client-incompatible.v1` before normal traffic. Local IPC supports only the current desktop protocol; see [contract evolution](../../api/evolve-contracts-compatibly.md). Server sync needs at least `1.4`. Each relay, update-distribution, or administration HTTP request must send the base64url-encoded `PeerHello` JSON in `x-eitmad-peer-hello`; Rust requires protocol `1.5` or newer and the route capability before it authenticates or dispatches the request.

Pricing and catalog-revision HTTP routes require protocol `1.16`. `/v1/catalog-revisions/synchronize` requires `eitmad.capability.catalog-revisions.v1`; pricing routes require `eitmad.capability.pricing.v1`. Catalog writes authorize the authenticated organization's Manager relationship, preserve immutable revisions under tenant RLS, and record redacted audit evidence. Price confirmation resolves cost and compatible options from those accepted revisions. The host also registers Material, Part, Product, Furniture, public catalog, and Pricing sync handlers. PostgreSQL migration 9 extends category kinds and binds retained catalog scope to the tenant's organization. See [catalog replication](synchronization.md#catalog-replication) and [Pricing contracts and recovery](pricing.md).

## Identity, authentication, and sessions

The CLI creates the first tenant, organization, owner account, owner relationships, default update assignment, audit row, and activation invitation in one transaction. The executable exposes activation of stored invitations; it has no later-account invitation creation or delivery workflow.

Login uses a tenant code and username. Usernames accept Arabic Unicode text after whitespace normalization and case folding, but reject bidirectional control characters. Tenant, organization, and other display names apply the same bidirectional-control rejection so mixed Arabic and Latin reports cannot be reordered by hidden format characters. Login denials are intentionally non-specific: an absent account and an inactive or locked account return the same redacted failure. Bootstrap serializes its tenant-count check through a PostgreSQL advisory lock so two concurrent bootstrap calls cannot both observe an empty `control.tenants` and both commit. Device-proof clock-skew validation uses checked arithmetic and rejects extreme client-supplied timestamps instead of overflowing. Passwords use Argon2 hashes. Access and refresh tokens are opaque random values; PostgreSQL stores only keyed HMAC-SHA-256 hashes. The default policy is:

| Limit | Default |
| --- | --- |
| Access-token lifetime | 15 minutes |
| Refresh-token lifetime | 30 days |
| Session idle limit | 14 days |
| Device-proof clock skew | 5 minutes |

Every access-authenticated request must include the access token, device ID, timestamp, nonce, and an Ed25519 signature over the canonical proof bytes. A nonce can be used once in its validity window. A registered device ID cannot be rebound to another public key. Refresh-token reuse revokes the token family. MFA remains unimplemented. Bootstrap returns the initial owner activation secret directly; no invitation delivery provider ships.

## Storage, scope, and audit invariants

Migrations `0001_control_foundation.sql`, `0002_sync_foundation.sql`, `0003_admin_foundation.sql`, `0004_server_audit_envelope.sql`, `0005_customer_branches.sql`, and `0006_catalog_images.sql` own the PostgreSQL schema. Every tenant-scoped table has a `tenant_id`, enables row-level security, and forces row-level security. Rust opens a transaction and sets the tenant context before scoped access. Application credentials must not have a PostgreSQL role that can bypass RLS. The [catalog image capability](catalog-images.md) owns immutable organization asset transfer through authenticated HTTP routes.

Every accepted state change adds a redacted audit record in the same transaction. `server/audit` is the only PostgreSQL audit contract. It records actor kind, optional session and principal, tenant and optional workspace, exact scope, target kind and target ID, operation, outcome, correlation, optional causation and idempotency, stable redacted failure ID, and time. Each control-plane and sync-plane entry point receives a caller-supplied correlation identifier, so records from one request remain joinable. Invalid and denied sync boundaries are recorded in a separate mandatory transaction before the operation returns; if that append fails, the boundary fails closed as unavailable. Successful mutations, conflicts, snapshots, and acknowledgements append in the authoritative state transaction. Token plaintext, password input, device private keys, domain payloads, and customer content must not enter logs or audit metadata.

Migration files are append-only after release. Back up PostgreSQL before migration and restore the complete cluster or database by the approved PostgreSQL recovery process. Do not edit rows, RLS policies, checkpoints, operation history, or conflict records as a repair shortcut.

## Sync modes and flows

Customer contact is the only registered domain and uses `LocalFirst`. The coordinator accepts an authorized local operation, assigns ordered server history, projects the record, and creates a conflict when the base revision is stale and the domain cannot resolve it safely. There is no server-authoritative command submission entry point. The host rejects unsupported command traffic; the client contract retains its typed mode and stored command state.

An idempotency key is stored with a deterministic request fingerprint and serialized result. An exact retry returns the first result, including the same conflict ID. Reusing the key for another intent returns `eitmad.error.server-idempotency-mismatch.v1`. Scope locking prevents concurrent writers from assigning the same next position.

An authenticated organization owner registers a branch with `POST /v1/customer-branches` before branch-scoped Customer sync. The route checks the owner relationship, writes the tenant-scoped registry and mandatory audit in one transaction, and accepts an exact retry. The customer handler resolves the branch to its organization in `control.branches`. A Manager relationship on that organization grants customer access to its branches; a Receptionist needs a direct relationship on the exact branch. The handler validates the submitted schema, payload identity, operation, and revision. The local submission route returns applied, replayed, conflicted, or rejected status for the submitted change ID. A stale revision records both inputs as an open conflict. PostgreSQL tenant context and forced row-level security protect the branch registry, customer operation, conflict, and checkpoint tables. See [maintain customer contact records](customers.md) for local projection and recovery.

Pull sessions return ordered history after a checkpoint. Clients acknowledge applied checkpoints separately through `eitmad.sync.acknowledge.v1`, which persists durable device checkpoints. The connection-level subscription acknowledgement message `eitmad.server.acknowledge.v1` is rejected with `eitmad.error.server-subscription-ack-unsupported.v1` until a durable subscription-checkpoint store exists; the server never reports success without changing cursor state.

Subscription resume cursors are scoped to their exact stream (tenant, scope kind, scope ID, schema ID, and event ID). A cursor from another stream returns `ResyncRequired` instead of silently skipping events. A stale base revision on a record the server has never stored — or removed by compaction — returns snapshot-required semantics instead of an availability error, so clients resynchronize rather than retry forever. Snapshot creation writes audit records inside its transaction. There is no history compaction operation in the executable.

`server/host` constructs `SubscriptionPageRequest` and passes it to `SyncCoordinator::subscription_page`. Keep authenticated session, exact scope, negotiated schema version, resume cursor, page limit, correlation ID, and request time together in this request boundary when adding another transport. Do not split these values into an untyped adapter API or authorize them outside `server/sync-plane`.

Operation history has a 90-day retention floor and is currently retained without compaction. When a requested checkpoint is unavailable, the host sends a manifest, bounded chunks, and completion checksum instead of pretending that incremental history is complete.

## Licensing and update assignment

There is no license service or provider-state ingestion workflow. Historical license tables and rows remain intact; current product domains have no license gate.

Update assignment reads device override, tenant default, then global `stable`. Bootstrap writes the initial tenant assignment; there is no channel-assignment mutation entry point. Existing stored assignments remain readable. The update check requires the authenticated device and assigned channel to match the client profile. Rust verifies Ed25519 signatures and owns compatibility, pause, revocation, staged rollout, and package selection. Platform adapters may install a selected update but must not calculate eligibility.

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
| `POST /v1/customer-branches` | Owner-authorized branch registration |
| `GET /v1/update-assignment` | Authorized effective channel query |
| `POST /v1/updates/check` | Authorized signed-manifest eligibility and package selection |
| `POST /v1/admin/update-manifests` | Owner-authorized signed-manifest publication |
| `/v1/relay/*` | Authenticated relay lifecycle, reconnect, failure, and health routes |
| `/v1/admin/*` | Owner-authorized diagnostics, status, audit, visibility, and support routes |
| `GET /v1/connect` | Authenticated WebSocket upgrade |

TLS is mandatory unless the server binds to a loopback address and the operator explicitly enables insecure loopback for development. The WebSocket uses one ordered connection for negotiation, sync pull/acknowledgement, snapshot transfer, and resumable subscription traffic.

The host revalidates the authenticated session every 60 seconds for the lifetime of each WebSocket. When the access token expires, the session ends, or the device or account-device link is revoked, the socket closes instead of serving further sync traffic with stale credentials.

The process-wide PostgreSQL connection budget is split evenly across control, sync, and administration pools (`pool_connection_budget`), so `EITMAD_SERVER_MAX_CONNECTIONS` bounds total pool connections. An empty domain registry is a valid base-server state: the negotiated schema list is empty, and every unknown domain request fails closed. `/readyz` reports database and process readiness, not product-domain readiness. Database or migration failure keeps it unsuccessful; product-domain readiness is checked during negotiation. A product deployment must verify that its required schema appears in negotiation before it routes product sync traffic. A malformed `bootstrap` invocation prints usage and exits with a failure code instead of succeeding silently.

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

Use [server troubleshooting](../../troubleshooting/server-authentication-and-sync.md) for symptom-led checks.

## Tests and verification

Focused unit tests cover password hashing, Arabic identifiers, device-proof time bounds, server audit envelopes, sync fingerprints and snapshot chunks, relay lifecycle and denial, signed manifest changes, channels, incompatible clients, backup status, administration authorization, router authentication, and the three-pool connection budget. Contract checks verify generated bindings. The repository policy check verifies the migration inventory and immutable SQL bytes. SQL text scans do not prove table creation, append-only enforcement, or tenant RLS; these require the live PostgreSQL exercise below. Existing `eitmad-sync` tests cover complete local-first and server-authoritative flows, duplicate delivery, conflicts, unauthorized remote changes, compatibility, and WAN relay fallback.

Run:

```powershell
cargo test -p eitmad-server-audit -p eitmad-control-plane -p eitmad-sync-plane -p eitmad-relay-plane -p eitmad-update-plane -p eitmad-admin-plane -p eitmad-server
cargo clippy --workspace --all-targets -- -D warnings
npm run contracts:verify --prefix crates/contracts/codegen
```

`eitmad-server check-config` requires synthetic loopback transport plus the manifest directory and trusted Ed25519 public-key configuration. A live PostgreSQL migration, bootstrap, login, RLS, snapshot, relay, administration, backup, restore, and readiness exercise remains a deployment-environment requirement.

## Tradeoffs and extension points

One process reduces initial deployment and operational cost. Separate crates, migrations, contracts, and ownership files preserve later service seams. PostgreSQL gives durable transactions, row locking, and defense-in-depth tenant RLS, but it adds an external operational dependency and does not remove the need for Rust scope checks.

To add a domain, implement `DomainHandler`, register one immutable `DomainDescriptor`, define its sync mode and schema range, authorize every action, define conflict and stale-data behavior, add Arabic/mixed-direction evidence, and add live PostgreSQL tests. Add provider integrations only when a current workflow needs them; keep relay routers, manifest repositories, and administration providers behind their existing seams rather than adding product logic to HTTP handlers.

Related documents: [ADR-0025](../../decisions/0025-modular-server-authority-foundation.md), [ADR-0026](../../decisions/0026-compose-authorized-operational-server-planes.md), [server operations](../../operations/run-server-authority.md), [server operations](../../operations/run-server-authority.md), [synchronization](synchronization.md), and [protocol contracts](../../api/index.md).
