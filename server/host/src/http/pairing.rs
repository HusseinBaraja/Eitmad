use super::{
    ApiError, HeaderMap, IntoResponse, Json, ServerConfig, ServerState, State,
    authenticate_headers, new_correlation_id, unix_millis_now,
};
use eitmad_contracts::pairing::{
    CompletePairing, OpenPairing, PairingChallenge, PairingIdentity, PairingOffer, PairingReceipt,
    StartPairing,
};
use rustls::pki_types::pem::PemObject as _;
use std::io::Read as _;

pub(super) async fn open(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<OpenPairing>,
) -> Result<impl IntoResponse, ApiError> {
    let actor = authenticate_headers(&state, &headers).await?;
    state
        .control
        .pairing
        .open(
            &actor,
            identity(&state)?,
            &request,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(|value| ([("Cache-Control", "no-store")], Json(value)))
        .map_err(map_error)
}
pub(super) async fn offer(
    State(state): State<ServerState>,
) -> Result<Json<PairingOffer>, ApiError> {
    state
        .control
        .pairing
        .offer(identity(&state)?, unix_millis_now())
        .await
        .map(Json)
        .map_err(map_error)
}
pub(super) async fn start(
    State(state): State<ServerState>,
    Json(request): Json<StartPairing>,
) -> Result<Json<PairingChallenge>, ApiError> {
    state
        .control
        .pairing
        .start(
            identity(&state)?,
            &request,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_error)
}
pub(super) async fn complete(
    State(state): State<ServerState>,
    Json(request): Json<CompletePairing>,
) -> Result<Json<PairingReceipt>, ApiError> {
    state
        .control
        .pairing
        .complete(
            identity(&state)?,
            &request,
            new_correlation_id(),
            unix_millis_now(),
        )
        .await
        .map(Json)
        .map_err(map_error)
}
fn identity(state: &ServerState) -> Result<&PairingIdentity, ApiError> {
    state
        .pairing_identity
        .as_ref()
        .ok_or_else(ApiError::unavailable)
}
fn map_error(error: eitmad_control_plane::PairingError) -> ApiError {
    match error {
        eitmad_control_plane::PairingError::Denied => {
            ApiError::forbidden("eitmad.error.pairing-denied.v1")
        }
        eitmad_control_plane::PairingError::Unavailable => ApiError::unavailable(),
    }
}

pub(super) fn configured_identity(
    config: &ServerConfig,
) -> Result<Option<PairingIdentity>, Box<dyn std::error::Error>> {
    let endpoint = std::env::var("EITMAD_SERVER_PAIRING_ENDPOINT").ok();
    let ca = std::env::var_os("EITMAD_SERVER_PUBLIC_CA");
    let (endpoint, ca) = match (endpoint, ca) {
        (None, None) => return Ok(None),
        (Some(endpoint), Some(ca)) if config.tls_certificate.is_some() => (endpoint, ca),
        _ => return Err("pairing TLS configuration is incomplete".into()),
    };
    let url = url::Url::parse(&endpoint)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("pairing endpoint is invalid".into());
    }
    let mut public_ca_pem = String::new();
    std::fs::File::open(ca)?
        .take(16_385)
        .read_to_string(&mut public_ca_pem)?;
    if public_ca_pem.len() > 16_384 || public_ca_pem.contains("PRIVATE KEY") {
        return Err("pairing CA is invalid".into());
    }
    let certificates = rustls::pki_types::CertificateDer::pem_slice_iter(public_ca_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()?;
    if certificates.is_empty() {
        return Err("pairing CA is empty".into());
    }
    let server_id = eitmad_server_discovery::load_or_create_identity(
        &config.update_manifest_directory.join("server-identity"),
    )?;
    Ok(Some(PairingIdentity {
        server_id,
        endpoint: url.to_string(),
        public_ca_pem,
    }))
}
