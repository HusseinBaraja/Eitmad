// Direct IPC authorization and durable uncertain/rejected order intent.
use super::*;
use eitmad_contracts::order::{
    CancelOrder, ConfirmOrder, ConvertQuotation, ListOrders, OrderNotice, OrderPage, OrderRecord,
};
use eitmad_orders::{OrderError as E, OrderServer};
use std::sync::{Mutex, atomic::AtomicUsize};
struct Server {
    error: Mutex<E>,
    calls: AtomicUsize,
}
impl OrderServer for Server {
    fn work_orders(
        &self,
        _: &AuthorizationContext,
        _: &eitmad_contracts::work_order::ListWorkOrders,
        _: UnixMillis,
    ) -> Result<eitmad_contracts::work_order::WorkOrderPage, E> {
        Err(E::Unavailable)
    }

    fn transition(
        &self,
        _: &AuthorizationContext,
        _: &ConfirmOrder,
        _: UnixMillis,
    ) -> Result<OrderRecord, E> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(*self.error.lock().unwrap())
    }
    fn orders(
        &self,
        _: &AuthorizationContext,
        _: &ListOrders,
        _: Option<Uuid>,
        _: UnixMillis,
    ) -> Result<OrderPage, E> {
        Err(*self.error.lock().unwrap())
    }
    fn watch(
        &self,
        _: &AuthorizationContext,
        _: &AtomicBool,
        _: &mut dyn FnMut(OrderNotice),
    ) -> Result<(), E> {
        Err(E::Unavailable)
    }
}
fn receptionist(dispatcher: &ProductDispatcher) {
    let actor = branch_authorization();
    let mutation = MutationContext {
        authorization: actor.clone(),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        causation_id: None,
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        occurred_at: UnixMillis(1),
    };
    dispatcher
        .authorization
        .bootstrap_owner(
            &mutation,
            &RelationshipSubject {
                principal_id: actor.identity.principal_id,
                principal_kind: PrincipalKind::User,
            },
        )
        .unwrap();
    dispatcher
        .authorization
        .grant_relationship(
            &MutationContext {
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                ..mutation
            },
            &GrantScopeRelationship {
                expected_policy_version: 1,
                subject: RelationshipSubject {
                    principal_id: actor.identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
                relation: RelationId::parse(eitmad_authorization::RECEPTIONIST_RELATION).unwrap(),
            },
        )
        .unwrap();
}
#[tokio::test]
async fn orders_dispatch_retains_uncertain_intent_rejects_changed_retry_and_restores_after_restart()
{
    let (directory, dispatcher, _) = dispatcher();
    receptionist(&dispatcher);
    let server = Arc::new(Server {
        error: Mutex::new(E::Unavailable),
        calls: AtomicUsize::new(0),
    });
    let dispatcher = dispatcher.with_orders(server.clone());
    let command = Command::ConvertQuotation(ConvertQuotation {
        draft_id: eitmad_contracts::quotation_draft::QuotationDraftId::new(Uuid::new_v4()),
        expected_revision: 2,
    });
    let error = dispatcher
        .dispatch_command(branch_context(991), command.clone())
        .await
        .unwrap_err();
    assert_eq!(error.code.as_str(), "eitmad.error.order-unavailable.v1");
    let key = dispatcher
        .store
        .pending_orders(&branch_authorization())
        .unwrap()[0]
        .request
        .idempotency_key;
    let changed = Command::CancelOrder(CancelOrder {
        order_id: Uuid::new_v4(),
        expected_revision: 1,
        reason: "اختبار".into(),
    });
    assert_eq!(
        dispatcher
            .dispatch_command(branch_context(992), changed)
            .await
            .unwrap_err()
            .code
            .as_str(),
        "eitmad.error.authorization-denied.v1"
    );
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    drop(dispatcher);
    let store = AuthorityStore::open(directory.path()).unwrap();
    let dispatcher = ProductDispatcher::new(store, EventBroker::new()).with_orders(server.clone());
    let page = dispatcher
        .dispatch_query(
            branch_context(993),
            Query::Orders(ListOrders {
                after: None,
                limit: 100,
            }),
        )
        .await
        .unwrap();
    let QueryResult::Orders(page) = page else {
        panic!("wrong order query result")
    };
    assert!(!page.server_available);
    assert_eq!(page.pending[0].request.idempotency_key, key);
    let mut changed = command.clone();
    if let Command::ConvertQuotation(c) = &mut changed {
        c.expected_revision = 3;
    }
    assert_eq!(
        dispatcher
            .dispatch_command(branch_context(991), changed)
            .await
            .unwrap_err()
            .code
            .as_str(),
        "eitmad.error.order-invalid.v1"
    );
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    *server.error.lock().unwrap() = E::Conflict;
    assert_eq!(
        dispatcher
            .dispatch_command(branch_context(991), command)
            .await
            .unwrap_err()
            .code
            .as_str(),
        "eitmad.error.order-conflict.v1"
    );
    assert_eq!(
        dispatcher
            .store
            .pending_orders(&branch_authorization())
            .unwrap()[0]
            .rejected_code
            .as_deref(),
        Some("eitmad.error.order-conflict.v1")
    );
}
#[tokio::test]
async fn orders_dispatch_denies_unassigned_queries_and_invalid_bounds() {
    let (_directory, dispatcher, _) = dispatcher();
    let error = dispatcher
        .dispatch_query(
            context(995),
            Query::Orders(ListOrders {
                after: None,
                limit: 10,
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code.as_str(), "eitmad.error.authorization-denied.v1");
    receptionist(&dispatcher);
    let denied = dispatcher
        .dispatch_query(
            branch_context(999),
            Query::WorkOrders(eitmad_contracts::work_order::ListWorkOrders {
                after: None,
                limit: 10,
                order_id: None,
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(denied.code.as_str(), "eitmad.error.authorization-denied.v1");
    let error = dispatcher
        .dispatch_query(
            branch_context(996),
            Query::Orders(ListOrders {
                after: None,
                limit: 101,
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code.as_str(), "eitmad.error.order-invalid.v1");
    let mut foreign = branch_context(997);
    foreign.authorization.scope.id = ScopeId::new(Uuid::new_v4());
    let error = dispatcher
        .dispatch_query(
            foreign,
            Query::Orders(ListOrders {
                after: None,
                limit: 10,
            }),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code.as_str(), "eitmad.error.authorization-denied.v1");
}

#[tokio::test]
async fn customer_documents_deny_unassigned_and_cross_scope_reads() {
    let (_directory, dispatcher, _) = dispatcher();
    let id = Uuid::new_v4();
    let queries = [
        Query::QuotationCustomerDocument(eitmad_contracts::quotation_draft::GetQuotationDraft {
            draft_id: eitmad_contracts::quotation_draft::QuotationDraftId::new(id),
        }),
        Query::OrderCustomerDocument(eitmad_contracts::order::GetOrder { order_id: id }),
        Query::OrderQuotationCustomerDocument(eitmad_contracts::order::GetOrder { order_id: id }),
    ];
    for query in &queries {
        assert_eq!(
            dispatcher
                .dispatch_query(branch_context(1010), query.clone())
                .await
                .unwrap_err()
                .code
                .as_str(),
            "eitmad.error.authorization-denied.v1"
        );
    }
    receptionist(&dispatcher);
    for query in queries {
        let mut foreign = branch_context(1011);
        foreign.authorization.scope.id = ScopeId::new(Uuid::new_v4());
        assert_eq!(
            dispatcher
                .dispatch_query(foreign, query)
                .await
                .unwrap_err()
                .code
                .as_str(),
            "eitmad.error.authorization-denied.v1"
        );
    }
}

fn document_snapshot() -> eitmad_contracts::quotation_draft::QuotationDraftSnapshot {
    use eitmad_contracts::{pricing::*, product::*, quotation::*, quotation_draft::*};
    let actor = branch_authorization();
    let reference = ProductReference {
        scope: authorization().scope,
        product_id: ProductId::new(Uuid::new_v4()),
        variant_id: ProductVariantId::new(Uuid::new_v4()),
        revision: 1,
        schema_version: 1,
    };
    let target = PriceTarget::Product(reference);
    let line_id = Uuid::new_v4();
    let customer_id = eitmad_contracts::customer::CustomerId::new(Uuid::new_v4());
    let intent = EvaluateQuotation {
        customer: Some(QuotationCustomerIntent {
            id: customer_id,
            revision: 1,
        }),
        discount_basis_points: 500,
        lines: vec![QuotationLineIntent {
            id: line_id,
            configuration: eitmad_contracts::sales_catalog::CheckSalesConfiguration {
                selection: PriceSelection {
                    target: target.clone(),
                    price_revision: 1,
                    quantity: 2,
                    color_id: None,
                    handle_id: None,
                },
                dimensions: None,
            },
        }],
    };
    let evaluation = QuotationEvaluation {
        scope: actor.scope.clone(),
        customer: Some(QuotationCustomerSnapshot {
            id: customer_id,
            revision: 1,
            name: "عميل تجريبي A-12".into(),
            phone: "+967-777123456".into(),
            address: Some("عنوان تجريبي B-7".into()),
        }),
        lines: vec![EvaluatedQuotationLine {
            id: line_id,
            name: "مرتبة A-12".into(),
            description: "وصف محفوظ".into(),
            variant_name: "مفرد".into(),
            color_id: None,
            color_name: None,
            handle_id: None,
            handle_name: None,
            dimensions: None,
            quantity: 2,
            price: SellingPrice {
                snapshot: PublishedPrice {
                    target,
                    currency: "YER".into(),
                    selling_price_yer: 12500,
                    colors: vec![],
                    handles: vec![],
                    revision: 1,
                    confirmed_at: UnixMillis(1),
                },
                unit_price_yer: 12500,
                total_yer: 25000,
            },
        }],
        currency: "YER".into(),
        discount_basis_points: 500,
        totals: Some(DiscountTotal {
            subtotal_yer: 25000,
            discount_yer: 1250,
            total_yer: 23750,
            approval_required: false,
        }),
        errors: vec![],
        server_available: true,
    };
    QuotationDraftSnapshot {
        cancelled: false,
        id: QuotationDraftId::new(Uuid::new_v4()),
        revision: 1,
        intent,
        evaluation,
    }
}

fn document_source() -> eitmad_contracts::quotation_lifecycle::QuotationRecord {
    use eitmad_contracts::quotation_lifecycle::{QuotationRecord, QuotationState};
    let actor = branch_authorization();
    QuotationRecord {
        scope: actor.scope,
        organization_id: actor.tenant_id.value(),
        revision: 2,
        document_revision: 1,
        state: QuotationState::Accepted,
        quotation: document_snapshot(),
        number: Some("QT-2026-00001".into()),
        validity_days: 30,
        issued_at: Some(UnixMillis(1)),
        valid_until: Some(UnixMillis(i64::MAX)),
        approval_request_id: None,
        approval_fingerprint: Some("INTERNAL_APPROVAL".into()),
        changed_at: UnixMillis(1),
        changed_by: actor.identity.principal_id,
        cancellation_reason: None,
        acceptance: None,
        permitted_actions: vec![],
    }
}

#[tokio::test]
async fn customer_documents_read_confirmed_cache_and_server_denial_never_falls_back() {
    let (_directory, dispatcher, _) = dispatcher();
    receptionist(&dispatcher);
    let source = document_source();
    dispatcher
        .drafts
        .cache_quotation(
            &branch_authorization(),
            &source,
            CorrelationId::new(Uuid::new_v4()),
        )
        .unwrap();
    let value = OrderRecord {
        id: Uuid::new_v4(),
        scope: source.scope.clone(),
        organization_id: source.organization_id,
        revision: 1,
        number: "OR-2026-00001".into(),
        state: eitmad_contracts::order::OrderState::Ready,
        source: source.clone(),
        work: vec![],
        delivery: None,
        fulfillment_note: Some("INTERNAL_NOTE".into()),
        cancellation_reason: None,
        created_at: UnixMillis(2),
        changed_at: UnixMillis(3),
        changed_by: source.changed_by,
        permitted_actions: vec![],
    };
    cache_document_order(&dispatcher, &value);
    let queries = [
        Query::QuotationCustomerDocument(eitmad_contracts::quotation_draft::GetQuotationDraft {
            draft_id: source.quotation.id,
        }),
        Query::OrderCustomerDocument(eitmad_contracts::order::GetOrder { order_id: value.id }),
        Query::OrderQuotationCustomerDocument(eitmad_contracts::order::GetOrder {
            order_id: value.id,
        }),
    ];
    for (index, query) in queries.into_iter().enumerate() {
        let QueryResult::CustomerDocument(document) = dispatcher
            .dispatch_query(branch_context(1013), query)
            .await
            .unwrap()
        else {
            panic!("wrong document result");
        };
        assert!(document.can_print);
        assert!(!document.is_draft);
        assert_eq!(
            document.number.as_deref(),
            Some(if index == 1 {
                "OR-2026-00001"
            } else {
                "QT-2026-00001"
            })
        );
        assert_eq!(
            (
                document.subtotal_yer,
                document.discount_yer,
                document.total_yer
            ),
            (25000, 1250, 23750)
        );
        assert_eq!(document.lines[0].total_yer, 25000);
        let json = serde_json::to_string(&document).unwrap();
        assert!(!json.contains("INTERNAL"));
        assert!(!json.contains("approvalRequired"));
    }
    let dispatcher = dispatcher.with_orders(Arc::new(Server {
        error: Mutex::new(E::Denied),
        calls: AtomicUsize::new(0),
    }));
    for query in [
        Query::OrderCustomerDocument(eitmad_contracts::order::GetOrder { order_id: value.id }),
        Query::OrderQuotationCustomerDocument(eitmad_contracts::order::GetOrder {
            order_id: value.id,
        }),
    ] {
        assert_eq!(
            dispatcher
                .dispatch_query(branch_context(1014), query)
                .await
                .unwrap_err()
                .code
                .as_str(),
            "eitmad.error.authorization-denied.v1"
        );
    }
}

fn cache_document_order(dispatcher: &ProductDispatcher, value: &OrderRecord) {
    let audit = eitmad_observability_audit::MutationAuditRecord::from_authorization(
        &branch_authorization(),
        UnixMillis(3),
        CorrelationId::new(Uuid::new_v4()),
        "eitmad.order.cache.v1",
        eitmad_observability_audit::AuditTarget {
            kind: "order".into(),
            identifiers: vec![value.id.to_string()],
        },
    );
    dispatcher
        .store
        .transact_pricing(true, |tx| tx.cache_order(value, &audit))
        .unwrap();
}
