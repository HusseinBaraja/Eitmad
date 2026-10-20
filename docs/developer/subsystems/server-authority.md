---
title: "Extend the modular server authority safely"
description: "Understand the combined server deployment, control and sync ownership, PostgreSQL isolation, authentication, compatibility, and extension points."
audience: "developer"
page_type: "explanation"
status: "active"
owner: "server platform maintainers"
last_verified: "2026-10-21"
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

Use the [shared server account lifecycle](identity-foundation.md#shared-server-account-lifecycle) to list, invite, activate, assign roles and branches, deactivate, and revoke sessions through supported contracts and authenticated routes. `IdentityService::list_accounts` and `change_account` own account administration, revisions, audit, and durable retry receipts. For persistent WPF test clients, use the [Manager and Receptionist test environment](../../operations/run-workflow-test-environment.md).

`eitmad-server` is the initial combined deployment for the control, sync, relay, update, and administration planes. The five planes are library crates; `server/host` owns the server executable. The Rust crates keep explicit ownership seams so a plane can move to a separate service later without moving product authority into a shell.

## Purpose and current scope

The foundation provides tenant and organization identity, accounts, registered devices, invitation activation, authentication tokens, session policy, relationship authorization, update-channel assignment, sync coordination, snapshots, operation history, resumable subscriptions, conflict records, WAN relay coordination, signed update distribution, operational status, fleet visibility, audit access, support workflows, and client compatibility negotiation.

The host registers branch-scoped Customer contacts and organization-scoped Material, Part, Product, Furniture, public catalog, and Pricing schemas. It does not provide billing, email, MFA challenge, package CDN, production relay payload routing, admin UI or customer conflict resolution. Privileged local server backup policy, scheduling decisions, and typed status are implemented in the host CLI; see [server backup and recovery](../../operations/backup-lan-installation.md#automatic-local-server-backups). The pricing boundary also stores immutable catalog dependencies for server cost validation; see [Pricing authority](pricing.md#authorization-and-durable-publication).

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

## Retained desktop connection

`eitmad-server-connection::DesktopEnrollment` owns desktop enrollment and online reauthentication. The thin Arabic sign-in screen sends transient credentials and optional `DesktopEnrollmentSettings` through `eitmad.ipc.desktop-sign-in.v1` with `serverLogin: true`. Rust validates HTTPS, the explicit CA, server login, and the account's organization and branch assignment through `POST /v1/auth/desktop-profile`. That endpoint returns only the authenticated account's selected assignment. An absent, ambiguous, or foreign assignment is denied. Device registration and session issuance retain their server audit boundary.

An unconfigured engine exposes a bootstrap-authenticated process context without creating a local tenant or user. Successful enrollment initializes the local authority from the server-confirmed tenant, organization, device, and branch, then imports the account with an audited local verifier. Each installation generates its own random device ID and Ed25519 key. `desktop-device.json` contains only the ID; the key remains in Rust secret storage under a separate device-key reference. Retained imported credentials can supply their existing key before expired tokens are discarded. Account tokens can then expire or disappear without losing the installation key.

`DesktopConnectionSettings` owns version 1 of bounded `connection.json`: `version`, `endpoint`, `trustedCertificate`, `credential`, `organization`, `branch`, and optional `tenantCode` and `serverId`. Paired enrollment writes `serverId`; loading it requires the matching retained pairing receipt, endpoint, CA, organization, branch, and tenant code. Missing or damaged pairing state fails closed. The optional code permits existing settings to load; the operator supplies it once through enrollment when it is absent. Rust copies the first verified trust input to `desktop-trust.pem` and commits settings by a flushed temporary file and rename. Relative CA paths resolve against the runtime directory. No password, token, or private key is stored in these files or returned to WPF.

Enrollment can commit on the server before its local response arrives. Retry with the same inputs: the durable device ID and key are reused, and existing installation and account identities must match. Partial local import is retained. A failed trust check, wrong password, foreign identity, unavailable server, or persistence failure does not reset business records or pending work. WPF restarts the supervised engine after initial configuration, including a retry in a process whose routes were not yet attached. It then starts a local user session only with live server confirmation. All protected access follows [ADR-0029](../../decisions/0029-require-live-server-authority-for-protected-access.md); a retained verifier cannot authorize disconnected access.

Ordinary startup attaches catalog-image, catalog-sync, pricing, customer, quotation, and order routes from the retained settings. The organization must match local authority. Each request still resolves its current actor and enforces authorization. A malformed, unreadable, oversized, or unsupported settings file fails startup. If settings are absent, existing per-capability environment routes remain available. A valid retained file takes precedence as a complete connection.

Preserve the runtime directory and credential store under the same Windows user. Never copy another PC's runtime directory to enroll a new installation. Do not reset data to repair authentication. Loss of both the installation signing key and every account credential containing that key fails closed; this flow does not replace a registered device's signing identity. See [enroll or recover a Windows desktop](../../operations/run-server-authority.md#enroll-or-recover-a-windows-desktop).

Focused proof: `desktop_connection` tests cover bounded loading and retained settings; `enrollment_rejects_server_and_retained_identity_mismatches_without_replacing_accounts` covers tenant, account, user, and device mismatch. The live `desktop_enrollment_recovers_interruption_expiry_and_credential_loss` scenario covers wrong TLS trust, partial setup, independent devices, expiry, missing account credentials, and retained records and pending publications. Set `EITMAD_DESKTOP_ENROLLMENT_ENGINE` to a current engine executable to include the real Windows shell controller and IPC path. `DesktopEnrollmentRenderedTests` covers Arabic input, keyboard focus, retry, and password clearing at the three requested window sizes.

### Recover a paired connection after restart or DHCP changes

`crates/server-connection/src/recovery.rs` owns paired connection recovery. `DirectServerConfig::new` reads the retained public pairing receipt beside the installation CA. It validates the TLS origin, CA bytes, route scope, and independent device ID. Existing manual diagnostic routes without pairing retain their explicit endpoint; discovery cannot create enrollment for them. A missing retained device file or signing key fails authentication instead of creating a replacement identity. Account-specific token loss can still recover through sign-in with the retained independent key.

Each connection first tries its last verified address, or the retained origin, within a one-second probe budget. If that address is unavailable or no longer authenticates the retained server, Rust runs one bounded [Private-LAN discovery scan](server-discovery.md), selects only fresh candidates for the retained installation ID, rejects identity conflicts, and tries at most three addresses within the caller's original deadline. This permits recovery when DHCP assigns the old address to another host, without sending credentials there. Candidate addresses and ports affect only TCP routing. The saved URL remains the TLS name, SNI, and HTTP authority. Rust does not rewrite settings or replay product mutations during recovery. Failed discovery or connection attempts wait at least two seconds before the next attempt on that configuration.

Every socket validates the certificate chain, TLS name, and expiry against the retained CA. TLS session resumption is disabled so a new socket cannot omit certificate validation. Before any password, token, device proof, or product request, Rust sends `GET /v1/server-identity` on that same TLS socket. The bounded, public `PairingIdentity` response must exactly match the retained installation ID, origin, and CA. This route grants no account session, scope access, or device admission. A different identity, name, chain, expired certificate, or replacement CA returns a non-retryable trust failure. Recovery never uses the PAKE bootstrap verifier.

`DirectServerConfig::recovery()` returns a cloneable Rust `ConnectionRecovery` handle. Its watch receiver exposes typed `Reconnecting`, `Connected`, `Unavailable`, `TrustFailure`, `SignInRequired`, and `Cancelled` states without technical settings or account data. `Connected` follows authenticated request or WebSocket establishment, not discovery or TLS alone. These are Rust backend states; this change adds no native screen or IPC message. `cancel()` invalidates the current attempt, drops active discovery, and closes registered sockets. Cancellable TLS reads check at 250 ms intervals; DNS admission, connect, and all I/O remain bounded by the caller's deadline. A later explicit operation can start a new attempt. A cancelled result cannot save a discovered address.

Recovery uses the existing account-specific credential references, serialized refresh, device proof, and server authorization. Switching accounts selects the current actor's token; it does not change installation identity or device key. A server denial produces `SignInRequired`. Revoked pairing approval continues to deny login, access tokens, and refresh. No recovery path grants offline access, resets enrollment, replaces a CA, removes saved work, or creates unaudited product mutations.

For `Unavailable`, check server readiness and the Private-network discovery path, then retry. For `TrustFailure`, preserve all retained files and check the server certificate, canonical TLS name, and installation identity through the trusted host procedure. Do not copy a new CA from discovery. For `SignInRequired`, sign in with the retained device; an Owner must resolve a confirmed revocation through the supported device policy. Do not delete enrollment files to bypass a denial.

Focused proof uses generated certificates from `tools/dev-certificate`, disposable loopback peers, and the ignored `recovery::tests` group with `EITMAD_DIRECT_TEST_*` paths. It covers app/server restart, address changes with the same retained TLS name, spoofed/stale/conflicted offers, name/chain/expiry failure, outages, cancellation, and account switching. The direct-route enrollment scenario additionally checks real TLS, retained credentials and pending work, and server-runtime restart against a fresh PostgreSQL database. The server pairing test checks revoked-device login, access, and refresh denial. See [server discovery verification](server-discovery.md#installation-and-focused-verification) for interface policy and two-PC acceptance.

## Pair a discovered server and approve its device

Protocol `1.30` declares `eitmad.capability.server-pairing.v1`. Rust contracts in `crates/contracts/src/pairing.rs` define pairing v1 independently of account authentication. `crates/pairing` owns the shared cryptographic implementation; `server/control-plane/src/pairing` owns durable approval and mandatory audit; `server-connection` owns the client key, PAKE transport, and retained trust. Native onboarding and its IPC presentation remain separate work.

Only a live authenticated tenant Owner can call `POST /v1/pairing/open` for a registered organization and branch. The response is `Cache-Control: no-store`. It carries one random eight-digit code and a public offer. One installation can have one live code, valid for three minutes and at most five started exchanges. Abandoning an exchange does not refund its attempt. The code is never stored in plaintext or sent by the joining client. Only the Owner receives it over the existing authenticated TLS connection.

Pairing v1 pins `opaque-ke` 3.0.0: OPAQUE draft-16, Ristretto255, TripleDH, and Argon2. This library has an [NCC Group audit history](https://github.com/facebook/opaque-ke#%EF%B8%8F-audit), which is not an audit of Eitmad's integration or every later library version. Its [documented context API](https://docs.rs/opaque-ke/3.0.0/opaque_ke/#key-exchange-context) authenticates the versioned server UUID, HTTPS endpoint, public CA bytes, tenant code and ID, organization, branch, session, expiry, attempt ID, client device ID, and Ed25519 public key. Do not replace this exchange with a code hash, ordinary password login, or custom curve arithmetic. A protocol or suite change requires a new declared pairing version.

`GET /v1/pairing/offer` publishes only the current untrusted offer. `POST /v1/pairing/start` charges an attempt under a PostgreSQL session-row lock before returning the OPAQUE response. The client bootstrap verifier is private to these two routes. It verifies TLS handshake signatures but does not yet trust a certificate issuer or name. No account password, token, completion, or product traffic uses this verifier. The client authenticates the entire offer with OPAQUE before constructing a normal CA- and hostname-verified TLS connection to the bound endpoint. It then sends `POST /v1/pairing/complete`, including OPAQUE client confirmation and an Ed25519 signature over the binding and confirmation. The device private key is generated and retained in that client's secret store; the server receives only its public key and signatures.

Completion rechecks that the opening Owner remains active and has the Owner relationship. It consumes the session, approves exactly one device in the exact workshop, clears completed PAKE secrets, and writes a redacted mandatory audit in one transaction. A failed audit rolls back approval and consumption. A changed key, different device, expired code, failed confirmation, or second completion is denied. The exact original signed completion can recover its original HMAC-SHA-256 receipt after response loss or restart, including after code expiry; it never approves another device or repeats approval. The receipt authenticates committed approval using the OPAQUE session key.

Migration 18 adds scoped pairing sessions, attempts, approvals, and first-desktop claims with forced tenant RLS. Its public directory contains routing handles only. Existing accounts, device registrations, invitation activation, and private keys are preserved. Existing account-device links continue to require their retained key. Ordinary password sign-in cannot register an unknown device. A newly paired device can link an active account only after password and fresh device-proof verification. Pairing grants device admission, not account permissions. A revoked pairing approval denies sign-in, access-token validation, and refresh even when an account-device link exists. The desktop profile must match a paired device's approved organization and branch.

`DesktopEnrollment::pair` returns authenticated connection settings. Before completion, it stores the public signed transcript in `desktop-pairing-pending.json` and its SHA-256 integrity hash plus the session key in secret storage. This preserves CA bundles larger than a native credential entry without storing the code or private key in that file. It writes `desktop-pairing-ca.pem` and public `desktop-pairing.json` only after receipt verification. Retry uses the same retained key and pending request. `paired_connection` recovers the accepted settings; `discard_pending_pairing` clears only an abandoned exchange. It never removes the device key, saved trust, or product data. Pairing does not claim that an account is signed in.

The first server desktop uses `DesktopEnrollment::first_desktop_request` and the private local `eitmad-server enroll-first-desktop` command. This command receives a `LoginRequest` on stdin, verifies the active Owner password and device signature, and commits approval, an irreversible first-enrollment claim, and audit together. It has no HTTP route, runs no migrations, and starts no services. It accepts only a workshop with one branch and no prior account-device enrollment. Migration 18 backfills claims for retained enrolled tenants while enforcing each tenant context. Revocation cannot reopen first enrollment. If its response is interrupted, normal sign-in with the retained key recovers the committed enrollment.

Configure `EITMAD_SERVER_PAIRING_ENDPOINT` with the canonical HTTPS origin ending in `/`, and `EITMAD_SERVER_PUBLIC_CA` with the public CA PEM path. Both are required together with TLS. The server loads the same persisted discovery identity from the update-manifest directory. Newly generated Windows host configuration supplies these references. Existing hosts can retain manual trusted enrollment and invitation activation until pairing configuration is supplied; no development step changes a live host.

Focused checks: `cargo test -p eitmad-pairing` covers incorrect codes, interception, every trust-field substitution, changed keys, and forged receipts. The ignored `server/host/tests/pairing.rs` test requires a fresh `EITMAD_PAIRING_TEST_DATABASE_URL` owned by a non-superuser without RLS bypass. It checks authorization, attempts, expiry, audit rollback, single-use approval, login enforcement, and restart recovery. The existing direct-route desktop enrollment scenario now pairs each independent client before sign-in and checks retained key and data recovery through real TLS. The ignored `pairing_migration_preserves_enrollment_and_claims_each_retained_tenant` control-plane test proves that migration 18 preserves retained device keys and links, backfills claims in two enforced tenant contexts, and is safe to rerun.

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

Pricing and catalog-revision HTTP routes and catalog sync streams require protocol `1.17`. Minor-16 HTTP peers are rejected with `eitmad.error.server-client-incompatible.v1` before authentication or catalog validation. `/v1/catalog-revisions/synchronize` requires `eitmad.capability.catalog-revisions.v1`; pricing routes require `eitmad.capability.pricing.v1`. Catalog writes authorize the authenticated organization's Manager relationship, preserve immutable revisions under tenant RLS, and record redacted audit evidence. Price confirmation resolves cost and compatible options from those accepted revisions. The host also registers Material, Part, Product, Furniture, public catalog, and Pricing sync handlers. PostgreSQL migration 9 extends category kinds and binds retained catalog scope to the tenant's organization. Migration 10 adds indexed public Furniture image references, populated in the same projection transaction and cleared on tombstones. Product field projection checks Manager access once per page in the caller's tenant transaction; it does not acquire another pool connection. See [catalog replication](synchronization.md#catalog-replication) and [Pricing contracts and recovery](pricing.md).

## Identity, authentication, and sessions

The CLI creates the first tenant, organization, owner account, owner relationships, default update assignment, audit row, and activation invitation in one transaction. Later accounts use the [supported account operations](../../operations/run-server-authority.md#manage-shared-server-accounts). Owner authority is an explicit tenant relationship, separate from operational assignments. Invitation delivery remains an operator task; no email provider is implemented.

Login uses a tenant code and username. Usernames accept Arabic Unicode text after whitespace normalization and case folding, but reject bidirectional control characters. Tenant, organization, and other display names apply the same bidirectional-control rejection so mixed Arabic and Latin reports cannot be reordered by hidden format characters. Login denials are intentionally non-specific: an absent account and an inactive or locked account return the same redacted failure. Bootstrap serializes its tenant-count check through a PostgreSQL advisory lock so two concurrent bootstrap calls cannot both observe an empty `control.tenants` and both commit. Device-proof clock-skew validation uses checked arithmetic and rejects extreme client-supplied timestamps instead of overflowing. Passwords use Argon2 hashes. Access and refresh tokens are opaque random values; PostgreSQL stores only keyed HMAC-SHA-256 hashes. The default policy is:

| Limit | Default |
| --- | --- |
| Access-token lifetime | 15 minutes |
| Refresh-token lifetime | 30 days |
| Session idle limit | 14 days |
| Device-proof clock skew | 5 minutes |

Every access-authenticated request must include the access token, device ID, timestamp, nonce, and an Ed25519 signature over the canonical proof bytes. A nonce can be used once in its validity window. A registered device ID cannot be rebound to another public key. Refresh-token reuse revokes the token family. MFA remains unimplemented. Bootstrap returns the initial owner activation secret directly; no invitation delivery provider ships.

## Storage, scope, and audit invariants

Migrations `0001_control_foundation.sql`, `0002_sync_foundation.sql`, `0003_admin_foundation.sql`, `0004_server_audit_envelope.sql`, `0005_customer_branches.sql`, `0006_catalog_images.sql`, `0007_pricing.sql`, `0008_catalog_revisions.sql`, `0009_catalog_synchronization.sql`, and `0010_catalog_image_references.sql` own the PostgreSQL schema. Control migration `0016_server_accounts.sql` adds the [shared account lifecycle](identity-foundation.md#shared-server-account-lifecycle) revisions and receipts. Migration `0017_desktop_accounts.sql` adds display names and the durable account-change cursor used by [connected desktop administration](identity-foundation.md#connected-desktop-account-administration). Every tenant-scoped table has a `tenant_id`, enables row-level security, and forces row-level security. Rust opens a transaction and sets the tenant context before scoped access. Application credentials must not have a PostgreSQL role that can bypass RLS. The [catalog image capability](catalog-images.md) owns immutable organization asset transfer through authenticated HTTP routes.

Every accepted state change adds a redacted audit record in the same transaction. `server/audit` is the only PostgreSQL audit contract. It records actor kind, optional session and principal, tenant and optional workspace, exact scope, target kind and target ID, operation, outcome, correlation, optional causation and idempotency, stable redacted failure ID, and time. Each control-plane and sync-plane entry point receives a caller-supplied correlation identifier, so records from one request remain joinable. Invalid and denied sync boundaries are recorded in a separate mandatory transaction before the operation returns; if that append fails, the boundary fails closed as unavailable. Successful mutations, conflicts, snapshots, and acknowledgements append in the authoritative state transaction. Token plaintext, password input, device private keys, domain payloads, and customer content must not enter logs or audit metadata.

Migration files are append-only after release. Back up PostgreSQL before migration and restore the complete cluster or database by the approved PostgreSQL recovery process. Do not edit rows, RLS policies, checkpoints, operation history, or conflict records as a repair shortcut.

## Sync modes and flows

Customer contacts and private Material, Part, Product, and Furniture streams use `LocalFirst`. Public catalog and Pricing streams use `ServerAuthoritative`. The coordinator accepts an authorized local operation, assigns ordered server history, projects the record, and creates a conflict when the base revision is stale and the domain cannot resolve it safely. There is no server-authoritative command submission entry point. The host rejects unsupported command traffic; the client contract retains its typed mode and stored command state.

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
