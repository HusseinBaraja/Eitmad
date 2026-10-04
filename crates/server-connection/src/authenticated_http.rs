//! Shared authenticated bounded HTTP mechanics for images and pricing.
use super::{
    DirectServerDriver, URL_SAFE_NO_PAD, connect_tls, device_proof, host_header,
    parse_http_response, remaining_io,
};
use base64::Engine as _;
use eitmad_contracts::{
    catalog_image::{IMAGE_CHUNK_BYTES, MAX_IMAGE_BYTES},
    identity::{AuthorizationContext, ScopeRef},
    secrets::SecretId,
};
use std::{
    io::{Read as _, Write as _},
    sync::Mutex,
    time::Instant,
};
#[derive(Clone, Copy, Debug)]
pub(crate) enum HttpError {
    Denied,
    Invalid,
    NotFound,
    Conflict,
    Unavailable,
}
pub(crate) struct AuthenticatedHttpClient {
    pub(super) driver: Mutex<DirectServerDriver>,
    credential_id: SecretId,
    remote_scope: ScopeRef,
}
impl AuthenticatedHttpClient {
    /// Binds image requests to a configured driver and native-store credential identifier.
    #[must_use]
    pub fn new(driver: DirectServerDriver, credential_id: SecretId) -> Self {
        Self {
            remote_scope: driver.config.scope.clone(),
            driver: Mutex::new(driver),
            credential_id,
        }
    }

    /// Opens a media route using native secret storage and the declared server trust anchor.
    /// # Panics
    /// Panics only if a built-in protocol identifier is invalid.
    #[must_use]
    pub fn from_config(
        config: super::DirectServerConfig,
        secrets: eitmad_secret_storage::SecretStore,
        credential_id: SecretId,
        capability: &str,
        minimum_minor: u16,
    ) -> Self {
        use eitmad_contracts::{
            transport::CapabilityId,
            updates::ReleaseVersion,
            versioning::{PeerHello, PeerKind, SupportedProtocol},
        };
        let hello = PeerHello {
            peer_kind: PeerKind::Engine,
            product_version: ReleaseVersion::new(semver::Version::new(0, 0, 0)),
            protocols: vec![SupportedProtocol {
                major: 1,
                minimum_minor,
                maximum_minor: minimum_minor,
            }],
            capabilities: eitmad_contracts::catalog::CAPABILITIES
                .iter()
                .map(|id| CapabilityId::parse(*id).expect("registered capability"))
                .collect(),
            required_capabilities: vec![
                CapabilityId::parse(capability).expect("registered capability"),
            ],
            schemas: vec![],
        };
        Self::new(
            DirectServerDriver::new(config, secrets, hello),
            credential_id,
        )
    }

    /// Resolves the registered server organization without accepting a shell-supplied scope.
    pub(crate) fn remote_scope(&self) -> ScopeRef {
        self.remote_scope.clone()
    }

    /// Binds the local actor to stored identity and bounds one authenticated media response.
    pub(crate) fn request<T: serde::de::DeserializeOwned>(
        &self,
        actor: &AuthorizationContext,
        route: &str,
        input: &impl serde::Serialize,
        budget: Instant,
    ) -> Result<T, HttpError> {
        remaining_io(budget).map_err(|_| HttpError::Unavailable)?;
        // Do not spend another image worker waiting behind a network operation.
        let driver = self.driver.try_lock().map_err(|_| HttpError::Unavailable)?;
        remaining_io(budget).map_err(|_| HttpError::Unavailable)?;
        if actor.scope.kind.as_str() != "organization"
            || actor.scope.id.value() != actor.tenant_id.value()
        {
            return Err(HttpError::Denied);
        }
        let mut credential = driver
            .load_credential(&self.credential_id)
            .map_err(|_| HttpError::Denied)?;
        driver
            .refresh_if_due(&self.credential_id, &mut credential, budget)
            .map_err(|_| HttpError::Unavailable)?;
        if Some(actor.identity.principal_id.value())
            != credential
                .user_id
                .map(eitmad_contracts::identity::UserId::value)
            || Some(actor.tenant_id) != credential.tenant_id
        {
            return Err(HttpError::Denied);
        }
        remaining_io(budget).map_err(|_| HttpError::Unavailable)?;
        let mut url = driver.config.endpoint.clone();
        url.set_path(route);
        let mut stream =
            connect_tls(&url, &driver.config.tls, budget).map_err(|_| HttpError::Unavailable)?;
        let host = host_header(&url).map_err(|_| HttpError::Unavailable)?;
        let proof = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&device_proof(&credential)).map_err(|_| HttpError::Invalid)?,
        );
        let peer = URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&driver.local_hello).map_err(|_| HttpError::Invalid)?);
        let body = serde_json::to_vec(input).map_err(|_| HttpError::Invalid)?;
        if body.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 + 4096 {
            return Err(HttpError::Invalid);
        }
        let header = format!(
            "POST {route} HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\nx-eitmad-device-proof: {proof}\r\nx-eitmad-peer-hello: {peer}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            credential.access_token,
            body.len()
        );
        stream
            .write_all(header.as_bytes())
            .and_then(|()| stream.write_all(&body))
            .and_then(|()| stream.flush())
            .map_err(|_| HttpError::Unavailable)?;
        let maximum = if route == "/v1/pricing/read" {
            4 * 1024 * 1024
        } else {
            IMAGE_CHUNK_BYTES * 2 + 8192
        };
        let mut response = Vec::new();
        let mut buffer = [0; 8192];
        while response.len() <= maximum {
            remaining_io(budget).map_err(|_| HttpError::Unavailable)?;
            let limit = buffer.len().min(maximum + 1 - response.len());
            let count = stream
                .read(&mut buffer[..limit])
                .map_err(|_| HttpError::Unavailable)?;
            if count == 0 {
                break;
            }
            response.extend_from_slice(&buffer[..count]);
        }
        remaining_io(budget).map_err(|_| HttpError::Unavailable)?;
        if response.len() > maximum {
            return Err(HttpError::Invalid);
        }
        let (status, bytes) = parse_http_response(&response).map_err(|_| HttpError::Invalid)?;
        match status {
            200 => serde_json::from_slice(&bytes).map_err(|_| HttpError::Invalid),
            401 | 403 => Err(HttpError::Denied),
            400 => Err(HttpError::Invalid),
            404 => Err(HttpError::NotFound),
            409 => Err(HttpError::Conflict),
            _ => Err(HttpError::Unavailable),
        }
    }
}
