//! Synthetic TLS proof using the existing disposable direct-route certificates.
use super::*;
use crate::authenticated_http::AuthenticatedHttpClient;
use eitmad_contracts::identity::{
    AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId, ScopeKind,
    SessionId, TenantId, UserId,
};
use eitmad_secret_storage::FallbackEncryptionKey;
use rustls::{ServerConfig, ServerConnection, pki_types::PrivateKeyDer};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

struct Fixture {
    _directory: tempfile::TempDir,
    store: SecretStore,
    route: SecretId,
    id: SecretId,
    value: StoredCredential,
    actor: AuthorizationContext,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store = SecretStore::open_encrypted_for_tests(
            directory.path(),
            FallbackEncryptionKey::new([7; 32]),
        )
        .unwrap();
        let route = SecretId::new(
            eitmad_contracts::secrets::SecretKind::parse("credential-tls-test").unwrap(),
            eitmad_contracts::config::SecretReferenceId::new(Uuid::new_v4()),
        );
        let tenant = TenantId::new(Uuid::new_v4());
        let user = UserId::new(Uuid::new_v4());
        let value = StoredCredential {
            user_id: user,
            tenant_id: tenant,
            account_id: AccountId::new(Uuid::new_v4()),
            device_id: DeviceId::new(Uuid::new_v4()),
            access_token: "synthetic-old".into(),
            refresh_token: "synthetic-refresh".into(),
            access_expires_at: UnixMillis(0),
            refresh_expires_at: UnixMillis(unix_millis_now().0 + 240_000),
            signing_seed: [7; 32],
        };
        let actor = AuthorizationContext {
            session_id: SessionId::new(Uuid::new_v4()),
            tenant_id: tenant,
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(tenant.value()),
            },
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(user.value()),
                principal_kind: PrincipalKind::User,
                device_id: Some(value.device_id),
                service_id: None,
            },
        };
        let id = account_credential_id(&route, actor.identity.principal_id);
        store
            .set(
                &id,
                SecretMaterial::new(serde_json::to_vec(&value).unwrap()).unwrap(),
            )
            .unwrap();
        Self {
            _directory: directory,
            store,
            route,
            id,
            value,
            actor,
        }
    }

    fn authentication(&self, access: &str) -> AuthenticationResult {
        serde_json::from_value(serde_json::json!({
            "session": { "sessionId": Uuid::new_v4(), "accountId": self.value.account_id, "userId": self.value.user_id, "deviceId": self.value.device_id, "tenantId": self.value.tenant_id, "issuedAt": unix_millis_now(), "expiresAt": self.value.refresh_expires_at },
            "tokens": { "accessToken": access, "refreshToken": "synthetic-next-refresh", "accessExpiresAt": unix_millis_now().0 + 120_000, "refreshExpiresAt": self.value.refresh_expires_at }
        })).unwrap()
    }

    fn client(&self, address: SocketAddr) -> Arc<AuthenticatedHttpClient> {
        let trust = std::env::var("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE")
            .expect("generated test trust anchor");
        let config = DirectServerConfig::new(
            &format!("https://127.0.0.1:{}/", address.port()),
            self.actor.scope.clone(),
            SchemaId::parse("eitmad.schema.catalog-image.v1").unwrap(),
            1,
            Path::new(&trust),
        )
        .unwrap();
        Arc::new(AuthenticatedHttpClient::from_config(
            config,
            self.store.clone(),
            self.route.clone(),
            "eitmad.capability.catalog-image.v1",
            14,
        ))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(
            self.store.delete(&self.id).is_ok(),
            "Remove only this disposable credential"
        );
    }
}

struct Peer {
    address: SocketAddr,
    refresh_entered: mpsc::Receiver<()>,
    release: mpsc::Sender<()>,
    work: std::thread::JoinHandle<()>,
    refreshes: Arc<AtomicUsize>,
}

fn tls_peer(authentication: &AuthenticationResult, requests: usize) -> Peer {
    let cert = std::env::var("EITMAD_DIRECT_TEST_CERTIFICATE").expect("generated test certificate");
    let key = std::env::var("EITMAD_DIRECT_TEST_PRIVATE_KEY").expect("generated test key");
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                CertificateDer::pem_file_iter(cert)
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
                PrivateKeyDer::from_pem_file(key).unwrap(),
            )
            .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (entered, refresh_entered) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let refreshes = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&refreshes);
    let body = serde_json::to_vec(authentication).unwrap();
    let work = std::thread::spawn(move || {
        let config = Arc::new(config);
        let wait = Mutex::new(wait);
        std::thread::scope(|scope| {
            for _ in 0..requests {
                let end = Instant::now() + Duration::from_secs(5);
                let socket = loop {
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                && Instant::now() < end =>
                        {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        _ => panic!("Synthetic TLS request did not arrive"),
                    }
                };
                // Windows accepted sockets can inherit the listener's nonblocking mode.
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let connection = ServerConnection::new(Arc::clone(&config)).unwrap();
                let (body, entered, wait, count) = (&body, &entered, &wait, &count);
                scope.spawn(move || {
                    serve(
                        StreamOwned::new(connection, socket),
                        body,
                        entered,
                        wait,
                        count,
                    );
                });
            }
        });
    });
    Peer {
        address,
        refresh_entered,
        release,
        work,
        refreshes,
    }
}

fn serve(
    mut tls: StreamOwned<ServerConnection, TcpStream>,
    refresh: &[u8],
    entered: &mpsc::Sender<()>,
    wait: &Mutex<mpsc::Receiver<()>>,
    count: &AtomicUsize,
) {
    let mut request = Zeroizing::new(Vec::new());
    let mut byte = [0];
    while !request.ends_with(b"\r\n\r\n") {
        assert!(request.len() < 8192);
        tls.read_exact(&mut byte).unwrap();
        request.push(byte[0]);
    }
    let headers = std::str::from_utf8(&request).unwrap();
    let length: usize = headers
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .unwrap_or("0")
        .parse()
        .unwrap();
    let mut body = Zeroizing::new(vec![0; length]);
    tls.read_exact(&mut body).unwrap();
    if headers.starts_with("GET /v1/connect?") {
        entered.send(()).unwrap();
        wait.lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        tls.write_all(
            b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .unwrap();
        tls.conn.send_close_notify();
        tls.flush().unwrap();
        return;
    }
    let response = if headers.starts_with("POST /v1/auth/refresh ") {
        assert_eq!(
            count.fetch_add(1, Ordering::AcqRel),
            0,
            "The rotating pair must refresh once"
        );
        entered.send(()).unwrap();
        wait.lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        refresh
    } else {
        assert!(
            headers.contains("Authorization: Bearer synthetic-rotated\r\n"),
            "Read must use the persisted replacement"
        );
        b"{\"confirmed\":true}"
    };
    write!(
        tls,
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.len()
    )
    .unwrap();
    tls.write_all(response).unwrap();
    tls.conn.send_close_notify();
    tls.flush().unwrap();
}

fn read(client: &AuthenticatedHttpClient, actor: &AuthorizationContext) -> bool {
    eitmad_observability_audit::operation_diagnostics::with_diagnostic_context(
        eitmad_contracts::transport::CorrelationId::new(Uuid::new_v4()),
        eitmad_observability_audit::operation_diagnostics::DiagnosticSource::Pricing,
        || {
            client
                .request::<serde_json::Value>(
                    actor,
                    "/v1/pricing/read",
                    &serde_json::json!({}),
                    Instant::now() + Duration::from_secs(3),
                )
                .is_ok_and(|value| value["confirmed"] == true)
        },
    )
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn concurrent_http_reads_share_one_refresh_and_both_confirm() {
    let fixture = Fixture::new();
    let peer = tls_peer(&fixture.authentication("synthetic-rotated"), 3);
    let client = fixture.client(peer.address);
    std::thread::scope(|scope| {
        let first = scope.spawn(|| read(&client, &fixture.actor));
        peer.refresh_entered
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        let (send, pending) = mpsc::channel();
        let (client, actor) = (&client, &fixture.actor);
        let second = scope.spawn(move || {
            send.send(read(client, actor)).unwrap();
        });
        let early = pending.recv_timeout(Duration::from_millis(100));
        peer.release.send(()).unwrap();
        assert!(first.join().unwrap());
        second.join().unwrap();
        assert!(early.is_err(), "Contending read must wait for rotation");
        assert!(pending.recv_timeout(Duration::from_secs(1)).unwrap());
    });
    peer.work.join().unwrap();
    assert_eq!(peer.refreshes.load(Ordering::Acquire), 1);
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn replacement_session_waits_for_refresh_and_is_not_overwritten() {
    let fixture = Fixture::new();
    let peer = tls_peer(&fixture.authentication("synthetic-rotated"), 2);
    let client = fixture.client(peer.address);
    std::thread::scope(|scope| {
        let reading = scope.spawn(|| read(&client, &fixture.actor));
        peer.refresh_entered
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        let (send, pending) = mpsc::channel();
        let fixture = &fixture;
        let replacing = scope.spawn(move || {
            send.send(store_account_session(
                &fixture.store,
                &fixture.route,
                fixture.authentication("synthetic-new-session"),
                [8; 32],
            ))
            .unwrap();
        });
        let early = pending.recv_timeout(Duration::from_millis(100));
        peer.release.send(()).unwrap();
        assert!(reading.join().unwrap());
        replacing.join().unwrap();
        assert!(early.is_err(), "A new session must not race an old refresh");
        assert!(
            pending
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .is_ok()
        );
    });
    peer.work.join().unwrap();
    let value = client.driver.load_credential(&fixture.id).unwrap();
    assert!(
        value.access_token == "synthetic-new-session",
        "Old refresh must not overwrite the new session"
    );
    assert!(value.signing_seed == [8; 32]);
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn rejected_connection_cannot_delete_a_replacement_session() {
    let fixture = Fixture::new();
    store_account_session(
        &fixture.store,
        &fixture.route,
        fixture.authentication("synthetic-rotated"),
        [7; 32],
    )
    .unwrap();
    let peer = tls_peer(&fixture.authentication("unused"), 1);
    let mut client = Arc::try_unwrap(fixture.client(peer.address)).ok().unwrap();
    std::thread::scope(|scope| {
        let connecting = scope.spawn(|| {
            client.driver.establish_before(
                &ConnectionTarget::WanServer {
                    endpoint: client.driver.config.endpoint.to_string(),
                },
                &TransportAuthentication::AccountDevice {
                    account_id: fixture.value.account_id,
                    device_id: fixture.value.device_id,
                    credential: fixture.id.clone(),
                },
                Instant::now() + Duration::from_secs(3),
            )
        });
        peer.refresh_entered
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        let (send, pending) = mpsc::channel();
        let fixture = &fixture;
        let replacing = scope.spawn(move || {
            send.send(store_account_session(
                &fixture.store,
                &fixture.route,
                fixture.authentication("synthetic-new-session"),
                [8; 32],
            ))
            .unwrap();
        });
        let early = pending.recv_timeout(Duration::from_millis(100));
        peer.release.send(()).unwrap();
        assert_eq!(
            connecting.join().unwrap().unwrap_err().kind,
            TransportFailureKind::AuthenticationFailed
        );
        replacing.join().unwrap();
        assert!(
            early.is_err(),
            "Replacement must wait for old authentication cleanup"
        );
        assert!(
            pending
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .is_ok()
        );
    });
    peer.work.join().unwrap();
    let value = client.driver.load_credential(&fixture.id).unwrap();
    assert!(
        value.access_token == "synthetic-new-session",
        "Old denial must not delete a new session"
    );
}

#[test]
#[ignore = "requires generated disposable direct-route TLS certificates"]
fn credential_wait_and_socket_share_the_establishment_deadline() {
    let fixture = Fixture::new();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut client = Arc::try_unwrap(fixture.client(listener.local_addr().unwrap()))
        .ok()
        .unwrap();
    // Keep credentials fresh so the remaining time is spent in socket/TLS setup.
    let mut value = client.driver.load_credential(&fixture.id).unwrap();
    value.access_expires_at = UnixMillis(unix_millis_now().0 + 120_000);
    fixture
        .store
        .set(
            &fixture.id,
            SecretMaterial::new(serde_json::to_vec(&value).unwrap()).unwrap(),
        )
        .unwrap();
    let gate = credential_gate(&fixture.id).unwrap();
    let (entered, ready) = mpsc::channel();
    let owner = std::thread::spawn(move || {
        let _held = gate
            .acquire(Instant::now() + Duration::from_secs(2))
            .unwrap();
        entered.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(100));
    });
    ready.recv_timeout(Duration::from_secs(1)).unwrap();
    let (release, wait) = mpsc::channel();
    listener.set_nonblocking(true).unwrap();
    let peer = std::thread::spawn(move || {
        // The shared deadline can expire before connect on a loaded host.
        // Never leave cleanup blocked in accept when no socket was opened.
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((_socket, _)) => {
                    let _ = wait.recv_timeout(Duration::from_secs(2));
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("Synthetic peer could not accept: {error}"),
            }
        }
    });
    let start = Instant::now();
    let result = client.driver.establish_before(
        &ConnectionTarget::WanServer {
            endpoint: client.driver.config.endpoint.to_string(),
        },
        &TransportAuthentication::AccountDevice {
            account_id: fixture.value.account_id,
            device_id: fixture.value.device_id,
            credential: fixture.id.clone(),
        },
        start + Duration::from_millis(250),
    );
    let elapsed = start.elapsed();
    let _ = release.send(());
    owner.join().unwrap();
    peer.join().unwrap();
    assert!(result.is_err());
    assert!(elapsed >= Duration::from_millis(250));
    assert!(
        elapsed < Duration::from_millis(750),
        "Admission must not start another socket budget"
    );
}
