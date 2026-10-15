use super::*;
use eitmad_engine_runtime::ManualRecovery;
use eitmad_observability_audit::MutationAuditRecord;
use eitmad_postgres_support::Row as _;

struct RecoveryFixture {
    reception: DraftTestClient,
    clients: ApprovalClients,
    _manager_directory: tempfile::TempDir,
    accepted: QuotationRecord,
    order: OrderRecord,
    retry: ConfirmOrder,
    pending_customer: Customer,
    pending_draft: QuotationDraft,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires isolated PostgreSQL, pg_dump/pg_restore, private artifacts and TLS certificates"]
async fn manual_two_client_recovery_retains_pending_work_and_accepted_receipts() {
    let _credentials = TestCredentials::new();
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut scenario = CatalogScenario::new().await;
    scenario.transfer_dependencies().await;
    scenario.publish_prices().await;
    let _pool = verify_server_permissions(&scenario).await;
    let fixture = prepare_fixture(&scenario).await;
    let root = required_path("EITMAD_RECOVERY_ARTIFACT_DIRECTORY");
    let category = pending_catalog(&scenario);
    let mut maintenance = fixture.reception.client.actor.clone();
    maintenance.scope = scenario.manager.scope.clone();
    backup_client(
        scenario.manager_store.path().parent().unwrap(),
        &scenario.manager,
        &root.join("catalog.sqlite3"),
    );
    backup_client(
        fixture.reception.client.directory.path(),
        &maintenance,
        &root.join("reception.sqlite3"),
    );
    let first_rows = sqlite_rows(&root.join("catalog.sqlite3"));
    let second_rows = sqlite_rows(&root.join("reception.sqlite3"));
    scenario.server.handle.handle.shutdown();
    wait_for_stopped_server(scenario.server.address).await;
    let started = Instant::now();
    let artifact_root = root.clone();
    scenario.database = tokio::task::spawn_blocking(move || postgres_recovery(&artifact_root))
        .await
        .unwrap();
    restore_client(
        scenario.manager_store.path().parent().unwrap(),
        &scenario.manager,
        &root.join("catalog.sqlite3"),
    );
    restore_client(
        fixture.reception.client.directory.path(),
        &maintenance,
        &root.join("reception.sqlite3"),
    );
    assert_sqlite_recovery(&first_rows, scenario.manager_store.path());
    assert_sqlite_recovery(&second_rows, fixture.reception.client.store.path());
    let database = restart_recovered_server(&mut scenario).await;
    let tenant = scenario.server.authentication.session.tenant_id;
    let before = server_counts(&database.pool(), tenant).await;
    let fixture = tokio::task::spawn_blocking(move || reconnect_fixture(fixture))
        .await
        .unwrap();
    let after = server_counts(&database.pool(), tenant).await;
    assert_eq!(
        before.0, after.0,
        "receipt replay must not create another order or revision"
    );
    assert!(
        after.1 > before.1,
        "restored pending sync must reach the server"
    );
    cycle(scenario.manager_client.clone(), scenario.manager.clone())
        .await
        .unwrap();
    assert!(
        scenario
            .manager_store
            .pending_catalog_revisions(&scenario.manager.scope, None)
            .unwrap()
            .iter()
            .all(|r| r.identity().1 != category.id.value())
    );
    let report = serde_json::json!({
        "status": "passed", "recoveryMillis": started.elapsed().as_millis(),
        "clientsRestored": 2, "postgresRowsetsVerified": true, "sqliteRowsetsVerified": true,
        "acceptedOrderRevision": fixture.order.revision, "pendingCustomerConfirmed": true,
        "pendingDraftConfirmed": true, "pendingCatalogConfirmed": true, "duplicateOrderEffects": 0,
    });
    std::fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
    scenario.server.handle.handle.shutdown();
}

async fn prepare_fixture(scenario: &CatalogScenario) -> RecoveryFixture {
    let reception = order_reception(scenario).await;
    let customer = reception
        .customers
        .create(
            &reception.mutation(),
            &CreateCustomer {
                name: CustomerName::parse("عميل استعادة اختباري").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    let tenant = scenario.server.authentication.session.tenant_id;
    let (catalog, product, furniture) = draft_transfer_targets(scenario, tenant.value());
    let manager_directory = tempfile::tempdir().unwrap();
    let clients = lifecycle_clients(&reception, scenario, manager_directory.path()).await;
    let auth = scenario.server.authentication.clone();
    let endpoint = scenario.endpoint.clone();
    let trust = scenario.trust.clone();
    let dimensions = scenario.furniture.variants[0].dimensions.clone();
    tokio::task::spawn_blocking(move || {
        let mut reception =
            DraftTestClient::new(reception, auth, [11; 32], (&endpoint, &trust, &catalog));
        let accepted = accept(
            &mut reception,
            customer.id,
            furniture,
            Some(dimensions),
            None,
            &clients,
        );
        let (order, retry) = accept_without_reply(&reception, &clients, &accepted);
        let pending_customer = reception
            .client
            .customers
            .create(
                &reception.client.mutation(),
                &CreateCustomer {
                    name: CustomerName::parse("عميل معلق اختباري").unwrap(),
                    phone: CustomerPhone::parse("777654321").unwrap(),
                    address: None,
                    notes: None,
                },
            )
            .unwrap()
            .customer;
        let pending_draft = reception
            .drafts
            .create(
                &reception.client.mutation(),
                &CreateQuotationDraft {
                    intent: draft_intent(customer.id, product),
                },
            )
            .unwrap();
        RecoveryFixture {
            reception,
            clients,
            _manager_directory: manager_directory,
            accepted,
            order,
            retry,
            pending_customer,
            pending_draft,
        }
    })
    .await
    .unwrap()
}

fn accept_without_reply(
    reception: &DraftTestClient,
    clients: &ApprovalClients,
    accepted: &QuotationRecord,
) -> (OrderRecord, ConfirmOrder) {
    let actor = &reception.client.actor;
    let conversion = request(
        &actor.scope,
        OrderAction::Convert(ConvertQuotation {
            draft_id: accepted.quotation.id,
            expected_revision: accepted.revision,
        }),
    );
    let order = send(&*clients.reception, actor, &conversion).unwrap();
    let store = &reception.client.store;
    store
        .transact_pricing(true, |tx| {
            tx.cache_quotation(
                &actor.scope,
                accepted,
                &recovery_audit(actor, "eitmad.quotation.confirmed.v1"),
            )?;
            tx.cache_order(&order, &recovery_audit(actor, "eitmad.order.confirmed.v1"))
        })
        .unwrap();
    // Retain the exact intent before transport, then discard the accepted response locally.
    let retry = request(
        &actor.scope,
        OrderAction::EditFulfillment(EditOrderFulfillment {
            order_id: order.id,
            expected_revision: order.revision,
            note: Some("ملاحظة اختبار الاستعادة".into()),
        }),
    );
    store.stage_order(actor, &retry).unwrap();
    let updated = send(&*clients.reception, actor, &retry).unwrap();
    assert_eq!(updated.revision, order.revision + 1);
    (updated, retry)
}

fn pending_catalog(scenario: &CatalogScenario) -> ProductCategory {
    eitmad_product::ProductService::new(
        scenario.manager_store.clone(),
        AuthorizationService::new(scenario.manager_store.clone()),
    )
    .save_category(
        &mutation(&scenario.manager),
        &SaveProductCategory {
            id: None,
            expected_revision: None,
            name: "فئة معلقة للاختبار".into(),
            archived: false,
        },
    )
    .unwrap()
}

fn backup_client(directory: &Path, actor: &AuthorizationContext, backup: &Path) {
    let recovery = ManualRecovery::acquire(directory).unwrap();
    recovery
        .backup(actor, backup, eitmad_authorization::now())
        .unwrap();
}

fn restore_client(directory: &Path, actor: &AuthorizationContext, backup: &Path) {
    let mut recovery = ManualRecovery::acquire(directory).unwrap();
    recovery
        .restore(actor, backup, eitmad_authorization::now())
        .unwrap();
    let previous = AuthorityStore::recovery_artifacts(directory).unwrap();
    assert_eq!(previous.len(), 1);
    AuthorityStore::validate_backup(&previous[0].path).unwrap();
}

async fn restart_recovered_server(scenario: &mut CatalogScenario) -> SyncDatabase {
    let database = SyncDatabase::connect(&scenario.database, 4).await.unwrap();
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
    database
}

fn reconnect_fixture(mut fixture: RecoveryFixture) -> RecoveryFixture {
    let actor = &fixture.reception.client.actor;
    assert_eq!(
        fixture
            .reception
            .client
            .store
            .pending_orders(actor)
            .unwrap()[0]
            .request,
        fixture.retry
    );
    let replay = send(&*fixture.clients.reception, actor, &fixture.retry).unwrap();
    assert!(
        replay == fixture.order,
        "accepted order result changed after recovery"
    );
    fixture
        .reception
        .client
        .store
        .finish_order(actor, fixture.retry.idempotency_key, None)
        .unwrap();
    assert!(
        fixture
            .reception
            .client
            .store
            .pending_orders(actor)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        lifecycle::current(
            &*fixture.clients.reception,
            actor,
            fixture.accepted.quotation.id
        )
        .state,
        QuotationState::Converted
    );
    fixture.reception.client.restart_engine();
    fixture.reception.engine = draft_engine(&fixture.reception.client);
    fixture.reception.client.run();
    fixture.reception.run();
    assert_eq!(
        fixture
            .reception
            .client
            .store
            .get_customer(
                &fixture.reception.client.actor.scope,
                fixture.pending_customer.id
            )
            .unwrap()
            .unwrap()
            .sync_state,
        CustomerSyncState::Confirmed
    );
    assert_eq!(
        fixture
            .reception
            .get(fixture.pending_draft.snapshot.id)
            .sync_state,
        QuotationDraftSyncState::Confirmed
    );
    // Repeat after acknowledgements are durable; the caller compares central history counts.
    fixture.reception.client.restart_engine();
    fixture.reception.engine = draft_engine(&fixture.reception.client);
    fixture.reception.client.run();
    fixture.reception.run();
    fixture
}
fn recovery_audit(actor: &AuthorizationContext, operation: &str) -> MutationAuditRecord {
    MutationAuditRecord::from_authorization(
        actor,
        eitmad_authorization::now(),
        CorrelationId::new(Uuid::new_v4()),
        operation,
        AuditTarget {
            kind: "recovery-fixture".into(),
            identifiers: vec![],
        },
    )
}

fn sqlite_rows(path: &Path) -> BTreeMap<String, Vec<Vec<rusqlite::types::Value>>> {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let tables: Vec<String> = connection.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap()
        .query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect();
    tables
        .into_iter()
        .map(|table| {
            let mut query = connection
                .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))
                .unwrap();
            let columns = query.column_count();
            let rows = query
                .query_map([], |r| {
                    (0..columns)
                        .map(|c| r.get(c))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap()
                .map(Result::unwrap)
                .collect();
            (table, rows)
        })
        .collect()
}

fn assert_sqlite_recovery(
    before: &BTreeMap<String, Vec<Vec<rusqlite::types::Value>>>,
    path: &Path,
) {
    let after = sqlite_rows(path);
    for (table, rows) in before {
        if table == "mutation_audit" {
            assert!(
                rows.iter().all(|row| after[table].contains(row)),
                "audit history must survive restore"
            );
            assert_eq!(after[table].len(), rows.len() + 1);
        } else {
            assert!(rows == &after[table], "restored table {table} differs");
        }
    }
}

async fn wait_for_stopped_server(address: SocketAddr) {
    for _ in 0..100 {
        if std::net::TcpStream::connect_timeout(&address, Duration::from_millis(20)).is_err() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("server did not stop before backup");
}

async fn server_counts(pool: &sqlx::PgPool, tenant: TenantId) -> (i64, i64) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(tenant.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let orders = sqlx::query_scalar("SELECT count(*) FROM sync.order_history WHERE tenant_id=$1")
        .bind(tenant.value())
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let operations = sqlx::query_scalar("SELECT count(*) FROM sync.operations WHERE tenant_id=$1")
        .bind(tenant.value())
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    (orders, operations)
}

fn postgres_recovery(root: &Path) -> String {
    let original = url::Url::parse(&env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap()).unwrap();
    let bin = required_path("EITMAD_RECOVERY_PG_BIN");
    let password = env::var("EITMAD_RECOVERY_ADMIN_PASSWORD").unwrap();
    let database = format!("eitmad_recovered_{}", Uuid::new_v4().simple());
    let backup = root.join("server.dump");
    assert!(!backup.exists());
    let run = |tool: &str, extra: &[&std::ffi::OsStr]| {
        let output = std::process::Command::new(bin.join(format!("{tool}.exe")))
            .env("PGPASSWORD", &password)
            .args([
                "--host",
                original.host_str().unwrap(),
                "--port",
                &original.port().unwrap().to_string(),
                "--username",
                "postgres",
            ])
            .args(extra)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{tool} failed; raw database diagnostics are withheld"
        );
        assert!(
            output.stderr.is_empty(),
            "{tool} reported diagnostics; raw output is withheld"
        );
    };
    let source = original.path().trim_start_matches('/');
    run(
        "pg_dump",
        &[
            "--format=custom".as_ref(),
            "--file".as_ref(),
            backup.as_os_str(),
            "--dbname".as_ref(),
            source.as_ref(),
        ],
    );
    run(
        "createdb",
        &[
            "--owner".as_ref(),
            original.username().as_ref(),
            database.as_ref(),
        ],
    );
    run(
        "pg_restore",
        &[
            "--exit-on-error".as_ref(),
            "--single-transaction".as_ref(),
            "--no-owner".as_ref(),
            "--role".as_ref(),
            original.username().as_ref(),
            "--dbname".as_ref(),
            database.as_ref(),
            backup.as_os_str(),
        ],
    );
    let mut restored = original.clone();
    restored.set_path(&format!("/{database}"));
    let mut admin_source = original;
    admin_source.set_username("postgres").unwrap();
    admin_source.set_password(Some(&password)).unwrap();
    let mut admin_restored = admin_source.clone();
    admin_restored.set_path(&format!("/{database}"));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        assert_eq!(
            postgres_rows(admin_source.as_str()).await,
            postgres_rows(admin_restored.as_str()).await
        );
    });
    restored.to_string()
}

async fn postgres_rows(database: &str) -> BTreeMap<String, String> {
    let pool = sqlx::PgPool::connect(database).await.unwrap();
    let tables = sqlx::query("SELECT schemaname,tablename FROM pg_tables WHERE schemaname NOT IN ('pg_catalog','information_schema') ORDER BY 1,2").fetch_all(&pool).await.unwrap();
    let mut rows = BTreeMap::new();
    for row in tables {
        let schema: String = row.get(0);
        let table: String = row.get(1);
        let digest: String = sqlx::query_scalar(&format!("SELECT md5(coalesce(string_agg(to_jsonb(t)::text, E'\\n' ORDER BY to_jsonb(t)::text),'')) FROM \"{schema}\".\"{table}\" t")).fetch_one(&pool).await.unwrap();
        rows.insert(format!("{schema}.{table}"), digest);
    }
    pool.close().await;
    rows
}
