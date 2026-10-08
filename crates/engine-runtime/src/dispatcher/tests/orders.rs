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
        Err(E::Unavailable)
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
