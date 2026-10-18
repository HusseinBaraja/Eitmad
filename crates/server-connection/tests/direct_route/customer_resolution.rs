use super::*;
use eitmad_contracts::{
    commands::{Command, CommandResult},
    customer::*,
    queries::{Query, QueryResult},
};
use eitmad_customer::{CustomerDirectory, CustomerError};
use eitmad_engine_runtime::{
    ProductDispatcher,
    local_ipc::{CommandDispatcher, DispatchContext, EventBroker, QueryDispatcher},
};
use eitmad_server_connection::DirectDiscountApprovalClient;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
struct LostReply {
    inner: Arc<dyn CustomerDirectory>,
    lose: AtomicBool,
}
impl CustomerDirectory for LostReply {
    fn read(
        &self,
        actor: &AuthorizationContext,
        query: &CustomerRead,
    ) -> Result<CustomerPage, CustomerError> {
        self.inner.read(actor, query)
    }
    fn watch(
        &self,
        actor: &AuthorizationContext,
        cancel: &AtomicBool,
        notify: &mut dyn FnMut(CustomerChangeNotice),
    ) -> Result<(), CustomerError> {
        self.inner.watch(actor, cancel, notify)
    }
    fn conflicts(
        &self,
        actor: &AuthorizationContext,
        query: &GetCustomerConflicts,
    ) -> Result<CustomerConflictPage, CustomerError> {
        self.inner.conflicts(actor, query)
    }
    fn resolve(
        &self,
        actor: &AuthorizationContext,
        input: &ConfirmCustomerResolution,
    ) -> Result<ChangeRecord, CustomerError> {
        let result = self.inner.resolve(actor, input)?;
        if self.lose.swap(false, Ordering::SeqCst) {
            Err(CustomerError::Unavailable)
        } else {
            Ok(result)
        }
    }
}

fn context(actor: &AuthorizationContext, key: IdempotencyKey) -> DispatchContext {
    DispatchContext {
        authorization: actor.clone(),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        causation_id: None,
        idempotency_key: Some(key),
        protocol_version: eitmad_contracts::PROTOCOL_VERSION,
        deadline: UnixMillis(eitmad_authorization::now().0 + 30_000),
    }
}
fn get(client: &CustomerTestClient, id: CustomerId) -> Customer {
    client
        .customers
        .get(
            &client.actor,
            &GetCustomer {
                customer_id: id,
                branch_id: None,
            },
        )
        .unwrap()
}
fn edit(
    client: &CustomerTestClient,
    customer: &Customer,
    name: &str,
    notes: Option<&str>,
) -> Customer {
    client
        .customers
        .update(
            &client.mutation(),
            &UpdateCustomer {
                customer_id: customer.id,
                expected_revision: customer.revision,
                name: CustomerName::parse(name).unwrap(),
                phone: customer.phone.clone(),
                address: customer.address.clone(),
                notes: notes.map(|v| CustomerNotes::parse(v).unwrap()),
            },
        )
        .unwrap()
        .customer
}
fn competing(
    mut first: CustomerTestClient,
    mut second: CustomerTestClient,
) -> (CustomerTestClient, CustomerTestClient, CustomerId) {
    let created = first
        .customers
        .create(
            &first.mutation(),
            &CreateCustomer {
                name: CustomerName::parse("عميل تجريبي").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    first.run();
    second.run();
    edit(&first, &get(&first, created.id), "اختيار الجهاز الأول", None);
    let local = edit(
        &second,
        &get(&second, created.id),
        "اختيار الجهاز الثاني",
        None,
    );
    edit(
        &second,
        &local,
        "تعديل لاحق مستقل",
        Some("ملاحظة محفوظة للمراجعة"),
    );
    first.run();
    second.run();
    second.restart_engine();
    second.store = AuthorityStore::open(second.directory.path()).unwrap();
    second.customers = CustomerService::new(
        second.store.clone(),
        AuthorizationService::new(second.store.clone()),
    );
    assert_eq!(
        get(&second, created.id).sync_state,
        CustomerSyncState::Conflicted
    );
    assert!(
        second
            .customers
            .sync_batch_for_author(&second.actor)
            .unwrap()
            .is_empty()
    );
    (first, second, created.id)
}
fn directory(
    client: &CustomerTestClient,
    server: &ProvisionedServer,
) -> Arc<dyn CustomerDirectory> {
    let secrets = SecretStore::open(
        client.directory.path().join("resolution-secrets"),
        Some(FallbackEncryptionKey::new([7; 32])),
    )
    .unwrap();
    let credential = SecretId::new(
        SecretKind::parse("resolution-test").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_account_session(
        &secrets,
        &credential,
        server.second_authentication.clone(),
        [12; 32],
    )
    .unwrap();
    let config = DirectServerConfig::new(
        &format!("https://localhost:{}/", server.address.port()),
        server.scope.clone(),
        SchemaId::parse("eitmad.schema.customer.v1").unwrap(),
        1,
        &required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE"),
    )
    .unwrap();
    Arc::new(DirectDiscountApprovalClient::from_config(
        config,
        secrets,
        credential,
        server.branch_scope.clone(),
    ))
}
async fn page(
    dispatcher: &ProductDispatcher,
    actor: &AuthorizationContext,
    id: CustomerId,
) -> CustomerConflictPage {
    let QueryResult::CustomerConflicts(page) = dispatcher
        .dispatch_query(
            context(actor, IdempotencyKey::new(Uuid::new_v4())),
            Query::CustomerConflicts(GetCustomerConflicts {
                customer: GetCustomer {
                    customer_id: id,
                    branch_id: Some(actor.scope.id),
                },
                after: None,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("conflict page")
    };
    *page
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and development TLS certificates"]
async fn customer_resolution_converges_after_restart_and_lost_reply_without_replaying_dependents() {
    let _credentials = TestCredentials::new();
    let _ = rustls::crypto::ring::default_provider().install_default();
    let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let server = provision_server(
        &database,
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    let endpoint = format!("https://localhost:{}/", server.address.port());
    let trust = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
    let first = CustomerTestClient::new(
        server.authentication.clone(),
        server.device_id,
        [11; 32],
        &server.branch_scope,
        &endpoint,
        &trust,
    );
    let second = CustomerTestClient::new(
        server.second_authentication.clone(),
        server.second_device_id,
        [12; 32],
        &server.branch_scope,
        &endpoint,
        &trust,
    );
    let (first, second, id) = tokio::task::spawn_blocking(move || competing(first, second))
        .await
        .unwrap();
    let dispatcher = resolve_with_lost_reply(&second, &server, id).await;
    review_dependent(&dispatcher, &second.actor, id).await;
    let (first, second) = tokio::task::spawn_blocking(move || {
        let (mut first, mut second) = (first, second);
        first.run();
        second.run();
        (first, second)
    })
    .await
    .unwrap();
    assert_eq!(get(&first, id).revision, 4);
    assert_eq!(get(&first, id).name, get(&second, id).name);
    assert_eq!(get(&first, id).notes, get(&second, id).notes);
    assert_eq!(get(&second, id).sync_state, CustomerSyncState::Confirmed);
    assert!(
        second
            .store
            .customer_pending_reviews(&second.actor.scope, id)
            .unwrap()
            .is_empty()
    );
    denial_and_evidence(&database, &server, id).await;
    server
        .handle
        .graceful_shutdown(Some(Duration::from_millis(100)));
}

async fn resolve_with_lost_reply(
    second: &CustomerTestClient,
    server: &ProvisionedServer,
    id: CustomerId,
) -> ProductDispatcher {
    let direct = directory(second, server);
    let lost = Arc::new(LostReply {
        inner: direct.clone(),
        lose: AtomicBool::new(true),
    });
    let dispatcher = ProductDispatcher::new(second.store.clone(), EventBroker::default())
        .with_customer_directory(lost);
    let initial = page(&dispatcher, &second.actor, id).await;
    assert_eq!(initial.items.len(), 1);
    assert_eq!(initial.current.revision, 2);
    assert_ne!(initial.items[0].local.name, initial.items[0].remote.name);
    let mut decision = ResolveCustomerConflict {
        customer: GetCustomer {
            customer_id: id,
            branch_id: Some(second.actor.scope.id),
        },
        source: CustomerResolutionSource::Conflict(CustomerConflictSource {
            conflict_id: initial.items[0].conflict.conflict_id,
        }),
        expected_server_revision: 1,
        contact: CustomerContact {
            name: initial.items[0].local.name.clone(),
            phone: initial.current.phone.clone(),
            address: None,
            notes: None,
        },
    };
    let stale = dispatcher
        .dispatch_command(
            context(&second.actor, IdempotencyKey::new(Uuid::new_v4())),
            Command::ResolveCustomerConflict(decision.clone()),
        )
        .await
        .unwrap_err();
    assert_eq!(
        stale.code.as_str(),
        "eitmad.error.customer-revision-conflict.v1"
    );
    decision.expected_server_revision = 2;
    let key = IdempotencyKey::new(Uuid::new_v4());
    let lost_reply = dispatcher
        .dispatch_command(
            context(&second.actor, key),
            Command::ResolveCustomerConflict(decision.clone()),
        )
        .await
        .unwrap_err();
    assert_eq!(
        lost_reply.code.as_str(),
        "eitmad.error.customer-unavailable.v1"
    );
    drop(dispatcher);
    let dispatcher = ProductDispatcher::new(
        AuthorityStore::open(second.directory.path()).unwrap(),
        EventBroker::default(),
    )
    .with_customer_directory(direct);
    let recovered = page(&dispatcher, &second.actor, id).await;
    assert!(recovered.items.is_empty());
    assert_eq!(
        recovered.pending_resolution.as_ref().unwrap().command,
        decision
    );
    let result = dispatcher
        .dispatch_command(
            context(&second.actor, key),
            Command::ResolveCustomerConflict(decision.clone()),
        )
        .await
        .unwrap();
    let CommandResult::CustomerConflictResolved(change) = result else {
        panic!("resolution")
    };
    assert_eq!(change.revision, 3);
    assert_eq!(
        dispatcher
            .dispatch_command(
                context(&second.actor, key),
                Command::ResolveCustomerConflict(decision.clone())
            )
            .await
            .unwrap(),
        CommandResult::CustomerConflictResolved(change)
    );
    changed_retry_is_rejected(&dispatcher, &second.actor, decision, key).await;
    dispatcher
}

async fn changed_retry_is_rejected(
    dispatcher: &ProductDispatcher,
    actor: &AuthorizationContext,
    mut decision: ResolveCustomerConflict,
    key: IdempotencyKey,
) {
    decision.contact.notes = Some(CustomerNotes::parse("Different retry bytes").unwrap());
    let error = dispatcher
        .dispatch_command(
            context(actor, key),
            Command::ResolveCustomerConflict(decision),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code.as_str(), "eitmad.error.contract-invalid.v1");
}

async fn review_dependent(
    dispatcher: &ProductDispatcher,
    actor: &AuthorizationContext,
    id: CustomerId,
) {
    let page = page(dispatcher, actor, id).await;
    assert_eq!(page.pending_reviews.len(), 1);
    assert_eq!(page.current.revision, 3);
    let pending = &page.pending_reviews[0];
    let command = ResolveCustomerConflict {
        customer: GetCustomer {
            customer_id: id,
            branch_id: Some(actor.scope.id),
        },
        source: CustomerResolutionSource::PendingReview(Box::new(pending.change.clone())),
        expected_server_revision: 3,
        contact: CustomerContact {
            name: page.current.name,
            phone: page.current.phone,
            address: None,
            notes: pending.contact.notes.clone(),
        },
    };
    let result = dispatcher
        .dispatch_command(
            context(actor, IdempotencyKey::new(Uuid::new_v4())),
            Command::ResolveCustomerConflict(command),
        )
        .await
        .unwrap();
    assert!(matches!(result,CommandResult::CustomerConflictResolved(v) if v.revision==4));
}

async fn denial_and_evidence(database: &str, server: &ProvisionedServer, id: CustomerId) {
    let db = SyncDatabase::connect(database, 2).await.unwrap();
    let coordinator = SyncCoordinator::new(&db, direct_test_domains(&db));
    let query = ReadCustomerConflicts {
        scope: server.scope.clone(),
        query: GetCustomerConflicts {
            customer: GetCustomer {
                customer_id: id,
                branch_id: Some(ScopeId::new(Uuid::new_v4())),
            },
            after: None,
        },
    };
    assert_eq!(
        coordinator
            .customer_conflicts(
                &server.authentication.session,
                &query,
                CorrelationId::new(Uuid::new_v4()),
                eitmad_authorization::now()
            )
            .await,
        Err(CustomerError::Denied)
    );
    let mut denied = server.authentication.session.clone();
    denied.user_id = eitmad_contracts::identity::UserId::new(Uuid::new_v4());
    let mut query = query;
    query.query.customer.branch_id = Some(server.branch_scope.id);
    assert_eq!(
        coordinator
            .customer_conflicts(
                &denied,
                &query,
                CorrelationId::new(Uuid::new_v4()),
                eitmad_authorization::now()
            )
            .await,
        Err(CustomerError::Denied)
    );
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(server.authentication.session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let conflicts:i64=sqlx::query_scalar("SELECT count(*) FROM sync.conflicts WHERE tenant_id=$1 AND record_id=$2 AND status='resolved'")
        .bind(server.authentication.session.tenant_id.value()).bind(id.value()).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(conflicts, 2);
    let rows: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT conflict_json FROM sync.conflicts WHERE tenant_id=$1 AND record_id=$2",
    )
    .bind(server.authentication.session.tenant_id.value())
    .bind(id.value())
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert!(rows.iter().all(|v| v.get("local").is_some()
        && v.get("remote").is_some()
        && v.get("resolution").is_some()));
    for (outcome, expected) in [
        ("succeeded", 2_i64),
        ("conflict", 1),
        ("denied", 2),
        ("invalid", 1),
    ] {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.server_records WHERE operation='eitmad.customer.resolve-conflict.v1' AND outcome=$1")
            .bind(outcome).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(count, expected);
    }
    tx.commit().await.unwrap();
    denied_resolution_and_audit_rollback(&db, &coordinator, server, id, &rows[0]).await;
}

async fn denied_resolution_and_audit_rollback(
    db: &SyncDatabase,
    coordinator: &SyncCoordinator,
    server: &ProvisionedServer,
    id: CustomerId,
    preserved: &serde_json::Value,
) {
    let conflict: eitmad_contracts::sync::ConflictRecord =
        serde_json::from_value(preserved.clone()).unwrap();
    let input = ConfirmCustomerResolution {
        scope: server.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        command: ResolveCustomerConflict {
            customer: GetCustomer {
                customer_id: id,
                branch_id: Some(server.branch_scope.id),
            },
            source: CustomerResolutionSource::PendingReview(Box::new(conflict.local)),
            expected_server_revision: 4,
            contact: CustomerContact {
                name: CustomerName::parse("Audit rollback test").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        },
    };
    let mut denied = server.authentication.session.clone();
    denied.user_id = eitmad_contracts::identity::UserId::new(Uuid::new_v4());
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(denied.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.receptionist.v1','branch',$3,1)")
        .bind(denied.tenant_id.value()).bind(denied.user_id.value()).bind(server.branch_scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        coordinator
            .resolve_customer(
                &denied,
                &input,
                CorrelationId::new(Uuid::new_v4()),
                eitmad_authorization::now()
            )
            .await,
        Err(CustomerError::Denied)
    );
    sqlx::raw_sql("CREATE FUNCTION public.fail_customer_resolution_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.customer.resolve-conflict.v1' THEN RAISE EXCEPTION 'test'; END IF; RETURN NEW; END $$;
        CREATE TRIGGER fail_customer_resolution_audit BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION public.fail_customer_resolution_audit()")
        .execute(&db.pool()).await.unwrap();
    let result = coordinator
        .resolve_customer(
            &server.authentication.session,
            &input,
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await;
    sqlx::raw_sql("DROP TRIGGER fail_customer_resolution_audit ON audit.server_records; DROP FUNCTION public.fail_customer_resolution_audit()")
        .execute(&db.pool()).await.unwrap();
    assert_eq!(result, Err(CustomerError::Unavailable));
    let page = coordinator
        .customer_conflicts(
            &server.authentication.session,
            &ReadCustomerConflicts {
                scope: server.scope.clone(),
                query: GetCustomerConflicts {
                    customer: input.command.customer.clone(),
                    after: None,
                },
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    assert_eq!(page.current.revision, 4);
    assert_ne!(page.current.name, input.command.contact.name);
}
