//! Startup worker recovery under a simulated outage followed by real authenticated TLS delivery.
use super::*;
use eitmad_contracts::{quotation_approval::*, quotation_lifecycle::*};
use eitmad_engine_runtime::{ProductDispatcher, local_ipc::EventBroker};
use eitmad_pricing::{ApprovalError, DiscountApprovalServer, QuotationServer};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct Connection {
    offline: AtomicBool,
    attempts: AtomicUsize,
    client: Arc<eitmad_server_connection::DirectDiscountApprovalClient>,
}
impl DiscountApprovalServer for Connection {
    fn transition(
        &self,
        actor: &AuthorizationContext,
        request: &ConfirmDiscountApproval,
        deadline: UnixMillis,
    ) -> Result<Option<DiscountApproval>, ApprovalError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        if self.offline.load(Ordering::SeqCst) {
            return Err(ApprovalError::Unavailable);
        }
        self.client.transition(actor, request, deadline)
    }
    fn list(
        &self,
        actor: &AuthorizationContext,
        query: &ListDiscountApprovals,
        deadline: UnixMillis,
    ) -> Result<DiscountApprovalPage, ApprovalError> {
        self.client.list(actor, query, deadline)
    }
    fn watch(
        &self,
        actor: &AuthorizationContext,
        cancel: &AtomicBool,
        notify: &mut dyn FnMut(DiscountApprovalNotice),
    ) -> Result<(), ApprovalError> {
        self.client.watch(actor, cancel, notify)
    }
}

fn start(
    store: AuthorityStore,
    connection: Arc<Connection>,
) -> (
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    let dispatcher = Arc::new(
        ProductDispatcher::new(store, EventBroker::new()).with_discount_approvals(connection),
    );
    let (cancel, receiver) = tokio::sync::watch::channel(false);
    let worker = dispatcher.start_quotation_draft_recovery(receiver).unwrap();
    (cancel, worker)
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ready() {
        assert!(Instant::now() < deadline, "saved draft recovery timed out");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn received(clients: &ApprovalClients, id: QuotationDraftId) -> Option<QuotationRecord> {
    clients
        .manager
        .quotations(
            &clients.manager_actor,
            &ListQuotations {
                after: None,
                limit: 100,
            },
            UnixMillis(i64::MAX),
        )
        .unwrap()
        .items
        .into_iter()
        .find(|q| q.quotation.id == id)
}

pub(super) fn run(
    reception: &DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
) {
    let connection = Arc::new(Connection {
        offline: AtomicBool::new(true),
        attempts: AtomicUsize::new(0),
        client: clients.reception.clone(),
    });
    let saved = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft {
                intent: draft_intent(customer, target.clone()),
            },
        )
        .unwrap();
    let (cancel, worker) = start(reception.client.store.clone(), connection.clone());
    wait_until(|| connection.attempts.load(Ordering::SeqCst) > 0);
    assert!(received(clients, saved.snapshot.id).is_none());
    connection.offline.store(false, Ordering::SeqCst);
    verify_received(reception, clients, &saved);
    cancel.send(true).unwrap();
    tokio::runtime::Handle::current().block_on(worker).unwrap();

    // Save offline, then start a fresh dispatcher/store with no desktop session or new save.
    connection.offline.store(true, Ordering::SeqCst);
    let mut intent = draft_intent(customer, target);
    intent.discount_basis_points = 600;
    let restarted = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft { intent },
        )
        .unwrap();
    assert!(received(clients, restarted.snapshot.id).is_none());
    let store = AuthorityStore::open(reception.client.directory.path()).unwrap();
    connection.offline.store(false, Ordering::SeqCst);
    let (cancel, worker) = start(store, connection);
    verify_received(reception, clients, &restarted);
    cancel.send(true).unwrap();
    tokio::runtime::Handle::current().block_on(worker).unwrap();
}

fn verify_received(reception: &DraftTestClient, clients: &ApprovalClients, saved: &QuotationDraft) {
    wait_until(|| received(clients, saved.snapshot.id).is_some());
    let received = received(clients, saved.snapshot.id).unwrap();
    let mut expected = saved.snapshot.clone();
    expected.evaluation.scope = received.scope.clone();
    assert_eq!(received.quotation, expected);
    assert_eq!(received.state, QuotationState::Draft);
    assert!(received.number.is_none());
    assert!(received.approval_request_id.is_none());
    assert_eq!(
        received.changed_by,
        reception.client.actor.identity.principal_id
    );
    wait_until(|| {
        reception.get(saved.snapshot.id).sync_state == QuotationDraftSyncState::Confirmed
    });
}
