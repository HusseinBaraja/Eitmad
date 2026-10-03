//! Bounded authenticated HTTP transfer for catalog images.
use super::{
    DirectServerDriver, URL_SAFE_NO_PAD, connect_tls, device_proof, host_header,
    parse_http_response,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_catalog_image::{CatalogImageTransfer, ImageError};
use eitmad_contracts::{
    catalog_image::{
        CatalogImageChunk, CatalogImageRef, DownloadCatalogImage, GetCatalogImage,
        IMAGE_CHUNK_BYTES, MAX_IMAGE_BYTES, UploadCatalogImage,
    },
    identity::AuthorizationContext,
    secrets::SecretId,
};
use std::{
    io::{Read as _, Write as _},
    sync::Mutex,
};

pub struct DirectCatalogImageClient {
    driver: Mutex<DirectServerDriver>,
    credential_id: SecretId,
}
impl DirectCatalogImageClient {
    #[must_use]
    pub fn new(driver: DirectServerDriver, credential_id: SecretId) -> Self {
        Self {
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
                minimum_minor: 14,
                maximum_minor: 14,
            }],
            capabilities: eitmad_contracts::catalog::CAPABILITIES
                .iter()
                .map(|id| CapabilityId::parse(*id).expect("registered capability"))
                .collect(),
            required_capabilities: vec![
                CapabilityId::parse("eitmad.capability.catalog-image.v1")
                    .expect("registered capability"),
            ],
            schemas: vec![],
        };
        Self::new(
            DirectServerDriver::new(config, secrets, hello),
            credential_id,
        )
    }

    fn remote_scope(&self) -> Result<eitmad_contracts::identity::ScopeRef, ImageError> {
        Ok(self
            .driver
            .lock()
            .map_err(|_| ImageError::Unavailable)?
            .config
            .scope
            .clone())
    }

    fn request<T: serde::de::DeserializeOwned>(
        &self,
        actor: &AuthorizationContext,
        route: &str,
        input: &impl serde::Serialize,
    ) -> Result<T, ImageError> {
        let driver = self.driver.lock().map_err(|_| ImageError::Unavailable)?;
        if actor.scope.kind.as_str() != "organization"
            || actor.scope.id.value() != actor.tenant_id.value()
        {
            return Err(ImageError::Denied);
        }
        let mut credential = driver
            .load_credential(&self.credential_id)
            .map_err(|_| ImageError::Denied)?;
        if actor.identity.principal_id.value() != credential.user_id.value()
            || actor.tenant_id != credential.tenant_id
        {
            return Err(ImageError::Denied);
        }
        driver
            .refresh_if_due(&self.credential_id, &mut credential)
            .map_err(|_| ImageError::Unavailable)?;
        let mut url = driver.config.endpoint.clone();
        url.set_path(route);
        let mut stream =
            connect_tls(&url, &driver.config.tls).map_err(|_| ImageError::Unavailable)?;
        let host = host_header(&url).map_err(|_| ImageError::Unavailable)?;
        let proof = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&device_proof(&credential)).map_err(|_| ImageError::Invalid)?,
        );
        let peer = URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&driver.local_hello).map_err(|_| ImageError::Invalid)?);
        let body = serde_json::to_vec(input).map_err(|_| ImageError::Invalid)?;
        if body.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 + 4096 {
            return Err(ImageError::Invalid);
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
            .map_err(|_| ImageError::Unavailable)?;
        let maximum = IMAGE_CHUNK_BYTES * 2 + 8192;
        let mut response = Vec::new();
        stream
            .take((maximum + 1) as u64)
            .read_to_end(&mut response)
            .map_err(|_| ImageError::Unavailable)?;
        if response.len() > maximum {
            return Err(ImageError::Invalid);
        }
        let (status, bytes) = parse_http_response(&response).map_err(|_| ImageError::Invalid)?;
        match status {
            200 => serde_json::from_slice(&bytes).map_err(|_| ImageError::Invalid),
            401 | 403 => Err(ImageError::Denied),
            400 => Err(ImageError::Invalid),
            404 => Err(ImageError::NotFound),
            _ => Err(ImageError::Unavailable),
        }
    }
}
impl CatalogImageTransfer for DirectCatalogImageClient {
    fn upload(
        &self,
        actor: &AuthorizationContext,
        image: &CatalogImageRef,
        content: &[u8],
    ) -> Result<(), ImageError> {
        eitmad_catalog_image::validate_asset(image, content)?;
        let received: CatalogImageRef = self.request(
            actor,
            "/v1/catalog-images/upload",
            &UploadCatalogImage {
                scope: self.remote_scope()?,
                reference: image.clone(),
                base64: STANDARD.encode(content),
            },
        )?;
        if received != *image {
            return Err(ImageError::Invalid);
        }
        Ok(())
    }
    fn download(
        &self,
        actor: &AuthorizationContext,
        image: &CatalogImageRef,
    ) -> Result<Vec<u8>, ImageError> {
        let mut content = Vec::new();
        let mut expected = None;
        loop {
            let query = GetCatalogImage {
                reference: image.clone(),
                offset: u32::try_from(content.len()).map_err(|_| ImageError::Invalid)?,
            };
            let value: CatalogImageChunk = self.request(
                actor,
                "/v1/catalog-images/read",
                &DownloadCatalogImage {
                    scope: self.remote_scope()?,
                    image: query.clone(),
                },
            )?;
            let bytes = STANDARD
                .decode(&value.base64)
                .map_err(|_| ImageError::Invalid)?;
            if value.reference != query.reference
                || value.offset != query.offset
                || value.total_bytes as usize > MAX_IMAGE_BYTES
                || value.offset >= value.total_bytes
                || bytes.len() != IMAGE_CHUNK_BYTES.min((value.total_bytes - value.offset) as usize)
                || expected.is_some_and(|size| size != value.total_bytes)
            {
                return Err(ImageError::Invalid);
            }
            expected = Some(value.total_bytes);
            content.extend_from_slice(&bytes);
            if content.len() == value.total_bytes as usize {
                break;
            }
        }
        eitmad_catalog_image::validate_asset(image, &content)?;
        Ok(content)
    }
}
