//! Real draft server delivery, lost acknowledgement, isolation, and competing offline edits.
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
    client.actor.tenant_id = tenant;
    client.request.object.tenant_id = tenant;
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
    authorize_draft_writer(&mut second, tenant);
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
    let catalog_scope = scenario.server.scope.clone();
    let mut target = scenario.product_target.clone();
    match &mut target {
        PriceTarget::Product(v) => {
            v.scope = ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(tenant.value()),
            }
        }
        PriceTarget::Furniture(_) => unreachable!(),
    }
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
    let command = CreateQuotationDraft {
        intent: draft_intent(fixture.created_customer.id, fixture.target),
    };
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
    assert_eq!(
        first.drafts.create(&save_context, &command).unwrap(),
        created
    );
    verify_competing_updates(&mut first, &mut second, &created)
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
    // Draft cycles must leave both existing customer checkpoints unchanged.
    assert_eq!(
        first.client.engine.metadata().checkpoint,
        second.client.engine.metadata().checkpoint
    );
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
