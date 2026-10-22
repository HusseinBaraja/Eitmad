//! All peers, credentials, files, and addresses here are disposable synthetic fixtures.
use super::*;
use crate::authenticated_http::{AuthenticatedHttpClient, HttpError};
use crate::{
    CertificateDer, SchemaId, ScopeRef, SecretId, SecretMaterial, SecretStore, SigningKey,
    StoredCredential, StreamOwned, URL_SAFE_NO_PAD, UnixMillis, account_credential_id,
};
use base64::Engine as _;
use eitmad_contracts::{
    identity::{
        AccountId, AuthenticatedIdentity, AuthorizationContext, OrganizationId, PrincipalId,
        PrincipalKind, ScopeId, ScopeKind, SessionId, TenantId, UserId,
    },
    pairing::{PairingBinding, PairingOffer, PairingReceipt},
    server::{DevicePublicKey, TenantCode},
};
use eitmad_secret_storage::FallbackEncryptionKey;
use rustls::{
    ServerConfig, ServerConnection,
    pki_types::{PrivateKeyDer, pem::PemObject as _},
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{SystemTime, UNIX_EPOCH},
};

struct Installation {
    directory: tempfile::TempDir,
    receipt: PairingReceipt,
}
impl Installation {
    fn first() -> Self {
        let installation = Self::new();
        let directory = installation.directory.path();
        std::fs::remove_file(directory.join("desktop-pairing.json")).unwrap();
        let store = eitmad_storage::AuthorityStore::open(directory).unwrap();
        let secrets =
            SecretStore::open(directory, Some(FallbackEncryptionKey::new([49; 32]))).unwrap();
        let device = installation.receipt.binding.device_id;
        let id = SecretId::new(
            eitmad_contracts::secrets::SecretKind::parse("desktop-device-key").unwrap(),
            eitmad_contracts::config::SecretReferenceId::new(device.value()),
        );
        secrets
            .set(&id, SecretMaterial::new(vec![49; 32]).unwrap())
            .unwrap();
        let enrollment = crate::DesktopEnrollment::new(directory.into(), store, secrets.clone());
        let offer = &installation.receipt.binding.offer;
        let request = enrollment
            .first_desktop_request(
                offer.tenant_code.clone(),
                "owner".into(),
                "synthetic-password".into(),
            )
            .unwrap();
        let handoff = eitmad_contracts::setup::SetupDesktopHandoff {
            version: 1,
            server_id: offer.identity.server_id,
            public_ca_pem: offer.identity.public_ca_pem.clone(),
            prepared: eitmad_contracts::setup::PreparedWindowsWorkshop {
                version: 1,
                endpoint: offer.identity.endpoint.clone(),
                workshop: eitmad_contracts::setup::InitializedWorkshop {
                    version: 1,
                    tenant_id: offer.tenant_id,
                    tenant_code: offer.tenant_code.clone(),
                    organization_id: offer.organization_id,
                    branch_id: offer.branch_id,
                    owner_account_id: AccountId::new(Uuid::new_v4()),
                },
            },
        };
        enrollment.retain_first_desktop(&handoff, &request).unwrap();
        // Interrupted setup can reuse the exact handoff, but cannot replace authenticated identity.
        enrollment.retain_first_desktop(&handoff, &request).unwrap();
        let before = std::fs::read(directory.join("desktop-first-server.json")).unwrap();
        for change in ["identity", "ca", "scope", "key"] {
            let mut changed = handoff.clone();
            let mut login = request.clone();
            match change {
                "identity" => {
                    changed.server_id =
                        eitmad_contracts::discovery::ServerInstallationId::new(Uuid::new_v4());
                }
                "ca" => changed.public_ca_pem.push(' '),
                "scope" => changed.prepared.workshop.branch_id = ScopeId::new(Uuid::new_v4()),
                _ => login.device_public_key.base64 = URL_SAFE_NO_PAD.encode([0; 32]),
            }
            assert!(enrollment.retain_first_desktop(&changed, &login).is_err());
            assert_eq!(
                std::fs::read(directory.join("desktop-first-server.json")).unwrap(),
                before
            );
        }
        secrets.delete(&id).unwrap();
        installation
    }

    fn metadata_path(&self) -> std::path::PathBuf {
        let first = self.directory.path().join("desktop-first-server.json");
        if first.exists() {
            first
        } else {
            self.directory.path().join("desktop-pairing.json")
        }
    }

    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ca = std::fs::read_to_string(
            std::env::var("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE")
                .expect("generate disposable certificates"),
        )
        .unwrap();
        let receipt = PairingReceipt {
            binding: PairingBinding {
                offer: PairingOffer {
                    version: 1,
                    session_id: Uuid::new_v4(),
                    identity: PairingIdentity {
                        server_id: eitmad_contracts::discovery::ServerInstallationId::new(
                            Uuid::new_v4(),
                        ),
                        endpoint: "https://localhost:1/".into(),
                        public_ca_pem: ca.clone(),
                    },
                    tenant_id: TenantId::new(Uuid::new_v4()),
                    tenant_code: TenantCode::parse("synthetic-workshop").unwrap(),
                    organization_id: OrganizationId::new(Uuid::new_v4()),
                    branch_id: ScopeId::new(Uuid::new_v4()),
                    expires_at: UnixMillis(0), // Enrollment expiry does not expire retained trust.
                },
                attempt_id: Uuid::new_v4(),
                device_id: DeviceId::new(Uuid::new_v4()),
                device_public_key: DevicePublicKey {
                    algorithm: "ed25519".into(),
                    base64: URL_SAFE_NO_PAD
                        .encode(SigningKey::from_bytes(&[49; 32]).verifying_key().as_bytes()),
                },
            },
            confirmation: "synthetic-receipt".into(),
        };
        std::fs::write(
            directory.path().join("desktop-pairing.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.path().join("desktop-device.json"),
            serde_json::to_vec(&receipt.binding.device_id).unwrap(),
        )
        .unwrap();
        std::fs::write(directory.path().join("trust.pem"), ca).unwrap();
        Self { directory, receipt }
    }
    fn config(&self) -> DirectServerConfig {
        DirectServerConfig::new(
            &self.receipt.binding.offer.identity.endpoint,
            ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(self.receipt.binding.offer.organization_id.value()),
            },
            SchemaId::parse("eitmad.schema.desktop-enrollment.v1").unwrap(),
            1,
            &self.directory.path().join("trust.pem"),
        )
        .unwrap()
    }
    fn candidate(&self, address: SocketAddr) -> DiscoveredServer {
        DiscoveredServer {
            server_id: self.receipt.binding.offer.identity.server_id,
            address: address.ip().to_string(),
            port: address.port(),
            expires_at: UnixMillis(unix_millis_now().0 + 15_000),
            identity_conflict: false,
        }
    }
}

struct Peer {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
    work: std::thread::JoinHandle<()>,
}
impl Peer {
    fn start(ip: &str, identity: PairingIdentity, connections: usize, status: u16) -> Self {
        let listener = TcpListener::bind((ip, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let certificates =
            CertificateDer::pem_file_iter(std::env::var("EITMAD_DIRECT_TEST_CERTIFICATE").unwrap())
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
        let key =
            PrivateKeyDer::from_pem_file(std::env::var("EITMAD_DIRECT_TEST_PRIVATE_KEY").unwrap())
                .unwrap();
        let tls = Arc::new(
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(certificates, key)
                .unwrap(),
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = requests.clone();
        let work = std::thread::spawn(move || {
            for _ in 0..connections {
                let until = Instant::now() + Duration::from_secs(5);
                let socket = loop {
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < until =>
                        {
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        _ => panic!("synthetic peer did not receive connection"),
                    }
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut stream =
                    StreamOwned::new(ServerConnection::new(tls.clone()).unwrap(), socket);
                let Some(header) = read_header(&mut stream) else {
                    continue;
                };
                assert!(header.starts_with("GET /v1/server-identity "));
                assert!(!header.to_ascii_lowercase().contains("authorization"));
                received.lock().unwrap().push(header);
                let body = serde_json::to_vec(&identity).unwrap();
                if write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                )
                .and_then(|()| stream.write_all(&body))
                .and_then(|()| stream.flush())
                .is_err()
                {
                    continue;
                }
                if let Some(header) = read_header(&mut stream) {
                    let length = header
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    let mut body = vec![0; length];
                    stream.read_exact(&mut body).unwrap();
                    received.lock().unwrap().push(header);
                    let response = format!(
                        "HTTP/1.1 {status} Synthetic\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                    stream.flush().unwrap();
                    stream.conn.send_close_notify();
                    let _ = stream.flush();
                }
            }
        });
        Self {
            address,
            requests,
            work,
        }
    }
}

fn read_header(stream: &mut impl Read) -> Option<String> {
    let mut bytes = Vec::new();
    while bytes.len() < 8192 && !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).ok()?;
        bytes.push(byte[0]);
    }
    String::from_utf8(bytes).ok()
}

fn recover(
    config: &DirectServerConfig,
    candidates: Vec<DiscoveredServer>,
) -> Result<TlsStream, TransportFailure> {
    config.recovery.as_ref().unwrap().connect_using(
        config,
        &config.endpoint,
        Instant::now() + Duration::from_secs(3),
        |_, _, _| Ok(candidates),
    )
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn restart_and_address_changes_retain_identity_name_trust_scope_and_device() {
    for installation in [Installation::new(), Installation::first()] {
        let before = std::fs::read(installation.metadata_path()).unwrap();
        let mut config = installation.config();
        for (index, ip) in ["127.0.0.1", "127.0.0.2", "127.0.0.1"]
            .into_iter()
            .enumerate()
        {
            if index == 2 {
                config = installation.config();
            } // App restart has no address cache.
            let peer = Peer::start(
                ip,
                installation.receipt.binding.offer.identity.clone(),
                1,
                200,
            );
            drop(recover(&config, vec![installation.candidate(peer.address)]).unwrap());
            assert_eq!(
                config
                    .recovery
                    .as_ref()
                    .unwrap()
                    .route
                    .lock()
                    .unwrap()
                    .address
                    .map(|(_, address)| address),
                Some(peer.address)
            );
            peer.work.join().unwrap();
        }
        assert_eq!(before, std::fs::read(installation.metadata_path()).unwrap());
        assert_eq!(config.endpoint.as_str(), "https://localhost:1/");
    }
}

#[derive(Debug)]
struct ExpiredTime;
impl rustls::time_provider::TimeProvider for ExpiredTime {
    fn current_time(&self) -> Option<rustls::pki_types::UnixTime> {
        Some(rustls::pki_types::UnixTime::since_unix_epoch(
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap() + Duration::from_secs(14 * 86400),
        ))
    }
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn rediscovered_sockets_validate_tls_name_chain_and_expiry() {
    for installation in [Installation::new(), Installation::first()] {
        for change in ["name", "chain", "expiry"] {
            let mut config = installation.config();
            match change {
                "name" => config.endpoint.set_host(Some("127.0.0.2")).unwrap(),
                "chain" => {
                    let wrong = std::fs::read(
                        std::env::var("EITMAD_DIRECT_TEST_WRONG_CERTIFICATE").unwrap(),
                    )
                    .unwrap();
                    config.tls = DirectServerConfig::from_ca_pem(
                        config.endpoint.as_str(),
                        config.scope.clone(),
                        config.schema_id.clone(),
                        1,
                        &wrong,
                    )
                    .unwrap()
                    .tls;
                }
                _ => Arc::make_mut(&mut config.tls).time_provider = Arc::new(ExpiredTime),
            }
            let peer = Peer::start(
                "127.0.0.1",
                installation.receipt.binding.offer.identity.clone(),
                1,
                200,
            );
            assert_eq!(
                recover(&config, vec![installation.candidate(peer.address)])
                    .err()
                    .unwrap()
                    .kind,
                TransportFailureKind::EncryptionRequired
            );
            peer.work.join().unwrap();
            assert!(peer.requests.lock().unwrap().is_empty());
        }
    }
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn spoofed_identity_and_replacement_ca_are_refused_before_credentials() {
    for installation in [Installation::new(), Installation::first()] {
        for change in ["identity", "ca", "endpoint"] {
            let config = installation.config();
            let mut identity = installation.receipt.binding.offer.identity.clone();
            match change {
                "identity" => {
                    identity.server_id =
                        eitmad_contracts::discovery::ServerInstallationId::new(Uuid::new_v4());
                }
                "ca" => identity.public_ca_pem.push(' '),
                _ => identity.endpoint = "https://other.test/".into(),
            }
            let peer = Peer::start("127.0.0.1", identity, 1, 200);
            assert_eq!(
                recover(&config, vec![installation.candidate(peer.address)])
                    .err()
                    .unwrap()
                    .kind,
                TransportFailureKind::EncryptionRequired
            );
            peer.work.join().unwrap();
            assert_eq!(peer.requests.lock().unwrap().len(), 1);
        }
    }
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn stale_conflicted_and_foreign_advertisements_cannot_select_a_server() {
    let installation = Installation::new();
    for change in ["stale", "conflict", "foreign"] {
        let config = installation.config();
        let mut candidate = installation.candidate("127.0.0.1:1".parse().unwrap());
        match change {
            "stale" => candidate.expires_at = UnixMillis(0),
            "conflict" => candidate.identity_conflict = true,
            _ => {
                candidate.server_id =
                    eitmad_contracts::discovery::ServerInstallationId::new(Uuid::new_v4());
            }
        }
        let failure = recover(&config, vec![candidate]).err().unwrap();
        assert_eq!(
            failure.kind,
            if change == "conflict" {
                TransportFailureKind::EncryptionRequired
            } else {
                TransportFailureKind::ServerUnavailable
            }
        );
        assert!(
            config
                .recovery
                .as_ref()
                .unwrap()
                .route
                .lock()
                .unwrap()
                .address
                .is_none()
        );
    }
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn reallocated_old_address_cannot_block_authenticated_rediscovery() {
    let installation = Installation::new();
    let config = installation.config();
    let mut other_identity = installation.receipt.binding.offer.identity.clone();
    other_identity.server_id =
        eitmad_contracts::discovery::ServerInstallationId::new(Uuid::new_v4());
    let old = Peer::start("127.0.0.1", other_identity, 1, 200);
    let retained = config.recovery.as_ref().unwrap();
    retained.route.lock().unwrap().address = Some((0, old.address));
    let current = Peer::start(
        "127.0.0.2",
        installation.receipt.binding.offer.identity.clone(),
        1,
        200,
    );
    drop(recover(&config, vec![installation.candidate(current.address)]).unwrap());
    old.work.join().unwrap();
    current.work.join().unwrap();
    assert_eq!(old.requests.lock().unwrap().len(), 1);
    assert_eq!(
        retained
            .route
            .lock()
            .unwrap()
            .address
            .map(|(_, address)| address),
        Some(current.address)
    );
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn outage_retries_are_paced_and_cancellation_discards_late_discovery() {
    let installation = Installation::new();
    let config = installation.config();
    assert!(recover(&config, Vec::new()).is_err());
    let retained = config.recovery.as_ref().unwrap();
    let result = retained.connect_using(
        &config,
        &config.endpoint,
        Instant::now() + Duration::from_secs(1),
        |_, _, _| panic!("outage retry was not paced"),
    );
    assert_eq!(
        result.err().unwrap().kind,
        TransportFailureKind::ServerUnavailable
    );
    retained.route.lock().unwrap().retry_at = None;
    let result = retained.connect_using(
        &config,
        &config.endpoint,
        Instant::now() + Duration::from_secs(1),
        |control, _, _| {
            control.cancel();
            Ok(Vec::new())
        },
    );
    assert_eq!(result.err().unwrap().phase, FailurePhase::Cancellation);
    assert!(retained.route.lock().unwrap().address.is_none());
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn missing_device_key_cannot_silently_reset_paired_enrollment() {
    let installation = Installation::new();
    let store = eitmad_storage::AuthorityStore::open(installation.directory.path()).unwrap();
    let secrets = SecretStore::open(
        installation.directory.path(),
        Some(FallbackEncryptionKey::new([41; 32])),
    )
    .unwrap();
    let enrollment = crate::DesktopEnrollment::new(
        installation.directory.path().into(),
        store.clone(),
        secrets.clone(),
    );
    let before = std::fs::read(installation.directory.path().join("desktop-device.json")).unwrap();
    assert!(
        enrollment
            .first_desktop_request(
                TenantCode::parse("synthetic-workshop").unwrap(),
                "owner".into(),
                "synthetic-password".into()
            )
            .is_err()
    );
    let key = SecretId::new(
        eitmad_contracts::secrets::SecretKind::parse("desktop-device-key").unwrap(),
        eitmad_contracts::config::SecretReferenceId::new(
            installation.receipt.binding.device_id.value(),
        ),
    );
    assert!(secrets.get(&key).unwrap().is_none());
    assert!(!store.has_local_installation().unwrap());
    assert_eq!(
        before,
        std::fs::read(installation.directory.path().join("desktop-device.json")).unwrap()
    );
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn cancellation_closes_an_active_identity_socket() {
    let installation = Installation::new();
    let config = installation.config();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let candidate = installation.candidate(listener.local_addr().unwrap());
    let control = config.recovery().unwrap();
    let worker = std::thread::spawn(move || recover(&config, vec![candidate]).err().unwrap());
    let (mut socket, _) = listener.accept().unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = [0; 4096];
    assert!(socket.read(&mut bytes).unwrap() > 0); // ClientHello; peer withholds TLS.
    let start = Instant::now();
    control.cancel();
    assert_eq!(worker.join().unwrap().phase, FailurePhase::Cancellation);
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn session_switching_uses_only_the_current_account_and_revocation_requires_sign_in() {
    for installation in [Installation::new(), Installation::first()] {
        let config = installation.config();
        let peer = Peer::start(
            "127.0.0.1",
            installation.receipt.binding.offer.identity.clone(),
            3,
            403,
        );
        drop(recover(&config, vec![installation.candidate(peer.address)]).unwrap());
        let store = SecretStore::open(
            installation.directory.path(),
            Some(FallbackEncryptionKey::new([49; 32])),
        )
        .unwrap();
        let route = SecretId::new(
            eitmad_contracts::secrets::SecretKind::parse("recovery-test").unwrap(),
            eitmad_contracts::config::SecretReferenceId::new(Uuid::new_v4()),
        );
        let client = AuthenticatedHttpClient::from_config(
            config,
            store.clone(),
            route.clone(),
            "eitmad.capability.shared-accounts.v1",
            25,
        );
        let mut ids = Vec::new();
        for (token, user) in [
            ("synthetic-user-a", UserId::new(Uuid::new_v4())),
            ("synthetic-user-b", UserId::new(Uuid::new_v4())),
        ] {
            let actor = AuthorizationContext {
                session_id: SessionId::new(Uuid::new_v4()),
                tenant_id: installation.receipt.binding.offer.tenant_id,
                workspace_id: None,
                scope: ScopeRef {
                    kind: ScopeKind::parse("organization").unwrap(),
                    id: ScopeId::new(installation.receipt.binding.offer.tenant_id.value()),
                },
                identity: AuthenticatedIdentity {
                    principal_id: PrincipalId::new(user.value()),
                    principal_kind: PrincipalKind::User,
                    device_id: Some(installation.receipt.binding.device_id),
                    service_id: None,
                },
            };
            let id = account_credential_id(&route, actor.identity.principal_id);
            let credential = StoredCredential {
                user_id: user,
                tenant_id: actor.tenant_id,
                account_id: AccountId::new(Uuid::new_v4()),
                device_id: installation.receipt.binding.device_id,
                access_token: token.into(),
                refresh_token: "synthetic-refresh".into(),
                access_expires_at: UnixMillis(unix_millis_now().0 + 120_000),
                refresh_expires_at: UnixMillis(unix_millis_now().0 + 240_000),
                signing_seed: [49; 32],
            };
            store
                .set(
                    &id,
                    SecretMaterial::new(serde_json::to_vec(&credential).unwrap()).unwrap(),
                )
                .unwrap();
            ids.push(id);
            let result = client.request::<serde_json::Value>(
                &actor,
                "/v1/accounts/watch",
                &(),
                Instant::now() + Duration::from_secs(3),
            );
            assert!(matches!(result, Err(HttpError::Denied)));
        }
        peer.work.join().unwrap();
        let requests = peer.requests.lock().unwrap();
        let authenticated: Vec<_> = requests
            .iter()
            .filter(|request| request.contains("Bearer "))
            .collect();
        assert_eq!(authenticated.len(), 2);
        assert!(authenticated[0].contains("Bearer synthetic-user-a\r\n"));
        assert!(authenticated[1].contains("Bearer synthetic-user-b\r\n"));
        for id in ids {
            store.delete(&id).unwrap();
        }
    }
}
