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
        LoginRequest, RefreshRequest, RegisterBranchRequest, RegisteredBranch, ServerClientMessage,
        ServerErrorCode, ServerFailure, ServerMessage,
    },
    sync::{
        ChangeRecord, LocalChangeDisposition, LocalChangeResult, SnapshotCompletion, SyncMessage,
    },
    sync_transport::SyncTransportPayload,
    transport::{CapabilityId, CorrelationId, SchemaId},
    updates::{ReleaseVersion, SignedUpdateManifest, UpdateCheckOutcome, UpdateClientProfile},
    versioning::{
        NegotiatedSession, NegotiationOutcome, PeerHello, PeerKind, SchemaSupport,
        SupportedProtocol, negotiate,
    },
};
use eitmad_control_plane::{
    AuthenticationError, BranchError, ControlPlane, UpdateAssignmentError, unix_millis_now,
};
use eitmad_relay_plane::{RelayCoordinator, RelayError};
use eitmad_sync_plane::{
    LocalOperationDraft, OperationError, OperationResult, SnapshotError, SubscriptionError,
    SyncCoordinator,
};
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
    approval_notifications: Arc<tokio::sync::OnceCell<ApprovalNotifications>>,
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
            approval_notifications: Arc::new(tokio::sync::OnceCell::new()),
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

    async fn approval_notifications(
        &self,
    ) -> Result<tokio::sync::broadcast::Receiver<()>, eitmad_pricing::ApprovalError> {
        let notifications=self.approval_notifications.get_or_try_init(|| async {
            // One listener per host, counted in the sync pool, independent of client count.
            let mut listener=self.sync.quotation_approvals().listener().await?;
            let (wake,_)=tokio::sync::broadcast::channel(16);
            let (stop,mut stopping)=tokio::sync::watch::channel(false);
            let publish=wake.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _=stopping.changed()=>break,
                        received=listener.recv()=> {
                            let _=publish.send(());
                            if received.is_err() {tokio::time::sleep(std::time::Duration::from_secs(1)).await;}
                        }
                    }
                }
            });
            Ok(ApprovalNotifications{wake,stop})
        }).await?;
        Ok(notifications.wake.subscribe())
    }

    pub fn set_ready(&self, value: bool) {
        self.ready.store(value, Ordering::Release);
    }
}

struct ApprovalNotifications {
    wake: tokio::sync::broadcast::Sender<()>,
    stop: tokio::sync::watch::Sender<bool>,
}
impl Drop for ApprovalNotifications {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}

/// Registers server routes with bounded request bodies and shared authority state.
pub fn router(state: ServerState) -> Router {
    Router::new()
        .route("/livez", get(live))
        .route("/readyz", get(ready))
        .route("/v1/auth/activate", post(activate))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/refresh", post(refresh))
        .route(
            "/v1/catalog-images/upload",
            post(upload_catalog_image)
                .layer(axum::extract::DefaultBodyLimit::max(12 * 1024 * 1024)),
        )
        .route("/v1/catalog-images/read", post(read_catalog_image))
        .route(
            "/v1/catalog-revisions/synchronize",
            post(synchronize_catalog_revisions)
                .layer(axum::extract::DefaultBodyLimit::max(4 * 1024 * 1024)),
        )
        .route(
            "/v1/pricing/publish",
            post(publish_price).layer(axum::extract::DefaultBodyLimit::max(64 * 1024)),
        )
        .route("/v1/pricing/read", post(read_prices))
        .route(
            "/v1/quotation-approvals/transition",
            post(quotation_approval_transition)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024)),
        )
        .route("/v1/quotations/transition", post(quotation_transition))
        .route("/v1/quotations/read", post(quotation_read))
        .route(
            "/v1/quotation-approvals/read",
            post(quotation_approval_read),
        )
        .route(
            "/v1/pricing/status",
            post(price_status).layer(axum::extract::DefaultBodyLimit::max(64 * 1024)),
        )
        .route("/v1/customer-branches", post(register_customer_branch))
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

/// Authenticates a bounded catalog transfer before writing immutable price dependencies.
async fn synchronize_catalog_revisions(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions>,
) -> Result<Json<()>, ApiError> {
    let actor =
        authenticate_negotiated(&state, &headers, "eitmad.capability.catalog-revisions.v1").await?;
    state
        .sync
        .pricing()
        .synchronize_catalog(
            &actor,
            &input,
            CorrelationId::new(Uuid::new_v4()),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(|e| {
            ApiError::new(
                match e {
                    eitmad_pricing::PricingError::Denied => StatusCode::FORBIDDEN,
                    eitmad_pricing::PricingError::Unconfirmed => StatusCode::SERVICE_UNAVAILABLE,
                    _ => StatusCode::BAD_REQUEST,
                },
                eitmad_pricing::error_code(e),
            )
        })
}

/// Negotiates discount support and authenticates the exact server transition.
async fn quotation_approval_transition(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::quotation_approval::ConfirmDiscountApproval>,
) -> Result<Json<Option<eitmad_contracts::quotation_approval::DiscountApproval>>, ApiError> {
    let actor =
        authenticate_negotiated(&state, &headers, "eitmad.capability.quotation-approval.v1")
            .await?;
    state
        .sync
        .quotation_approvals()
        .transition(&actor, &input, new_correlation_id(), unix_millis_now())
        .await
        .map(Json)
        .map_err(map_approval)
}
async fn quotation_approval_read(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::quotation_approval::ReadDiscountApprovals>,
) -> Result<Json<eitmad_contracts::quotation_approval::DiscountApprovalPage>, ApiError> {
    let actor =
        authenticate_negotiated(&state, &headers, "eitmad.capability.quotation-approval.v1")
            .await?;
    state
        .sync
        .quotation_approvals()
        .list(&actor, &input)
        .await
        .map(Json)
        .map_err(map_approval)
}
fn map_approval(e: eitmad_pricing::ApprovalError) -> ApiError {
    use eitmad_pricing::ApprovalError as E;
    ApiError::new(
        match e {
            E::Denied => StatusCode::FORBIDDEN,
            E::Conflict => StatusCode::CONFLICT,
            E::Invalid => StatusCode::BAD_REQUEST,
            E::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        },
        eitmad_pricing::approval_error_code(e),
    )
}

/// Negotiates pricing support and authenticates the public organization read.
async fn read_prices(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::pricing::ReadPublishedPrices>,
) -> Result<Json<eitmad_contracts::pricing::PublishedPricePage>, ApiError> {
    let actor = authenticate_negotiated(&state, &headers, "eitmad.capability.pricing.v1").await?;
    state
        .sync
        .pricing()
        .read(&actor, &input)
        .await
        .map(Json)
        .map_err(|e| {
            ApiError::new(
                match e {
                    eitmad_pricing::PricingError::Denied => StatusCode::FORBIDDEN,
                    eitmad_pricing::PricingError::Invalid => StatusCode::BAD_REQUEST,
                    _ => StatusCode::SERVICE_UNAVAILABLE,
                },
                eitmad_pricing::error_code(e),
            )
        })
}
async fn publish_price(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::pricing::ConfirmPrice>,
) -> Result<Json<eitmad_contracts::pricing::PublishedPrice>, ApiError> {
    let actor = authenticate_negotiated(&state, &headers, "eitmad.capability.pricing.v1").await?;
    state
        .sync
        .pricing()
        .publish(&actor, &input, new_correlation_id(), unix_millis_now())
        .await
        .map(Json)
        .map_err(|e| {
            let status = match e {
                eitmad_pricing::PricingError::Denied => StatusCode::FORBIDDEN,
                eitmad_pricing::PricingError::Conflict { .. } => StatusCode::CONFLICT,
                eitmad_pricing::PricingError::Unconfirmed => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::BAD_REQUEST,
            };
            ApiError::new(status, eitmad_pricing::error_code(e))
        })
}

async fn price_status(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::pricing::ConfirmPrice>,
) -> Result<Json<Option<eitmad_contracts::pricing::PublishedPrice>>, ApiError> {
    let actor = authenticate_negotiated(&state, &headers, "eitmad.capability.pricing.v1").await?;
    state
        .sync
        .pricing()
        .status(&actor, &input)
        .await
        .map(Json)
        .map_err(|e| {
            let status = match e {
                eitmad_pricing::PricingError::Denied => StatusCode::FORBIDDEN,
                eitmad_pricing::PricingError::Unconfirmed => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::BAD_REQUEST,
            };
            ApiError::new(status, eitmad_pricing::error_code(e))
        })
}

async fn upload_catalog_image(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::catalog_image::UploadCatalogImage>,
) -> Result<Json<eitmad_contracts::catalog_image::CatalogImageRef>, ApiError> {
    let actor =
        authenticate_negotiated(&state, &headers, "eitmad.capability.catalog-image.v1").await?;
    state
        .sync
        .catalog_images()
        .upload(
            &actor,
            input,
            CorrelationId::new(Uuid::new_v4()),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_image)
}
/// Authenticates and negotiates media reads before requesting an authorized bounded chunk.
async fn read_catalog_image(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::catalog_image::DownloadCatalogImage>,
) -> Result<Json<eitmad_contracts::catalog_image::CatalogImageChunk>, ApiError> {
    let actor =
        authenticate_negotiated(&state, &headers, "eitmad.capability.catalog-image.v1").await?;
    state
        .sync
        .catalog_images()
        .download(&actor, &input)
        .await
        .map(Json)
        .map_err(map_image)
}
/// Projects redacted image failures into the HTTP boundary's registered error categories.
fn map_image(error: eitmad_catalog_image::ImageError) -> ApiError {
    match error {
        eitmad_catalog_image::ImageError::Denied => {
            ApiError::forbidden("eitmad.error.authorization-denied.v1")
        }
        eitmad_catalog_image::ImageError::Invalid => {
            ApiError::bad_request("eitmad.error.catalog-image-invalid.v1")
        }
        eitmad_catalog_image::ImageError::NotFound => ApiError::new(
            StatusCode::NOT_FOUND,
            "eitmad.error.catalog-image-not-found.v1",
        ),
        eitmad_catalog_image::ImageError::Unavailable => ApiError::unavailable(),
    }
}

async fn register_customer_branch(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<RegisterBranchRequest>,
) -> Result<Json<RegisteredBranch>, ApiError> {
    let session = authenticate_negotiated(
        &state,
        &headers,
        "eitmad.capability.server-administration.v1",
    )
    .await?;
    state
        .control
        .branches
        .register(&session, &request, new_correlation_id(), unix_millis_now())
        .await
        .map(Json)
        .map_err(|error| match error {
            BranchError::Denied => ApiError::forbidden("eitmad.error.authorization-denied.v1"),
            BranchError::Invalid => ApiError::bad_request("eitmad.error.contract-invalid.v1"),
            BranchError::Unavailable => ApiError::unavailable(),
        })
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
        .max_message_size(1024 * 1024)
        .max_frame_size(1024 * 1024)
        .on_upgrade(move |socket| {
            stream_session(
                socket,
                state,
                StreamContext { token, session },
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
    session: eitmad_contracts::server::AuthenticatedServerSession,
}

enum ApprovalCursor {
    Unsubscribed,
    Subscribed(Option<eitmad_contracts::transport::EventCursor>),
}

async fn stream_session(
    mut socket: WebSocket,
    state: ServerState,
    context: StreamContext,
    scope: ScopeRef,
    schema_id: SchemaId,
    schema_version: u32,
) {
    let StreamContext { token, session } = context;
    let approval_stream = matches!(
        schema_id.as_str(),
        eitmad_pricing::DISCOUNT_APPROVAL_SCHEMA | eitmad_pricing::QUOTATION_LIFECYCLE_SCHEMA
    );
    let mut approval_listener = if approval_stream {
        if let Ok(listener) = state.approval_notifications().await {
            Some(listener)
        } else {
            let _ = send_failure(
                &mut socket,
                "eitmad.error.quotation-approval-unavailable.v1",
            )
            .await;
            return;
        }
    } else {
        None
    };
    let mut approval_cursor = ApprovalCursor::Unsubscribed;
    let mut negotiated: Option<NegotiatedSession> = None;
    let mut revalidation = tokio::time::interval(SESSION_REVALIDATION_INTERVAL);
    revalidation.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    revalidation.reset();
    loop {
        let received = tokio::select! {
            notification=async {approval_listener.as_mut().expect("approval receiver").recv().await}, if matches!(approval_cursor,ApprovalCursor::Subscribed(_)) => {
                if matches!(notification,Err(tokio::sync::broadcast::error::RecvError::Closed)) || revalidate_stream(&state,&token,&session).await.is_err() {break;}
                if let ApprovalCursor::Subscribed(cursor)=&mut approval_cursor {
                    if let Err(e)=send_approval_events(&mut socket,&state,&session,&scope,&schema_id,cursor).await {let _=send_failure(&mut socket,e.code.as_str()).await;break;}
                }
                continue;
            }
            _ = revalidation.tick() => {
                if let Err(error) = revalidate_stream(&state, &token, &session).await {
                    let _ = send_failure(&mut socket, error.code.as_str())
                        .await;
                    break;
                }
                if let ApprovalCursor::Subscribed(cursor)=&mut approval_cursor {
                    if send_approval_events(&mut socket,&state,&session,&scope,&schema_id,cursor).await.is_err(){
                        let _=send_failure(&mut socket,"eitmad.error.authorization-denied.v1").await;break;
                    }
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
        if negotiated.is_none() {
            let Ok(session) =
                accept_stream_hello(&mut socket, &state, message, schema_id.as_str()).await
            else {
                let _ =
                    send_failure(&mut socket, "eitmad.error.server-client-incompatible.v1").await;
                break;
            };
            negotiated = Some(session);
            continue;
        }
        let request_context = StreamRequestContext {
            session: &session,
            scope: &scope,
            schema_id: &schema_id,
            schema_version,
            negotiated: negotiated.as_ref().expect("hello negotiated"),
        };
        let result = handle_stream_message(
            &mut socket,
            &state,
            &request_context,
            message,
            &mut approval_cursor,
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

async fn accept_stream_hello(
    socket: &mut WebSocket,
    state: &ServerState,
    message: ServerClientMessage,
    schema: &str,
) -> Result<NegotiatedSession, ApiError> {
    let incompatible = || ApiError::bad_request("eitmad.error.server-client-incompatible.v1");
    let ServerClientMessage::Hello(hello) = message else {
        return Err(incompatible());
    };
    if hello.api_version != eitmad_contracts::server::SERVER_API_VERSION {
        return Err(incompatible());
    }
    let NegotiationOutcome::Accepted(session) = negotiate(&state.server_hello, &hello.peer) else {
        return Err(incompatible());
    };
    let lifecycle = schema == eitmad_pricing::QUOTATION_LIFECYCLE_SCHEMA;
    let live = lifecycle || schema == eitmad_pricing::DISCOUNT_APPROVAL_SCHEMA;
    if live
        && (session.protocol.minor < if lifecycle { 21 } else { 20 }
            || !session.capabilities.iter().any(|c| {
                c.as_str()
                    == if lifecycle {
                        "eitmad.capability.quotation-lifecycle.v1"
                    } else {
                        "eitmad.capability.quotation-approval.v1"
                    }
            }))
    {
        return Err(incompatible());
    }
    send_server_message(socket, &ServerMessage::Hello(state.server_hello.clone()))
        .await
        .map_err(|()| ApiError::unavailable())?;
    Ok(session)
}

async fn send_approval_events(
    socket: &mut WebSocket,
    state: &ServerState,
    session: &eitmad_contracts::server::AuthenticatedServerSession,
    scope: &ScopeRef,
    schema: &SchemaId,
    cursor: &mut Option<eitmad_contracts::transport::EventCursor>,
) -> Result<(), ApiError> {
    if schema.as_str() == eitmad_pricing::QUOTATION_LIFECYCLE_SCHEMA {
        state
            .sync
            .quotations()
            .expire_due(session, scope, unix_millis_now())
            .await
            .map_err(map_quotation)?;
    }
    loop {
        let page = state
            .sync
            .subscription_page(eitmad_sync_plane::SubscriptionPageRequest {
                session,
                scope,
                schema_id: schema,
                schema_version: 1,
                resume_after: *cursor,
                maximum_events: 100,
                correlation_id: new_correlation_id(),
                now: unix_millis_now(),
            })
            .await
            .map_err(map_subscription)?;
        for event in page.events {
            let next = event.cursor;
            send_server_message(socket, &ServerMessage::Event(event))
                .await
                .map_err(|()| ApiError::unavailable())?;
            *cursor = Some(next);
        }
        if !page.has_more {
            return Ok(());
        }
    }
}

struct StreamRequestContext<'a> {
    session: &'a eitmad_contracts::server::AuthenticatedServerSession,
    scope: &'a ScopeRef,
    schema_id: &'a SchemaId,
    schema_version: u32,
    negotiated: &'a NegotiatedSession,
}

async fn handle_stream_message(
    socket: &mut WebSocket,
    state: &ServerState,
    context: &StreamRequestContext<'_>,
    message: ServerClientMessage,
    approval_cursor: &mut ApprovalCursor,
) -> Result<(), ApiError> {
    match message {
        ServerClientMessage::Subscribe(request)
            if request.schema_id == *context.schema_id
                && matches!(
                    context.schema_id.as_str(),
                    eitmad_pricing::DISCOUNT_APPROVAL_SCHEMA
                        | eitmad_pricing::QUOTATION_LIFECYCLE_SCHEMA
                ) =>
        {
            let mut cursor = request.resume_after;
            send_approval_events(
                socket,
                state,
                context.session,
                context.scope,
                context.schema_id,
                &mut cursor,
            )
            .await?;
            *approval_cursor = ApprovalCursor::Subscribed(cursor);
            Ok(())
        }
        ServerClientMessage::Subscribe(request) if request.schema_id == *context.schema_id => {
            let page = state
                .sync
                .subscription_page(eitmad_sync_plane::SubscriptionPageRequest {
                    session: context.session,
                    scope: context.scope,
                    schema_id: &request.schema_id,
                    schema_version: context.schema_version,
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
        ServerClientMessage::Acknowledge(_) => Err(ApiError::bad_request(
            "eitmad.error.server-subscription-ack-unsupported.v1",
        )),
        ServerClientMessage::Sync(frame)
            if frame.protocol_version == context.negotiated.protocol =>
        {
            handle_sync_frame(socket, state, context, frame).await
        }
        ServerClientMessage::Hello(_)
        | ServerClientMessage::Subscribe(_)
        | ServerClientMessage::Sync(_) => Err(ApiError::bad_request(
            "eitmad.error.server-client-incompatible.v1",
        )),
    }
}

async fn handle_sync_frame(
    socket: &mut WebSocket,
    state: &ServerState,
    context: &StreamRequestContext<'_>,
    frame: eitmad_contracts::sync_transport::SyncTransportFrame,
) -> Result<(), ApiError> {
    match frame.payload {
        SyncTransportPayload::Message(message) => match *message {
            SyncMessage::SubmitLocal(submission) => {
                let change = submission.change;
                let submitted_change_id = change.change_id;
                let disposition =
                    local_disposition(state, context, change, frame.correlation_id).await?;
                send_server_message(
                    socket,
                    &ServerMessage::Sync(SyncMessage::LocalResult(LocalChangeResult {
                        submitted_change_id,
                        disposition,
                    })),
                )
                .await
                .map_err(|()| ApiError::unavailable())
            }
            SyncMessage::Pull(request) => match state
                .sync
                .pull(eitmad_sync_plane::PullPageRequest {
                    session: context.session,
                    scope: context.scope,
                    schema_id: context.schema_id,
                    schema_version: context.schema_version,
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
                        context.session,
                        context.scope,
                        context.schema_id,
                        context.schema_version,
                        frame.correlation_id,
                    )
                    .await
                }
                Err(error) => Err(map_operation(error)),
            },
            SyncMessage::Acknowledge(acknowledgement) => {
                state
                    .sync
                    .acknowledge(eitmad_sync_plane::AcknowledgeRequest {
                        session: context.session,
                        scope: context.scope,
                        schema_id: context.schema_id,
                        schema_version: context.schema_version,
                        acknowledgement: &acknowledgement,
                        correlation_id: frame.correlation_id,
                        now: unix_millis_now(),
                    })
                    .await
                    .map_err(map_operation)?;
                send_server_message(
                    socket,
                    &ServerMessage::Sync(SyncMessage::Acknowledge(acknowledgement)),
                )
                .await
                .map_err(|()| ApiError::unavailable())
            }
            _ => Err(ApiError::bad_request("eitmad.error.contract-invalid.v1")),
        },
        SyncTransportPayload::Cancel(cancellation) if cancellation.stream_id == frame.stream_id => {
            Ok(())
        }
        _ => Err(ApiError::bad_request("eitmad.error.contract-invalid.v1")),
    }
}

async fn local_disposition(
    state: &ServerState,
    context: &StreamRequestContext<'_>,
    change: ChangeRecord,
    correlation_id: CorrelationId,
) -> Result<LocalChangeDisposition, ApiError> {
    if change.scope != *context.scope
        || change.revision == 0
        || change.base_revision.unwrap_or(0).checked_add(1) != Some(change.revision)
        || change.payload.as_ref().is_none_or(|payload| {
            payload.schema_id != *context.schema_id
                || payload.schema_version != context.schema_version
        })
    {
        return Ok(rejected_local_change("eitmad.error.contract-invalid.v1"));
    }
    let outcome = state
        .sync
        .apply_local_operation(
            context.session,
            &LocalOperationDraft {
                change_id: change.change_id,
                scope: change.scope,
                schema_id: context.schema_id.clone(),
                schema_version: context.schema_version,
                record_id: change.record_id,
                operation: change.operation,
                base_revision: change.base_revision,
                idempotency_key: change.idempotency_key,
                payload: change.payload,
            },
            correlation_id,
            unix_millis_now(),
        )
        .await;
    match outcome {
        Ok(OperationResult::Applied { change }) => Ok(LocalChangeDisposition::Applied {
            authoritative_change: *change,
        }),
        Ok(OperationResult::Replayed { change }) => Ok(LocalChangeDisposition::Replayed {
            authoritative_change: *change,
        }),
        Ok(OperationResult::ConflictRecorded { conflict_id }) => {
            Ok(LocalChangeDisposition::Conflicted { conflict_id })
        }
        Err(error) => local_submission_error(error),
    }
}

fn rejected_local_change(reason: &'static str) -> LocalChangeDisposition {
    LocalChangeDisposition::Rejected {
        reason: eitmad_contracts::sync::ErrorCodeRef::parse(reason).expect("static sync error"),
    }
}

fn local_submission_error(error: OperationError) -> Result<LocalChangeDisposition, ApiError> {
    let code = match error {
        OperationError::Denied => "eitmad.error.authorization-denied.v1",
        OperationError::Invalid | OperationError::WrongMode => "eitmad.error.contract-invalid.v1",
        OperationError::IdempotencyMismatch => "eitmad.error.server-idempotency-mismatch.v1",
        OperationError::UnknownRecord | OperationError::SnapshotRequired => {
            "eitmad.error.server-snapshot-required.v1"
        }
        OperationError::UnknownDomain => "eitmad.error.server-client-incompatible.v1",
        OperationError::Unavailable => return Err(ApiError::unavailable()),
    };
    Ok(rejected_local_change(code))
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

/// Requires the operation's protocol version and capability before token and device authentication.
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
    let minimum_minor =
        if required_capability.as_str() == "eitmad.capability.quotation-lifecycle.v1" {
            21
        } else if required_capability.as_str() == "eitmad.capability.quotation-approval.v1" {
            20
        } else if matches!(
            required_capability.as_str(),
            "eitmad.capability.pricing.v1" | "eitmad.capability.catalog-revisions.v1"
        ) {
            17
        } else {
            5
        };
    let mut boundary = state.server_hello.clone();
    boundary.required_capabilities = vec![required_capability];
    let NegotiationOutcome::Accepted(negotiated) = negotiate(&boundary, &peer) else {
        return Err(ApiError::bad_request(
            "eitmad.error.server-client-incompatible.v1",
        ));
    };
    if negotiated.protocol.major != 1 || negotiated.protocol.minor < minimum_minor {
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

async fn revalidate_stream(
    state: &ServerState,
    token: &str,
    session: &eitmad_contracts::server::AuthenticatedServerSession,
) -> Result<(), ApiError> {
    state
        .control
        .authentication
        .revalidate_access(token, session, unix_millis_now())
        .await
        .map(|_| ())
        .map_err(ApiError::authentication)
}

/// Advertises supported schemas and capabilities while requiring only shared transport foundations.
fn server_hello(schemas: Vec<SchemaSupport>) -> PeerHello {
    let capabilities = [
        "eitmad.capability.quotation-lifecycle.v1",
        "eitmad.capability.quotation-approval.v1",
        "eitmad.capability.sync.v1",
        "eitmad.capability.catalog-image.v1",
        "eitmad.capability.pricing.v1",
        "eitmad.capability.catalog-revisions.v1",
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
            maximum_minor: eitmad_contracts::PROTOCOL_VERSION.minor,
        }],
        required_capabilities: capabilities
            .iter()
            .filter(|c| {
                !matches!(
                    c.as_str(),
                    "eitmad.capability.quotation-lifecycle.v1"
                        | "eitmad.capability.quotation-approval.v1"
                        | "eitmad.capability.catalog-image.v1"
                        | "eitmad.capability.pricing.v1"
                        | "eitmad.capability.catalog-revisions.v1"
                )
            })
            .cloned()
            .collect(),
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

async fn quotation_transition(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::quotation_lifecycle::ConfirmQuotation>,
) -> Result<Json<eitmad_contracts::quotation_lifecycle::QuotationRecord>, ApiError> {
    let actor =
        authenticate_negotiated(&state, &headers, "eitmad.capability.quotation-lifecycle.v1")
            .await?;
    state
        .sync
        .quotations()
        .transition(&actor, &input, new_correlation_id(), unix_millis_now())
        .await
        .map(Json)
        .map_err(map_quotation)
}
async fn quotation_read(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<eitmad_contracts::quotation_lifecycle::ReadQuotations>,
) -> Result<Json<eitmad_contracts::quotation_lifecycle::QuotationPage>, ApiError> {
    let actor =
        authenticate_negotiated(&state, &headers, "eitmad.capability.quotation-lifecycle.v1")
            .await?;
    state
        .sync
        .quotations()
        .list(&actor, &input, unix_millis_now())
        .await
        .map(Json)
        .map_err(map_quotation)
}
fn map_quotation(e: eitmad_pricing::QuotationError) -> ApiError {
    use eitmad_pricing::QuotationError as E;
    let status = match e {
        E::Denied => StatusCode::FORBIDDEN,
        E::Invalid => StatusCode::BAD_REQUEST,
        E::Conflict | E::StalePrice | E::ApprovalRequired => StatusCode::CONFLICT,
        E::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    ApiError::new(status, eitmad_pricing::quotation_error_code(e))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks the declared protocol range and mandatory device-proof transport capability.
    #[test]
    fn server_requires_all_remote_boundary_capabilities() {
        let hello = server_hello(Vec::new());
        assert_eq!(hello.protocols[0].minimum_minor, 4);
        assert_eq!(
            hello.protocols[0].maximum_minor,
            eitmad_contracts::PROTOCOL_VERSION.minor
        );
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
    /// Checks that pricing and catalog routes reject missing authentication or capability negotiation.
    #[tokio::test]
    async fn pricing_http_denies_unauthenticated_and_incompatible_requests_before_storage() {
        use eitmad_contracts::{
            pricing::{ConfirmPrice, PriceTarget, PublishPrice, ReadPublishedPrices},
            product::{ProductId, ProductReference, ProductVariantId},
            transport::IdempotencyKey,
        };
        use tower::ServiceExt as _;
        let scope = ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(Uuid::from_u128(50)),
        };
        let publication = ConfirmPrice {
            command: PublishPrice {
                target: PriceTarget::Product(ProductReference {
                    scope: scope.clone(),
                    product_id: ProductId::new(Uuid::from_u128(51)),
                    variant_id: ProductVariantId::new(Uuid::from_u128(52)),
                    revision: 1,
                    schema_version: 1,
                }),
                expected_revision: None,
                selling_price_yer: 12500,
                confirm_below_cost: false,
            },
            cost_yer: 10000,
            colors: vec![],
            handles: vec![],
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(53)),
        };
        let body = serde_json::to_vec(&publication).unwrap();
        let catalog = serde_json::to_vec(
            &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions {
                scope: scope.clone(),
                records: vec![],
            },
        )
        .unwrap();
        let read = serde_json::to_vec(&ReadPublishedPrices {
            scope,
            after: None,
            limit: 100,
        })
        .unwrap();
        for (compatible, minor, status) in [
            (true, 17, StatusCode::UNAUTHORIZED),
            (true, 16, StatusCode::BAD_REQUEST),
            (false, 17, StatusCode::BAD_REQUEST),
        ] {
            let mut peer = server_hello(Vec::new());
            peer.peer_kind = PeerKind::Engine;
            peer.protocols[0].minimum_minor = minor;
            peer.protocols[0].maximum_minor = minor;
            if !compatible {
                peer.capabilities.retain(|c| {
                    !matches!(
                        c.as_str(),
                        "eitmad.capability.pricing.v1" | "eitmad.capability.catalog-revisions.v1"
                    )
                });
            }
            let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&peer).unwrap());
            for (uri, payload) in [
                ("/v1/pricing/publish", &body),
                ("/v1/pricing/status", &body),
                ("/v1/pricing/read", &read),
                ("/v1/catalog-revisions/synchronize", &catalog),
            ] {
                let response = router(test_state())
                    .oneshot(
                        axum::http::Request::builder()
                            .method("POST")
                            .uri(uri)
                            .header("x-eitmad-peer-hello", &encoded)
                            .header("Content-Type", "application/json")
                            .body(axum::body::Body::from(payload.clone()))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), status, "{uri}");
                if status == StatusCode::BAD_REQUEST {
                    let bytes = axum::body::to_bytes(response.into_body(), 4096)
                        .await
                        .unwrap();
                    let failure: ServerFailure = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(
                        failure.code.as_str(),
                        "eitmad.error.server-client-incompatible.v1"
                    );
                }
            }
        }
    }
}
