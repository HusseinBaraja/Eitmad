use std::{
    collections::BTreeMap,
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
use eitmad_admin_plane::AdminDatabase;
use eitmad_authorization::{
    AuthorizationGate, AuthorizationService, BoundaryAuditContext, BoundaryKind, MANAGER_RELATION,
    MutationContext, RelationshipPolicy,
};
use eitmad_contracts::{
    authorization::{
        ActionId, AuthorizationRequest, ObjectId, ObjectKind, PermissionRule, RelationId,
        RelationshipSubject, RelationshipTuple, ScopedObject, TupleSubject,
    },
    commands::{CreateCustomer, GrantScopeRelationship, UpdateCustomer},
    config::SecretReferenceId,
    customer::{
        CustomerId, CustomerName, CustomerPhone, CustomerSyncPayload, CustomerSyncState,
        GetCustomer,
    },
    identity::{
        AccountId, AuthenticatedIdentity, AuthorizationContext, DeviceId, PrincipalId,
        PrincipalKind, ScopeId, ScopeKind, ScopeRef, SessionId,
    },
    secrets::{SecretId, SecretKind},
    server::{
        ActivateAccountRequest, AuthenticationResult, DevicePublicKey, LoginRequest,
        RegisterBranchRequest, TenantCode,
    },
    sync::{
        BatchAcknowledgement, ChangeId, ChangeOperation, ChangeRecord, EncodedDomainPayload,
        LocalChangeDisposition, LocalChangeSubmission, PullRequest, RecordId, SyncMessage,
        SyncMode,
    },
    sync_transport::{
        SyncCancellationReason, SyncFrameId, SyncStreamId, SyncTransportFrame, SyncTransportPayload,
    },
    transport::{CapabilityId, CorrelationId, IdempotencyKey, SchemaId, UnixMillis},
    updates::ReleaseVersion,
    versioning::{PeerHello, PeerKind, SchemaSupport, SupportedProtocol},
};
use eitmad_control_plane::{
    BootstrapInput, BranchService, ControlDatabase, ControlPlane, TokenKey,
};
use eitmad_customer::{CustomerService, CustomerSyncCycle};
use eitmad_observability_audit::AuditTarget;
use eitmad_postgres_support as sqlx;
use eitmad_secret_storage::{FallbackEncryptionKey, SecretStore};
use eitmad_server::{ServerState, router};
use eitmad_server_audit::AuditDatabase;
use eitmad_server_connection::{DirectServerConfig, DirectServerDriver, store_session};
use eitmad_storage::AuthorityStore;
use eitmad_sync::{
    HealthStatus, ReceiveOutcome, RetryPolicy, SyncAuthorization, SyncEngine, SyncTransport,
    TransportAuthentication, TransportFailureKind, WanAdapter,
};
use eitmad_sync_plane::{
    AuthoritativeChangeDraft, CommandSubmission, CustomerSyncHandler, DomainDescriptor,
    DomainRegistry, DomainSyncHandler, DomainValidationError, LocalOperationDraft, SyncCoordinator,
    SyncDatabase, SyncIntent,
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

fn customer_hello() -> PeerHello {
    let mut peer = hello();
    peer.schemas[0].schema_id = SchemaId::parse("eitmad.schema.customer.v1").unwrap();
    peer
}

fn customer_change(
    scope: &ScopeRef,
    customer_id: CustomerId,
    name: &str,
    base_revision: Option<u64>,
) -> ChangeRecord {
    let revision = base_revision.unwrap_or(0) + 1;
    let payload = CustomerSyncPayload {
        customer_id,
        name: CustomerName::parse(name).unwrap(),
        phone: CustomerPhone::parse("+967777123456").unwrap(),
        address: None,
        notes: None,
        revision,
    };
    ChangeRecord {
        change_id: ChangeId::new(Uuid::new_v4()),
        record_id: RecordId::new(customer_id.value()),
        scope: scope.clone(),
        operation: ChangeOperation::Upsert,
        base_revision,
        revision,
        changed_at: eitmad_control_plane::unix_millis_now(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        payload: Some(EncodedDomainPayload {
            schema_id: SchemaId::parse("eitmad.schema.customer.v1").unwrap(),
            schema_version: 1,
            base64: base64::engine::general_purpose::STANDARD
                .encode(serde_json::to_vec(&payload).unwrap()),
        }),
        merge: None,
    }
}

fn exchange(transport: &mut WanAdapter<DirectServerDriver>, message: SyncMessage) -> SyncMessage {
    let mut request = frame(transport.negotiated_session().unwrap().protocol);
    request.payload = SyncTransportPayload::Message(message);
    transport
        .send(&request, eitmad_control_plane::unix_millis_now())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            Instant::now() < deadline,
            "customer response deadline elapsed"
        );
        match transport.receive(eitmad_control_plane::unix_millis_now()) {
            Ok(ReceiveOutcome::NoFrame) => {}
            Ok(ReceiveOutcome::Frame(reply)) => {
                let SyncTransportPayload::Message(message) = reply.payload else {
                    panic!("expected sync message")
                };
                return message;
            }
            other => panic!("unexpected customer response: {other:?}"),
        }
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

fn sync_and_acknowledge(
    mut transport: WanAdapter<DirectServerDriver>,
) -> WanAdapter<DirectServerDriver> {
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
            Ok(ReceiveOutcome::NoFrame) => {}
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
            Ok(ReceiveOutcome::NoFrame) => {}
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
}

struct ProvisionedServer {
    branches: BranchService,
    state: ServerState,
    address: SocketAddr,
    handle: axum_server::Handle<SocketAddr>,
    authentication: AuthenticationResult,
    device_id: DeviceId,
    second_authentication: AuthenticationResult,
    second_device_id: DeviceId,
    scope: ScopeRef,
    branch_scope: ScopeRef,
}

async fn provision_server(
    database_url: &str,
    certificate: &Path,
    private_key: &Path,
) -> ProvisionedServer {
    let control_database = ControlDatabase::connect(database_url, 4).await.unwrap();
    control_database.migrate().await.unwrap();
    let sync_database = SyncDatabase::connect(database_url, 4).await.unwrap();
    sync_database.migrate().await.unwrap();
    AdminDatabase::connect(database_url, 4)
        .await
        .unwrap()
        .migrate()
        .await
        .unwrap();
    AuditDatabase::from_pool(control_database.pool())
        .migrate()
        .await
        .unwrap();
    let control = ControlPlane::new(control_database.pool(), TokenKey::new([9; 32]));
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
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
    let device_id = DeviceId::new(Uuid::new_v4());
    let signing = SigningKey::from_bytes(&[11; 32]);
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
    let second_device_id = DeviceId::new(Uuid::new_v4());
    let second_signing = SigningKey::from_bytes(&[12; 32]);
    let second_authentication = control
        .authentication
        .login(
            &LoginRequest {
                tenant_code: TenantCode::parse("direct-test").unwrap(),
                username: "owner".to_owned(),
                password: "synthetic-test-password-123".to_owned(),
                device_id: second_device_id,
                device_label: "اختبار العميل الثاني".to_owned(),
                device_public_key: DevicePublicKey {
                    algorithm: "ed25519".to_owned(),
                    base64: URL_SAFE_NO_PAD.encode(second_signing.verifying_key().as_bytes()),
                },
                device_proof: None,
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
    let scope = ScopeRef {
        kind: ScopeKind::parse("organization").unwrap(),
        id: ScopeId::new(bootstrap.organization_id.value()),
    };
    let branch_scope = ScopeRef {
        kind: ScopeKind::parse("branch").unwrap(),
        id: ScopeId::new(Uuid::new_v4()),
    };
    control
        .branches
        .register(
            &authentication.session,
            &RegisterBranchRequest {
                organization_id: bootstrap.organization_id,
                branch_id: branch_scope.id,
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
    let mut transaction = control_database.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(authentication.session.tenant_id.value().to_string())
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples
        (tenant_id, subject_principal_id, subject_kind, relation, object_kind, object_id, created_at)
        VALUES ($1, $2, 'user', 'eitmad.relation.organization.manager.v1', 'organization', $3, $4)")
        .bind(authentication.session.tenant_id.value())
        .bind(authentication.session.user_id.value())
        .bind(scope.id.value())
        .bind(eitmad_control_plane::unix_millis_now().0)
        .execute(&mut *transaction).await.unwrap();
    transaction.commit().await.unwrap();
    let registry = DomainRegistry::new([
        Arc::new(TestDomain) as Arc<dyn DomainSyncHandler>,
        Arc::new(CustomerSyncHandler::new(sync_database.pool())) as Arc<dyn DomainSyncHandler>,
    ])
    .unwrap();
    let state = ServerState::new(control, SyncCoordinator::new(&sync_database, registry));
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let handle = start_server(address, state.clone(), certificate, private_key).await;
    ProvisionedServer {
        branches: BranchService::new(control_database.pool()),
        state,
        address,
        handle,
        authentication,
        device_id,
        second_authentication,
        second_device_id,
        scope,
        branch_scope,
    }
}

struct RejectedConnectionInputs<'a> {
    endpoint: &'a str,
    scope: &'a ScopeRef,
    trusted_certificate: &'a Path,
    wrong_certificate: &'a Path,
    store: &'a SecretStore,
    invalid_id: &'a SecretId,
    credential_id: &'a SecretId,
    account_id: AccountId,
    device_id: DeviceId,
}

async fn assert_rejected_connections(inputs: RejectedConnectionInputs<'_>) {
    let config = DirectServerConfig::new(
        inputs.endpoint,
        inputs.scope.clone(),
        schema_id(),
        1,
        inputs.trusted_certificate,
    )
    .unwrap();
    let wan_endpoint = config.wan_endpoint();
    let auth = TransportAuthentication::AccountDevice {
        account_id: inputs.account_id,
        device_id: inputs.device_id,
        credential: inputs.invalid_id.clone(),
    };
    let bad_driver = DirectServerDriver::new(config, inputs.store.clone(), hello());
    let mut bad_transport = WanAdapter::new(
        wan_endpoint,
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

    let wrong_config = DirectServerConfig::new(
        inputs.endpoint,
        inputs.scope.clone(),
        schema_id(),
        1,
        inputs.wrong_certificate,
    )
    .unwrap();
    let wrong_endpoint = wrong_config.wan_endpoint();
    let wrong_driver = DirectServerDriver::new(wrong_config, inputs.store.clone(), hello());
    let mut wrong_transport = WanAdapter::new(
        wrong_endpoint,
        wrong_driver,
        hello(),
        TransportAuthentication::AccountDevice {
            account_id: inputs.account_id,
            device_id: inputs.device_id,
            credential: inputs.credential_id.clone(),
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
}

async fn assert_reconnect_after_shutdown(
    transport: WanAdapter<DirectServerDriver>,
    handle: axum_server::Handle<SocketAddr>,
    state: ServerState,
    address: SocketAddr,
    certificate: &Path,
    private_key: &Path,
) {
    handle.graceful_shutdown(Some(Duration::from_secs(1)));
    tokio::time::sleep(Duration::from_millis(100)).await;
    let (mut transport, failure) = tokio::task::spawn_blocking(move || {
        let mut transport = transport;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(Instant::now() < deadline, "transport loss deadline elapsed");
            match transport.receive(eitmad_control_plane::unix_millis_now()) {
                Ok(ReceiveOutcome::NoFrame) => {}
                Err(failure) => break (transport, failure),
                other => panic!("unexpected frame after shutdown: {other:?}"),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(failure.kind, TransportFailureKind::ServerUnavailable);
    assert_eq!(transport.health().status, HealthStatus::Offline);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _second_handle = start_server(address, state, certificate, private_key).await;
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
                Ok(ReceiveOutcome::NoFrame) => {}
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
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires a disposable PostgreSQL database and a generated trusted development certificate"]
async fn real_server_authentication_tls_sync_and_reconnect() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database_url = env::var("EITMAD_DIRECT_TEST_DATABASE_URL")
        .expect("set EITMAD_DIRECT_TEST_DATABASE_URL to an empty disposable PostgreSQL database");
    let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
    let private_key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
    let trusted_certificate = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
    let wrong_certificate = required_path("EITMAD_DIRECT_TEST_WRONG_CERTIFICATE");
    let ProvisionedServer {
        state,
        address,
        handle,
        authentication,
        device_id,
        scope,
        ..
    } = provision_server(&database_url, &certificate, &private_key).await;
    let signing_seed = [11; 32];
    let account_id = authentication.session.account_id;
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
    assert_rejected_connections(RejectedConnectionInputs {
        endpoint: &endpoint,
        scope: &scope,
        trusted_certificate: &trusted_certificate,
        wrong_certificate: &wrong_certificate,
        store: &store,
        invalid_id: &invalid_id,
        credential_id: &credential_id,
        account_id,
        device_id,
    })
    .await;
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
    let wan_endpoint = config.wan_endpoint();
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
    transport = tokio::task::spawn_blocking(move || sync_and_acknowledge(transport))
        .await
        .unwrap();
    assert_ne!(
        old_material.expose_secret(),
        store.get(&credential_id).unwrap().unwrap().expose_secret(),
        "refresh must replace the stored token pair"
    );
    assert_reconnect_after_shutdown(
        transport,
        handle,
        state,
        address,
        &certificate,
        &private_key,
    )
    .await;
    store.delete(&credential_id).unwrap();
    store.delete(&invalid_id).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires an empty disposable PostgreSQL database and generated development certificates"]
async fn customer_changes_use_the_real_route_and_postgres_scope() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database_url = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
    let private_key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
    let trusted_certificate = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
    let provisioned = provision_server(&database_url, &certificate, &private_key).await;
    let branch = provisioned.branch_scope.clone();
    let tenant_id = provisioned.authentication.session.tenant_id;
    let account_id = provisioned.authentication.session.account_id;
    let workspace = tempfile::tempdir().unwrap();
    let secrets =
        SecretStore::open(workspace.path(), Some(FallbackEncryptionKey::new([7; 32]))).unwrap();
    let credential_id = SecretId::new(
        SecretKind::parse("customer-test-session").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_session(
        &secrets,
        &credential_id,
        provisioned.authentication,
        [11; 32],
    )
    .unwrap();
    let endpoint = format!("https://localhost:{}/", provisioned.address.port());
    let config = DirectServerConfig::new(
        &endpoint,
        branch.clone(),
        SchemaId::parse("eitmad.schema.customer.v1").unwrap(),
        1,
        &trusted_certificate,
    )
    .unwrap();
    let driver = DirectServerDriver::new(config, secrets, customer_hello());
    let mut transport = WanAdapter::new(
        eitmad_sync::WanEndpoint {
            server: endpoint,
            relay: None,
        },
        driver,
        customer_hello(),
        TransportAuthentication::AccountDevice {
            account_id,
            device_id: provisioned.device_id,
            credential: credential_id,
        },
        RetryPolicy::default(),
    )
    .unwrap();
    tokio::task::spawn_blocking(move || {
        transport
            .connect(eitmad_control_plane::unix_millis_now())
            .unwrap();
        let customer_id = CustomerId::new(Uuid::new_v4());
        let initial = customer_change(&branch, customer_id, "عميل اختباري", None);
        let first = exchange(
            &mut transport,
            SyncMessage::SubmitLocal(LocalChangeSubmission {
                change: initial.clone(),
            }),
        );
        assert!(matches!(first, SyncMessage::LocalResult(result)
            if result.submitted_change_id == initial.change_id
            && matches!(result.disposition, LocalChangeDisposition::Applied { .. })));
        let replay = exchange(
            &mut transport,
            SyncMessage::SubmitLocal(LocalChangeSubmission {
                change: initial.clone(),
            }),
        );
        assert!(matches!(replay, SyncMessage::LocalResult(result)
            if matches!(result.disposition, LocalChangeDisposition::Replayed { .. })));
        let edit = customer_change(&branch, customer_id, "اسم الجهاز الأول", Some(1));
        let applied = exchange(
            &mut transport,
            SyncMessage::SubmitLocal(LocalChangeSubmission { change: edit }),
        );
        assert!(matches!(applied, SyncMessage::LocalResult(result)
            if matches!(result.disposition, LocalChangeDisposition::Applied { .. })));
        let stale = customer_change(&branch, customer_id, "اسم الجهاز الثاني", Some(1));
        let conflict = exchange(
            &mut transport,
            SyncMessage::SubmitLocal(LocalChangeSubmission { change: stale }),
        );
        assert!(matches!(conflict, SyncMessage::LocalResult(result)
            if matches!(result.disposition, LocalChangeDisposition::Conflicted { .. })));
        let pulled = exchange(
            &mut transport,
            SyncMessage::Pull(PullRequest {
                after: None,
                maximum_records: 10,
            }),
        );
        let SyncMessage::Changes(batch) = pulled else {
            panic!("expected incremental customer history")
        };
        assert_eq!(batch.records.len(), 2);
        assert_eq!(batch.records[1].revision, 2);
        transport.disconnect(eitmad_control_plane::unix_millis_now());
    })
    .await
    .unwrap();
    let database = SyncDatabase::connect(&database_url, 2).await.unwrap();
    let mut other_tenant = database.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(Uuid::new_v4().to_string())
        .execute(&mut *other_tenant)
        .await
        .unwrap();
    let invisible: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sync.records WHERE tenant_id = $1")
            .bind(tenant_id.value())
            .fetch_one(&mut *other_tenant)
            .await
            .unwrap();
    assert_eq!(invisible, 0);
    provisioned
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
}

struct CustomerTestClient {
    _directory: tempfile::TempDir,
    store: AuthorityStore,
    customers: CustomerService,
    actor: AuthorizationContext,
    request: AuthorizationRequest,
    audit: BoundaryAuditContext,
    engine: SyncEngine,
    transport: WanAdapter<DirectServerDriver>,
}

impl CustomerTestClient {
    fn new(
        authentication: AuthenticationResult,
        device_id: DeviceId,
        signing_seed: [u8; 32],
        branch: &ScopeRef,
        endpoint: &str,
        certificate: &Path,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let actor = AuthorizationContext {
            session_id: SessionId::new(Uuid::new_v4()),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(authentication.session.user_id.value()),
                principal_kind: PrincipalKind::User,
                device_id: Some(device_id),
                service_id: None,
            },
            tenant_id: authentication.session.tenant_id,
            workspace_id: None,
            scope: branch.clone(),
        };
        let authorization = AuthorizationService::new(store.clone());
        let mutation = |version: u32| MutationContext {
            authorization: actor.clone(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            occurred_at: UnixMillis(i64::from(version)),
        };
        let subject = RelationshipSubject {
            principal_id: actor.identity.principal_id,
            principal_kind: actor.identity.principal_kind,
        };
        authorization
            .bootstrap_owner(&mutation(1), &subject)
            .unwrap();
        authorization
            .grant_relationship(
                &mutation(2),
                &GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject,
                    relation: RelationId::parse(MANAGER_RELATION).unwrap(),
                },
            )
            .unwrap();
        let customers = CustomerService::new(store.clone(), authorization);
        let request = AuthorizationRequest {
            action: ActionId::parse("eitmad.action.sync.write.v1").unwrap(),
            object: ScopedObject {
                tenant_id: actor.tenant_id,
                workspace_id: None,
                kind: ObjectKind::parse("branch").unwrap(),
                id: ObjectId::new(branch.id.value()),
            },
            attributes: BTreeMap::new(),
        };
        let audit = BoundaryAuditContext {
            kind: BoundaryKind::Sync,
            operation: "eitmad.customer.sync-cycle.v1".to_owned(),
            target: AuditTarget {
                kind: "branch".to_owned(),
                identifiers: vec![branch.id.value().to_string()],
            },
            occurred_at: eitmad_control_plane::unix_millis_now(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: None,
            extension_points: Vec::new(),
        };
        let engine = SyncEngine::open(
            store.clone(),
            branch.clone(),
            SyncMode::LocalFirst,
            customer_sync_authorization(&store, &actor, &request),
            &actor,
            &audit,
        )
        .unwrap();
        let secrets = SecretStore::open(
            directory.path().join("secrets"),
            Some(FallbackEncryptionKey::new([7; 32])),
        )
        .unwrap();
        let credential = SecretId::new(
            SecretKind::parse("customer-test-session").unwrap(),
            SecretReferenceId::new(Uuid::new_v4()),
        );
        let account_id = authentication.session.account_id;
        store_session(&secrets, &credential, authentication, signing_seed).unwrap();
        let config = DirectServerConfig::new(
            endpoint,
            branch.clone(),
            SchemaId::parse("eitmad.schema.customer.v1").unwrap(),
            1,
            certificate,
        )
        .unwrap();
        let wan_endpoint = config.wan_endpoint();
        let transport = WanAdapter::new(
            wan_endpoint,
            DirectServerDriver::new(config, secrets, customer_hello()),
            customer_hello(),
            TransportAuthentication::AccountDevice {
                account_id,
                device_id,
                credential,
            },
            RetryPolicy::default(),
        )
        .unwrap();
        Self {
            _directory: directory,
            store,
            customers,
            actor,
            request,
            audit,
            engine,
            transport,
        }
    }

    fn run(&mut self) {
        CustomerSyncCycle {
            customers: &self.customers,
            engine: &mut self.engine,
            transport: &mut self.transport,
            actor: &self.actor,
            request: &self.request,
            audit: &self.audit,
        }
        .run()
        .unwrap();
    }

    fn restart_engine(&mut self) {
        self.engine = SyncEngine::open(
            self.store.clone(),
            self.actor.scope.clone(),
            SyncMode::LocalFirst,
            customer_sync_authorization(&self.store, &self.actor, &self.request),
            &self.actor,
            &self.audit,
        )
        .unwrap();
    }

    fn mutation(&self) -> MutationContext {
        MutationContext {
            authorization: self.actor.clone(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            occurred_at: eitmad_control_plane::unix_millis_now(),
        }
    }
}

fn customer_sync_authorization(
    store: &AuthorityStore,
    actor: &AuthorizationContext,
    request: &AuthorizationRequest,
) -> SyncAuthorization {
    let relation = RelationId::parse(MANAGER_RELATION).unwrap();
    let policy = RelationshipPolicy::new(
        vec![RelationshipTuple {
            subject: TupleSubject::Principal(RelationshipSubject {
                principal_id: actor.identity.principal_id,
                principal_kind: actor.identity.principal_kind,
            }),
            relation: relation.clone(),
            object: request.object.clone(),
            condition: None,
        }],
        vec![PermissionRule {
            action: request.action.clone(),
            object_kind: ObjectKind::parse("branch").unwrap(),
            relations: vec![relation],
            inherits_via: Vec::new(),
        }],
    )
    .unwrap();
    SyncAuthorization::new(AuthorizationGate::new(policy, store.clone()))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires an empty disposable PostgreSQL database and development certificates"]
async fn two_isolated_customer_engines_recover_and_preserve_conflicts() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database_url = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
    let private_key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
    let trusted_certificate = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
    let provisioned = provision_server(&database_url, &certificate, &private_key).await;
    let branch = provisioned.branch_scope.clone();
    let tenant_id = provisioned.authentication.session.tenant_id;
    let manager_session = provisioned.authentication.session.clone();
    let endpoint = format!("https://localhost:{}/", provisioned.address.port());
    let mut first = CustomerTestClient::new(
        provisioned.authentication,
        provisioned.device_id,
        [11; 32],
        &branch,
        &endpoint,
        &trusted_certificate,
    );
    let mut second = CustomerTestClient::new(
        provisioned.second_authentication,
        provisioned.second_device_id,
        [12; 32],
        &branch,
        &endpoint,
        &trusted_certificate,
    );
    assert_ne!(first._directory.path(), second._directory.path());
    second = tokio::task::spawn_blocking(move || {
        second.run();
        second
    })
    .await
    .unwrap();
    let created = first
        .customers
        .create(
            &first.mutation(),
            &CreateCustomer {
                name: CustomerName::parse("عميل مشترك").unwrap(),
                phone: CustomerPhone::parse("+967777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap();
    assert_eq!(created.customer.sync_state, CustomerSyncState::Pending);
    let first_delivery = first.customers.sync_batch(&branch, 1).unwrap().remove(0);
    tokio::task::spawn_blocking(move || {
        first.restart_engine();
        first.run();
        let replay = exchange(
            &mut first.transport,
            SyncMessage::SubmitLocal(LocalChangeSubmission {
                change: first_delivery.clone(),
            }),
        );
        assert!(matches!(replay, SyncMessage::LocalResult(result)
            if result.submitted_change_id == first_delivery.change_id
            && matches!(result.disposition, LocalChangeDisposition::Replayed { .. })));
        let confirmed = first
            .customers
            .get(
                &first.actor,
                &GetCustomer {
                    customer_id: created.customer.id,
                },
            )
            .unwrap();
        assert_eq!(confirmed.sync_state, CustomerSyncState::Confirmed);
        second.run();
        let downloaded = second
            .customers
            .get(
                &second.actor,
                &GetCustomer {
                    customer_id: created.customer.id,
                },
            )
            .unwrap();
        assert_eq!(downloaded.name, created.customer.name);
        assert_eq!(downloaded.sync_state, CustomerSyncState::Confirmed);

        let offline = second
            .customers
            .update(
                &second.mutation(),
                &UpdateCustomer {
                    customer_id: created.customer.id,
                    expected_revision: 1,
                    name: CustomerName::parse("تعديل العميل الثاني").unwrap(),
                    phone: downloaded.phone.clone(),
                    address: None,
                    notes: None,
                },
            )
            .unwrap();
        second.restart_engine();
        let first_edit = first
            .customers
            .update(
                &first.mutation(),
                &UpdateCustomer {
                    customer_id: created.customer.id,
                    expected_revision: 1,
                    name: CustomerName::parse("تعديل العميل الأول").unwrap(),
                    phone: downloaded.phone,
                    address: None,
                    notes: None,
                },
            )
            .unwrap();
        first.run();
        second.run();
        let preserved = second
            .customers
            .get(
                &second.actor,
                &GetCustomer {
                    customer_id: created.customer.id,
                },
            )
            .unwrap();
        assert_eq!(preserved.name, offline.customer.name);
        assert_ne!(preserved.name, first_edit.customer.name);
        assert_eq!(preserved.sync_state, CustomerSyncState::Conflicted);
        assert_eq!(second.engine.conflicts().len(), 1);
        (first, second)
    })
    .await
    .unwrap();

    let database = SyncDatabase::connect(&database_url, 2).await.unwrap();
    let mut transaction = database.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(tenant_id.value().to_string())
        .execute(&mut *transaction)
        .await
        .unwrap();
    let effects: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sync.operations WHERE tenant_id = $1 AND schema_id = 'eitmad.schema.customer.v1'",
    ).bind(tenant_id.value()).fetch_one(&mut *transaction).await.unwrap();
    assert_eq!(effects, 2);
    let other_branch = ScopeRef {
        kind: ScopeKind::parse("branch").unwrap(),
        id: ScopeId::new(Uuid::new_v4()),
    };
    transaction.commit().await.unwrap();
    provisioned
        .branches
        .register(
            &manager_session,
            &RegisterBranchRequest {
                organization_id: eitmad_contracts::identity::OrganizationId::new(
                    provisioned.scope.id.value(),
                ),
                branch_id: other_branch.id,
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
    let handler = CustomerSyncHandler::new(database.pool());
    assert!(
        handler
            .authorize(&manager_session, &other_branch, SyncIntent::Read)
            .await
    );
    assert!(
        !handler
            .authorize(
                &manager_session,
                &ScopeRef {
                    kind: ScopeKind::parse("branch").unwrap(),
                    id: ScopeId::new(Uuid::new_v4()),
                },
                SyncIntent::Read
            )
            .await
    );
    let mut other_tenant = database.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(Uuid::new_v4().to_string())
        .execute(&mut *other_tenant)
        .await
        .unwrap();
    let invisible: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sync.operations WHERE tenant_id = $1")
            .bind(tenant_id.value())
            .fetch_one(&mut *other_tenant)
            .await
            .unwrap();
    assert_eq!(invisible, 0);
    let invisible_branches: i64 =
        sqlx::query_scalar("SELECT count(*) FROM control.branches WHERE tenant_id = $1")
            .bind(tenant_id.value())
            .fetch_one(&mut *other_tenant)
            .await
            .unwrap();
    assert_eq!(invisible_branches, 0);
    provisioned
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
}
