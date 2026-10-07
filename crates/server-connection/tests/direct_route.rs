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
        Customer, CustomerId, CustomerName, CustomerPhone, CustomerSyncPayload, CustomerSyncState,
        GetCustomer,
    },
    identity::{
        AccountId, AuthenticatedIdentity, AuthorizationContext, DeviceId, OrganizationId,
        PrincipalId, PrincipalKind, ScopeId, ScopeKind, ScopeRef, SessionId, TenantId,
    },
    secrets::{SecretId, SecretKind},
    server::{
        ActivateAccountRequest, AuthenticatedServerSession, AuthenticationResult, DevicePublicKey,
        LoginRequest, RegisterBranchRequest, TenantCode,
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
    CustomerSyncHandler, DomainDescriptor, DomainRegistry, DomainSyncHandler,
    DomainValidationError, LocalOperationDraft, SyncCoordinator, SyncDatabase, SyncIntent,
};
use uuid::Uuid;
#[path = "direct_route/catalog_sync.rs"]
mod catalog_sync;

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
    request.payload = SyncTransportPayload::Message(Box::new(message));
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
                return *message;
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
        payload: SyncTransportPayload::Message(Box::new(SyncMessage::Pull(PullRequest {
            after: None,
            maximum_records: 10,
        }))),
    }
}

struct LiveTestServer {
    handle: axum_server::Handle<SocketAddr>,
}

impl LiveTestServer {
    fn graceful_shutdown(&self, duration: Option<Duration>) {
        self.handle.graceful_shutdown(duration);
    }
}

impl Drop for LiveTestServer {
    fn drop(&mut self) {
        self.handle.shutdown();
    }
}

async fn start_server(
    address: SocketAddr,
    state: ServerState,
    certificate: &Path,
    private_key: &Path,
) -> LiveTestServer {
    let tls = RustlsConfig::from_pem_file(certificate, private_key)
        .await
        .unwrap();
    let handle = axum_server::Handle::new();
    let server_handle = handle.clone();
    tokio::task::spawn_blocking(move || {
        // Model a separate server process: upgraded sessions must end with its runtime.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            axum_server::bind_rustls(address, tls)
                .handle(server_handle)
                .serve(router(state).into_make_service())
                .await
                .unwrap();
        });
    });
    let server = LiveTestServer { handle };
    for _ in 0..50 {
        if std::net::TcpStream::connect_timeout(&address, Duration::from_millis(25)).is_ok() {
            return server;
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
                    reply.payload.as_message(),
                    Some(SyncMessage::Changes(_))
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
                let SyncTransportPayload::Message(message) = reply.payload else {
                    panic!("expected a real change batch");
                };
                let SyncMessage::Changes(batch) = *message else {
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
        SyncTransportPayload::Message(Box::new(SyncMessage::Acknowledge(acknowledgement.clone())));
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
                    SyncTransportPayload::Message(Box::new(SyncMessage::Acknowledge(
                        acknowledgement
                    )))
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
    handle: LiveTestServer,
    authentication: AuthenticationResult,
    device_id: DeviceId,
    second_authentication: AuthenticationResult,
    second_device_id: DeviceId,
    scope: ScopeRef,
    branch_scope: ScopeRef,
}

fn direct_test_domains(sync_database: &SyncDatabase) -> DomainRegistry {
    let mut handlers = eitmad_sync_plane::CatalogSyncHandler::handlers(&sync_database.pool());
    handlers.extend([
        Arc::new(TestDomain) as Arc<dyn DomainSyncHandler>,
        Arc::new(CustomerSyncHandler::new(sync_database.pool())) as Arc<dyn DomainSyncHandler>,
        Arc::new(eitmad_sync_plane::QuotationDraftSyncHandler::new(
            sync_database.pool(),
        )) as Arc<dyn DomainSyncHandler>,
    ]);
    handlers.push(Arc::new(eitmad_sync_plane::QuotationApprovalServer::new(
        sync_database.pool(),
    )));
    DomainRegistry::new(handlers).unwrap()
}
async fn provision_server(
    database_url: &str,
    certificate: &Path,
    private_key: &Path,
) -> ProvisionedServer {
    let (control_database, sync_database) = migrate_test_databases(database_url).await;
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
    register_test_branch(
        &control,
        &authentication,
        bootstrap.organization_id,
        branch_scope.id,
    )
    .await;
    grant_organization_manager(&control_database, &authentication, &scope).await;
    let registry = direct_test_domains(&sync_database);
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

async fn register_test_branch(
    control: &ControlPlane,
    authentication: &AuthenticationResult,
    organization_id: OrganizationId,
    branch_id: ScopeId,
) {
    control
        .branches
        .register(
            &authentication.session,
            &RegisterBranchRequest {
                organization_id,
                branch_id,
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now(),
        )
        .await
        .unwrap();
}

async fn migrate_test_databases(database_url: &str) -> (ControlDatabase, SyncDatabase) {
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
    (control_database, sync_database)
}

async fn grant_organization_manager(
    database: &ControlDatabase,
    authentication: &AuthenticationResult,
    scope: &ScopeRef,
) {
    let mut transaction = database.pool().begin().await.unwrap();
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
    handle: LiveTestServer,
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
                        reply.payload.as_message(),
                        Some(SyncMessage::Changes(_))
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

fn remove_stored_session_identity(store: &SecretStore, credential_id: &SecretId) {
    let material = store.get(credential_id).unwrap().unwrap();
    let mut legacy: serde_json::Value = serde_json::from_slice(material.expose_secret()).unwrap();
    legacy.as_object_mut().unwrap().remove("userId");
    legacy.as_object_mut().unwrap().remove("tenantId");
    store
        .set(
            credential_id,
            eitmad_secret_storage::SecretMaterial::new(serde_json::to_vec(&legacy).unwrap())
                .unwrap(),
        )
        .unwrap();
}

fn assert_stored_session_identity(store: &SecretStore, credential_id: &SecretId) {
    let material = store.get(credential_id).unwrap().unwrap();
    let refreshed: serde_json::Value = serde_json::from_slice(material.expose_secret()).unwrap();
    assert!(refreshed["userId"].is_string());
    assert!(refreshed["tenantId"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn legacy_credentials_refresh_identity_before_access_token_expiry() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
    let key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
    let trust = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
    let server = provision_server(&database, &certificate, &key).await;
    assert!(
        server.authentication.tokens.access_expires_at.0
            > eitmad_control_plane::unix_millis_now().0
    );
    let workspace = tempfile::tempdir().unwrap();
    let store =
        SecretStore::open(workspace.path(), Some(FallbackEncryptionKey::new([7; 32]))).unwrap();
    let credential_id = SecretId::new(
        SecretKind::parse("legacy-test-session").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_session(
        &store,
        &credential_id,
        server.authentication.clone(),
        [11; 32],
    )
    .unwrap();
    remove_stored_session_identity(&store, &credential_id);
    let endpoint = format!("https://localhost:{}/", server.address.port());
    let config = DirectServerConfig::new(&endpoint, server.scope, schema_id(), 1, &trust).unwrap();
    let wan_endpoint = config.wan_endpoint();
    let driver = DirectServerDriver::new(config, store.clone(), hello());
    let mut transport = WanAdapter::new(
        wan_endpoint,
        driver,
        hello(),
        TransportAuthentication::AccountDevice {
            account_id: server.authentication.session.account_id,
            device_id: server.device_id,
            credential: credential_id.clone(),
        },
        RetryPolicy::default(),
    )
    .unwrap();
    tokio::task::spawn_blocking(move || {
        transport
            .connect(eitmad_control_plane::unix_millis_now())
            .unwrap();
        transport.disconnect(eitmad_control_plane::unix_millis_now());
    })
    .await
    .unwrap();
    assert_stored_session_identity(&store, &credential_id);
    server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
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
    let transport = WanAdapter::new(
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
    tokio::task::spawn_blocking(move || assert_customer_route(transport, &branch))
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

fn assert_customer_route(mut transport: WanAdapter<DirectServerDriver>, branch: &ScopeRef) {
    transport
        .connect(eitmad_control_plane::unix_millis_now())
        .unwrap();
    let customer_id = CustomerId::new(Uuid::new_v4());
    let initial = customer_change(branch, customer_id, "عميل اختباري", None);
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
    let edit = customer_change(branch, customer_id, "اسم الجهاز الأول", Some(1));
    let applied = exchange(
        &mut transport,
        SyncMessage::SubmitLocal(LocalChangeSubmission { change: edit }),
    );
    assert!(matches!(applied, SyncMessage::LocalResult(result)
            if matches!(result.disposition, LocalChangeDisposition::Applied { .. })));
    let stale = customer_change(branch, customer_id, "اسم الجهاز الثاني", Some(1));
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
}

struct CustomerTestClient {
    directory: tempfile::TempDir,
    store: AuthorityStore,
    customers: CustomerService,
    actor: AuthorizationContext,
    server_scope: ScopeRef,
    request: AuthorizationRequest,
    audit: BoundaryAuditContext,
    engine: SyncEngine,
    transport: WanAdapter<DirectServerDriver>,
}

fn customer_test_transport(
    directory: &Path,
    authentication: AuthenticationResult,
    device_id: DeviceId,
    signing_seed: [u8; 32],
    branch: &ScopeRef,
    endpoint: &str,
    certificate: &Path,
) -> WanAdapter<DirectServerDriver> {
    let secrets = SecretStore::open(
        directory.join("secrets"),
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
    WanAdapter::new(
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
    .unwrap()
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
        let local_branch = ScopeRef {
            kind: ScopeKind::parse("branch").unwrap(),
            id: ScopeId::new(Uuid::new_v4()),
        };
        let actor = AuthorizationContext {
            session_id: SessionId::new(Uuid::new_v4()),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(authentication.session.user_id.value()),
                principal_kind: PrincipalKind::User,
                device_id: Some(device_id),
                service_id: None,
            },
            tenant_id: eitmad_contracts::identity::TenantId::new(Uuid::new_v4()),
            workspace_id: None,
            scope: local_branch,
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
                id: ObjectId::new(actor.scope.id.value()),
            },
            attributes: BTreeMap::new(),
        };
        let audit = BoundaryAuditContext {
            kind: BoundaryKind::Sync,
            operation: "eitmad.customer.sync-cycle.v1".to_owned(),
            target: AuditTarget {
                kind: "branch".to_owned(),
                identifiers: vec![actor.scope.id.value().to_string()],
            },
            occurred_at: eitmad_control_plane::unix_millis_now(),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: None,
            extension_points: Vec::new(),
        };
        let engine = SyncEngine::open(
            store.clone(),
            actor.scope.clone(),
            SyncMode::LocalFirst,
            customer_sync_authorization(&store, &actor, &request),
            &actor,
            &audit,
        )
        .unwrap();
        let transport = customer_test_transport(
            directory.path(),
            authentication,
            device_id,
            signing_seed,
            branch,
            endpoint,
            certificate,
        );
        Self {
            directory,
            store,
            customers,
            actor,
            server_scope: branch.clone(),
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
            server_scope: &self.server_scope,
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

fn assert_isolated_customer_cycle(
    mut first: CustomerTestClient,
    mut second: CustomerTestClient,
    customer: &Customer,
    first_delivery: &ChangeRecord,
) -> (CustomerTestClient, CustomerTestClient) {
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
                customer_id: customer.id,
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
                customer_id: customer.id,
            },
        )
        .unwrap();
    assert_eq!(downloaded.name, customer.name);
    assert_eq!(downloaded.sync_state, CustomerSyncState::Confirmed);

    let offline = second
        .customers
        .update(
            &second.mutation(),
            &UpdateCustomer {
                customer_id: customer.id,
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
                customer_id: customer.id,
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
                customer_id: customer.id,
            },
        )
        .unwrap();
    assert_eq!(preserved.name, offline.customer.name);
    assert_ne!(preserved.name, first_edit.customer.name);
    assert_eq!(preserved.sync_state, CustomerSyncState::Conflicted);
    let reopened = AuthorityStore::open(second.directory.path()).unwrap();
    let customers = CustomerService::new(reopened.clone(), AuthorizationService::new(reopened));
    let persisted = customers
        .get(
            &second.actor,
            &GetCustomer {
                customer_id: customer.id,
            },
        )
        .unwrap();
    assert_eq!(persisted.name, offline.customer.name);
    assert_eq!(persisted.sync_state, CustomerSyncState::Conflicted);
    assert!(
        customers
            .sync_batch(&second.actor.scope, 10)
            .unwrap()
            .is_empty()
    );
    (first, second)
}

async fn assert_server_customer_isolation(
    database_url: &str,
    tenant_id: TenantId,
    provisioned: &ProvisionedServer,
    manager_session: &AuthenticatedServerSession,
) {
    let database = SyncDatabase::connect(database_url, 2).await.unwrap();
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
            manager_session,
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
            .authorize(manager_session, &other_branch, SyncIntent::Read)
            .await
    );
    assert!(
        !handler
            .authorize(
                manager_session,
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
    let first = CustomerTestClient::new(
        provisioned.authentication.clone(),
        provisioned.device_id,
        [11; 32],
        &branch,
        &endpoint,
        &trusted_certificate,
    );
    let mut second = CustomerTestClient::new(
        provisioned.second_authentication.clone(),
        provisioned.second_device_id,
        [12; 32],
        &branch,
        &endpoint,
        &trusted_certificate,
    );
    assert_ne!(first.directory.path(), second.directory.path());
    assert_ne!(first.actor.tenant_id, second.actor.tenant_id);
    assert_ne!(first.actor.scope, second.actor.scope);
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
    let mut first_delivery = first
        .customers
        .sync_batch(&first.actor.scope, 1)
        .unwrap()
        .remove(0);
    first_delivery.scope = branch;
    tokio::task::spawn_blocking(move || {
        assert_isolated_customer_cycle(first, second, &created.customer, &first_delivery)
    })
    .await
    .unwrap();

    assert_server_customer_isolation(&database_url, tenant_id, &provisioned, &manager_session)
        .await;
    provisioned
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
}

fn catalog_image_client(
    directory: &Path,
    authentication: AuthenticationResult,
    seed: [u8; 32],
    remote_scope: &ScopeRef,
    endpoint: &str,
    certificate: &Path,
) -> eitmad_server_connection::DirectCatalogImageClient {
    let secrets = SecretStore::open(
        directory.join("secrets"),
        Some(FallbackEncryptionKey::new([7; 32])),
    )
    .unwrap();
    let credential = SecretId::new(
        SecretKind::parse("catalog-image-test").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_session(&secrets, &credential, authentication, seed).unwrap();
    let config = DirectServerConfig::new(
        endpoint,
        remote_scope.clone(),
        SchemaId::parse("eitmad.schema.catalog-image.v1").unwrap(),
        1,
        certificate,
    )
    .unwrap();
    eitmad_server_connection::DirectCatalogImageClient::from_config(config, secrets, credential)
}
fn catalog_local_authority(
    directory: &Path,
    session: &eitmad_contracts::server::AuthenticatedServerSession,
) -> (AuthorityStore, AuthorizationContext) {
    let store = AuthorityStore::open(directory).unwrap();
    let actor = AuthorizationContext {
        session_id: session.session_id,
        identity: AuthenticatedIdentity {
            principal_id: PrincipalId::new(session.user_id.value()),
            principal_kind: PrincipalKind::User,
            device_id: Some(session.device_id),
            service_id: None,
        },
        tenant_id: session.tenant_id,
        workspace_id: None,
        scope: ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(session.tenant_id.value()),
        },
    };
    let auth = AuthorizationService::new(store.clone());
    let mutation = MutationContext {
        authorization: actor.clone(),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        causation_id: None,
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        occurred_at: eitmad_control_plane::unix_millis_now(),
    };
    let subject = RelationshipSubject {
        principal_id: actor.identity.principal_id,
        principal_kind: PrincipalKind::User,
    };
    auth.bootstrap_owner(&mutation, &subject).unwrap();
    auth.grant_relationship(
        &MutationContext {
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            ..mutation
        },
        &GrantScopeRelationship {
            expected_policy_version: 1,
            subject,
            relation: RelationId::parse(MANAGER_RELATION).unwrap(),
        },
    )
    .unwrap();
    (store, actor)
}
async fn import_catalog_test_asset(
    directory: &Path,
    server: &ProvisionedServer,
    endpoint: &str,
    trust: &Path,
) -> (
    AuthorizationContext,
    eitmad_contracts::catalog_image::CatalogImageRef,
    Vec<u8>,
) {
    use eitmad_catalog_image::CatalogImageService;
    use eitmad_contracts::catalog_image::{CatalogImageKind, ImportCatalogImage};
    let (store, actor) = catalog_local_authority(directory, &server.authentication.session);
    let upload = Arc::new(catalog_image_client(
        directory,
        server.authentication.clone(),
        [11; 32],
        &server.scope,
        endpoint,
        trust,
    ));
    let source = directory.join("synthetic.png");
    // Enough synthetic entropy to exercise more than one bounded transfer chunk.
    let bitmap = image::RgbImage::from_fn(480, 320, |x, y| {
        image::Rgb([
            u8::try_from((x * 17 + y * 13) % 256).unwrap(),
            u8::try_from((x * 31 + y * 29) % 256).unwrap(),
            u8::try_from((x * y + 47) % 256).unwrap(),
        ])
    });
    bitmap
        .save_with_format(&source, image::ImageFormat::Png)
        .unwrap();
    let service = CatalogImageService::new(store.clone(), AuthorizationService::new(store.clone()))
        .with_transfer(upload.clone());
    let mutation = MutationContext {
        authorization: actor.clone(),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        causation_id: None,
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        occurred_at: eitmad_control_plane::unix_millis_now(),
    };
    let reference = service
        .import(
            &mutation,
            &ImportCatalogImage {
                kind: CatalogImageKind::Product,
                source_path: source.to_str().unwrap().into(),
            },
            eitmad_contracts::transport::UnixMillis(i64::MAX),
        )
        .unwrap();
    let expected = store
        .catalog_image(&actor.scope, &reference)
        .unwrap()
        .unwrap();
    assert!(expected.len() > 64 * 1024);
    std::fs::remove_file(source).unwrap();
    assert_eq!(
        tokio::task::spawn_blocking(move || service.retry_uploads())
            .await
            .unwrap(),
        Ok(1)
    );
    (actor, reference, expected)
}

async fn reject_invalid_catalog_server_upload(database: &str, server: &ProvisionedServer) {
    use base64::Engine as _;
    use eitmad_catalog_image::ImageError;
    use eitmad_contracts::catalog_image::{CatalogImageKind, UploadCatalogImage};
    let media = eitmad_sync_plane::CatalogImageServer::new(
        SyncDatabase::connect(database, 2).await.unwrap().pool(),
    );
    let malformed = b"synthetic malformed image";
    let request = UploadCatalogImage {
        scope: server.scope.clone(),
        reference: eitmad_catalog_image::reference(CatalogImageKind::Product, malformed),
        base64: base64::engine::general_purpose::STANDARD.encode(malformed),
    };
    assert_eq!(
        media
            .upload(
                &server.authentication.session,
                request,
                CorrelationId::new(Uuid::new_v4()),
                eitmad_control_plane::unix_millis_now()
            )
            .await,
        Err(ImageError::Invalid)
    );
}

async fn receptionist_catalog_image_access(
    database: &str,
    server: &ProvisionedServer,
    reference: &eitmad_contracts::catalog_image::CatalogImageRef,
    content: &[u8],
) {
    use base64::Engine as _;
    use eitmad_catalog_image::ImageError;
    use eitmad_contracts::{
        catalog_image::{
            CatalogImageKind, DownloadCatalogImage, GetCatalogImage, UploadCatalogImage,
        },
        identity::UserId,
    };
    let pool = SyncDatabase::connect(database, 2).await.unwrap().pool();
    let media = eitmad_sync_plane::CatalogImageServer::new(pool.clone());
    let mut receptionist = server.authentication.session.clone();
    receptionist.user_id = UserId::new(Uuid::new_v4());
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(receptionist.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at)
        VALUES($1,$2,'user','eitmad.relation.organization.receptionist.v1','organization',$3,1)")
        .bind(receptionist.tenant_id.value()).bind(receptionist.user_id.value()).bind(server.scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let query = DownloadCatalogImage {
        scope: server.scope.clone(),
        image: GetCatalogImage {
            reference: reference.clone(),
            offset: 0,
        },
    };
    assert!(media.download(&receptionist, &query).await.is_ok());
    assert_eq!(
        media
            .upload(
                &receptionist,
                UploadCatalogImage {
                    scope: server.scope.clone(),
                    reference: reference.clone(),
                    base64: base64::engine::general_purpose::STANDARD.encode(content)
                },
                CorrelationId::new(Uuid::new_v4()),
                eitmad_control_plane::unix_millis_now()
            )
            .await,
        Err(ImageError::Denied)
    );
    let mut furniture = query.clone();
    furniture.image.reference.kind = CatalogImageKind::Furniture;
    assert_eq!(
        media.download(&receptionist, &furniture).await,
        Err(ImageError::Denied)
    );
    receptionist.user_id = UserId::new(Uuid::new_v4());
    assert_eq!(
        media.download(&receptionist, &query).await,
        Err(ImageError::Denied)
    );
}

async fn concurrent_catalog_uploads_respect_retention_and_exact_retries(
    database: &str,
    server: &ProvisionedServer,
    reference: &eitmad_contracts::catalog_image::CatalogImageRef,
    content: &[u8],
) {
    use base64::Engine as _;
    use eitmad_catalog_image::{ImageError, normalize, reference as image_reference};
    use eitmad_contracts::catalog_image::{CatalogImageKind, UploadCatalogImage};
    let pool = SyncDatabase::connect(database, 2).await.unwrap().pool();
    let media = eitmad_sync_plane::CatalogImageServer::new(pool.clone());
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(server.authentication.session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    // Synthetic storage fixture leaves exactly one slot; application uploads still validate codecs.
    sqlx::query("INSERT INTO sync.catalog_images(tenant_id,organization_id,id,kind,sha256,content)
        SELECT $1,$2,md5('quota-fixture-' || n::text)::uuid,'product','synthetic',decode('00','hex') FROM generate_series(1,4094) n")
        .bind(server.authentication.session.tenant_id.value()).bind(server.scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let input = |width| {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(width, 1)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let bytes = normalize(&png.into_inner()).unwrap();
        UploadCatalogImage {
            scope: server.scope.clone(),
            reference: image_reference(CatalogImageKind::Product, &bytes),
            base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        }
    };
    let (first, second) = tokio::join!(
        media.upload(
            &server.authentication.session,
            input(1),
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now()
        ),
        media.upload(
            &server.authentication.session,
            input(2),
            CorrelationId::new(Uuid::new_v4()),
            eitmad_control_plane::unix_millis_now()
        ),
    );
    assert!(matches!(
        (&first, &second),
        (Ok(_), Err(ImageError::Invalid)) | (Err(ImageError::Invalid), Ok(_))
    ));
    let retry = UploadCatalogImage {
        scope: server.scope.clone(),
        reference: reference.clone(),
        base64: base64::engine::general_purpose::STANDARD.encode(content),
    };
    assert_eq!(
        media
            .upload(
                &server.authentication.session,
                retry,
                CorrelationId::new(Uuid::new_v4()),
                eitmad_control_plane::unix_millis_now()
            )
            .await,
        Ok(reference.clone())
    );
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(server.authentication.session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let retained: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sync.catalog_images WHERE tenant_id=$1 AND organization_id=$2",
    )
    .bind(server.authentication.session.tenant_id.value())
    .bind(server.scope.id.value())
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(retained, 4096);
    tx.commit().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn catalog_image_transfers_between_authorized_clients_and_survives_restart() {
    use eitmad_catalog_image::{CatalogImageService, CatalogImageTransfer, ImageError};
    use eitmad_contracts::catalog_image::GetCatalogImage;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
    let key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
    let trust = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
    let server = provision_server(&database, &certificate, &key).await;
    reject_invalid_catalog_server_upload(&database, &server).await;
    let endpoint = format!("https://localhost:{}/", server.address.port());
    let first = tempfile::TempDir::new().unwrap();
    let second = tempfile::TempDir::new().unwrap();
    let (actor, reference, expected) =
        import_catalog_test_asset(first.path(), &server, &endpoint, &trust).await;
    receptionist_catalog_image_access(&database, &server, &reference, &expected).await;
    server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let handle = start_server(server.address, server.state.clone(), &certificate, &key).await;
    let (second_store, second_actor) =
        catalog_local_authority(second.path(), &server.second_authentication.session);
    let download = Arc::new(catalog_image_client(
        second.path(),
        server.second_authentication.clone(),
        [12; 32],
        &server.scope,
        &endpoint,
        &trust,
    ));
    let reader = CatalogImageService::new(
        second_store.clone(),
        AuthorizationService::new(second_store.clone()),
    )
    .with_transfer(download);
    let query = GetCatalogImage {
        reference: reference.clone(),
        offset: 0,
    };
    let read_actor = second_actor.clone();
    let chunk = tokio::task::spawn_blocking(move || {
        reader.get(
            &read_actor,
            &query,
            eitmad_contracts::transport::UnixMillis(i64::MAX),
        )
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(chunk.total_bytes as usize, expected.len());
    assert_eq!(
        second_store
            .catalog_image(&second_actor.scope, &reference)
            .unwrap()
            .unwrap(),
        expected
    );
    assert_reopened_catalog_access(second.path(), &second_actor, &reference);
    let mut foreign = server.scope.clone();
    foreign.id = ScopeId::new(Uuid::new_v4());
    let foreign_client = catalog_image_client(
        first.path(),
        server.authentication.clone(),
        [11; 32],
        &foreign,
        &endpoint,
        &trust,
    );
    let rejected = reference.clone();
    let rejected_actor = actor.clone();
    assert_eq!(
        tokio::task::spawn_blocking(move || foreign_client.download(
            &rejected_actor,
            &rejected,
            eitmad_contracts::transport::UnixMillis(i64::MAX)
        ))
        .await
        .unwrap(),
        Err(ImageError::Denied)
    );
    concurrent_catalog_uploads_respect_retention_and_exact_retries(
        &database, &server, &reference, &expected,
    )
    .await;
    handle.graceful_shutdown(Some(Duration::from_secs(1)));
}

fn assert_reopened_catalog_access(
    directory: &Path,
    actor: &AuthorizationContext,
    reference: &eitmad_contracts::catalog_image::CatalogImageRef,
) {
    use eitmad_catalog_image::{CatalogImageService, ImageError};
    let reopened = AuthorityStore::open(directory).unwrap();
    let offline = CatalogImageService::new(reopened.clone(), AuthorizationService::new(reopened));
    let query = eitmad_contracts::catalog_image::GetCatalogImage {
        reference: reference.clone(),
        offset: 0,
    };
    assert!(offline.get(actor, &query, UnixMillis(i64::MAX)).is_ok());
    let mut denied = actor.clone();
    denied.identity.principal_id = PrincipalId::new(Uuid::new_v4());
    assert_eq!(
        offline.get(&denied, &query, UnixMillis(i64::MAX)),
        Err(ImageError::Denied)
    );
}

fn pricing_test_client(
    directory: &Path,
    server: &ProvisionedServer,
    endpoint: &str,
    trust: &Path,
) -> eitmad_server_connection::DirectPriceClient {
    let secrets = SecretStore::open(
        directory.join("secrets"),
        Some(FallbackEncryptionKey::new([7; 32])),
    )
    .unwrap();
    let credential = SecretId::new(
        SecretKind::parse("pricing-test").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_session(
        &secrets,
        &credential,
        server.authentication.clone(),
        [11; 32],
    )
    .unwrap();
    let config = DirectServerConfig::new(
        endpoint,
        server.scope.clone(),
        SchemaId::parse("eitmad.schema.pricing.v1").unwrap(),
        1,
        trust,
    )
    .unwrap();
    eitmad_server_connection::DirectPriceClient::from_config(config, secrets, credential)
}

/// Creates the catalog authority fixture separately from its price proposal.
fn pricing_catalog_fixture(
    input: &eitmad_contracts::pricing::ConfirmPrice,
) -> eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions {
    use eitmad_contracts::{
        catalog_revision::{CatalogRevision, SynchronizeCatalogRevisions},
        product::{Product, ProductCategory, ProductCategoryId, ProductVariant},
    };
    let eitmad_contracts::pricing::PriceTarget::Product(target) = &input.command.target else {
        panic!("product fixture")
    };
    let category = ProductCategory {
        id: ProductCategoryId::new(Uuid::from_u128(9_000_001)),
        scope: target.scope.clone(),
        name: "مراتب".into(),
        archived: false,
        revision: 1,
        updated_at: UnixMillis(1),
    };
    let product = Product {
        image: None,
        id: target.product_id,
        scope: target.scope.clone(),
        name: "مرتبة اختبار".into(),
        category_id: category.id,
        category_name: category.name.clone(),
        description: String::new(),
        notes: String::new(),
        variants: vec![ProductVariant {
            id: target.variant_id,
            name: "مفرد".into(),
            purchase_cost_yer: Some(90_000),
            archived: false,
        }],
        archived: false,
        revision: target.revision,
        updated_at: UnixMillis(1),
    };
    SynchronizeCatalogRevisions {
        scope: target.scope.clone(),
        records: vec![
            CatalogRevision::ProductCategory(Box::new(category)),
            CatalogRevision::Product(Box::new(product)),
        ],
    }
}

/// Exercises the real `PostgreSQL` cost authority, immutable replay, and missing-catalog rejection.
async fn check_server_catalog_cost_policy(
    database: &str,
    server: &ProvisionedServer,
    input: &eitmad_contracts::pricing::ConfirmPrice,
) {
    use eitmad_contracts::pricing::PriceTarget;
    use eitmad_pricing::PricingError;
    let pricing = eitmad_sync_plane::PricingServer::new(
        SyncDatabase::connect(database, 2).await.unwrap().pool(),
    );
    let mut remote = input.clone();
    if let PriceTarget::Product(r) = &mut remote.command.target {
        r.scope = server.scope.clone();
    }
    remote.cost_yer = 0;
    remote.command.selling_price_yer = 50_000;
    remote.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
    let actor = &server.authentication.session;
    let correlation = CorrelationId::new(Uuid::new_v4());
    assert_eq!(
        pricing
            .publish(actor, &remote, correlation, UnixMillis(1))
            .await,
        Err(PricingError::Reference)
    );
    let catalog = pricing_catalog_fixture(&remote);
    pricing
        .synchronize_catalog(actor, &catalog, correlation, UnixMillis(2))
        .await
        .unwrap();
    pricing
        .synchronize_catalog(actor, &catalog, correlation, UnixMillis(2))
        .await
        .unwrap();
    assert_eq!(
        pricing
            .publish(actor, &remote, correlation, UnixMillis(3))
            .await,
        Err(PricingError::BelowCost)
    );
    remote.command.confirm_below_cost = true;
    assert_eq!(
        pricing
            .publish(actor, &remote, correlation, UnixMillis(3))
            .await,
        Err(PricingError::Reference)
    );
    let mut altered = catalog;
    if let eitmad_contracts::catalog_revision::CatalogRevision::Product(p) = &mut altered.records[1]
    {
        p.variants[0].purchase_cost_yer = Some(0);
    }
    assert_eq!(
        pricing
            .synchronize_catalog(actor, &altered, correlation, UnixMillis(4))
            .await,
        Err(PricingError::Reference)
    );
}
/// Checks server authorization for Receptionist publication, catalog transfer, and scoped price reads.
async fn deny_receptionist_price_publication(
    database: &str,
    server: &ProvisionedServer,
    input: &eitmad_contracts::pricing::ConfirmPrice,
) {
    use eitmad_contracts::{identity::UserId, pricing::ReadPublishedPrices};
    use eitmad_pricing::PricingError;
    let pool = SyncDatabase::connect(database, 2).await.unwrap().pool();
    let pricing = eitmad_sync_plane::PricingServer::new(pool.clone());
    let mut receptionist = server.authentication.session.clone();
    receptionist.user_id = UserId::new(Uuid::new_v4());
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(receptionist.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.receptionist.v1','organization',$3,1)")
        .bind(receptionist.tenant_id.value()).bind(receptionist.user_id.value()).bind(server.scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        pricing
            .publish(
                &receptionist,
                input,
                CorrelationId::new(Uuid::new_v4()),
                UnixMillis(2000)
            )
            .await,
        Err(PricingError::Denied)
    );
    let query = ReadPublishedPrices {
        scope: server.scope.clone(),
        after: None,
        limit: 100,
    };
    assert_eq!(
        pricing.status(&receptionist, input).await,
        Err(PricingError::Denied)
    );
    let page = pricing.read(&receptionist, &query).await.unwrap();
    assert_eq!(
        pricing
            .synchronize_catalog(
                &receptionist,
                &pricing_catalog_fixture(input),
                CorrelationId::new(Uuid::new_v4()),
                UnixMillis(2000)
            )
            .await,
        Err(PricingError::Denied)
    );
    assert_eq!(page.items.len(), 1);
    let json = serde_json::to_string(&page).unwrap();
    assert!(!json.contains("cost") && !json.contains("margin") && !json.contains("90000"));
    let mut foreign = query;
    foreign.scope.id = ScopeId::new(Uuid::new_v4());
    assert_eq!(
        pricing.read(&receptionist, &foreign).await,
        Err(PricingError::Denied)
    );
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(receptionist.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        sqlx::query("DELETE FROM sync.price_revisions WHERE tenant_id=$1")
            .bind(receptionist.tenant_id.value())
            .execute(&mut *tx)
            .await
            .is_err()
    );
}
/// Exercises durable catalog costs, receipt recovery, revision conflicts, and denial over real TLS.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn pricing_tls_confirmation_persists_retries_conflicts_and_denies_receptionists() {
    use eitmad_contracts::{
        pricing::{ConfirmPrice, PriceTarget, PublishPrice},
        product::{ProductId, ProductReference, ProductVariantId},
    };
    use eitmad_pricing::{PriceConfirmation, PricingError};
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
    let key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
    let trust = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
    let server = provision_server(&database, &certificate, &key).await;
    let endpoint = format!("https://localhost:{}/", server.address.port());
    let local = tempfile::TempDir::new().unwrap();
    let (_, actor) = catalog_local_authority(local.path(), &server.authentication.session);
    let client = Arc::new(pricing_test_client(
        local.path(),
        &server,
        &endpoint,
        &trust,
    ));
    let input = ConfirmPrice {
        command: PublishPrice {
            target: PriceTarget::Product(ProductReference {
                scope: actor.scope.clone(),
                product_id: ProductId::new(Uuid::new_v4()),
                variant_id: ProductVariantId::new(Uuid::new_v4()),
                revision: 1,
                schema_version: 1,
            }),
            expected_revision: None,
            selling_price_yer: 100_000,
            confirm_below_cost: false,
        },
        cost_yer: 90000,
        colors: vec![],
        handles: vec![],
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
    };
    check_server_catalog_cost_policy(&database, &server, &input).await;
    let c = client.clone();
    let a = actor.clone();
    let catalog = pricing_catalog_fixture(&input);
    let request = input.clone();
    let first = tokio::task::spawn_blocking(move || {
        c.synchronize_catalog(&a, &catalog, UnixMillis(i64::MAX))?;
        c.confirm(&a, &request, UnixMillis(i64::MAX))
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(first.revision, 1);
    server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let handle = start_server(server.address, server.state.clone(), &certificate, &key).await;
    let c = client.clone();
    let a = actor.clone();
    let request = input.clone();
    assert_eq!(
        tokio::task::spawn_blocking(move || c.status(&a, &request, UnixMillis(i64::MAX)))
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        first
    );
    let mut update = input.clone();
    update.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
    update.command.expected_revision = Some(1);
    update.command.selling_price_yer = 110_000;
    let c = client.clone();
    let a = actor.clone();
    let request = update.clone();
    assert_eq!(
        tokio::task::spawn_blocking(move || c.confirm(&a, &request, UnixMillis(i64::MAX)))
            .await
            .unwrap()
            .unwrap()
            .revision,
        2
    );
    update.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
    let c = client.clone();
    let a = actor.clone();
    let request = update.clone();
    assert!(matches!(
        tokio::task::spawn_blocking(move || c.confirm(&a, &request, UnixMillis(i64::MAX)))
            .await
            .unwrap(),
        Err(PricingError::Conflict { .. })
    ));
    let mut remote = update;
    if let PriceTarget::Product(target) = &mut remote.command.target {
        target.scope = server.scope.clone();
    }
    deny_receptionist_price_publication(&database, &server, &remote).await;
    handle.graceful_shutdown(Some(Duration::from_secs(1)));
}
