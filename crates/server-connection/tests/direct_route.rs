use std::{
    env,
    net::{SocketAddr, TcpListener},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use axum_server::tls_rustls::RustlsConfig;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::SigningKey;
use eitmad_contracts::{
    config::SecretReferenceId,
    identity::{DeviceId, ScopeId, ScopeKind, ScopeRef},
    secrets::{SecretId, SecretKind},
    server::{ActivateAccountRequest, DevicePublicKey, TenantCode},
    sync::{BatchAcknowledgement, PullRequest, SyncMessage, SyncMode},
    sync_transport::{
        SyncCancellationReason, SyncFrameId, SyncStreamId, SyncTransportFrame, SyncTransportPayload,
    },
    transport::{CapabilityId, CorrelationId, IdempotencyKey, SchemaId, UnixMillis},
    updates::ReleaseVersion,
    versioning::{PeerHello, PeerKind, SchemaSupport, SupportedProtocol},
};
use eitmad_control_plane::{BootstrapInput, ControlDatabase, ControlPlane, TokenKey};
use eitmad_secret_storage::{FallbackEncryptionKey, SecretStore};
use eitmad_server::{ServerState, router};
use eitmad_server_audit::AuditDatabase;
use eitmad_server_connection::{DirectServerConfig, DirectServerDriver, store_session};
use eitmad_sync::{
    HealthStatus, ReceiveOutcome, RetryPolicy, SyncTransport, TransportAuthentication,
    TransportFailureKind, WanAdapter,
};
use eitmad_sync_plane::{
    AuthoritativeChangeDraft, CommandSubmission, DomainDescriptor, DomainRegistry,
    DomainSyncHandler, DomainValidationError, LocalOperationDraft, SyncCoordinator, SyncDatabase,
    SyncIntent,
};
use uuid::Uuid;

struct TestDomain;

#[async_trait]
impl DomainSyncHandler for TestDomain {
    fn descriptor(&self) -> DomainDescriptor {
        DomainDescriptor {
            schema_id: schema_id(),
            minimum_schema_version: 1,
            maximum_schema_version: 1,
            mode: SyncMode::LocalFirst,
        }
    }

    async fn authorize(
        &self,
        _session: &eitmad_contracts::server::AuthenticatedServerSession,
        _scope: &ScopeRef,
        _intent: SyncIntent,
    ) -> bool {
        true
    }

    fn validate_local(&self, _draft: &LocalOperationDraft) -> Result<(), DomainValidationError> {
        Err(DomainValidationError::Denied)
    }

    fn execute_command(
        &self,
        _session: &eitmad_contracts::server::AuthenticatedServerSession,
        _command: &CommandSubmission,
    ) -> Result<AuthoritativeChangeDraft, DomainValidationError> {
        Err(DomainValidationError::Denied)
    }
}

fn schema_id() -> SchemaId {
    SchemaId::parse("eitmad.schema.direct-route-test.v1").unwrap()
}

fn hello() -> PeerHello {
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
    .map(|value| CapabilityId::parse(value).unwrap())
    .collect();
    PeerHello {
        peer_kind: PeerKind::Engine,
        product_version: ReleaseVersion::new(semver::Version::new(0, 0, 0)),
        protocols: vec![SupportedProtocol {
            major: 1,
            minimum_minor: 4,
            maximum_minor: 6,
        }],
        capabilities,
        required_capabilities: vec![CapabilityId::parse("eitmad.capability.sync.v1").unwrap()],
        schemas: vec![SchemaSupport {
            schema_id: schema_id(),
            minimum_version: 1,
            maximum_version: 1,
            required: true,
        }],
    }
}

fn frame(protocol: eitmad_contracts::versioning::ProtocolVersion) -> SyncTransportFrame {
    SyncTransportFrame {
        frame_id: SyncFrameId::new(Uuid::new_v4()),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        protocol_version: protocol,
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        stream_id: SyncStreamId::new(Uuid::new_v4()),
        sequence: 0,
        end_of_stream: false,
        payload: SyncTransportPayload::Message(SyncMessage::Pull(PullRequest {
            after: None,
            maximum_records: 10,
        })),
    }
}

async fn start_server(
    address: SocketAddr,
    state: ServerState,
    certificate: &Path,
    private_key: &Path,
) -> axum_server::Handle<SocketAddr> {
    let tls = RustlsConfig::from_pem_file(certificate, private_key)
        .await
        .unwrap();
    let handle = axum_server::Handle::new();
    let server_handle = handle.clone();
    tokio::spawn(async move {
        axum_server::bind_rustls(address, tls)
            .handle(server_handle)
            .serve(router(state).into_make_service())
            .await
            .unwrap();
    });
    for _ in 0..50 {
        if std::net::TcpStream::connect_timeout(&address, Duration::from_millis(25)).is_ok() {
            return handle;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("test server did not start");
}

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(env::var_os(name).expect("set the live test certificate path"))
}

fn cancel_then_pull(
    mut transport: WanAdapter<DirectServerDriver>,
) -> WanAdapter<DirectServerDriver> {
    let protocol = transport.negotiated_session().unwrap().protocol;
    transport
        .cancel(
            SyncStreamId::new(Uuid::new_v4()),
            CorrelationId::new(Uuid::new_v4()),
            SyncCancellationReason::ClientRequested,
            eitmad_control_plane::unix_millis_now(),
        )
        .unwrap();
    transport
        .send(&frame(protocol), eitmad_control_plane::unix_millis_now())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            Instant::now() < deadline,
            "post-cancel response deadline elapsed"
        );
        match transport.receive(eitmad_control_plane::unix_millis_now()) {
            Ok(ReceiveOutcome::NoFrame) => {}
            Ok(ReceiveOutcome::Frame(reply)) => {
                assert!(matches!(
                    reply.payload,
                    SyncTransportPayload::Message(SyncMessage::Changes(_))
                ));
                return transport;
            }
            other => panic!("unexpected post-cancel response: {other:?}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires a disposable PostgreSQL database and a generated trusted development certificate"]
async fn real_server_authentication_tls_sync_and_reconnect() {
    let database_url = env::var("EITMAD_DIRECT_TEST_DATABASE_URL")
        .expect("set EITMAD_DIRECT_TEST_DATABASE_URL to an empty disposable PostgreSQL database");
    let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
    let private_key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
    let trusted_certificate = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
    let wrong_certificate = required_path("EITMAD_DIRECT_TEST_WRONG_CERTIFICATE");
    let control_database = ControlDatabase::connect(&database_url, 4).await.unwrap();
    control_database.migrate().await.unwrap();
    let sync_database = SyncDatabase::connect(&database_url, 4).await.unwrap();
    sync_database.migrate().await.unwrap();
    AuditDatabase::from_pool(control_database.pool())
        .migrate()
        .await
        .unwrap();
    let control = ControlPlane::new(control_database.pool(), TokenKey::new([9; 32]));
    let now = eitmad_control_plane::unix_millis_now();
    let bootstrap = control
        .identity
        .bootstrap(
            &BootstrapInput {
                tenant_code: TenantCode::parse("direct-test").unwrap(),
                tenant_display_name: "اختبار الاتصال".to_owned(),
                organization_display_name: "مصنع الاختبار".to_owned(),
                owner_username: "owner".to_owned(),
            },
            CorrelationId::new(Uuid::new_v4()),
            now,
        )
        .await
        .unwrap();
    let device_id = DeviceId::new(Uuid::new_v4());
    let signing_seed = [11; 32];
    let signing = SigningKey::from_bytes(&signing_seed);
    let authentication = control
        .authentication
        .activate(
            &ActivateAccountRequest {
                invite_token: bootstrap.invite_token,
                password: "synthetic-test-password-123".to_owned(),
                device_id,
                device_label: "اختبار سطح المكتب".to_owned(),
                device_public_key: DevicePublicKey {
                    algorithm: "ed25519".to_owned(),
                    base64: URL_SAFE_NO_PAD.encode(signing.verifying_key().as_bytes()),
                },
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
    let account_id = authentication.session.account_id;
    let scope = ScopeRef {
        kind: ScopeKind::parse("organization").unwrap(),
        id: ScopeId::new(bootstrap.organization_id.value()),
    };
    let registry =
        DomainRegistry::new([Arc::new(TestDomain) as Arc<dyn DomainSyncHandler>]).unwrap();
    let state = ServerState::new(control, SyncCoordinator::new(&sync_database, registry));
    let address: SocketAddr = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let handle = start_server(address, state.clone(), &certificate, &private_key).await;
    let endpoint = format!("https://localhost:{}/", address.port());
    let workspace = tempfile::tempdir().unwrap();
    let store =
        SecretStore::open(workspace.path(), Some(FallbackEncryptionKey::new([7; 32]))).unwrap();
    let credential_id = SecretId::new(
        SecretKind::parse("direct-test-session").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    let mut invalid_authentication: eitmad_contracts::server::AuthenticationResult =
        serde_json::from_slice(&serde_json::to_vec(&authentication).unwrap()).unwrap();
    let mut expiring_authentication: eitmad_contracts::server::AuthenticationResult =
        serde_json::from_slice(&serde_json::to_vec(&authentication).unwrap()).unwrap();
    invalid_authentication.tokens.access_token = "invalid-token".to_owned();
    let invalid_id = SecretId::new(
        SecretKind::parse("direct-test-session").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_session(&store, &invalid_id, invalid_authentication, signing_seed).unwrap();
    store_session(&store, &credential_id, authentication, signing_seed).unwrap();
    let config = DirectServerConfig::new(
        &endpoint,
        scope.clone(),
        schema_id(),
        1,
        &trusted_certificate,
    )
    .unwrap();
    let wan_endpoint = config.wan_endpoint();
    let auth = TransportAuthentication::AccountDevice {
        account_id,
        device_id,
        credential: invalid_id.clone(),
    };
    let bad_driver = DirectServerDriver::new(config, store.clone(), hello());
    let mut bad_transport = WanAdapter::new(
        wan_endpoint.clone(),
        bad_driver,
        hello(),
        auth,
        RetryPolicy::default(),
    )
    .unwrap();
    let failure = tokio::task::spawn_blocking(move || {
        bad_transport
            .connect(eitmad_control_plane::unix_millis_now())
            .unwrap_err()
    })
    .await
    .unwrap();
    assert_eq!(failure.kind, TransportFailureKind::AuthenticationFailed);
    let wrong_config =
        DirectServerConfig::new(&endpoint, scope.clone(), schema_id(), 1, &wrong_certificate)
            .unwrap();
    let wrong_endpoint = wrong_config.wan_endpoint();
    let wrong_driver = DirectServerDriver::new(wrong_config, store.clone(), hello());
    let mut wrong_transport = WanAdapter::new(
        wrong_endpoint,
        wrong_driver,
        hello(),
        TransportAuthentication::AccountDevice {
            account_id,
            device_id,
            credential: credential_id.clone(),
        },
        RetryPolicy::default(),
    )
    .unwrap();
    let failure = tokio::task::spawn_blocking(move || {
        wrong_transport
            .connect(eitmad_control_plane::unix_millis_now())
            .unwrap_err()
    })
    .await
    .unwrap();
    assert_eq!(failure.kind, TransportFailureKind::EncryptionRequired);
    expiring_authentication.tokens.access_expires_at =
        UnixMillis(eitmad_control_plane::unix_millis_now().0 - 1);
    store_session(
        &store,
        &credential_id,
        expiring_authentication,
        signing_seed,
    )
    .unwrap();
    let old_material = store.get(&credential_id).unwrap().unwrap();
    let config =
        DirectServerConfig::new(&endpoint, scope, schema_id(), 1, &trusted_certificate).unwrap();
    let driver = DirectServerDriver::new(config, store.clone(), hello());
    let mut transport = WanAdapter::new(
        wan_endpoint,
        driver,
        hello(),
        TransportAuthentication::AccountDevice {
            account_id,
            device_id,
            credential: credential_id.clone(),
        },
        RetryPolicy::default(),
    )
    .unwrap();
    transport = tokio::task::spawn_blocking(move || {
        let negotiated = transport
            .connect(eitmad_control_plane::unix_millis_now())
            .unwrap();
        let request = frame(negotiated.protocol);
        transport
            .send(&request, eitmad_control_plane::unix_millis_now())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let batch = loop {
            assert!(Instant::now() < deadline, "sync response deadline elapsed");
            match transport.receive(eitmad_control_plane::unix_millis_now()) {
                Ok(ReceiveOutcome::NoFrame) => continue,
                Ok(ReceiveOutcome::Frame(reply)) => {
                    let SyncTransportPayload::Message(SyncMessage::Changes(batch)) = reply.payload
                    else {
                        panic!("expected a real change batch");
                    };
                    break batch;
                }
                other => panic!("unexpected sync response: {other:?}"),
            }
        };
        let acknowledgement = BatchAcknowledgement {
            delivery_id: batch.delivery_id,
            checkpoint: batch.checkpoint,
            accepted_records: u32::try_from(batch.records.len()).unwrap(),
        };
        let mut acknowledge_frame = frame(negotiated.protocol);
        acknowledge_frame.payload =
            SyncTransportPayload::Message(SyncMessage::Acknowledge(acknowledgement.clone()));
        transport
            .send(&acknowledge_frame, eitmad_control_plane::unix_millis_now())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(
                Instant::now() < deadline,
                "acknowledgement deadline elapsed"
            );
            match transport.receive(eitmad_control_plane::unix_millis_now()) {
                Ok(ReceiveOutcome::NoFrame) => continue,
                Ok(ReceiveOutcome::Frame(reply)) => {
                    assert_eq!(
                        reply.payload,
                        SyncTransportPayload::Message(SyncMessage::Acknowledge(acknowledgement))
                    );
                    break;
                }
                other => panic!("unexpected acknowledgement response: {other:?}"),
            }
        }
        cancel_then_pull(transport)
    })
    .await
    .unwrap();
    assert_ne!(
        old_material.expose_secret(),
        store.get(&credential_id).unwrap().unwrap().expose_secret(),
        "refresh must replace the stored token pair"
    );
    handle.graceful_shutdown(Some(Duration::from_secs(1)));
    tokio::time::sleep(Duration::from_millis(100)).await;
    let (mut transport, failure) = tokio::task::spawn_blocking(move || {
        let failure = transport
            .receive(eitmad_control_plane::unix_millis_now())
            .unwrap_err();
        (transport, failure)
    })
    .await
    .unwrap();
    assert_eq!(failure.kind, TransportFailureKind::ServerUnavailable);
    assert_eq!(transport.health().status, HealthStatus::Offline);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _second_handle = start_server(address, state, &certificate, &private_key).await;
    let retry_at = transport.health().next_retry_at.unwrap();
    transport = tokio::task::spawn_blocking(move || {
        let negotiated = transport.connect(UnixMillis(retry_at.0)).unwrap();
        transport
            .send(&frame(negotiated.protocol), UnixMillis(retry_at.0))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(
                Instant::now() < deadline,
                "reconnected sync response deadline elapsed"
            );
            match transport.receive(eitmad_control_plane::unix_millis_now()) {
                Ok(ReceiveOutcome::NoFrame) => continue,
                Ok(ReceiveOutcome::Frame(reply)) => {
                    assert!(matches!(
                        reply.payload,
                        SyncTransportPayload::Message(SyncMessage::Changes(_))
                    ));
                    break;
                }
                other => panic!("unexpected sync response after reconnect: {other:?}"),
            }
        }
        transport
    })
    .await
    .unwrap();
    transport.disconnect(eitmad_control_plane::unix_millis_now());
    store.delete(&credential_id).unwrap();
    store.delete(&invalid_id).unwrap();
}
