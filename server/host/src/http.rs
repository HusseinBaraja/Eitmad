use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use axum::{
    Json, Router,
    extract::{
        Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use eitmad_admin_plane::{AdministrationService, AdministrativeError};
use eitmad_contracts::{
    administration::{
        BackupStatus, DeviceVisibility, DiagnosticSummary, MigrationStatus, ServiceHealth,
        StartSupportWorkflow, SupportWorkflow, TenantVisibility,
    },
    identity::{ScopeId, ScopeKind, ScopeRef},
    relay::{
        OpenRelaySession, RelayFailureReport, RelayHealth, RelaySessionId, RelaySessionMetadata,
    },
    server::{
        ActivateAccountRequest, AuthenticationResult, DeviceProof, EffectiveUpdateAssignment,
        LoginRequest, RefreshRequest, ServerClientMessage, ServerErrorCode, ServerFailure,
        ServerMessage,
    },
    sync::{SnapshotCompletion, SyncMessage},
    sync_transport::SyncTransportPayload,
    transport::{CapabilityId, CorrelationId, SchemaId},
    updates::{ReleaseVersion, SignedUpdateManifest, UpdateCheckOutcome, UpdateClientProfile},
    versioning::{
        NegotiationOutcome, PeerHello, PeerKind, SchemaSupport, SupportedProtocol, negotiate,
    },
};
use eitmad_control_plane::{
    AuthenticationError, ControlPlane, UpdateAssignmentError, unix_millis_now,
};
use eitmad_relay_plane::{RelayCoordinator, RelayError};
use eitmad_sync_plane::{OperationError, SnapshotError, SubscriptionError, SyncCoordinator};
use eitmad_update_plane::{UpdateCatalog, UpdatePlaneError};
use serde::Deserialize;
use uuid::Uuid;

use crate::ServerConfig;

#[derive(Clone)]
pub struct ServerState {
    control: ControlPlane,
    sync: SyncCoordinator,
    server_hello: PeerHello,
    ready: Arc<AtomicBool>,
    relay: Option<RelayCoordinator>,
    updates: Option<UpdateCatalog>,
    administration: Option<AdministrationService>,
}

impl ServerState {
    #[must_use]
    pub fn new(control: ControlPlane, sync: SyncCoordinator) -> Self {
        let schemas = sync
            .domains()
            .descriptors()
            .into_iter()
            .map(|descriptor| SchemaSupport {
                schema_id: descriptor.schema_id,
                minimum_version: descriptor.minimum_schema_version,
                maximum_version: descriptor.maximum_schema_version,
                required: false,
            })
            .collect();
        Self {
            control,
            sync,
            server_hello: server_hello(schemas),
            ready: Arc::new(AtomicBool::new(true)),
            relay: None,
            updates: None,
            administration: None,
        }
    }

    #[must_use]
    pub fn with_planes(
        mut self,
        relay: RelayCoordinator,
        updates: UpdateCatalog,
        administration: AdministrationService,
    ) -> Self {
        self.relay = Some(relay);
        self.updates = Some(updates);
        self.administration = Some(administration);
        self
    }

    pub fn set_ready(&self, value: bool) {
        self.ready.store(value, Ordering::Release);
    }
}

pub fn router(state: ServerState) -> Router {
    Router::new()
        .route("/livez", get(live))
        .route("/readyz", get(ready))
        .route("/v1/auth/activate", post(activate))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/update-assignment", get(update_assignment))
        .route("/v1/updates/check", post(check_update))
        .route("/v1/admin/update-manifests", post(publish_update_manifest))
        .route("/v1/relay/sessions", post(open_relay_session))
        .route(
            "/v1/relay/sessions/{session_id}/heartbeat",
            post(relay_heartbeat),
        )
        .route(
            "/v1/relay/sessions/{session_id}/reconnect",
            post(schedule_relay_reconnect),
        )
        .route(
            "/v1/relay/sessions/{session_id}/reconnect/attempt",
            post(attempt_relay_reconnect),
        )
        .route(
            "/v1/relay/sessions/{session_id}/close",
            post(close_relay_session),
        )
        .route("/v1/relay/failures", post(report_relay_failure))
        .route("/v1/relay/health", get(relay_health))
        .route("/v1/admin/diagnostics", get(admin_diagnostics))
        .route("/v1/admin/health", get(admin_health))
        .route("/v1/admin/backup-status", get(admin_backup_status))
        .route("/v1/admin/migration-status", get(admin_migration_status))
        .route("/v1/admin/audit", get(admin_audit))
        .route("/v1/admin/tenant", get(admin_tenant))
        .route("/v1/admin/devices", get(admin_devices))
        .route("/v1/admin/support-workflows", post(start_support_workflow))
        .route("/v1/connect", get(connect))
        .with_state(state)
}

/// Runs the combined host with TLS, or with explicit loopback-only development transport.
///
/// # Errors
///
/// Returns an I/O or TLS configuration error after graceful shutdown is requested.
pub async fn run(
    config: &ServerConfig,
    state: ServerState,
) -> Result<(), Box<dyn std::error::Error>> {
    let application = router(state.clone());
    let address = config.listen;
    if let (Some(certificate), Some(private_key)) =
        (&config.tls_certificate, &config.tls_private_key)
    {
        let tls =
            axum_server::tls_rustls::RustlsConfig::from_pem_file(certificate, private_key).await?;
        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();
        let shutdown_state = state.clone();
        tokio::spawn(async move {
            shutdown_signal(shutdown_state).await;
            shutdown_handle.graceful_shutdown(Some(std::time::Duration::from_secs(30)));
        });
        axum_server::bind_rustls(address, tls)
            .handle(handle)
            .serve(application.into_make_service())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, application)
            .with_graceful_shutdown(shutdown_signal(state))
            .await?;
    }
    Ok(())
}

async fn shutdown_signal(state: ServerState) {
    let _ = tokio::signal::ctrl_c().await;
    state.set_ready(false);
}

async fn live() -> StatusCode {
    StatusCode::NO_CONTENT
}

fn new_correlation_id() -> CorrelationId {
    CorrelationId::new(Uuid::new_v4())
}

async fn ready(State(state): State<ServerState>) -> StatusCode {
    if state.ready.load(Ordering::Acquire) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn activate(
    State(state): State<ServerState>,
    Json(request): Json<ActivateAccountRequest>,
) -> Result<Json<AuthenticationResult>, ApiError> {
    state
        .control
        .authentication
        .activate(&request, new_correlation_id(), unix_millis_now())
        .await
        .map(Json)
        .map_err(ApiError::authentication)
}

async fn login(
    State(state): State<ServerState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthenticationResult>, ApiError> {
    state
        .control
        .authentication
        .login(&request, new_correlation_id(), unix_millis_now())
        .await
        .map(Json)
        .map_err(ApiError::authentication)
}

async fn refresh(
    State(state): State<ServerState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<AuthenticationResult>, ApiError> {
    state
        .control
        .authentication
        .refresh(&request, new_correlation_id(), unix_millis_now())
        .await
        .map(Json)
        .map_err(ApiError::authentication)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateQuery {
    device_id: Uuid,
}

async fn update_assignment(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<UpdateQuery>,
) -> Result<Json<EffectiveUpdateAssignment>, ApiError> {
    let session = authenticate_headers(&state, &headers).await?;
    if session.device_id.value() != query.device_id {
        return Err(ApiError::forbidden("eitmad.error.authorization-denied.v1"));
    }
    state
        .control
        .update_assignments
        .effective(session.tenant_id, session.device_id)
        .await
        .map(Json)
        .map_err(|error| match error {
            UpdateAssignmentError::Invalid => {
                ApiError::bad_request("eitmad.error.contract-invalid.v1")
            }
            UpdateAssignmentError::Denied => {
                ApiError::forbidden("eitmad.error.authorization-denied.v1")
            }
            UpdateAssignmentError::Unavailable => ApiError::unavailable(),
        })
}

async fn check_update(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(client): Json<UpdateClientProfile>,
) -> Result<Json<UpdateCheckOutcome>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-update-distribution.v1",
    )
    .await?;
    if client.device_id != session.device_id {
        return Err(ApiError::forbidden("eitmad.error.authorization-denied.v1"));
    }
    let assignment = state
        .control
        .update_assignments
        .effective(session.tenant_id, session.device_id)
        .await
        .map_err(|_| ApiError::unavailable())?;
    if assignment.channel != client.channel {
        return Err(ApiError::forbidden("eitmad.error.authorization-denied.v1"));
    }
    let updates = state.updates.as_ref().ok_or_else(ApiError::unavailable)?;
    updates
        .check(&client, unix_millis_now())
        .map(Json)
        .map_err(map_update_plane)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishManifestRequest {
    manifest: SignedUpdateManifest,
    correlation_id: CorrelationId,
}

async fn publish_update_manifest(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<PublishManifestRequest>,
) -> Result<StatusCode, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-update-distribution.v1",
    )
    .await?;
    state
        .updates
        .as_ref()
        .ok_or_else(ApiError::unavailable)?
        .publish(
            &session,
            &request.manifest,
            request.correlation_id,
            unix_millis_now(),
        )
        .await
        .map_err(map_update_plane)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn open_relay_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<OpenRelaySession>,
) -> Result<Json<RelaySessionMetadata>, ApiError> {
    let session =
        authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1").await?;
    state
        .relay
        .as_ref()
        .ok_or_else(ApiError::unavailable)?
        .open(&session, &request, unix_millis_now())
        .await
        .map(Json)
        .map_err(map_relay)
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelayActionRequest {
    correlation_id: CorrelationId,
}

async fn relay_heartbeat(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<Uuid>,
    Json(request): Json<RelayActionRequest>,
) -> Result<Json<RelaySessionMetadata>, ApiError> {
    let session =
        authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1").await?;
    relay(&state)?
        .heartbeat(
            &session,
            RelaySessionId::new(session_id),
            request.correlation_id,
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_relay)
}

async fn schedule_relay_reconnect(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<Uuid>,
    Json(request): Json<RelayActionRequest>,
) -> Result<Json<RelaySessionMetadata>, ApiError> {
    let session =
        authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1").await?;
    relay(&state)?
        .schedule_reconnect(
            &session,
            RelaySessionId::new(session_id),
            request.correlation_id,
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_relay)
}

async fn attempt_relay_reconnect(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<Uuid>,
    Json(request): Json<RelayActionRequest>,
) -> Result<Json<RelaySessionMetadata>, ApiError> {
    let session =
        authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1").await?;
    relay(&state)?
        .reconnect_due(
            &session,
            RelaySessionId::new(session_id),
            request.correlation_id,
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_relay)
}

async fn close_relay_session(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<Uuid>,
    Json(request): Json<RelayActionRequest>,
) -> Result<Json<RelaySessionMetadata>, ApiError> {
    let session =
        authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1").await?;
    relay(&state)?
        .close(
            &session,
            RelaySessionId::new(session_id),
            request.correlation_id,
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_relay)
}

async fn report_relay_failure(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(report): Json<RelayFailureReport>,
) -> Result<StatusCode, ApiError> {
    let session =
        authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1").await?;
    relay(&state)?
        .report_failure(&session, report)
        .await
        .map_err(map_relay)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn relay_health(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<RelayHealth>, ApiError> {
    let session =
        authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1").await?;
    relay(&state)?
        .health(&session, new_correlation_id(), unix_millis_now())
        .await
        .map(Json)
        .map_err(map_relay)
}

async fn admin_diagnostics(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<DiagnosticSummary>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    administration(&state)?
        .diagnostics(
            &session,
            session.tenant_id,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_admin)
}

async fn admin_health(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ServiceHealth>>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    administration(&state)?
        .health(
            &session,
            session.tenant_id,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_admin)
}

async fn admin_backup_status(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<BackupStatus>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    administration(&state)?
        .backup_status(
            &session,
            session.tenant_id,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_admin)
}

async fn admin_migration_status(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<MigrationStatus>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    administration(&state)?
        .migration_status(
            &session,
            session.tenant_id,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_admin)
}

#[derive(Deserialize)]
struct AdminAuditQuery {
    limit: Option<u32>,
}

async fn admin_audit(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<AdminAuditQuery>,
) -> Result<Json<Vec<eitmad_contracts::administration::AdministrativeAuditRecord>>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    administration(&state)?
        .audit_records(
            &session,
            session.tenant_id,
            query.limit.unwrap_or(100),
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_admin)
}

async fn admin_tenant(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<TenantVisibility>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    administration(&state)?
        .tenant_visibility(
            &session,
            session.tenant_id,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_admin)
}

async fn admin_devices(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DeviceVisibility>>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    administration(&state)?
        .device_visibility(
            &session,
            session.tenant_id,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_admin)
}

async fn start_support_workflow(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<StartSupportWorkflow>,
) -> Result<Json<SupportWorkflow>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    administration(&state)?
        .start_support_workflow(&session, &request, unix_millis_now())
        .await
        .map(Json)
        .map_err(map_admin)
}

fn relay(state: &ServerState) -> Result<&RelayCoordinator, ApiError> {
    state.relay.as_ref().ok_or_else(ApiError::unavailable)
}

fn administration(state: &ServerState) -> Result<&AdministrationService, ApiError> {
    state
        .administration
        .as_ref()
        .ok_or_else(ApiError::unavailable)
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectQuery {
    scope_kind: String,
    scope_id: Uuid,
    schema_id: String,
    schema_version: u32,
}

async fn connect(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<ConnectQuery>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let (token, proof) = connection_credentials(&headers)?;
    let session = authenticate_access(&state, &token, &proof).await?;
    let scope = ScopeRef {
        kind: ScopeKind::parse(query.scope_kind)
            .map_err(|_| ApiError::bad_request("eitmad.error.contract-invalid.v1"))?,
        id: ScopeId::new(query.scope_id),
    };
    let schema_id = SchemaId::parse(query.schema_id)
        .map_err(|_| ApiError::bad_request("eitmad.error.contract-invalid.v1"))?;
    Ok(upgrade
        .on_upgrade(move |socket| {
            stream_session(
                socket,
                state,
                StreamContext {
                    token,
                    proof,
                    session,
                },
                scope,
                schema_id,
                query.schema_version,
            )
        })
        .into_response())
}

const SESSION_REVALIDATION_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

struct StreamContext {
    token: String,
    proof: DeviceProof,
    session: eitmad_contracts::server::AuthenticatedServerSession,
}

async fn stream_session(
    mut socket: WebSocket,
    state: ServerState,
    context: StreamContext,
    scope: ScopeRef,
    schema_id: SchemaId,
    schema_version: u32,
) {
    let StreamContext {
        token,
        proof,
        session,
    } = context;
    let mut negotiated = false;
    let mut revalidation = tokio::time::interval(SESSION_REVALIDATION_INTERVAL);
    revalidation.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    revalidation.reset();
    loop {
        let received = tokio::select! {
            _ = revalidation.tick() => {
                if authenticate_access(&state, &token, &proof).await.is_err() {
                    let _ = send_failure(&mut socket, "eitmad.error.server-authentication-failed.v1")
                        .await;
                    break;
                }
                continue;
            }
            received = socket.recv() => received,
        };
        let Some(Ok(message)) = received else {
            break;
        };
        let Message::Text(text) = message else {
            continue;
        };
        let Ok(message) = serde_json::from_str::<ServerClientMessage>(&text) else {
            if send_failure(&mut socket, "eitmad.error.contract-invalid.v1")
                .await
                .is_err()
            {
                break;
            }
            continue;
        };
        if !negotiated {
            let ServerClientMessage::Hello(hello) = message else {
                let _ =
                    send_failure(&mut socket, "eitmad.error.server-client-incompatible.v1").await;
                break;
            };
            if hello.api_version != eitmad_contracts::server::SERVER_API_VERSION
                || !matches!(
                    negotiate(&state.server_hello, &hello.peer),
                    NegotiationOutcome::Accepted(_)
                )
            {
                let _ =
                    send_failure(&mut socket, "eitmad.error.server-client-incompatible.v1").await;
                break;
            }
            if send_server_message(
                &mut socket,
                &ServerMessage::Hello(state.server_hello.clone()),
            )
            .await
            .is_err()
            {
                break;
            }
            negotiated = true;
            continue;
        }
        let result = handle_stream_message(
            &mut socket,
            &state,
            &session,
            &scope,
            &schema_id,
            schema_version,
            message,
        )
        .await;
        if let Err(error) = result {
            if send_failure(&mut socket, error.code.as_str())
                .await
                .is_err()
            {
                break;
            }
        }
    }
}
async fn handle_stream_message(
    socket: &mut WebSocket,
    state: &ServerState,
    session: &eitmad_contracts::server::AuthenticatedServerSession,
    scope: &ScopeRef,
    schema_id: &SchemaId,
    schema_version: u32,
    message: ServerClientMessage,
) -> Result<(), ApiError> {
    match message {
        ServerClientMessage::Subscribe(request) if request.schema_id == *schema_id => {
            let page = state
                .sync
                .subscription_page(eitmad_sync_plane::SubscriptionPageRequest {
                    session,
                    scope,
                    schema_id: &request.schema_id,
                    schema_version,
                    resume_after: request.resume_after,
                    maximum_events: u32::try_from(eitmad_contracts::sync::MAX_SYNC_BATCH_RECORDS)
                        .unwrap_or(u32::MAX),
                    correlation_id: new_correlation_id(),
                    now: unix_millis_now(),
                })
                .await
                .map_err(map_subscription)?;
            for event in page.events {
                send_server_message(socket, &ServerMessage::Event(event))
                    .await
                    .map_err(|()| ApiError::unavailable())?;
            }
            Ok(())
        }
        ServerClientMessage::Hello(_) | ServerClientMessage::Subscribe(_) => Err(
            ApiError::bad_request("eitmad.error.server-client-incompatible.v1"),
        ),
        ServerClientMessage::Acknowledge(_) => Err(ApiError::bad_request(
            "eitmad.error.server-subscription-ack-unsupported.v1",
        )),
        ServerClientMessage::Sync(frame) => match frame.payload {
            SyncTransportPayload::Message(SyncMessage::Pull(request)) => match state
                .sync
                .pull(eitmad_sync_plane::PullPageRequest {
                    session,
                    scope,
                    schema_id,
                    schema_version,
                    after: request.after,
                    maximum_records: request.maximum_records,
                    correlation_id: frame.correlation_id,
                    now: unix_millis_now(),
                })
                .await
            {
                Ok(batch) => {
                    send_server_message(socket, &ServerMessage::Sync(SyncMessage::Changes(batch)))
                        .await
                        .map_err(|()| ApiError::unavailable())
                }
                Err(OperationError::SnapshotRequired) => {
                    send_snapshot(
                        socket,
                        state,
                        session,
                        scope,
                        schema_id,
                        schema_version,
                        frame.correlation_id,
                    )
                    .await
                }
                Err(error) => Err(map_operation(error)),
            },
            SyncTransportPayload::Message(SyncMessage::Acknowledge(acknowledgement)) => state
                .sync
                .acknowledge(eitmad_sync_plane::AcknowledgeRequest {
                    session,
                    scope,
                    schema_id,
                    schema_version,
                    acknowledgement: &acknowledgement,
                    correlation_id: frame.correlation_id,
                    now: unix_millis_now(),
                })
                .await
                .map_err(map_operation),
            _ => Err(ApiError::bad_request("eitmad.error.contract-invalid.v1")),
        },
    }
}

async fn send_snapshot(
    socket: &mut WebSocket,
    state: &ServerState,
    session: &eitmad_contracts::server::AuthenticatedServerSession,
    scope: &ScopeRef,
    schema_id: &SchemaId,
    schema_version: u32,
    correlation_id: CorrelationId,
) -> Result<(), ApiError> {
    const SNAPSHOT_VALIDITY_MS: i64 = 24 * 60 * 60 * 1_000;

    let bundle = state
        .sync
        .create_snapshot(
            eitmad_sync_plane::SnapshotRequest {
                session,
                scope,
                schema_id,
                schema_version,
            },
            correlation_id,
            unix_millis_now(),
            SNAPSHOT_VALIDITY_MS,
        )
        .await
        .map_err(|error| map_snapshot(&error))?;
    let completion = SnapshotCompletion {
        snapshot_id: bundle.manifest.snapshot_id,
        checksum: bundle.manifest.checksum.clone(),
    };
    send_server_message(
        socket,
        &ServerMessage::Sync(SyncMessage::SnapshotManifest(bundle.manifest)),
    )
    .await
    .map_err(|()| ApiError::unavailable())?;
    for chunk in bundle.chunks {
        send_server_message(
            socket,
            &ServerMessage::Sync(SyncMessage::SnapshotChunk(chunk)),
        )
        .await
        .map_err(|()| ApiError::unavailable())?;
    }
    send_server_message(
        socket,
        &ServerMessage::Sync(SyncMessage::SnapshotComplete(completion)),
    )
    .await
    .map_err(|()| ApiError::unavailable())
}

async fn send_server_message(socket: &mut WebSocket, message: &ServerMessage) -> Result<(), ()> {
    let encoded = serde_json::to_string(message).map_err(|_| ())?;
    socket
        .send(Message::Text(encoded.into()))
        .await
        .map_err(|_| ())
}

async fn send_failure(socket: &mut WebSocket, code: &str) -> Result<(), ()> {
    let code = ServerErrorCode::parse(code).map_err(|_| ())?;
    send_server_message(
        socket,
        &ServerMessage::Failure(ServerFailure {
            code,
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            retry_after_ms: None,
        }),
    )
    .await
}

async fn authenticate_headers(
    state: &ServerState,
    headers: &HeaderMap,
) -> Result<eitmad_contracts::server::AuthenticatedServerSession, ApiError> {
    let (token, proof) = connection_credentials(headers)?;
    authenticate_access(state, &token, &proof).await
}

async fn authenticate_negotiated(
    state: &ServerState,
    headers: &HeaderMap,
    required_capability: &'static str,
) -> Result<eitmad_contracts::server::AuthenticatedServerSession, ApiError> {
    let required_capability =
        CapabilityId::parse(required_capability).expect("required server capability must be valid");
    let peer = headers
        .get("x-eitmad-peer-hello")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| URL_SAFE_NO_PAD.decode(value).ok())
        .and_then(|value| serde_json::from_slice::<PeerHello>(&value).ok())
        .ok_or_else(|| ApiError::bad_request("eitmad.error.server-client-incompatible.v1"))?;
    let mut boundary = state.server_hello.clone();
    boundary.required_capabilities = vec![required_capability];
    let NegotiationOutcome::Accepted(negotiated) = negotiate(&boundary, &peer) else {
        return Err(ApiError::bad_request(
            "eitmad.error.server-client-incompatible.v1",
        ));
    };
    if negotiated.protocol.major != 1 || negotiated.protocol.minor < 5 {
        return Err(ApiError::bad_request(
            "eitmad.error.server-client-incompatible.v1",
        ));
    }
    authenticate_headers(state, headers).await
}

fn connection_credentials(headers: &HeaderMap) -> Result<(String, DeviceProof), ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer ").map(str::to_owned))
        .ok_or_else(|| ApiError::unauthorized("eitmad.error.server-authentication-failed.v1"))?;
    let proof = headers
        .get("x-eitmad-device-proof")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| URL_SAFE_NO_PAD.decode(value).ok())
        .and_then(|value| serde_json::from_slice::<DeviceProof>(&value).ok())
        .ok_or_else(|| ApiError::unauthorized("eitmad.error.server-device-proof-invalid.v1"))?;
    Ok((token, proof))
}

async fn authenticate_access(
    state: &ServerState,
    token: &str,
    proof: &DeviceProof,
) -> Result<eitmad_contracts::server::AuthenticatedServerSession, ApiError> {
    state
        .control
        .authentication
        .authenticate_access(token, proof, unix_millis_now())
        .await
        .map_err(ApiError::authentication)
}

fn server_hello(schemas: Vec<SchemaSupport>) -> PeerHello {
    let capabilities = [
        "eitmad.capability.sync.v1",
        "eitmad.capability.server-connection.v1",
        "eitmad.capability.server-device-proof.v1",
        "eitmad.capability.server-snapshot-chunks.v1",
        "eitmad.capability.server-subscription-resume.v1",
        "eitmad.capability.server-relay.v1",
        "eitmad.capability.server-update-distribution.v1",
        "eitmad.capability.server-administration.v1",
    ]
    .into_iter()
    .map(|value| CapabilityId::parse(value).expect("server capability must be valid"))
    .collect::<Vec<_>>();
    PeerHello {
        peer_kind: PeerKind::Server,
        product_version: ReleaseVersion::new(semver::Version::new(0, 0, 0)),
        protocols: vec![SupportedProtocol {
            major: 1,
            minimum_minor: 4,
            maximum_minor: 6,
        }],
        required_capabilities: capabilities.clone(),
        capabilities,
        schemas,
    }
}

#[derive(Clone, Debug)]
struct ApiError {
    status: StatusCode,
    code: ServerErrorCode,
}

impl ApiError {
    fn authentication(error: AuthenticationError) -> Self {
        let code = match error {
            AuthenticationError::TokenExpired => "eitmad.error.server-token-expired.v1",
            AuthenticationError::TokenReuse => "eitmad.error.server-token-reuse.v1",
            AuthenticationError::InvalidDeviceProof => {
                "eitmad.error.server-device-proof-invalid.v1"
            }
            AuthenticationError::Unavailable => return Self::unavailable(),
            _ => "eitmad.error.server-authentication-failed.v1",
        };
        Self::unauthorized(code)
    }

    fn unauthorized(code: &str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code)
    }

    fn forbidden(code: &str) -> Self {
        Self::new(StatusCode::FORBIDDEN, code)
    }

    fn bad_request(code: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code)
    }

    fn unavailable() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "eitmad.error.config-unavailable.v1",
        )
    }

    fn new(status: StatusCode, code: &str) -> Self {
        Self {
            status,
            code: ServerErrorCode::parse(code).expect("server error code must be valid"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ServerFailure {
                code: self.code,
                correlation_id: CorrelationId::new(Uuid::new_v4()),
                retry_after_ms: None,
            }),
        )
            .into_response()
    }
}

fn map_operation(error: OperationError) -> ApiError {
    match error {
        OperationError::Denied => ApiError::forbidden("eitmad.error.authorization-denied.v1"),
        OperationError::IdempotencyMismatch => {
            ApiError::bad_request("eitmad.error.server-idempotency-mismatch.v1")
        }
        OperationError::UnknownDomain => {
            ApiError::bad_request("eitmad.error.server-client-incompatible.v1")
        }
        OperationError::SnapshotRequired | OperationError::UnknownRecord => {
            ApiError::bad_request("eitmad.error.server-snapshot-required.v1")
        }
        OperationError::Unavailable => ApiError::unavailable(),
        _ => ApiError::bad_request("eitmad.error.contract-invalid.v1"),
    }
}

fn map_relay(error: RelayError) -> ApiError {
    match error {
        RelayError::Denied => ApiError::forbidden("eitmad.error.authorization-denied.v1"),
        RelayError::Invalid | RelayError::RetryNotDue => {
            ApiError::bad_request("eitmad.error.contract-invalid.v1")
        }
        RelayError::NotFound => ApiError::new(
            StatusCode::NOT_FOUND,
            "eitmad.error.relay-session-not-found.v1",
        ),
        RelayError::RouteUnavailable | RelayError::Unavailable => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "eitmad.error.relay-unavailable.v1",
        ),
    }
}

fn map_update_plane(error: UpdatePlaneError) -> ApiError {
    match error {
        UpdatePlaneError::Denied => ApiError::forbidden("eitmad.error.authorization-denied.v1"),
        UpdatePlaneError::Invalid | UpdatePlaneError::Conflict => {
            ApiError::bad_request("eitmad.error.update-manifest-invalid.v1")
        }
        UpdatePlaneError::NotFound => ApiError::new(
            StatusCode::NOT_FOUND,
            "eitmad.error.update-manifest-not-found.v1",
        ),
        UpdatePlaneError::Unavailable | UpdatePlaneError::ReconciliationRequired(_) => {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "eitmad.error.update-distribution-unavailable.v1",
            )
        }
    }
}

fn map_admin(error: AdministrativeError) -> ApiError {
    match error {
        AdministrativeError::Denied => ApiError::forbidden("eitmad.error.authorization-denied.v1"),
        AdministrativeError::Invalid => ApiError::bad_request("eitmad.error.contract-invalid.v1"),
        AdministrativeError::Unavailable => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "eitmad.error.admin-unavailable.v1",
        ),
    }
}

fn map_subscription(error: SubscriptionError) -> ApiError {
    match error {
        SubscriptionError::Denied => ApiError::forbidden("eitmad.error.authorization-denied.v1"),
        SubscriptionError::ResyncRequired => {
            ApiError::bad_request("eitmad.error.server-snapshot-required.v1")
        }
        SubscriptionError::Unavailable => ApiError::unavailable(),
        SubscriptionError::Invalid => ApiError::bad_request("eitmad.error.contract-invalid.v1"),
    }
}

fn map_snapshot(error: &SnapshotError) -> ApiError {
    match error {
        SnapshotError::Denied => ApiError::forbidden("eitmad.error.authorization-denied.v1"),
        SnapshotError::Domain => {
            ApiError::bad_request("eitmad.error.server-client-incompatible.v1")
        }
        SnapshotError::Empty => ApiError::bad_request("eitmad.error.server-snapshot-required.v1"),
        SnapshotError::Unavailable => ApiError::unavailable(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_requires_all_remote_boundary_capabilities() {
        let hello = server_hello(Vec::new());
        assert_eq!(hello.protocols[0].minimum_minor, 4);
        assert_eq!(hello.protocols[0].maximum_minor, 6);
        assert!(hello.required_capabilities.iter().any(|capability| {
            capability.as_str() == "eitmad.capability.server-device-proof.v1"
        }));
    }

    #[test]
    fn authentication_errors_are_redacted_and_stable() {
        let error = ApiError::authentication(AuthenticationError::Failed);
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            error.code.as_str(),
            "eitmad.error.server-authentication-failed.v1"
        );
    }

    #[test]
    fn server_rejects_clients_without_required_remote_capabilities() {
        let server = server_hello(Vec::new());
        let client = PeerHello {
            peer_kind: PeerKind::Engine,
            product_version: ReleaseVersion::new(semver::Version::new(1, 0, 0)),
            protocols: vec![SupportedProtocol {
                major: 1,
                minimum_minor: 4,
                maximum_minor: 4,
            }],
            capabilities: vec![CapabilityId::parse("eitmad.capability.sync.v1").unwrap()],
            required_capabilities: Vec::new(),
            schemas: Vec::new(),
        };
        assert!(matches!(
            negotiate(&server, &client),
            NegotiationOutcome::Rejected(_)
        ));
    }

    fn test_state() -> ServerState {
        let pool = eitmad_postgres_support::PgPoolOptions::new()
            .connect_lazy("postgresql://unreachable.invalid/eitmad")
            .unwrap();
        let control = ControlPlane::new(pool.clone(), eitmad_control_plane::TokenKey::new([0; 32]));
        let registry = eitmad_sync_plane::DomainRegistry::new(std::iter::empty()).unwrap();
        let database = eitmad_sync_plane::SyncDatabase::from_pool(pool);
        let sync = SyncCoordinator::new(&database, registry);
        ServerState::new(control, sync)
    }

    #[tokio::test]
    async fn readyz_reports_unavailable_after_readiness_is_cleared() {
        use tower::ServiceExt as _;
        let state = test_state();
        let response = router(state.clone())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/readyz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        state.set_ready(false);
        let response = router(state)
            .oneshot(
                axum::http::Request::builder()
                    .uri("/readyz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn operational_http_boundaries_negotiate_before_authentication() {
        let state = test_state();
        let headers = HeaderMap::new();
        let error = authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1")
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            error.code.as_str(),
            "eitmad.error.server-client-incompatible.v1"
        );

        let mut peer = server_hello(Vec::new());
        peer.peer_kind = PeerKind::Engine;
        peer.protocols[0].minimum_minor = 5;
        let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&peer).unwrap());
        let mut headers = HeaderMap::new();
        headers.insert("x-eitmad-peer-hello", encoded.parse().unwrap());
        let error = authenticate_negotiated(&state, &headers, "eitmad.capability.server-relay.v1")
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn update_assignment_requires_authorization_header() {
        use tower::ServiceExt as _;
        let response = router(test_state())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/update-assignment?deviceId=00000000-0000-0000-0000-000000000000")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn relay_and_administration_routes_require_authentication() {
        use tower::ServiceExt as _;
        let mut peer = server_hello(Vec::new());
        peer.peer_kind = PeerKind::Engine;
        peer.protocols[0].minimum_minor = 5;
        let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&peer).unwrap());
        for uri in ["/v1/relay/health", "/v1/admin/health", "/v1/admin/devices"] {
            let response = router(test_state())
                .oneshot(
                    axum::http::Request::builder()
                        .uri(uri)
                        .header("x-eitmad-peer-hello", &encoded)
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{uri}");
        }
    }
}
