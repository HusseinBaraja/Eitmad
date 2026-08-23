use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use axum::{
    Json, Router,
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use eitmad_contracts::{
    identity::{ScopeId, ScopeKind, ScopeRef},
    server::{
        ActivateAccountRequest, AuthenticationResult, DeviceProof, EffectiveUpdateAssignment,
        LoginRequest, RefreshRequest, ServerClientMessage, ServerErrorCode, ServerFailure,
        ServerMessage,
    },
    sync::{SnapshotCompletion, SyncMessage},
    sync_transport::SyncTransportPayload,
    transport::{CapabilityId, CorrelationId, SchemaId},
    updates::ReleaseVersion,
    versioning::{
        NegotiationOutcome, PeerHello, PeerKind, SchemaSupport, SupportedProtocol, negotiate,
    },
};
use eitmad_control_plane::{
    AuthenticationError, ControlPlane, UpdateAssignmentError, unix_millis_now,
};
use eitmad_sync_plane::{OperationError, SnapshotError, SubscriptionError, SyncCoordinator};
use serde::Deserialize;
use uuid::Uuid;

use crate::ServerConfig;

#[derive(Clone)]
pub struct ServerState {
    control: ControlPlane,
    sync: SyncCoordinator,
    server_hello: PeerHello,
    ready: Arc<AtomicBool>,
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
        }
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
    let session = authenticate_headers(&state, &headers).await?;
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
                session,
                scope,
                schema_id,
                query.schema_version,
            )
        })
        .into_response())
}

async fn stream_session(
    mut socket: WebSocket,
    state: ServerState,
    session: eitmad_contracts::server::AuthenticatedServerSession,
    scope: ScopeRef,
    schema_id: SchemaId,
    schema_version: u32,
) {
    let mut negotiated = false;
    while let Some(Ok(message)) = socket.recv().await {
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
                .subscription_page(
                    session,
                    scope,
                    &request.schema_id,
                    schema_version,
                    request.resume_after,
                    eitmad_contracts::sync::MAX_SYNC_BATCH_RECORDS as u32,
                )
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
        ServerClientMessage::Acknowledge(_) => Ok(()),
        ServerClientMessage::Sync(frame) => match frame.payload {
            SyncTransportPayload::Message(SyncMessage::Pull(request)) => match state
                .sync
                .pull(
                    session,
                    scope,
                    schema_id,
                    schema_version,
                    request.after,
                    request.maximum_records,
                )
                .await
            {
                Ok(batch) => {
                    send_server_message(socket, &ServerMessage::Sync(SyncMessage::Changes(batch)))
                        .await
                        .map_err(|()| ApiError::unavailable())
                }
                Err(OperationError::SnapshotRequired) => {
                    send_snapshot(socket, state, session, scope, schema_id, schema_version).await
                }
                Err(error) => Err(map_operation(error)),
            },
            SyncTransportPayload::Message(SyncMessage::Acknowledge(acknowledgement)) => state
                .sync
                .acknowledge(
                    session,
                    scope,
                    schema_id,
                    &acknowledgement,
                    unix_millis_now(),
                )
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
) -> Result<(), ApiError> {
    const SNAPSHOT_VALIDITY_MS: i64 = 24 * 60 * 60 * 1_000;

    let bundle = state
        .sync
        .create_snapshot(
            session,
            scope,
            schema_id,
            schema_version,
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
    let token = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::unauthorized("eitmad.error.server-authentication-failed.v1"))?;
    let proof = headers
        .get("x-eitmad-device-proof")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| URL_SAFE_NO_PAD.decode(value).ok())
        .and_then(|value| serde_json::from_slice::<DeviceProof>(&value).ok())
        .ok_or_else(|| ApiError::unauthorized("eitmad.error.server-device-proof-invalid.v1"))?;
    state
        .control
        .authentication
        .authenticate_access(token, &proof, unix_millis_now())
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
            maximum_minor: 4,
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
}
