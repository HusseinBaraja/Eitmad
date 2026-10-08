use super::{DirectServerDriver, remaining_io};
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
use std::time::{Duration, Instant};

pub struct DirectCatalogImageClient {
    http: crate::authenticated_http::AuthenticatedHttpClient,
}
impl DirectCatalogImageClient {
    #[must_use]
    pub fn new(driver: DirectServerDriver, credential_id: SecretId) -> Self {
        Self {
            http: crate::authenticated_http::AuthenticatedHttpClient::new(driver, credential_id),
        }
    }
    #[must_use]
    pub fn from_config(
        config: super::DirectServerConfig,
        secrets: eitmad_secret_storage::SecretStore,
        credential_id: SecretId,
    ) -> Self {
        Self {
            http: crate::authenticated_http::AuthenticatedHttpClient::from_config(
                config,
                secrets,
                credential_id,
                "eitmad.capability.catalog-image.v1",
                14,
            ),
        }
    }
    fn remote_scope(&self) -> ScopeRef {
        self.http.remote_scope()
    }
    fn request<T: serde::de::DeserializeOwned>(
        &self,
        actor: &AuthorizationContext,
        route: &str,
        input: &impl serde::Serialize,
        budget: Instant,
    ) -> Result<T, ImageError> {
        self.http
            .request(actor, route, input, budget)
            .map_err(|e| match e {
                crate::authenticated_http::HttpError::Denied => ImageError::Denied,
                crate::authenticated_http::HttpError::Invalid => ImageError::Invalid,
                crate::authenticated_http::HttpError::NotFound => ImageError::NotFound,
                _ => ImageError::Unavailable,
            })
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
            user_id: eitmad_contracts::identity::UserId::new(user),
            tenant_id: tenant,
            account_id: eitmad_contracts::identity::AccountId::new(Uuid::new_v4()),
            device_id: eitmad_contracts::identity::DeviceId::new(Uuid::new_v4()),
            access_token: "synthetic-access".into(),
            refresh_token: "synthetic-refresh".into(),
            access_expires_at: UnixMillis(if refresh {
                0
            } else {
                super::super::unix_millis_now().0 + 120_000
            }),
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
    fn catalog_image_hello_negotiates_with_a_minor_14_server() {
        use eitmad_contracts::versioning::{
            NegotiationOutcome, PeerKind, SupportedProtocol, negotiate,
        };
        let _ = rustls::crypto::ring::default_provider().install_default();
        let directory = tempfile::TempDir::new().unwrap();
        let store =
            SecretStore::open(directory.path(), Some(FallbackEncryptionKey::new([7; 32]))).unwrap();
        let id = SecretId::new(
            SecretKind::parse("image-protocol-test").unwrap(),
            SecretReferenceId::new(Uuid::new_v4()),
        );
        let scope = ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(Uuid::new_v4()),
        };
        let client = test_client(
            url::Url::parse("https://127.0.0.1:8443/").unwrap(),
            scope,
            store,
            id,
        );
        let hello = client.http.driver.lock().unwrap().local_hello.clone();
        let mut server = hello.clone();
        server.peer_kind = PeerKind::Server;
        server.protocols = vec![SupportedProtocol {
            major: 1,
            minimum_minor: 14,
            maximum_minor: 14,
        }];
        let NegotiationOutcome::Accepted(session) = negotiate(&server, &hello) else {
            panic!("Catalog images must negotiate their declared protocol");
        };
        assert!(
            session
                .capabilities
                .contains(&hello.required_capabilities[0])
        );
        server.capabilities.clear();
        assert!(matches!(
            negotiate(&server, &hello),
            NegotiationOutcome::Rejected(_)
        ));
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
                client.http.driver.try_lock().is_ok(),
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
