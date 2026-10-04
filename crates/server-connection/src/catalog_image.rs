//! Bounded authenticated HTTP transfer for catalog images.
use super::{
    DirectServerDriver, URL_SAFE_NO_PAD, connect_tls, device_proof, host_header,
    parse_http_response, remaining_io,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_catalog_image::{CatalogImageTransfer, ImageError};
use eitmad_contracts::{
    catalog_image::{
        CatalogImageChunk, CatalogImageRef, DownloadCatalogImage, GetCatalogImage,
        IMAGE_CHUNK_BYTES, MAX_IMAGE_BYTES, UploadCatalogImage,
    },
    identity::{AuthorizationContext, ScopeRef},
    secrets::SecretId,
    transport::UnixMillis,
};
use std::{
    io::{Read as _, Write as _},
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct DirectCatalogImageClient {
    driver: Mutex<DirectServerDriver>,
    credential_id: SecretId,
    remote_scope: ScopeRef,
}
impl DirectCatalogImageClient {
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

    /// Resolves the registered server organization without accepting a shell-supplied scope.
    fn remote_scope(&self) -> ScopeRef {
        self.remote_scope.clone()
    }

    /// Binds the local actor to stored identity and bounds one authenticated media response.
    fn request<T: serde::de::DeserializeOwned>(
        &self,
        actor: &AuthorizationContext,
        route: &str,
        input: &impl serde::Serialize,
        budget: Instant,
    ) -> Result<T, ImageError> {
        remaining_io(budget).map_err(|_| ImageError::Unavailable)?;
        // Do not spend another image worker waiting behind a network operation.
        let driver = self
            .driver
            .try_lock()
            .map_err(|_| ImageError::Unavailable)?;
        remaining_io(budget).map_err(|_| ImageError::Unavailable)?;
        if actor.scope.kind.as_str() != "organization"
            || actor.scope.id.value() != actor.tenant_id.value()
        {
            return Err(ImageError::Denied);
        }
        let mut credential = driver
            .load_credential(&self.credential_id)
            .map_err(|_| ImageError::Denied)?;
        driver
            .refresh_if_due(&self.credential_id, &mut credential, budget)
            .map_err(|_| ImageError::Unavailable)?;
        if Some(actor.identity.principal_id.value())
            != credential
                .user_id
                .map(eitmad_contracts::identity::UserId::value)
            || Some(actor.tenant_id) != credential.tenant_id
        {
            return Err(ImageError::Denied);
        }
        remaining_io(budget).map_err(|_| ImageError::Unavailable)?;
        let mut url = driver.config.endpoint.clone();
        url.set_path(route);
        let mut stream =
            connect_tls(&url, &driver.config.tls, budget).map_err(|_| ImageError::Unavailable)?;
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
        let mut buffer = [0; 8192];
        while response.len() <= maximum {
            remaining_io(budget).map_err(|_| ImageError::Unavailable)?;
            let limit = buffer.len().min(maximum + 1 - response.len());
            let count = stream
                .read(&mut buffer[..limit])
                .map_err(|_| ImageError::Unavailable)?;
            if count == 0 {
                break;
            }
            response.extend_from_slice(&buffer[..count]);
        }
        remaining_io(budget).map_err(|_| ImageError::Unavailable)?;
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
    /// Validates outgoing content and accepts only an exact immutable server acknowledgement.
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
                scope: self.remote_scope(),
                reference: image.clone(),
                base64: STANDARD.encode(content),
            },
            Instant::now() + Duration::from_secs(30),
        )?;
        if received != *image {
            return Err(ImageError::Invalid);
        }
        Ok(())
    }
    /// Checks the deadline and each chunk before validating the assembled immutable asset.
    fn download(
        &self,
        actor: &AuthorizationContext,
        image: &CatalogImageRef,
        deadline: UnixMillis,
    ) -> Result<Vec<u8>, ImageError> {
        eitmad_catalog_image::check_deadline(deadline)?;
        let remaining = u64::try_from(deadline.0.saturating_sub(super::unix_millis_now().0))
            .map_err(|_| ImageError::Unavailable)?;
        let budget = Instant::now()
            .checked_add(Duration::from_millis(remaining))
            .ok_or(ImageError::Unavailable)?;
        let mut content = Vec::new();
        let mut expected = None;
        loop {
            eitmad_catalog_image::check_deadline(deadline)?;
            let query = GetCatalogImage {
                reference: image.clone(),
                offset: u32::try_from(content.len()).map_err(|_| ImageError::Invalid)?,
            };
            let value: CatalogImageChunk = self.request(
                actor,
                "/v1/catalog-images/read",
                &DownloadCatalogImage {
                    scope: self.remote_scope(),
                    image: query.clone(),
                },
                budget,
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
        eitmad_catalog_image::check_deadline(deadline)?;
        eitmad_catalog_image::validate_asset(image, &content)?;
        eitmad_catalog_image::check_deadline(deadline)?;
        remaining_io(budget).map_err(|_| ImageError::Unavailable)?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::{
        catalog_image::CatalogImageKind,
        config::SecretReferenceId,
        identity::{
            AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId,
            TenantId,
        },
        secrets::SecretKind,
        transport::SchemaId,
    };
    use eitmad_secret_storage::{FallbackEncryptionKey, SecretMaterial, SecretStore};
    use std::{
        net::TcpListener,
        sync::{Arc, mpsc},
    };
    use uuid::Uuid;

    fn credential(refresh: bool, user: Uuid, tenant: TenantId) -> super::super::StoredCredential {
        super::super::StoredCredential {
            user_id: (!refresh).then_some(eitmad_contracts::identity::UserId::new(user)),
            tenant_id: (!refresh).then_some(tenant),
            account_id: eitmad_contracts::identity::AccountId::new(Uuid::new_v4()),
            device_id: eitmad_contracts::identity::DeviceId::new(Uuid::new_v4()),
            access_token: "synthetic-access".into(),
            refresh_token: "synthetic-refresh".into(),
            access_expires_at: UnixMillis(super::super::unix_millis_now().0 + 120_000),
            refresh_expires_at: UnixMillis(super::super::unix_millis_now().0 + 240_000),
            signing_seed: [7; 32],
        }
    }

    fn test_client(
        endpoint: url::Url,
        scope: ScopeRef,
        store: SecretStore,
        id: SecretId,
    ) -> Arc<DirectCatalogImageClient> {
        let config = super::super::DirectServerConfig {
            endpoint,
            scope,
            schema_id: SchemaId::parse("eitmad.schema.catalog-image.v1").unwrap(),
            schema_version: 1,
            tls: Arc::new(
                rustls::ClientConfig::builder()
                    .with_root_certificates(rustls::RootCertStore::empty())
                    .with_no_client_auth(),
            ),
        };
        Arc::new(DirectCatalogImageClient::from_config(config, store, id))
    }

    #[test]
    fn stalled_media_and_credential_refresh_release_transfer_admission() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        for refresh in [false, true] {
            let directory = tempfile::TempDir::new().unwrap();
            let store =
                SecretStore::open(directory.path(), Some(FallbackEncryptionKey::new([7; 32])))
                    .unwrap();
            let id = SecretId::new(
                SecretKind::parse("image-deadline-test").unwrap(),
                SecretReferenceId::new(Uuid::new_v4()),
            );
            let user = Uuid::new_v4();
            let tenant = TenantId::new(Uuid::new_v4());
            let credential = credential(refresh, user, tenant);
            store
                .set(
                    &id,
                    SecretMaterial::new(serde_json::to_vec(&credential).unwrap()).unwrap(),
                )
                .unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let (entered, accepted) = mpsc::channel();
            let (release, waiting) = mpsc::channel();
            let endpoint =
                url::Url::parse(&format!("https://{}/", listener.local_addr().unwrap())).unwrap();
            let peer = std::thread::spawn(move || {
                let (_socket, _) = listener.accept().unwrap();
                entered.send(()).unwrap();
                let _ = waiting.recv_timeout(Duration::from_secs(3));
            });
            let scope = ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(tenant.value()),
            };
            let client = test_client(endpoint, scope.clone(), store.clone(), id.clone());
            let actor = AuthorizationContext {
                session_id: SessionId::new(Uuid::new_v4()),
                tenant_id: tenant,
                workspace_id: None,
                scope: scope.clone(),
                identity: AuthenticatedIdentity {
                    principal_id: PrincipalId::new(user),
                    principal_kind: PrincipalKind::User,
                    device_id: None,
                    service_id: None,
                },
            };
            let image = CatalogImageRef {
                id: Uuid::new_v4(),
                kind: CatalogImageKind::Product,
                sha256: "00".repeat(32),
            };
            let worker = Arc::clone(&client);
            let worker_actor = actor.clone();
            let worker_image = image.clone();
            let (finished, completed) = mpsc::channel();
            let deadline = UnixMillis(super::super::unix_millis_now().0 + 500);
            let work = std::thread::spawn(move || {
                finished
                    .send(worker.download(&worker_actor, &worker_image, deadline))
                    .unwrap();
            });
            accepted.recv_timeout(Duration::from_secs(2)).unwrap();
            let admission_start = Instant::now();
            assert_eq!(
                client.remote_scope(),
                scope,
                "Scope reads must not wait behind network I/O"
            );
            assert_eq!(
                client.download(
                    &actor,
                    &image,
                    UnixMillis(super::super::unix_millis_now().0 + 50)
                ),
                Err(ImageError::Unavailable)
            );
            assert!(
                admission_start.elapsed() < Duration::from_millis(200),
                "Scope reads and busy admission must not wait for the stalled peer"
            );
            assert_eq!(
                completed.recv_timeout(Duration::from_secs(2)).unwrap(),
                Err(ImageError::Unavailable)
            );
            assert!(
                client.driver.try_lock().is_ok(),
                "An expired transfer must release its admission before the peer exits"
            );
            // A timed-out refresh must retain the credential for a later retry.
            assert!(store.get(&id).unwrap().is_some());
            release.send(()).unwrap();
            peer.join().unwrap();
            work.join().unwrap();
            store.delete(&id).unwrap();
        }
    }
}
