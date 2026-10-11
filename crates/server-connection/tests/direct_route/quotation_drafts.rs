#[path = "quotation_lifecycle.rs"]
mod lifecycle;
#[path = "orders.rs"]
mod orders;
use super::*;
use eitmad_contracts::{
    quotation::{EvaluateQuotation, QuotationCustomerIntent, QuotationLineIntent},
    quotation_draft::*,
};
use eitmad_pricing::{QUOTATION_DRAFT_SCHEMA, QuotationDraftService, QuotationDraftSyncCycle};

fn draft_hello() -> PeerHello {
    let mut h = hello();
    h.schemas[0].schema_id = SchemaId::parse(QUOTATION_DRAFT_SCHEMA).unwrap();
    h
}
fn draft_test_transport(
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
        SecretKind::parse("draft-test-session").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    let account_id = authentication.session.account_id;
    store_session(&secrets, &credential, authentication, signing_seed).unwrap();
    let config = DirectServerConfig::new(
        endpoint,
        branch.clone(),
        SchemaId::parse("eitmad.schema.quotation-draft.v1").unwrap(),
        1,
        certificate,
    )
    .unwrap();
    let wan_endpoint = config.wan_endpoint();
    WanAdapter::new(
        wan_endpoint,
        DirectServerDriver::new(config, secrets, draft_hello()),
        draft_hello(),
        TransportAuthentication::AccountDevice {
            account_id,
            device_id,
            credential,
        },
        RetryPolicy::default(),
    )
    .unwrap()
}

fn authorize_draft_writer(client: &mut CustomerTestClient, tenant: TenantId) {
    let auth = AuthorizationService::new(client.store.clone());
    auth.grant_relationship(
        &client.mutation(),
        &GrantScopeRelationship {
            expected_policy_version: 2,
            subject: RelationshipSubject {
                principal_id: client.actor.identity.principal_id,
                principal_kind: PrincipalKind::User,
            },
            relation: RelationId::parse(eitmad_authorization::RECEPTIONIST_RELATION).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(client.actor.tenant_id, tenant);
    assert_eq!(client.request.object.tenant_id, tenant);
    client.restart_engine();
}
fn draft_engine(client: &CustomerTestClient) -> SyncEngine {
    SyncEngine::open_domain(
        client.store.clone(),
        client.actor.scope.clone(),
        (
            SyncMode::LocalFirst,
            SchemaId::parse(QUOTATION_DRAFT_SCHEMA).unwrap(),
        ),
        customer_sync_authorization(&client.store, &client.actor, &client.request),
        &client.actor,
        &client.audit,
    )
    .unwrap()
}
fn run_drafts(
    client: &CustomerTestClient,
    drafts: &QuotationDraftService,
    engine: &mut SyncEngine,
    transport: &mut WanAdapter<DirectServerDriver>,
    catalog: &ScopeRef,
) {
    QuotationDraftSyncCycle {
        drafts,
        engine,
        transport,
        actor: &client.actor,
        server_scope: &client.server_scope,
        server_catalog_scope: catalog,
        request: &client.request,
        audit: &client.audit,
    }
    .run()
    .unwrap();
}
fn server_change(mut change: ChangeRecord, branch: &ScopeRef, catalog: &ScopeRef) -> ChangeRecord {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    change.scope = branch.clone();
    let p = change.payload.as_mut().unwrap();
    let mut snapshot: QuotationDraftSnapshot =
        serde_json::from_slice(&STANDARD.decode(&p.base64).unwrap()).unwrap();
    snapshot.evaluation.scope = branch.clone();
    for line in &mut snapshot.intent.lines {
        match &mut line.configuration.selection.target {
            PriceTarget::Product(v) => v.scope = catalog.clone(),
            PriceTarget::Furniture(v) => v.scope = catalog.clone(),
        }
    }
    for line in &mut snapshot.evaluation.lines {
        match &mut line.price.snapshot.target {
            PriceTarget::Product(v) => v.scope = catalog.clone(),
            PriceTarget::Furniture(v) => v.scope = catalog.clone(),
        }
    }
    p.base64 = STANDARD.encode(serde_json::to_vec(&snapshot).unwrap());
    change
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn quotation_drafts_restart_transfer_replay_and_conflict_through_real_server() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut scenario = CatalogScenario::new().await;
    scenario.transfer_dependencies().await;
    scenario.publish_prices().await;
    let tenant = scenario.server.authentication.session.tenant_id;
    let branch = scenario.server.branch_scope.clone();
    let pool = verify_server_permissions(&scenario).await;
    let mut first = CustomerTestClient::new(
        scenario.server.authentication.clone(),
        scenario.server.device_id,
        [11; 32],
        &branch,
        &scenario.endpoint,
        &scenario.trust,
    );
    let mut second = CustomerTestClient::new(
        scenario.server.second_authentication.clone(),
        scenario.server.second_device_id,
        [12; 32],
        &branch,
        &scenario.endpoint,
        &scenario.trust,
    );
    authorize_draft_writer(&mut first, tenant);
    // The second desktop initially has Manager read authority without draft-write authority.
    second.actor.tenant_id = tenant;
    second.request.object.tenant_id = tenant;
    second.restart_engine();
    let (_, catalog_actor_a) = catalog_local_authority(
        first.directory.path(),
        &scenario.server.authentication.session,
    );
    let (_, catalog_actor_b) = catalog_local_authority(
        second.directory.path(),
        &scenario.server.second_authentication.session,
    );
    let catalog_a = replication(
        first.directory.path(),
        first.store.clone(),
        scenario.server.authentication.clone(),
        [11; 32],
        &scenario.server.scope,
        &scenario.endpoint,
        &scenario.trust,
    );
    let catalog_b = replication(
        second.directory.path(),
        second.store.clone(),
        scenario.server.second_authentication.clone(),
        [12; 32],
        &scenario.server.scope,
        &scenario.endpoint,
        &scenario.trust,
    );
    cycle(catalog_a, catalog_actor_a).await.unwrap();
    cycle(catalog_b, catalog_actor_b).await.unwrap();
    let (catalog_scope, target, furniture_target) =
        draft_transfer_targets(&scenario, tenant.value());
    let created_customer = first
        .customers
        .create(
            &first.mutation(),
            &CreateCustomer {
                name: CustomerName::parse("عميل المسودة التجريبي").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    let auth_a = scenario.server.authentication.clone();
    let auth_b = scenario.server.second_authentication.clone();
    let endpoint = scenario.endpoint.clone();
    let trust = scenario.trust.clone();
    let fixture = DraftTransferFixture {
        created_customer,
        target,
        furniture_target,
        dimensions: scenario.furniture.variants[0].dimensions.clone(),
        auth_a,
        auth_b,
        endpoint,
        trust,
        catalog_scope,
    };
    let final_local = tokio::task::spawn_blocking(move || verify_clients(first, second, fixture))
        .await
        .unwrap();

    verify_postgres_history(&pool, tenant, final_local).await;
    scenario
        .server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
}

fn draft_transfer_targets(
    scenario: &CatalogScenario,
    tenant_id: Uuid,
) -> (ScopeRef, PriceTarget, PriceTarget) {
    let organization_scope = ScopeRef {
        kind: ScopeKind::parse("organization").unwrap(),
        id: ScopeId::new(tenant_id),
    };
    let mut target = scenario.product_target.clone();
    let mut furniture_target = scenario.furniture_target.clone();
    match &mut target {
        PriceTarget::Product(v) => v.scope = organization_scope.clone(),
        PriceTarget::Furniture(_) => unreachable!(),
    }
    if let PriceTarget::Furniture(v) = &mut furniture_target {
        v.scope = organization_scope;
    }
    (scenario.server.scope.clone(), target, furniture_target)
}

async fn verify_server_permissions(scenario: &CatalogScenario) -> sqlx::PgPool {
    let tenant = scenario.server.authentication.session.tenant_id;
    let branch = scenario.server.branch_scope.clone();
    let handler = eitmad_sync_plane::QuotationDraftSyncHandler::new(
        SyncDatabase::connect(&scenario.database, 2)
            .await
            .unwrap()
            .pool(),
    );
    let session = &scenario.server.authentication.session;
    assert!(handler.authorize(session, &branch, SyncIntent::Read).await);
    assert!(!handler.authorize(session, &branch, SyncIntent::Write).await);
    let pool = SyncDatabase::connect(&scenario.database, 2)
        .await
        .unwrap()
        .pool();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(tenant.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.receptionist.v1','branch',$3,1)").bind(tenant.value()).bind(session.user_id.value()).bind(branch.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    assert!(handler.authorize(session, &branch, SyncIntent::Write).await);
    let other_branch = ScopeRef {
        id: ScopeId::new(Uuid::new_v4()),
        ..branch.clone()
    };
    assert!(
        !handler
            .authorize(session, &other_branch, SyncIntent::Read)
            .await
    );
    pool
}
struct DraftTransferFixture {
    created_customer: Customer,
    target: PriceTarget,
    furniture_target: PriceTarget,
    dimensions: eitmad_contracts::furniture::FurnitureDimensions,
    auth_a: AuthenticationResult,
    auth_b: AuthenticationResult,
    endpoint: String,
    trust: PathBuf,
    catalog_scope: ScopeRef,
}

struct DraftTestClient {
    client: CustomerTestClient,
    drafts: QuotationDraftService,
    engine: SyncEngine,
    transport: WanAdapter<DirectServerDriver>,
    catalog: ScopeRef,
}
impl DraftTestClient {
    fn new(
        mut client: CustomerTestClient,
        auth: AuthenticationResult,
        seed: [u8; 32],
        connection: (&str, &Path, &ScopeRef),
    ) -> Self {
        client.run();
        let drafts = QuotationDraftService::new(
            client.store.clone(),
            AuthorizationService::new(client.store.clone()),
        );
        let engine = draft_engine(&client);
        let device = auth.session.device_id;
        let transport = draft_test_transport(
            client.directory.path(),
            auth,
            device,
            seed,
            &client.server_scope,
            connection.0,
            connection.1,
        );
        Self {
            client,
            drafts,
            engine,
            transport,
            catalog: connection.2.clone(),
        }
    }
    fn run(&mut self) {
        run_drafts(
            &self.client,
            &self.drafts,
            &mut self.engine,
            &mut self.transport,
            &self.catalog,
        );
    }
    fn get(&self, id: QuotationDraftId) -> QuotationDraft {
        self.drafts
            .get(&self.client.actor, &GetQuotationDraft { draft_id: id })
            .unwrap()
    }
    fn update(&self, command: &UpdateQuotationDraft) -> QuotationDraft {
        self.drafts
            .update(&self.client.mutation(), command)
            .unwrap()
    }
}
fn draft_intent(customer: CustomerId, target: PriceTarget) -> EvaluateQuotation {
    EvaluateQuotation {
        customer: Some(QuotationCustomerIntent {
            id: customer,
            revision: 1,
        }),
        lines: vec![QuotationLineIntent {
            id: Uuid::new_v4(),
            configuration: CheckSalesConfiguration {
                selection: PriceSelection {
                    target,
                    price_revision: 1,
                    quantity: 1,
                    color_id: None,
                    handle_id: None,
                },
                dimensions: None,
            },
        }],
        discount_basis_points: 500,
    }
}
fn verify_clients(
    first: CustomerTestClient,
    second: CustomerTestClient,
    fixture: DraftTransferFixture,
) -> QuotationDraft {
    let mut first = DraftTestClient::new(
        first,
        fixture.auth_a,
        [11; 32],
        (&fixture.endpoint, &fixture.trust, &fixture.catalog_scope),
    );
    let mut second = DraftTestClient::new(
        second,
        fixture.auth_b,
        [12; 32],
        (&fixture.endpoint, &fixture.trust, &fixture.catalog_scope),
    );
    let first_customer_state = customer_sync_state(&first);
    let second_customer_state = customer_sync_state(&second);
    let mut intent = draft_intent(fixture.created_customer.id, fixture.target);
    intent.lines.push(QuotationLineIntent {
        id: Uuid::new_v4(),
        configuration: CheckSalesConfiguration {
            selection: PriceSelection {
                target: fixture.furniture_target,
                price_revision: 1,
                quantity: 2,
                color_id: None,
                handle_id: None,
            },
            dimensions: Some(fixture.dimensions),
        },
    });
    let command = CreateQuotationDraft { intent };
    let save_context = first.client.mutation();
    let created = first.drafts.create(&save_context, &command).unwrap();
    let reopened = AuthorityStore::open(first.client.directory.path()).unwrap();
    let reopened_service =
        QuotationDraftService::new(reopened.clone(), AuthorizationService::new(reopened));
    assert_eq!(
        reopened_service
            .get(
                &first.client.actor,
                &GetQuotationDraft {
                    draft_id: created.snapshot.id
                }
            )
            .unwrap(),
        created
    );
    verify_interrupted_delivery(&mut first);
    first.engine = draft_engine(&first.client);
    first.run();
    second.run();
    let received = second.get(created.snapshot.id);
    let mut expected = created.snapshot.clone();
    expected.evaluation.scope = second.client.actor.scope.clone();
    assert_eq!(received.snapshot, expected);
    assert_eq!(received.sync_state, QuotationDraftSyncState::Confirmed);
    assert_eq!(received.snapshot.evaluation.lines.len(), 2);
    assert_eq!(
        received
            .snapshot
            .evaluation
            .totals
            .as_ref()
            .unwrap()
            .total_yer,
        104_500
    );
    assert_eq!(
        second.drafts.update(
            &second.client.mutation(),
            &UpdateQuotationDraft {
                draft_id: received.snapshot.id,
                expected_revision: received.snapshot.revision,
                intent: received.snapshot.intent.clone(),
            },
        ),
        Err(eitmad_pricing::QuotationDraftError::Denied),
    );
    assert_eq!(
        first.drafts.create(&save_context, &command).unwrap(),
        created
    );
    // Switch the second desktop to an authorized editor only for the competing-write checks.
    let tenant = second.client.actor.tenant_id;
    authorize_draft_writer(&mut second.client, tenant);
    let conflict = verify_competing_updates(&mut first, &mut second, &created);
    // Compare durable customer state, including revision and bytes, per client.
    assert_eq!(customer_sync_state(&first), first_customer_state);
    assert_eq!(customer_sync_state(&second), second_customer_state);
    conflict
}

fn customer_sync_state(client: &DraftTestClient) -> eitmad_storage::StoredSyncState {
    client
        .client
        .store
        .read_sync_state(&client.client.actor.scope)
        .unwrap()
        .unwrap()
}

fn verify_interrupted_delivery(client: &mut DraftTestClient) {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    client
        .transport
        .connect(eitmad_authorization::now())
        .unwrap();
    let pending = client
        .drafts
        .sync_batch(&client.client.actor, 50)
        .unwrap()
        .remove(0);
    let delivered = server_change(pending, &client.client.server_scope, &client.catalog);
    for forge_customer in [false, true] {
        let mut forged = delivered.clone();
        forged.change_id = ChangeId::new(Uuid::new_v4());
        forged.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
        let payload = forged.payload.as_mut().unwrap();
        let mut snapshot: QuotationDraftSnapshot =
            serde_json::from_slice(&STANDARD.decode(&payload.base64).unwrap()).unwrap();
        if forge_customer {
            snapshot.evaluation.customer.as_mut().unwrap().name = "عميل محرّف".into();
        } else {
            snapshot.evaluation.lines[0].price.total_yer += 1;
        }
        payload.base64 = STANDARD.encode(serde_json::to_vec(&snapshot).unwrap());
        let rejected = exchange(
            &mut client.transport,
            SyncMessage::SubmitLocal(LocalChangeSubmission { change: forged }),
        );
        assert!(
            matches!(rejected,SyncMessage::LocalResult(r) if matches!(r.disposition,LocalChangeDisposition::Rejected {..}))
        );
    }
    let reply = exchange(
        &mut client.transport,
        SyncMessage::SubmitLocal(LocalChangeSubmission {
            change: delivered.clone(),
        }),
    );
    assert!(
        matches!(reply,SyncMessage::LocalResult(r) if matches!(r.disposition,LocalChangeDisposition::Applied {..}))
    );
    let retry = exchange(
        &mut client.transport,
        SyncMessage::SubmitLocal(LocalChangeSubmission { change: delivered }),
    );
    assert!(
        matches!(retry,SyncMessage::LocalResult(r) if matches!(r.disposition,LocalChangeDisposition::Replayed {..}))
    );
    // Client stops before committing projection or acknowledgement of accepted work.
    client.transport.disconnect(eitmad_authorization::now());
}
fn verify_competing_updates(
    first: &mut DraftTestClient,
    second: &mut DraftTestClient,
    created: &QuotationDraft,
) -> QuotationDraft {
    let mut update = UpdateQuotationDraft {
        draft_id: created.snapshot.id,
        expected_revision: 1,
        intent: created.snapshot.intent.clone(),
    };
    update.intent.discount_basis_points = 100;
    first.update(&update);
    update.intent.discount_basis_points = 200;
    second.update(&update);
    update.expected_revision = 2;
    update.intent.discount_basis_points = 300;
    let local = second.update(&update);
    first.run();
    second.run();
    let conflict = second.get(created.snapshot.id);
    assert_eq!(conflict.snapshot, local.snapshot);
    assert_eq!(conflict.sync_state, QuotationDraftSyncState::Conflicted);
    assert!(
        conflict
            .conflict
            .as_ref()
            .unwrap()
            .server_conflict_id
            .is_some()
    );
    assert!(conflict.conflict.as_ref().unwrap().remote.is_some());
    second.engine = draft_engine(&second.client);
    second.run();
    assert_eq!(second.get(created.snapshot.id), conflict);
    conflict
}
async fn verify_postgres_history(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    final_local: QuotationDraft,
) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(tenant.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sync.quotation_draft_revisions WHERE tenant_id=$1",
    )
    .bind(tenant.value())
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(count, 2);
    let conflicts:i64=sqlx::query_scalar("SELECT count(*) FROM sync.conflicts WHERE tenant_id=$1 AND schema_id='eitmad.schema.quotation-draft.v1'").bind(tenant.value()).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(conflicts, 1);
    let retained: serde_json::Value = sqlx::query_scalar(
        "SELECT conflict_json FROM sync.conflicts WHERE tenant_id=$1 AND conflict_id=$2",
    )
    .bind(tenant.value())
    .bind(
        final_local
            .conflict
            .unwrap()
            .server_conflict_id
            .unwrap()
            .value(),
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(serde_json::from_value::<eitmad_contracts::sync::ConflictRecord>(retained).is_ok());
    assert!(
        sqlx::query("UPDATE sync.quotation_draft_revisions SET revision=9 WHERE tenant_id=$1")
            .bind(tenant.value())
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(Uuid::new_v4().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let hidden: i64 = sqlx::query_scalar("SELECT count(*) FROM sync.quotation_draft_revisions")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(hidden, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn discount_approval_cross_client_live_replay_invalidation_and_rejection() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut scenario = CatalogScenario::new().await;
    scenario.transfer_dependencies().await;
    scenario.publish_prices().await;
    let tenant = scenario.server.authentication.session.tenant_id;
    let branch = scenario.server.branch_scope.clone();
    let pool = verify_server_permissions(&scenario).await;
    let mut reception = CustomerTestClient::new(
        scenario.server.authentication.clone(),
        scenario.server.device_id,
        [11; 32],
        &branch,
        &scenario.endpoint,
        &scenario.trust,
    );
    authorize_draft_writer(&mut reception, tenant);
    let (_, catalog_actor) = catalog_local_authority(
        reception.directory.path(),
        &scenario.server.authentication.session,
    );
    let replication = replication(
        reception.directory.path(),
        reception.store.clone(),
        scenario.server.authentication.clone(),
        [11; 32],
        &scenario.server.scope,
        &scenario.endpoint,
        &scenario.trust,
    );
    cycle(replication, catalog_actor).await.unwrap();
    let (catalog, target, _) = draft_transfer_targets(&scenario, tenant.value());
    let customer = reception
        .customers
        .create(
            &reception.mutation(),
            &CreateCustomer {
                name: CustomerName::parse("عميل موافقة تجريبي").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    let manager_directory = tempfile::tempdir().unwrap();
    let clients = lifecycle_clients(&reception, &scenario, manager_directory.path()).await;
    let auth = scenario.server.authentication.clone();
    let endpoint = scenario.endpoint.clone();
    let trust = scenario.trust.clone();
    let lifecycle_pool = pool.clone();
    let lifecycle_actor = auth.session.clone();
    let final_request = tokio::task::spawn_blocking(move || {
        let mut reception =
            DraftTestClient::new(reception, auth, [11; 32], (&endpoint, &trust, &catalog));
        let result = run_approval_workflow(&mut reception, customer.id, target.clone(), &clients);
        lifecycle::run(
            &mut reception,
            customer.id,
            target,
            &clients,
            &lifecycle_pool,
            &lifecycle_actor,
        );
        result
    })
    .await
    .unwrap();
    verify_approval_audit_rollback(&pool, &scenario.server.authentication.session, &branch).await;
    verify_approval_history(&pool, tenant, final_request.quotation.id).await;
    assert_eq!(
        final_request.state,
        eitmad_contracts::quotation_approval::DiscountApprovalState::Rejected
    );
    scenario
        .server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
}

async fn lifecycle_clients(
    reception: &CustomerTestClient,
    scenario: &CatalogScenario,
    manager_directory: &Path,
) -> ApprovalClients {
    let tenant = scenario.server.authentication.session.tenant_id;
    let reception_connection = approval_connection(
        reception.directory.path(),
        scenario.server.authentication.clone(),
        [11; 32],
        scenario,
    );
    let manager_auth = approval_manager(scenario).await;
    let manager_connection =
        approval_connection(manager_directory, manager_auth.clone(), [12; 32], scenario);
    let mut manager_actor = reception.actor.clone();
    manager_actor.identity.principal_id = PrincipalId::new(manager_auth.session.user_id.value());
    manager_actor.scope = ScopeRef {
        kind: ScopeKind::parse("organization").unwrap(),
        id: ScopeId::new(tenant.value()),
    };
    let pure_reception = approval_connection(
        scenario.reception_directory.path(),
        scenario.reception_auth.clone(),
        [19; 32],
        scenario,
    );
    let mut pure_actor = reception.actor.clone();
    pure_actor.identity.principal_id =
        PrincipalId::new(scenario.reception_auth.session.user_id.value());
    ApprovalClients {
        reception: reception_connection,
        manager: manager_connection,
        manager_actor,
        manager_auth,
        pure_reception,
        pure_actor,
        pricing: scenario.pricing.clone(),
        pricing_actor: scenario.manager.clone(),
    }
}

struct ApprovalClients {
    manager_auth: AuthenticationResult,
    pricing: PricingService,
    pricing_actor: AuthorizationContext,
    reception: Arc<eitmad_server_connection::DirectDiscountApprovalClient>,
    manager: Arc<eitmad_server_connection::DirectDiscountApprovalClient>,
    manager_actor: AuthorizationContext,
    pure_reception: Arc<eitmad_server_connection::DirectDiscountApprovalClient>,
    pure_actor: AuthorizationContext,
}
type ApprovalEvents =
    std::sync::mpsc::Receiver<eitmad_contracts::quotation_approval::DiscountApprovalNotice>;

fn run_approval_workflow(
    reception: &mut DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
) -> eitmad_contracts::quotation_approval::DiscountApproval {
    use eitmad_contracts::quotation_approval::*;
    use eitmad_pricing::DiscountApprovalServer;
    let mut intent = draft_intent(customer, target);
    intent.discount_basis_points = 600;
    let initial = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft { intent },
        )
        .unwrap();
    // Promotion must independently validate and persist a saved draft, without a prior draft WAN cycle.
    let reception_actor = reception.client.actor.clone();
    let (manager_events, manager_cancel, manager_watch) =
        watch_approvals(clients.manager.clone(), clients.manager_actor.clone());
    let (reception_events, reception_cancel, reception_watch) =
        watch_approvals(clients.reception.clone(), reception_actor.clone());
    let request = ConfirmDiscountApproval {
        scope: reception_actor.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: DiscountApprovalAction::Request(initial.snapshot.clone()),
    };
    let pending = clients
        .reception
        .transition(&reception_actor, &request, UnixMillis(i64::MAX))
        .unwrap()
        .unwrap();
    assert_eq!(pending.state, DiscountApprovalState::Pending);
    assert_eq!(
        clients
            .reception
            .transition(&reception_actor, &request, UnixMillis(i64::MAX))
            .unwrap(),
        Some(pending.clone())
    );
    wait_approval(
        &manager_events,
        &*clients.manager,
        &clients.manager_actor,
        pending.quotation.id,
        DiscountApprovalState::Pending,
    );
    let (decide, duplicate) =
        verify_approved_and_duplicate(clients, &reception_actor, &pending, &reception_events);
    let next = invalidate_and_request(
        reception,
        clients,
        &initial.snapshot,
        &pending,
        &duplicate,
        &manager_events,
    );
    let rejected = verify_rejected(clients, &reception_actor, next, decide, &reception_events);
    manager_cancel.store(true, std::sync::atomic::Ordering::Release);
    reception_cancel.store(true, std::sync::atomic::Ordering::Release);
    manager_watch.join().unwrap().unwrap();
    reception_watch.join().unwrap().unwrap();
    rejected
}
fn verify_denied_decisions(
    clients: &ApprovalClients,
    reception_actor: &AuthorizationContext,
    decide: &eitmad_contracts::quotation_approval::ConfirmDiscountApproval,
) {
    use eitmad_contracts::quotation_approval::*;
    use eitmad_pricing::{ApprovalError, DiscountApprovalServer};
    let ApprovalClients {
        reception: reception_connection,
        pure_reception,
        pure_actor,
        ..
    } = clients;
    let self_decision = ConfirmDiscountApproval {
        scope: reception_actor.scope.clone(),
        ..decide.clone()
    };
    assert_eq!(
        reception_connection.transition(reception_actor, &self_decision, UnixMillis(i64::MAX)),
        Err(ApprovalError::Denied)
    );
    let forged = ConfirmDiscountApproval {
        scope: pure_actor.scope.clone(),
        ..decide.clone()
    };
    assert_eq!(
        pure_reception.transition(pure_actor, &forged, UnixMillis(i64::MAX)),
        Err(ApprovalError::Denied)
    );
}
fn verify_approved_and_duplicate(
    clients: &ApprovalClients,
    reception_actor: &AuthorizationContext,
    pending: &eitmad_contracts::quotation_approval::DiscountApproval,
    reception_events: &ApprovalEvents,
) -> (
    eitmad_contracts::quotation_approval::ConfirmDiscountApproval,
    eitmad_contracts::quotation_approval::ConfirmDiscountApproval,
) {
    use eitmad_contracts::quotation_approval::*;
    use eitmad_pricing::{ApprovalError, DiscountApprovalServer};
    let ApprovalClients {
        reception: reception_connection,
        manager: manager_connection,
        manager_actor,
        ..
    } = clients;
    let decision = DecideDiscountApproval {
        draft_id: pending.quotation.id,
        request_id: pending.request_id,
        quotation_revision: pending.quotation.revision,
        expected_revision: pending.revision,
        fingerprint: pending.fingerprint.clone(),
        decision: DiscountDecision::Approve,
        reason: None,
    };
    let decide = ConfirmDiscountApproval {
        scope: manager_actor.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: DiscountApprovalAction::Decide(decision.clone()),
    };
    verify_denied_decisions(clients, reception_actor, &decide);
    let approved = manager_connection
        .transition(manager_actor, &decide, UnixMillis(i64::MAX))
        .unwrap()
        .unwrap();
    assert_eq!(approved.state, DiscountApprovalState::Approved);
    wait_approval(
        reception_events,
        &**reception_connection,
        reception_actor,
        pending.quotation.id,
        DiscountApprovalState::Approved,
    );
    assert_eq!(
        manager_connection
            .transition(manager_actor, &decide, UnixMillis(i64::MAX))
            .unwrap(),
        Some(approved)
    );
    let duplicate = ConfirmDiscountApproval {
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        ..decide.clone()
    };
    assert_eq!(
        manager_connection.transition(manager_actor, &duplicate, UnixMillis(i64::MAX)),
        Err(ApprovalError::Conflict)
    );
    let mut reused = decide.clone();
    if let DiscountApprovalAction::Decide(c) = &mut reused.action {
        c.decision = DiscountDecision::Reject;
        c.reason = Some("سبب تجريبي".into());
    }
    assert_eq!(
        manager_connection.transition(manager_actor, &reused, UnixMillis(i64::MAX)),
        Err(ApprovalError::Invalid)
    );
    (decide, duplicate)
}
fn invalidate_and_request(
    reception: &mut DraftTestClient,
    clients: &ApprovalClients,
    initial: &QuotationDraftSnapshot,
    original: &eitmad_contracts::quotation_approval::DiscountApproval,
    duplicate: &eitmad_contracts::quotation_approval::ConfirmDiscountApproval,
    manager_events: &ApprovalEvents,
) -> eitmad_contracts::quotation_approval::DiscountApproval {
    use eitmad_contracts::quotation_approval::*;
    use eitmad_pricing::{ApprovalError, DiscountApprovalServer};
    let ApprovalClients {
        reception: reception_connection,
        manager: manager_connection,
        manager_actor,
        ..
    } = clients;
    let reception_actor = &reception.client.actor.clone();
    // Match engine promotion: acknowledge the locally saved snapshot only after server confirmation.
    for change in reception.drafts.sync_batch(reception_actor, 50).unwrap() {
        reception
            .drafts
            .project_confirmed(reception_actor, &change, CorrelationId::new(Uuid::new_v4()))
            .unwrap();
    }
    // A normal draft transfer must invalidate an already-approved commercial revision.
    reception.run();
    let mut intent = initial.intent.clone();
    intent.discount_basis_points = 700;
    let edited = reception.update(&UpdateQuotationDraft {
        draft_id: initial.id,
        expected_revision: 1,
        intent,
    });
    reception.run();
    wait_approval(
        manager_events,
        &**manager_connection,
        manager_actor,
        original.quotation.id,
        DiscountApprovalState::Invalidated,
    );
    assert_eq!(
        manager_connection.transition(manager_actor, duplicate, UnixMillis(i64::MAX)),
        Err(ApprovalError::Conflict)
    );
    let request = ConfirmDiscountApproval {
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: DiscountApprovalAction::Request(edited.snapshot),
        scope: reception_actor.scope.clone(),
    };
    let pending = reception_connection
        .transition(reception_actor, &request, UnixMillis(i64::MAX))
        .unwrap()
        .unwrap();
    assert_ne!(pending.request_id, original.request_id);
    assert_ne!(pending.fingerprint, original.fingerprint);
    wait_approval(
        manager_events,
        &**manager_connection,
        manager_actor,
        pending.quotation.id,
        DiscountApprovalState::Pending,
    );
    pending
}
fn verify_rejected(
    clients: &ApprovalClients,
    reception_actor: &AuthorizationContext,
    pending: eitmad_contracts::quotation_approval::DiscountApproval,
    decide: eitmad_contracts::quotation_approval::ConfirmDiscountApproval,
    reception_events: &ApprovalEvents,
) -> eitmad_contracts::quotation_approval::DiscountApproval {
    use eitmad_contracts::quotation_approval::*;
    use eitmad_pricing::DiscountApprovalServer;
    let ApprovalClients {
        reception: reception_connection,
        manager: manager_connection,
        manager_actor,
        ..
    } = clients;
    let reject = ConfirmDiscountApproval {
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: DiscountApprovalAction::Decide(DecideDiscountApproval {
            draft_id: pending.quotation.id,
            request_id: pending.request_id,
            quotation_revision: pending.quotation.revision,
            expected_revision: pending.revision,
            fingerprint: pending.fingerprint,
            decision: DiscountDecision::Reject,
            reason: Some("الخصم مرتفع".into()),
        }),
        ..decide
    };
    let rejected = manager_connection
        .transition(manager_actor, &reject, UnixMillis(i64::MAX))
        .unwrap()
        .unwrap();
    assert_eq!(rejected.reason.as_deref(), Some("الخصم مرتفع"));
    wait_approval(
        reception_events,
        &**reception_connection,
        reception_actor,
        pending.quotation.id,
        DiscountApprovalState::Rejected,
    );
    assert_eq!(
        manager_connection
            .transition(manager_actor, &reject, UnixMillis(i64::MAX))
            .unwrap(),
        Some(rejected.clone())
    );
    rejected
}

async fn verify_approval_history(
    pool: &sqlx::PgPool,
    tenant: TenantId,
    draft_id: QuotationDraftId,
) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(tenant.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let history: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sync.quotation_approval_history WHERE tenant_id=$1 AND draft_id=$2",
    )
    .bind(tenant.value())
    .bind(draft_id.value())
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(history, 5);
    let audit:i64=sqlx::query_scalar("SELECT count(*) FROM audit.server_records WHERE tenant_id=$1 AND operation IN ('eitmad.quotation-approval.approve.v1','eitmad.quotation-approval.reject.v1') AND outcome='succeeded' AND target_id=$2").bind(tenant.value()).bind(draft_id.value()).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(audit, 2);
    assert!(
        sqlx::query("UPDATE sync.quotation_approval_history SET revision=9 WHERE tenant_id=$1")
            .bind(tenant.value())
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(Uuid::new_v4().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let hidden: i64 = sqlx::query_scalar("SELECT count(*) FROM sync.quotation_approval_history")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(hidden, 0);
}

fn approval_connection(
    directory: &Path,
    authentication: AuthenticationResult,
    seed: [u8; 32],
    scenario: &CatalogScenario,
) -> Arc<eitmad_server_connection::DirectDiscountApprovalClient> {
    let secrets = SecretStore::open(
        directory.join("approval-secrets"),
        Some(FallbackEncryptionKey::new([7; 32])),
    )
    .unwrap();
    let credential = SecretId::new(
        SecretKind::parse("approval-test-session").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_session(&secrets, &credential, authentication, seed).unwrap();
    let config = DirectServerConfig::new(
        &scenario.endpoint,
        scenario.server.scope.clone(),
        SchemaId::parse(eitmad_pricing::DISCOUNT_APPROVAL_SCHEMA).unwrap(),
        1,
        &scenario.trust,
    )
    .unwrap();
    Arc::new(
        eitmad_server_connection::DirectDiscountApprovalClient::from_config(
            config,
            secrets,
            credential,
            scenario.server.branch_scope.clone(),
        ),
    )
}
fn watch_approvals(
    client: Arc<eitmad_server_connection::DirectDiscountApprovalClient>,
    actor: AuthorizationContext,
) -> (
    std::sync::mpsc::Receiver<eitmad_contracts::quotation_approval::DiscountApprovalNotice>,
    Arc<std::sync::atomic::AtomicBool>,
    std::thread::JoinHandle<Result<(), eitmad_pricing::ApprovalError>>,
) {
    use eitmad_pricing::DiscountApprovalServer;
    let (send, receive) = std::sync::mpsc::channel();
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stopping = cancel.clone();
    let thread = std::thread::spawn(move || {
        client.watch(&actor, &stopping, &mut |notice| {
            send.send(notice).unwrap();
        })
    });
    (receive, cancel, thread)
}
fn wait_approval(
    events: &std::sync::mpsc::Receiver<
        eitmad_contracts::quotation_approval::DiscountApprovalNotice,
    >,
    client: &dyn eitmad_pricing::DiscountApprovalServer,
    actor: &AuthorizationContext,
    id: QuotationDraftId,
    state: eitmad_contracts::quotation_approval::DiscountApprovalState,
) {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let notice = events
            .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap_or_else(|e| panic!("live approval state {state:?} was not delivered: {e:?}"));
        if notice.draft_id != id {
            continue;
        }
        let page = client
            .list(
                actor,
                &eitmad_contracts::quotation_approval::ListDiscountApprovals {
                    after: None,
                    limit: 100,
                },
                UnixMillis(i64::MAX),
            )
            .unwrap();
        if page
            .items
            .iter()
            .any(|a| a.quotation.id == id && a.state == state)
        {
            return;
        }
    }
}

async fn approval_manager(scenario: &CatalogScenario) -> AuthenticationResult {
    let pool = SyncDatabase::connect(&scenario.database, 2)
        .await
        .unwrap()
        .pool();
    let tenant = scenario.server.authentication.session.tenant_id;
    let user = UserId::new(Uuid::new_v4());
    let account = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(tenant.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.users VALUES($1,$2,1)")
        .bind(tenant.value())
        .bind(user.value())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.accounts(tenant_id,account_id,user_id,username,canonical_username,status,password_hash,created_at,activated_at) SELECT tenant_id,$2,$3,'approval-manager','approval-manager','active',password_hash,1,1 FROM control.accounts WHERE tenant_id=$1 AND account_id=$4").bind(tenant.value()).bind(account).bind(user.value()).bind(scenario.server.authentication.session.account_id.value()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.manager.v1','organization',$3,1)").bind(tenant.value()).bind(user.value()).bind(scenario.server.scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let signing = SigningKey::from_bytes(&[12; 32]);
    ControlPlane::new(pool, TokenKey::new([9; 32]))
        .authentication
        .login(
            &LoginRequest {
                tenant_code: TenantCode::parse("direct-test").unwrap(),
                username: "approval-manager".into(),
                password: "synthetic-test-password-123".into(),
                device_id: DeviceId::new(Uuid::new_v4()),
                device_label: "مدير الموافقة".into(),
                device_public_key: DevicePublicKey {
                    algorithm: "ed25519".into(),
                    base64: URL_SAFE_NO_PAD.encode(signing.verifying_key().as_bytes()),
                },
                device_proof: None,
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap()
}

async fn verify_approval_audit_rollback(
    pool: &sqlx::PgPool,
    actor: &AuthenticatedServerSession,
    branch: &ScopeRef,
) {
    use eitmad_contracts::quotation_approval::*;
    let server = eitmad_sync_plane::QuotationApprovalServer::new(pool.clone());
    let page = server
        .list(
            actor,
            &ReadDiscountApprovals {
                scope: branch.clone(),
                query: ListDiscountApprovals {
                    after: None,
                    limit: 100,
                },
            },
        )
        .await
        .unwrap();
    let retained = &page.items[0];
    let request = ConfirmDiscountApproval {
        scope: branch.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: DiscountApprovalAction::Request(retained.quotation.clone()),
    };
    // Force the last mandatory write to fail after history and events have been staged.
    sqlx::raw_sql("CREATE FUNCTION audit.fail_approval_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.quotation-approval.request.v1' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_approval_test BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION audit.fail_approval_test();").execute(pool).await.unwrap();
    let result = server
        .transition(
            actor,
            &request,
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await;
    sqlx::raw_sql("DROP TRIGGER fail_approval_test ON audit.server_records; DROP FUNCTION audit.fail_approval_test();").execute(pool).await.unwrap();
    assert_eq!(result, Err(eitmad_pricing::ApprovalError::Unavailable));
    let page = server
        .list(
            actor,
            &ReadDiscountApprovals {
                scope: branch.clone(),
                query: ListDiscountApprovals {
                    after: None,
                    limit: 100,
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(&page.items[0], retained);
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(actor.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM sync.quotation_approval_receipts WHERE tenant_id=$1 AND idempotency_key=$2").bind(actor.tenant_id.value()).bind(request.idempotency_key.value()).fetch_one(&mut *tx).await.unwrap();
    assert_eq!(receipts, 0);
}
