//! Live `PostgreSQL` proof at the authenticated-session sync authority boundary.
use super::*;
use eitmad_contracts::{
    customer::*, identity::UserId, quotation_draft::*, sync::ReconcileResolution,
};
use eitmad_customer::CustomerError;
use eitmad_pricing::QuotationDraftError;
use sha2::{Digest as _, Sha256};

async fn manager_relationship(
    pool: &sqlx::PgPool,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    enabled: bool,
) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(actor.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    if enabled {
        sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.manager.v1','organization',$3,1)")
            .bind(actor.tenant_id.value()).bind(actor.user_id.value()).bind(scope.id.value()).execute(&mut *tx).await.unwrap();
    } else {
        sqlx::query("DELETE FROM control.relationship_tuples WHERE tenant_id=$1 AND subject_principal_id=$2 AND relation='eitmad.relation.organization.manager.v1'")
            .bind(actor.tenant_id.value()).bind(actor.user_id.value()).execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and development TLS certificates"]
async fn resolution_reconciliation_serializes_late_requests_preserves_receipts_and_current_authority()
 {
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
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(server.authentication.session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    for schema in [
        eitmad_customer::CUSTOMER_SCHEMA_ID,
        eitmad_pricing::QUOTATION_DRAFT_SCHEMA,
    ] {
        sqlx::query("INSERT INTO sync.scopes (tenant_id,scope_kind,scope_id,schema_id,mode,head_sequence) VALUES ($1,'branch',$2,$3,'local_first',0)")
            .bind(server.authentication.session.tenant_id.value()).bind(server.branch_scope.id.value()).bind(schema).execute(&mut *tx).await.unwrap();
    }
    tx.commit().await.unwrap();
    let original = &server.authentication.session;
    // These trusted contexts test the sync boundary; HTTP credential sign-in is covered elsewhere.
    let manager_b = AuthenticatedServerSession {
        user_id: UserId::new(Uuid::new_v4()),
        ..original.clone()
    };
    manager_relationship(&db.pool(), &manager_b, &server.scope, true).await;
    let input = customer_input(&server);
    let recovery = ReconcileResolution {
        original_user: original.user_id,
        request: input.clone(),
    };
    let correlation = CorrelationId::new(Uuid::new_v4());
    let now = eitmad_authorization::now();
    manager_relationship(&db.pool(), original, &server.scope, false).await;
    assert_eq!(
        coordinator
            .reconcile_customer_resolution(original, &recovery, correlation, now)
            .await,
        Err(CustomerError::Denied)
    );
    let (one, two) = tokio::join!(
        coordinator.reconcile_customer_resolution(&manager_b, &recovery, correlation, now),
        coordinator.reconcile_customer_resolution(&manager_b, &recovery, correlation, now)
    );
    assert_eq!(one.unwrap(), None);
    assert_eq!(two.unwrap(), None);
    manager_relationship(&db.pool(), original, &server.scope, true).await;
    assert_eq!(
        coordinator
            .resolve_customer(original, &input, correlation, now)
            .await,
        Err(CustomerError::IdempotencyMismatch)
    );
    check_draft_closure(&coordinator, &manager_b, original, &server.branch_scope).await;
    check_original_receipt(&db, &coordinator, &server, &manager_b, input).await;
    server
        .handle
        .graceful_shutdown(Some(Duration::from_millis(100)));
}

fn customer_input(server: &ProvisionedServer) -> ConfirmCustomerResolution {
    ConfirmCustomerResolution {
        scope: server.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        command: ResolveCustomerConflict {
            customer: GetCustomer {
                customer_id: CustomerId::new(Uuid::new_v4()),
                branch_id: Some(server.branch_scope.id),
            },
            source: CustomerResolutionSource::Conflict(CustomerConflictSource {
                conflict_id: eitmad_contracts::sync::ConflictId::new(Uuid::new_v4()),
            }),
            expected_server_revision: 1,
            contact: CustomerContact {
                name: CustomerName::parse("عميل تجريبي").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        },
    }
}
async fn check_draft_closure(
    coordinator: &SyncCoordinator,
    manager: &AuthenticatedServerSession,
    original: &AuthenticatedServerSession,
    branch: &ScopeRef,
) {
    let request = ConfirmQuotationDraftResolution {
        scope: branch.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        command: ResolveQuotationDraft {
            draft_id: QuotationDraftId::new(Uuid::new_v4()),
            branch_id: branch.id,
            expected_server_revision: 1,
            source: QuotationDraftResolutionSource::Conflict(QuotationDraftConflictSource {
                conflict_id: eitmad_contracts::sync::ConflictId::new(Uuid::new_v4()),
            }),
            choice: QuotationDraftResolutionChoice::KeepCurrent,
        },
    };
    let recovery = ReconcileResolution {
        original_user: original.user_id,
        request: request.clone(),
    };
    let now = eitmad_authorization::now();
    let correlation = CorrelationId::new(Uuid::new_v4());
    assert_eq!(
        coordinator
            .reconcile_quotation_draft_resolution(manager, &recovery, correlation, now)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        coordinator
            .resolve_quotation_draft(original, &request, correlation, now)
            .await,
        Err(QuotationDraftError::IdempotencyMismatch)
    );
    let forged = AuthenticatedServerSession {
        user_id: UserId::new(Uuid::new_v4()),
        ..manager.clone()
    };
    assert_eq!(
        coordinator
            .reconcile_quotation_draft_resolution(&forged, &recovery, correlation, now)
            .await,
        Err(QuotationDraftError::Denied)
    );
}
async fn check_original_receipt(
    db: &SyncDatabase,
    coordinator: &SyncCoordinator,
    server: &ProvisionedServer,
    manager: &AuthenticatedServerSession,
    mut input: ConfirmCustomerResolution,
) {
    // Exercise an actual customer mutation and its original audit, then lose the caller's receipt.
    let change =
        customer_visibility_change(&server.branch_scope, input.command.customer.customer_id);
    coordinator
        .apply_local_operation(
            &server.authentication.session,
            &LocalOperationDraft {
                change_id: change.change_id,
                scope: change.scope.clone(),
                schema_id: SchemaId::parse(eitmad_customer::CUSTOMER_SCHEMA_ID).unwrap(),
                schema_version: 1,
                record_id: change.record_id,
                operation: change.operation,
                base_revision: None,
                idempotency_key: change.idempotency_key,
                payload: change.payload.clone(),
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap();
    input.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
    input.command.source = CustomerResolutionSource::PendingReview(Box::new(change));
    let correlation = CorrelationId::new(Uuid::new_v4());
    let now = eitmad_authorization::now();
    let receipt = coordinator
        .resolve_customer(&server.authentication.session, &input, correlation, now)
        .await
        .unwrap();
    let recovery = ReconcileResolution {
        original_user: server.authentication.session.user_id,
        request: input.clone(),
    };
    manager_relationship(
        &db.pool(),
        &server.authentication.session,
        &server.scope,
        false,
    )
    .await;
    assert_eq!(
        coordinator
            .reconcile_customer_resolution(manager, &recovery, correlation, now)
            .await
            .unwrap(),
        Some(receipt.clone())
    );
    let changed = ReconcileResolution {
        request: ConfirmCustomerResolution {
            command: ResolveCustomerConflict {
                expected_server_revision: 99,
                ..input.command
            },
            ..input
        },
        ..recovery
    };
    assert_eq!(
        coordinator
            .reconcile_customer_resolution(manager, &changed, correlation, now)
            .await,
        Err(CustomerError::IdempotencyMismatch)
    );
    let mut tx = db.pool().begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(manager.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    let authors: Vec<Uuid> = sqlx::query_scalar("SELECT principal_id FROM audit.server_records WHERE operation='eitmad.customer.resolve-conflict.v1' AND outcome='succeeded' AND idempotency_key=$1")
        .bind(receipt.idempotency_key.value()).fetch_all(&mut *tx).await.unwrap();
    assert_eq!(authors, vec![server.authentication.session.user_id.value()]);
    let fingerprint: Vec<u8> = sqlx::query_scalar(
        "SELECT request_fingerprint FROM sync.idempotency_results WHERE idempotency_key=$1",
    )
    .bind(receipt.idempotency_key.value())
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        fingerprint,
        Sha256::digest(serde_json::to_vec(&(recovery.original_user, &recovery.request)).unwrap())
            .to_vec()
    );
}
fn customer_visibility_change(scope: &ScopeRef, id: CustomerId) -> ChangeRecord {
    let schema = SchemaId::parse(eitmad_customer::CUSTOMER_SCHEMA_ID).unwrap();
    ChangeRecord {
        change_id: ChangeId::new(Uuid::new_v4()),
        scope: scope.clone(),
        record_id: RecordId::new(id.value()),
        operation: ChangeOperation::Upsert,
        base_revision: None,
        revision: 1,
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        changed_at: eitmad_authorization::now(),
        merge: None,
        payload: Some(EncodedDomainPayload {
            schema_id: schema,
            schema_version: 1,
            base64: base64::engine::general_purpose::STANDARD.encode(
                serde_json::to_vec(&CustomerSyncPayload {
                    customer_id: id,
                    name: CustomerName::parse("عميل تجريبي").unwrap(),
                    phone: CustomerPhone::parse("777123456").unwrap(),
                    address: None,
                    notes: None,
                    revision: 1,
                })
                .unwrap(),
            ),
        }),
    }
}
