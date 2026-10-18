//! Real `PostgreSQL`, authenticated HTTP/WebSocket, and Rust customer authority.
use super::*;
use eitmad_contracts::customer::{
    CustomerRead, CustomerSearchTerm, ReadCustomers, SearchCustomers,
};
use eitmad_customer::{CustomerDirectory as _, CustomerError};

fn search(after: Option<eitmad_contracts::customer::CustomerCursor>, limit: u32) -> CustomerRead {
    CustomerRead::Search(
        SearchCustomers::new(CustomerSearchTerm::parse("اِعتماد").unwrap(), after, limit).unwrap(),
    )
}

async fn submit(
    coordinator: &SyncCoordinator,
    actor: &AuthenticatedServerSession,
    change: &ChangeRecord,
) {
    coordinator
        .apply_local_operation(
            actor,
            &LocalOperationDraft {
                change_id: change.change_id,
                scope: change.scope.clone(),
                schema_id: SchemaId::parse(eitmad_customer::CUSTOMER_SCHEMA_ID).unwrap(),
                schema_version: 1,
                record_id: change.record_id,
                operation: change.operation,
                base_revision: change.base_revision,
                idempotency_key: change.idempotency_key,
                payload: change.payload.clone(),
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
}

async fn second_branch(server: &ProvisionedServer) -> ScopeRef {
    let branch = ScopeRef {
        kind: ScopeKind::parse("branch").unwrap(),
        id: ScopeId::new(Uuid::new_v4()),
    };
    server
        .branches
        .register(
            &server.authentication.session,
            &RegisterBranchRequest {
                organization_id: OrganizationId::new(server.scope.id.value()),
                branch_id: branch.id,
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    branch
}

fn direct_client(
    directory: &Path,
    server: &ProvisionedServer,
) -> eitmad_server_connection::DirectDiscountApprovalClient {
    let secrets = SecretStore::open(directory, Some(FallbackEncryptionKey::new([7; 32]))).unwrap();
    let credential = SecretId::new(
        SecretKind::parse("customer-directory-test").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_account_session(
        &secrets,
        &credential,
        server.authentication.clone(),
        [11; 32],
    )
    .unwrap();
    let config = DirectServerConfig::new(
        &format!("https://localhost:{}/", server.address.port()),
        server.scope.clone(),
        SchemaId::parse(eitmad_customer::CUSTOMER_SCHEMA_ID).unwrap(),
        1,
        &required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE"),
    )
    .unwrap();
    eitmad_server_connection::DirectDiscountApprovalClient::from_config(
        config,
        secrets,
        credential,
        server.branch_scope.clone(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn manager_customer_visibility_is_bounded_scoped_and_live() {
    let _credentials = TestCredentials::new();
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let server = provision_server(
        &database,
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    let db = SyncDatabase::connect(&database, 4).await.unwrap();
    let coordinator = SyncCoordinator::new(&db, direct_test_domains(&db));
    let branch = second_branch(&server).await;
    let customer = CustomerId::new(Uuid::from_u128(10));
    // An equal customer UUID in another branch must not be skipped by pagination.
    for scope in [&server.branch_scope, &branch] {
        submit(
            &coordinator,
            &server.authentication.session,
            &customer_change(scope, customer, "إعـتماد القيسي", None),
        )
        .await;
    }
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        coordinator
            .read_customers(
                &server.authentication.session,
                &ReadCustomers {
                    scope: server.scope.clone(),
                    query: search(None, 1)
                },
                CorrelationId::new(Uuid::new_v4()),
                eitmad_authorization::now()
            )
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    let (store, actor) = catalog_local_authority(directory.path(), &server.authentication.session);
    let client = Arc::new(direct_client(&directory.path().join("secrets"), &server));
    let service = CustomerService::new(store.clone(), AuthorizationService::new(store))
        .with_directory(client.clone());
    let read_actor = actor.clone();
    let copy = service.clone();
    let (first, second) = tokio::task::spawn_blocking(move || {
        let CustomerRead::Search(query) = search(None, 1) else {
            unreachable!()
        };
        let first = copy.search(&read_actor, &query).unwrap();
        let CustomerRead::Search(query) = search(first.next, 1) else {
            unreachable!()
        };
        let second = copy.search(&read_actor, &query).unwrap();
        assert_eq!(first.items.len(), 1);
        assert_eq!(second.items.len(), 1);
        assert!(second.next.is_none());
        assert_ne!(first.items[0].scope, second.items[0].scope);
        for value in [&first.items[0], &second.items[0]] {
            assert_eq!(
                copy.get(
                    &read_actor,
                    &GetCustomer {
                        customer_id: value.id,
                        branch_id: Some(value.scope.id)
                    }
                )
                .unwrap(),
                *value
            );
        }
        (first, second)
    })
    .await
    .unwrap();
    assert_eq!(first.items[0].id, second.items[0].id);
    branch_isolation(&db, &coordinator, &server, &branch).await;
    live_update(client, actor, &coordinator, &server, &branch, customer).await;
    sparse_pages(&db, &coordinator, &server).await;
    revoked_manager(&db, &coordinator, &server).await;
    server
        .handle
        .graceful_shutdown(Some(Duration::from_millis(100)));
}

async fn revoked_manager(
    db: &SyncDatabase,
    coordinator: &SyncCoordinator,
    server: &ProvisionedServer,
) {
    use eitmad_sync_plane::{CustomerSyncHandler, DomainSyncHandler as _, SyncIntent};
    let handler = CustomerSyncHandler::new(db.pool());
    assert!(
        !handler
            .authorize(
                &server.authentication.session,
                &server.scope,
                SyncIntent::Write
            )
            .await
    );
    assert!(
        !handler
            .authorize(
                &server.authentication.session,
                &server.scope,
                SyncIntent::ResolveConflict
            )
            .await
    );
    let mut foreign = server.scope.clone();
    foreign.id = ScopeId::new(Uuid::new_v4());
    assert!(
        !handler
            .authorize(&server.authentication.session, &foreign, SyncIntent::Read)
            .await
    );
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(server.authentication.session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM control.relationship_tuples WHERE tenant_id=$1 AND subject_principal_id=$2 AND relation='eitmad.relation.organization.manager.v1' AND object_kind='organization' AND object_id=$3")
        .bind(server.authentication.session.tenant_id.value()).bind(server.authentication.session.user_id.value()).bind(server.scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        coordinator
            .read_customers(
                &server.authentication.session,
                &ReadCustomers {
                    scope: server.scope.clone(),
                    query: search(None, 100)
                },
                CorrelationId::new(Uuid::new_v4()),
                eitmad_authorization::now()
            )
            .await,
        Err(CustomerError::Denied)
    );
    assert!(
        !handler
            .authorize(
                &server.authentication.session,
                &server.scope,
                SyncIntent::Read
            )
            .await
    );
}

/// A page can contain no matches while its candidate budget requires continuation.
async fn sparse_pages(
    db: &SyncDatabase,
    coordinator: &SyncCoordinator,
    server: &ProvisionedServer,
) {
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(server.authentication.session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let changes: Vec<_> = (100..1101)
        .map(|id| {
            customer_change(
                &server.branch_scope,
                CustomerId::new(Uuid::from_u128(id)),
                "عميل آخر",
                None,
            )
        })
        .collect();
    for change in &changes {
        sqlx::query("INSERT INTO sync.records(tenant_id,scope_kind,scope_id,schema_id,record_id,revision,tombstone,change_json,changed_at) VALUES($1,'branch',$2,$3,$4,1,false,$5,$6)")
            .bind(server.authentication.session.tenant_id.value()).bind(server.branch_scope.id.value())
            .bind(eitmad_customer::CUSTOMER_SCHEMA_ID).bind(change.record_id.value())
            .bind(serde_json::to_value(change).unwrap()).bind(change.changed_at.0).execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
    let query = |after| ReadCustomers {
        scope: server.scope.clone(),
        query: CustomerRead::Search(
            SearchCustomers::new(CustomerSearchTerm::parse("غير موجود").unwrap(), after, 100)
                .unwrap(),
        ),
    };
    let first = coordinator
        .read_customers(
            &server.authentication.session,
            &query(None),
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    assert!(first.items.is_empty());
    assert!(first.next.is_some());
    let last = coordinator
        .read_customers(
            &server.authentication.session,
            &query(first.next),
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    assert!(last.items.is_empty());
    assert!(last.next.is_none());
}

async fn branch_isolation(
    db: &SyncDatabase,
    coordinator: &SyncCoordinator,
    server: &ProvisionedServer,
    second: &ScopeRef,
) {
    let mut receptionist = server.authentication.session.clone();
    receptionist.user_id = eitmad_contracts::identity::UserId::new(Uuid::new_v4());
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(receptionist.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.receptionist.v1','branch',$3,1)")
        .bind(receptionist.tenant_id.value()).bind(receptionist.user_id.value()).bind(server.branch_scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let read = |scope: ScopeRef, query| ReadCustomers { scope, query };
    let page = coordinator
        .read_customers(
            &receptionist,
            &read(server.branch_scope.clone(), search(None, 100)),
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(page.next.is_none());
    for scope in [second.clone(), server.scope.clone()] {
        assert_eq!(
            coordinator
                .read_customers(
                    &receptionist,
                    &read(scope, search(None, 100)),
                    CorrelationId::new(Uuid::new_v4()),
                    eitmad_authorization::now()
                )
                .await,
            Err(CustomerError::Denied)
        );
    }
    assert_eq!(
        coordinator
            .read_customers(
                &receptionist,
                &read(
                    server.branch_scope.clone(),
                    CustomerRead::Get(GetCustomer {
                        customer_id: page.items[0].id,
                        branch_id: Some(second.id),
                    })
                ),
                CorrelationId::new(Uuid::new_v4()),
                eitmad_authorization::now()
            )
            .await
            .unwrap()
            .items
            .len(),
        0
    );
    let schema = SchemaId::parse(eitmad_customer::CUSTOMER_SCHEMA_ID).unwrap();
    let events = coordinator
        .subscription_page(eitmad_sync_plane::SubscriptionPageRequest {
            session: &receptionist,
            scope: &server.branch_scope,
            schema_id: &schema,
            schema_version: 1,
            resume_after: None,
            maximum_events: 100,
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            now: eitmad_authorization::now(),
        })
        .await
        .unwrap();
    assert!(
        events
            .events
            .iter()
            .all(|e| e.change.scope == server.branch_scope)
    );
    let mut foreign = server.authentication.session.clone();
    foreign.tenant_id = TenantId::new(Uuid::new_v4());
    assert_eq!(
        coordinator
            .read_customers(
                &foreign,
                &read(server.scope.clone(), search(None, 100)),
                CorrelationId::new(Uuid::new_v4()),
                eitmad_authorization::now()
            )
            .await,
        Err(CustomerError::Denied)
    );
}

async fn live_update(
    client: Arc<eitmad_server_connection::DirectDiscountApprovalClient>,
    actor: AuthorizationContext,
    coordinator: &SyncCoordinator,
    server: &ProvisionedServer,
    branch: &ScopeRef,
    customer: CustomerId,
) {
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stopping = cancel.clone();
    let (send, receive) = std::sync::mpsc::channel();
    let watch = std::thread::spawn(move || {
        client.watch(&actor, &stopping, &mut |notice| {
            let _ = send.send(notice);
        })
    });
    let receive = tokio::task::spawn_blocking(move || {
        // Drain both initial branch events so the edit occurs on an established stream.
        for _ in 0..2 {
            assert_eq!(
                receive
                    .recv_timeout(Duration::from_secs(15))
                    .unwrap()
                    .revision,
                1
            );
        }
        receive
    })
    .await
    .unwrap();
    submit(
        coordinator,
        &server.authentication.session,
        &customer_change(branch, customer, "إعتماد محدث", Some(1)),
    )
    .await;
    tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let notice = receive
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap();
            if notice.customer_id == customer && notice.revision == 2 {
                break;
            }
        }
        cancel.store(true, std::sync::atomic::Ordering::Release);
        watch.join().unwrap().unwrap();
    })
    .await
    .unwrap();
}
