use super::*;
use eitmad_contracts::{order::*, quotation_lifecycle::*};
use eitmad_orders::{OrderError as E, OrderServer};
use eitmad_pricing::{DiscountApprovalServer, QuotationServer};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn orders_cross_client_conversion_transitions_and_restart() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut scenario = CatalogScenario::new().await;
    scenario.transfer_dependencies().await;
    scenario.publish_prices().await;
    let pool = verify_server_permissions(&scenario).await;
    assign_second_receptionist(&pool, &scenario).await;
    let tenant = scenario.server.authentication.session.tenant_id;
    let reception = order_reception(&scenario).await;
    let (catalog, product, furniture) = draft_transfer_targets(&scenario, tenant.value());
    let customer = reception
        .customers
        .create(
            &reception.mutation(),
            &CreateCustomer {
                name: CustomerName::parse("عميل طلب تجريبي").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    let manager_directory = tempfile::tempdir().unwrap();
    let clients = lifecycle_clients(&reception, &scenario, manager_directory.path()).await;
    let dimensions = scenario.furniture.variants[0].dimensions.clone();
    let auth = scenario.server.authentication.clone();
    let endpoint = scenario.endpoint.clone();
    let trust = scenario.trust.clone();
    let (mut reception, clients, actor, accepted) = tokio::task::spawn_blocking(move || {
        let mut reception =
            DraftTestClient::new(reception, auth, [11; 32], (&endpoint, &trust, &catalog));
        let actor = reception.client.actor.clone();
        let accepted = accept(
            &mut reception,
            customer.id,
            furniture,
            Some(dimensions),
            &clients,
        );
        (reception, clients, actor, accepted)
    })
    .await
    .unwrap();
    let reserved = conversion_rollback(
        &pool,
        &scenario.server.authentication.session,
        &scenario.server.branch_scope,
        &accepted,
    )
    .await;
    let (clients, actor, final_order, request) = tokio::task::spawn_blocking(move || {
        let (order, request) = competing_conversion(&clients, &actor, &accepted);
        let watches = OrderWatches::new(&clients, &actor);
        assert!(
            order
                .number
                .rsplit('-')
                .next()
                .unwrap()
                .parse::<i64>()
                .unwrap()
                > reserved
        );
        watches.wait(order.id, order.revision);
        let final_order = fulfillment(&clients, &actor, order);
        watches.wait(final_order.id, final_order.revision);
        watches.stop();
        ready_and_cancelled(&mut reception, customer.id, product, &clients);
        (clients, actor, final_order, request)
    })
    .await
    .unwrap();
    verify_database(&pool, &scenario.server.authentication.session, &final_order).await;
    verify_sync_projection(
        &pool,
        &scenario.reception_auth.session,
        &scenario.server.authentication.session,
        &scenario.server.branch_scope,
        &scenario.server.scope,
    )
    .await;
    verify_restart(
        &mut scenario,
        manager_directory.path(),
        clients,
        actor,
        final_order,
        request,
    )
    .await;
    scenario
        .server
        .handle
        .graceful_shutdown(Some(Duration::from_millis(200)));
}
async fn order_reception(scenario: &CatalogScenario) -> CustomerTestClient {
    let tenant = scenario.server.authentication.session.tenant_id;
    let mut reception = CustomerTestClient::new(
        scenario.server.authentication.clone(),
        scenario.server.device_id,
        [11; 32],
        &scenario.server.branch_scope,
        &scenario.endpoint,
        &scenario.trust,
    );
    authorize_draft_writer(&mut reception, tenant);
    let (_, catalog_actor) = catalog_local_authority(
        reception.directory.path(),
        &scenario.server.authentication.session,
    );
    cycle(
        replication(
            reception.directory.path(),
            reception.store.clone(),
            scenario.server.authentication.clone(),
            [11; 32],
            &scenario.server.scope,
            &scenario.endpoint,
            &scenario.trust,
        ),
        catalog_actor,
    )
    .await
    .unwrap();
    reception
}
async fn verify_restart(
    scenario: &mut CatalogScenario,
    manager_directory: &Path,
    clients: ApprovalClients,
    actor: AuthorizationContext,
    final_order: OrderRecord,
    request: ConfirmOrder,
) {
    scenario
        .server
        .handle
        .graceful_shutdown(Some(Duration::from_millis(200)));
    tokio::time::sleep(Duration::from_millis(500)).await;
    let database = SyncDatabase::connect(&scenario.database, 4).await.unwrap();
    database.migrate().await.unwrap();
    let control = ControlPlane::new(
        ControlDatabase::connect(&scenario.database, 4)
            .await
            .unwrap()
            .pool(),
        TokenKey::new([9; 32]),
    );
    scenario.server.state = ServerState::new(
        control,
        SyncCoordinator::new(&database, direct_test_domains(&database)),
    );
    scenario.server.handle = start_server(
        scenario.server.address,
        scenario.server.state.clone(),
        &scenario.certificate,
        &scenario.key,
    )
    .await;
    // Reopen encrypted client credentials rather than reusing an in-memory session after restart.
    let restarted = approval_connection(
        manager_directory,
        clients.manager_auth.clone(),
        [12; 32],
        scenario,
    );
    tokio::task::spawn_blocking(move || {
        let watches = OrderWatches::new(&clients, &actor);
        watches.wait(final_order.id, final_order.revision);
        watches.stop();
        let page = restarted
            .orders(
                &clients.manager_actor,
                &ListOrders {
                    after: None,
                    limit: 1,
                },
                Some(final_order.id),
                UnixMillis(i64::MAX),
            )
            .unwrap();
        assert_eq!(page.items[0].state, OrderState::Delivered);
        lifecycle::assert_frozen(
            &page.items[0].source.quotation,
            &final_order.source.quotation,
        );
        let reception_page = clients
            .reception
            .orders(
                &actor,
                &ListOrders {
                    after: None,
                    limit: 100,
                },
                None,
                UnixMillis(i64::MAX),
            )
            .unwrap();
        assert_eq!(
            reception_page
                .items
                .iter()
                .find(|o| o.id == final_order.id)
                .unwrap()
                .revision,
            final_order.revision
        );
        assert_eq!(
            OrderServer::transition(&*clients.reception, &actor, &request, UnixMillis(i64::MAX))
                .unwrap()
                .id,
            final_order.id
        );
    })
    .await
    .unwrap();
}

fn accept(
    reception: &mut DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    dimensions: Option<eitmad_contracts::furniture::FurnitureDimensions>,
    clients: &ApprovalClients,
) -> QuotationRecord {
    let mut intent = draft_intent(customer, target);
    intent.lines[0].configuration.dimensions = dimensions;
    let original = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft { intent },
        )
        .unwrap();
    let actor = &reception.client.actor;
    DiscountApprovalServer::transition(
        &*clients.reception,
        actor,
        &eitmad_contracts::quotation_approval::ConfirmDiscountApproval {
            scope: actor.scope.clone(),
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            action: eitmad_contracts::quotation_approval::DiscountApprovalAction::Refresh(
                original.snapshot.clone(),
            ),
        },
        UnixMillis(i64::MAX),
    )
    .unwrap();
    let pending = lifecycle::current(&*clients.reception, actor, original.snapshot.id);
    let issued = clients
        .reception
        .quotation_transition(
            actor,
            &lifecycle::issue_request(&pending, &actor.scope),
            UnixMillis(i64::MAX),
        )
        .unwrap();
    let request = ConfirmQuotation {
        scope: actor.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: QuotationAction::Accept(AcceptQuotation {
            draft_id: issued.quotation.id,
            expected_revision: issued.revision,
            method: AcceptanceMethod::Phone,
            note: Some("وافق العميل".into()),
        }),
    };
    assert_eq!(
        clients.manager.quotation_transition(
            &clients.manager_actor,
            &request,
            UnixMillis(i64::MAX)
        ),
        Err(eitmad_pricing::QuotationError::Denied)
    );
    let accepted = clients
        .reception
        .quotation_transition(actor, &request, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(accepted.quotation, original.snapshot);
    assert_eq!(
        accepted.acceptance.as_ref().unwrap().document_revision,
        accepted.document_revision
    );
    accepted
}
fn request(scope: &ScopeRef, action: OrderAction) -> ConfirmOrder {
    ConfirmOrder {
        scope: scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action,
    }
}
fn send(
    client: &dyn OrderServer,
    actor: &AuthorizationContext,
    request: &ConfirmOrder,
) -> Result<OrderRecord, E> {
    client.transition(actor, request, UnixMillis(i64::MAX))
}
fn competing_conversion(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    accepted: &QuotationRecord,
) -> (OrderRecord, ConfirmOrder) {
    let request = request(
        &actor.scope,
        OrderAction::Convert(ConvertQuotation {
            draft_id: accepted.quotation.id,
            expected_revision: accepted.revision,
        }),
    );
    assert_eq!(
        send(&*clients.manager, &clients.manager_actor, &request),
        Err(E::Denied)
    );
    let competitor = ConfirmOrder {
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        ..request.clone()
    };
    let (a, b) = std::thread::scope(|threads| {
        let first = threads.spawn(|| send(&*clients.reception, actor, &request));
        let second =
            threads.spawn(|| send(&*clients.pure_reception, &clients.pure_actor, &competitor));
        (
            first.join().unwrap().unwrap(),
            second.join().unwrap().unwrap(),
        )
    });
    assert_eq!(a.id, b.id);
    assert_eq!(a.number, b.number);
    assert!(a.number.starts_with("OR-"));
    assert_eq!(a.source.quotation, accepted.quotation);
    assert_eq!(a.source.acceptance, accepted.acceptance);
    assert_eq!(a.work.len(), 1);
    assert_eq!(
        a.work[0].line_ids,
        accepted
            .quotation
            .intent
            .lines
            .iter()
            .map(|l| l.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(send(&*clients.reception, actor, &request).unwrap().id, a.id);
    let changed = ConfirmOrder {
        action: OrderAction::Deliver(RecordOrderDelivery {
            order_id: a.id,
            expected_revision: 1,
            recipient: "مستلم".into(),
            method: AcceptanceMethod::InPerson,
            note: None,
        }),
        ..request.clone()
    };
    assert_eq!(send(&*clients.reception, actor, &changed), Err(E::Invalid));
    assert_eq!(
        lifecycle::current(&*clients.reception, actor, accepted.quotation.id).state,
        QuotationState::Converted
    );
    (a, request)
}
fn fulfillment(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    mut order: OrderRecord,
) -> OrderRecord {
    let commercial = order.source.quotation.clone();
    let cancel = request(
        &actor.scope,
        OrderAction::Cancel(CancelOrder {
            order_id: order.id,
            expected_revision: order.revision,
            reason: "اختبار رفض".into(),
        }),
    );
    assert_eq!(
        send(&*clients.pure_reception, &clients.pure_actor, &cancel),
        Err(E::Denied)
    );
    let delivery = request(
        &actor.scope,
        OrderAction::Deliver(RecordOrderDelivery {
            order_id: order.id,
            expected_revision: order.revision,
            recipient: "مستلم تجريبي".into(),
            method: AcceptanceMethod::Written,
            note: None,
        }),
    );
    assert_eq!(
        send(&*clients.reception, actor, &delivery),
        Err(E::Conflict)
    );
    let mut work = TransitionOrderWork {
        order_id: order.id,
        expected_revision: order.revision,
        work_id: order.work[0].id,
        due_at: Some(UnixMillis(eitmad_authorization::now().0 + 86_400_000)),
        assignment: Some("ورشة تجريبية".into()),
    };
    let start = request(
        &clients.manager_actor.scope,
        OrderAction::StartWork(work.clone()),
    );
    assert_eq!(
        send(&*clients.pure_reception, &clients.pure_actor, &start),
        Err(E::Denied)
    );
    order = send(&*clients.manager, &clients.manager_actor, &start).unwrap();
    assert_eq!(order.state, OrderState::InProduction);
    work.expected_revision = order.revision;
    order = send(
        &*clients.manager,
        &clients.manager_actor,
        &request(
            &clients.manager_actor.scope,
            OrderAction::CompleteWork(work),
        ),
    )
    .unwrap();
    assert_eq!(order.state, OrderState::Ready);
    let edit = request(
        &clients.manager_actor.scope,
        OrderAction::EditFulfillment(EditOrderFulfillment {
            order_id: order.id,
            expected_revision: order.revision,
            note: Some("تغليف عند التسليم".into()),
        }),
    );
    order = send(&*clients.manager, &clients.manager_actor, &edit).unwrap();
    assert_eq!(
        send(
            &*clients.manager,
            &clients.manager_actor,
            &request(
                &clients.manager_actor.scope,
                OrderAction::EditFulfillment(EditOrderFulfillment {
                    order_id: order.id,
                    expected_revision: 1,
                    note: None
                })
            )
        ),
        Err(E::Conflict)
    );
    lifecycle::assert_frozen(&order.source.quotation, &commercial);
    deliver(clients, actor, order)
}
fn deliver(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    mut order: OrderRecord,
) -> OrderRecord {
    let delivered = request(
        &actor.scope,
        OrderAction::Deliver(RecordOrderDelivery {
            order_id: order.id,
            expected_revision: order.revision,
            recipient: "مستلم تجريبي".into(),
            method: AcceptanceMethod::Written,
            note: Some("تم الاستلام".into()),
        }),
    );
    order = send(&*clients.reception, actor, &delivered).unwrap();
    assert_eq!(order.state, OrderState::Delivered);
    assert_eq!(
        send(&*clients.reception, actor, &delivered)
            .unwrap()
            .delivery,
        order.delivery
    );
    let duplicate = ConfirmOrder {
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        ..delivered
    };
    assert_eq!(
        send(&*clients.reception, actor, &duplicate),
        Err(E::Conflict)
    );
    let cancel = request(
        &clients.manager_actor.scope,
        OrderAction::Cancel(CancelOrder {
            order_id: order.id,
            expected_revision: order.revision,
            reason: "اختبار".into(),
        }),
    );
    assert_eq!(
        send(&*clients.manager, &clients.manager_actor, &cancel),
        Err(E::Conflict)
    );

    order
}
fn ready_and_cancelled(
    reception: &mut DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
) {
    let accepted = accept(reception, customer, target, None, clients);
    let actor = &reception.client.actor;
    let order = send(
        &*clients.reception,
        actor,
        &request(
            &actor.scope,
            OrderAction::Convert(ConvertQuotation {
                draft_id: accepted.quotation.id,
                expected_revision: accepted.revision,
            }),
        ),
    )
    .unwrap();
    assert_eq!(order.state, OrderState::Ready);
    assert!(order.work.is_empty());
    let cancelled = send(
        &*clients.manager,
        &clients.manager_actor,
        &request(
            &clients.manager_actor.scope,
            OrderAction::Cancel(CancelOrder {
                order_id: order.id,
                expected_revision: order.revision,
                reason: "ألغى العميل الطلب".into(),
            }),
        ),
    )
    .unwrap();
    assert_eq!(cancelled.state, OrderState::Cancelled);
}
async fn verify_database(
    pool: &sqlx::PgPool,
    actor: &AuthenticatedServerSession,
    order: &OrderRecord,
) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(actor.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM sync.orders WHERE draft_id=$1")
        .bind(order.source.quotation.id.value())
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let deliveries: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sync.order_deliveries WHERE order_id=$1")
            .bind(order.id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(deliveries, 1);
    tx.commit().await.unwrap();
    let server = eitmad_sync_plane::OrderServer::new(pool.clone());
    let foreign = ReadOrders {
        scope: ScopeRef {
            kind: ScopeKind::parse("branch").unwrap(),
            id: ScopeId::new(Uuid::new_v4()),
        },
        query: ListOrders {
            after: None,
            limit: 100,
        },
        order_id: None,
    };
    assert_eq!(server.list(actor, &foreign).await, Err(E::Denied));
    let unscoped: i64 = sqlx::query_scalar("SELECT count(*) FROM sync.orders")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(unscoped, 0);
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(actor.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE sync.order_history SET revision=revision WHERE order_id=$1")
            .bind(order.id)
            .execute(&mut *tx)
            .await
            .is_err()
    );
}

async fn assign_second_receptionist(pool: &sqlx::PgPool, scenario: &CatalogScenario) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(
            scenario
                .reception_auth
                .session
                .tenant_id
                .value()
                .to_string(),
        )
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.receptionist.v1','branch',$3,1)").bind(scenario.reception_auth.session.tenant_id.value()).bind(scenario.reception_auth.session.user_id.value()).bind(scenario.server.branch_scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}

type WatchHandle = (
    Arc<std::sync::atomic::AtomicBool>,
    std::thread::JoinHandle<Result<(), E>>,
);
struct OrderWatches {
    receivers: Vec<std::sync::mpsc::Receiver<OrderNotice>>,
    handles: Vec<WatchHandle>,
}
impl OrderWatches {
    fn new(clients: &ApprovalClients, actor: &AuthorizationContext) -> Self {
        let mut result = Self {
            receivers: vec![],
            handles: vec![],
        };
        for (client, actor) in [
            (clients.reception.clone(), actor.clone()),
            (clients.manager.clone(), clients.manager_actor.clone()),
        ] {
            let (send, receive) = std::sync::mpsc::channel();
            let (ready, started) = std::sync::mpsc::channel();
            let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stop = cancel.clone();
            let handle = std::thread::spawn(move || {
                let outcome = OrderServer::watch(&*client, &actor, &stop, &mut |notice| {
                    let _ = ready.send(());
                    let _ = send.send(notice);
                });
                assert!(outcome.is_ok(), "order watch ended: {outcome:?}");
                outcome
            });
            started
                .recv_timeout(Duration::from_secs(10))
                .expect("durable order replay ready");
            result.receivers.push(receive);
            result.handles.push((cancel, handle));
        }
        result
    }
    fn wait(&self, id: Uuid, revision: u64) {
        for receiver in &self.receivers {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            loop {
                let notice = receiver
                    .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .unwrap();
                if notice.order_id == id && notice.revision >= revision {
                    break;
                }
            }
        }
    }
    fn stop(mut self) {
        for (cancel, _) in &self.handles {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        }
        for (_, handle) in self.handles.drain(..) {
            handle.join().unwrap().unwrap();
        }
    }
}
impl Drop for OrderWatches {
    fn drop(&mut self) {
        for (cancel, _) in &self.handles {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        }
    }
}
async fn conversion_rollback(
    pool: &sqlx::PgPool,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    accepted: &QuotationRecord,
) -> i64 {
    let server = eitmad_sync_plane::OrderServer::new(pool.clone());
    let request = request(
        scope,
        OrderAction::Convert(ConvertQuotation {
            draft_id: accepted.quotation.id,
            expected_revision: accepted.revision,
        }),
    );
    assert_eq!(
        server
            .transition(
                actor,
                &request,
                CorrelationId::new(Uuid::new_v4()),
                UnixMillis(accepted.valid_until.unwrap().0 + 1)
            )
            .await,
        Err(E::Conflict)
    );
    sqlx::raw_sql("CREATE FUNCTION audit.fail_order_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.order.convert.v1' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_order_test BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION audit.fail_order_test();").execute(pool).await.unwrap();
    let result = server
        .transition(
            actor,
            &request,
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await;
    sqlx::raw_sql("DROP TRIGGER fail_order_test ON audit.server_records; DROP FUNCTION audit.fail_order_test();").execute(pool).await.unwrap();
    assert_eq!(result, Err(E::Unavailable));
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(actor.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    for query in [
        "SELECT count(*) FROM sync.orders",
        "SELECT count(*) FROM sync.order_history",
        "SELECT count(*) FROM sync.order_work_history",
        "SELECT count(*) FROM sync.order_receipts",
        "SELECT count(*) FROM sync.operations WHERE schema_id='eitmad.schema.order.v1'",
    ] {
        let count: i64 = sqlx::query_scalar(query).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(count, 0);
    }
    let reserved: i64 =
        sqlx::query_scalar("SELECT last_number FROM sync.order_numbers WHERE record_type='OR'")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(reserved, 1);
    tx.commit().await.unwrap();
    let quotation = eitmad_sync_plane::QuotationLifecycleServer::new(pool.clone())
        .list(
            actor,
            &ReadQuotations {
                scope: scope.clone(),
                query: ListQuotations {
                    after: None,
                    limit: 100,
                },
            },
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    assert_eq!(
        quotation
            .items
            .iter()
            .find(|q| q.quotation.id == accepted.quotation.id)
            .unwrap()
            .state,
        QuotationState::Accepted
    );
    reserved
}

async fn verify_sync_projection(
    pool: &sqlx::PgPool,
    reception: &AuthenticatedServerSession,
    manager: &AuthenticatedServerSession,
    branch: &ScopeRef,
    organization: &ScopeRef,
) {
    use base64::Engine as _;
    let database = SyncDatabase::from_pool(pool.clone());
    let coordinator = SyncCoordinator::new(&database, direct_test_domains(&database));
    let schema = SchemaId::parse(eitmad_orders::ORDER_SCHEMA).unwrap();
    for (actor, scope, private) in [(reception, branch, false), (manager, organization, true)] {
        let page = coordinator
            .pull(eitmad_sync_plane::PullPageRequest {
                session: actor,
                scope,
                schema_id: &schema,
                schema_version: 1,
                after: None,
                maximum_records: 100,
                correlation_id: CorrelationId::new(Uuid::new_v4()),
                now: eitmad_authorization::now(),
            })
            .await
            .unwrap();
        assert!(!page.records.is_empty());
        let mut assignments = false;
        for change in page.records {
            let payload = change.payload.unwrap();
            let value: OrderRecord = serde_json::from_slice(
                &base64::engine::general_purpose::STANDARD
                    .decode(payload.base64)
                    .unwrap(),
            )
            .unwrap();
            assignments |= value.work.iter().any(|w| w.assignment.is_some());
        }
        assert_eq!(assignments, private);
    }
}
