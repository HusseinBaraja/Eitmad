use super::*;
use crate::{ApprovalError, DiscountApprovalServer, QuotationDraftService};
use eitmad_contracts::{quotation_approval::*, quotation_draft::*};
use std::{collections::HashMap, sync::Mutex};

#[derive(Default)]
struct Server {
    failure: Mutex<Option<ApprovalError>>,
    calls: Mutex<Vec<(AuthorizationContext, ConfirmDiscountApproval)>>,
    receipts: Mutex<HashMap<IdempotencyKey, ConfirmDiscountApproval>>,
}
impl DiscountApprovalServer for Server {
    fn transition(
        &self,
        actor: &AuthorizationContext,
        request: &ConfirmDiscountApproval,
        _: UnixMillis,
    ) -> Result<Option<DiscountApproval>, ApprovalError> {
        self.calls
            .lock()
            .unwrap()
            .push((actor.clone(), request.clone()));
        if let Some(error) = *self.failure.lock().unwrap() {
            return Err(error);
        }
        let mut receipts = self.receipts.lock().unwrap();
        if let Some(prior) = receipts.insert(request.idempotency_key, request.clone()) {
            assert_eq!(prior, *request);
        }
        Ok(None)
    }
    fn list(
        &self,
        _: &AuthorizationContext,
        _: &ListDiscountApprovals,
        _: UnixMillis,
    ) -> Result<DiscountApprovalPage, ApprovalError> {
        unreachable!()
    }
    fn watch(
        &self,
        _: &AuthorizationContext,
        _: &std::sync::atomic::AtomicBool,
        _: &mut dyn FnMut(DiscountApprovalNotice),
    ) -> Result<(), ApprovalError> {
        unreachable!()
    }
}

fn service(store: &AuthorityStore) -> QuotationDraftService {
    QuotationDraftService::new(store.clone(), AuthorizationService::new(store.clone()))
}
fn deliver_customer(store: &AuthorityStore, actor: &AuthorizationContext) {
    let customers = eitmad_customer::CustomerService::new(
        store.clone(),
        AuthorizationService::new(store.clone()),
    );
    for change in customers.sync_batch_for_author(actor).unwrap() {
        customers
            .project_confirmed(actor, &change, mutation(actor.clone(), 1200).correlation_id)
            .unwrap();
    }
}
fn recover(drafts: &QuotationDraftService, server: &Server) -> Result<usize, ApprovalError> {
    drafts.recover_pending(server, UnixMillis(i64::MAX))
}
fn retained(store: &AuthorityStore) -> i64 {
    rusqlite::Connection::open(store.path())
        .unwrap()
        .query_row("SELECT count(*) FROM quotation_draft_outbox", [], |r| {
            r.get(0)
        })
        .unwrap()
}

fn grant_receptionist(store: &AuthorityStore, subject: &AuthorizationContext) {
    let auth = AuthorizationService::new(store.clone());
    let mut owner = actor(99, 50);
    owner.scope = subject.scope.clone();
    let version = auth
        .list_relationships(
            &owner,
            &eitmad_contracts::queries::ListScopeRelationships::new(None, 100).unwrap(),
        )
        .unwrap()
        .policy_version;
    auth.grant_relationship(
        &mutation(owner, u128::from(1240 + version)),
        &GrantScopeRelationship {
            expected_policy_version: version,
            subject: RelationshipSubject {
                principal_id: subject.identity.principal_id,
                principal_kind: subject.identity.principal_kind,
            },
            relation: RelationId::parse(RECEPTIONIST_RELATION).unwrap(),
        },
    )
    .unwrap();
}

#[test]
fn quotation_recovery_keeps_each_revision_author_after_account_switch_and_revocation() {
    let (dir, store, _, original, input, _) = fixture_evaluation();
    let mut next = actor(102, 50);
    grant_receptionist(&store, &next);
    next.scope = original.scope.clone();
    grant_receptionist(&store, &next);
    let drafts = service(&store);
    let saved = drafts
        .create(
            &mutation(original.clone(), 1230),
            &CreateQuotationDraft {
                intent: input.clone(),
            },
        )
        .unwrap();
    drafts
        .update(
            &mutation(next.clone(), 1231),
            &UpdateQuotationDraft {
                draft_id: saved.snapshot.id,
                expected_revision: 1,
                intent: input,
            },
        )
        .unwrap();
    deliver_customer(&store, &original);
    let auth = AuthorizationService::new(store.clone());
    let mut owner = actor(99, 50);
    owner.scope = original.scope.clone();
    let relationships = auth
        .list_relationships(
            &owner,
            &eitmad_contracts::queries::ListScopeRelationships::new(None, 100).unwrap(),
        )
        .unwrap();
    let relationship = relationships
        .relationships
        .iter()
        .find(|r| r.subject.principal_id == original.identity.principal_id)
        .unwrap();
    auth.revoke_relationship(
        &mutation(owner, 1232),
        &eitmad_contracts::commands::RevokeScopeRelationship {
            expected_policy_version: relationships.policy_version,
            relationship_id: relationship.relationship_id,
        },
    )
    .unwrap();
    let server = Server::default();
    let restarted = service(&AuthorityStore::open(dir.path()).unwrap());
    assert_eq!(recover(&restarted, &server), Ok(0));
    assert!(server.calls.lock().unwrap().is_empty());
    assert_eq!(retained(&store), 2); // The next author cannot overtake a revoked predecessor.
    grant_receptionist(&store, &original);
    assert_eq!(recover(&restarted, &server), Ok(0)); // Wrap the bounded cursor.
    assert_eq!(recover(&restarted, &server), Ok(1));
    assert_eq!(recover(&restarted, &server), Ok(1));
    let calls = server.calls.lock().unwrap();
    assert_eq!(calls[0].0, original);
    assert_eq!(calls[1].0, next);
    assert_eq!(
        calls[0].1.idempotency_key,
        mutation(original, 1230).idempotency_key
    );
    assert_eq!(
        calls[1].1.idempotency_key,
        mutation(next, 1231).idempotency_key
    );
}

#[test]
fn quotation_recovery_waits_for_customer_and_preserves_saved_revisions_after_restart() {
    let (dir, store, _, actor, input, mut entry) = fixture_evaluation();
    let drafts = service(&store);
    let saved = drafts
        .create(
            &mutation(actor.clone(), 1201),
            &CreateQuotationDraft {
                intent: input.clone(),
            },
        )
        .unwrap();
    let mut changed = input;
    changed.discount_basis_points = 600;
    let updated = drafts
        .update(
            &mutation(actor.clone(), 1202),
            &UpdateQuotationDraft {
                draft_id: saved.snapshot.id,
                expected_revision: 1,
                intent: changed,
            },
        )
        .unwrap();
    let server = Server::default();
    assert_eq!(recover(&drafts, &server), Ok(0));
    assert!(server.calls.lock().unwrap().is_empty());
    // A rejected dependency cannot cause a draft rejection or bypass delivery.
    let customers = eitmad_customer::CustomerService::new(
        store.clone(),
        AuthorizationService::new(store.clone()),
    );
    let dependency = customers.sync_batch_for_author(&actor).unwrap().remove(0);
    customers
        .mark_sync_exception(
            &actor,
            &dependency,
            eitmad_contracts::customer::CustomerSyncState::Rejected,
            mutation(actor.clone(), 1203).correlation_id,
            UnixMillis(1),
        )
        .unwrap();
    assert_eq!(recover(&service(&store), &server), Ok(0));
    customers
        .project_confirmed(
            &actor,
            &dependency,
            mutation(actor.clone(), 1204).correlation_id,
        )
        .unwrap();
    entry.price.revision = 2;
    entry.price.selling_price_yer = 2000;
    let catalog_actor = AuthorizationContext {
        scope: entry.price.target.scope().clone(),
        ..actor.clone()
    };
    project_sales(&store, &catalog_actor, vec![(1, 2, Some(entry))]);
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    let drafts = service(&reopened);
    assert_eq!(recover(&drafts, &server), Ok(1));
    assert_eq!(recover(&drafts, &server), Ok(1));
    let calls = server.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, actor);
    assert_eq!(
        calls[0].1.idempotency_key,
        mutation(actor.clone(), 1201).idempotency_key
    );
    assert_eq!(
        calls[0].1.action,
        DiscountApprovalAction::Refresh(saved.snapshot)
    );
    assert_eq!(
        calls[1].1.action,
        DiscountApprovalAction::Refresh(updated.snapshot.clone())
    );
    let confirmed = drafts
        .get(
            &actor,
            &GetQuotationDraft {
                draft_id: updated.snapshot.id,
            },
        )
        .unwrap();
    assert_eq!(confirmed.snapshot, updated.snapshot);
    assert_eq!(confirmed.sync_state, QuotationDraftSyncState::Confirmed);
    assert_eq!(retained(&reopened), 0);
}

#[test]
fn quotation_recovery_replays_exact_key_after_lost_local_acknowledgement() {
    let (dir, store, _, actor, input, _) = fixture_evaluation();
    let drafts = service(&store);
    let saved = drafts
        .create(
            &mutation(actor.clone(), 1210),
            &CreateQuotationDraft { intent: input },
        )
        .unwrap();
    deliver_customer(&store, &actor);
    let db = rusqlite::Connection::open(store.path()).unwrap();
    db.execute_batch("CREATE TRIGGER fail_draft_ack BEFORE DELETE ON quotation_draft_outbox BEGIN SELECT RAISE(ABORT,'synthetic acknowledgement failure'); END;").unwrap();
    let server = Server::default();
    assert_eq!(recover(&drafts, &server), Err(ApprovalError::Unavailable));
    assert_eq!(retained(&store), 1);
    db.execute_batch("DROP TRIGGER fail_draft_ack").unwrap();
    let drafts = service(&AuthorityStore::open(dir.path()).unwrap());
    assert_eq!(recover(&drafts, &server), Ok(1));
    assert_eq!(recover(&drafts, &server), Ok(0));
    assert_eq!(server.calls.lock().unwrap().len(), 2);
    assert_eq!(server.receipts.lock().unwrap().len(), 1);
    assert_eq!(
        drafts
            .get(
                &actor,
                &GetQuotationDraft {
                    draft_id: saved.snapshot.id
                }
            )
            .unwrap()
            .sync_state,
        QuotationDraftSyncState::Confirmed
    );
}

#[test]
fn quotation_recovery_distinguishes_temporary_failure_conflict_and_rejection() {
    for (error, state) in [
        (ApprovalError::Unavailable, QuotationDraftSyncState::Pending),
        (ApprovalError::Conflict, QuotationDraftSyncState::Conflicted),
        (ApprovalError::Denied, QuotationDraftSyncState::Rejected),
        (ApprovalError::Invalid, QuotationDraftSyncState::Rejected),
    ] {
        let (dir, store, _, actor, input, _) = fixture_evaluation();
        let drafts = service(&store);
        let saved = drafts
            .create(
                &mutation(actor.clone(), 1220),
                &CreateQuotationDraft { intent: input },
            )
            .unwrap();
        deliver_customer(&store, &actor);
        let server = Server::default();
        *server.failure.lock().unwrap() = Some(error);
        let result = recover(&drafts, &server);
        assert_eq!(
            result,
            if error == ApprovalError::Unavailable {
                Err(error)
            } else {
                Ok(0)
            }
        );
        assert_eq!(retained(&store), 1);
        let reopened = AuthorityStore::open(dir.path()).unwrap();
        let drafts = service(&reopened);
        let current = drafts
            .get(
                &actor,
                &GetQuotationDraft {
                    draft_id: saved.snapshot.id,
                },
            )
            .unwrap();
        assert_eq!(current.snapshot, saved.snapshot);
        assert_eq!(current.sync_state, state);
        *server.failure.lock().unwrap() = None;
        assert_eq!(
            recover(&drafts, &server),
            Ok(usize::from(error == ApprovalError::Unavailable))
        );
        assert_eq!(
            server.calls.lock().unwrap().len(),
            if error == ApprovalError::Unavailable {
                2
            } else {
                1
            }
        );
    }
}

#[test]
fn quotation_recovery_bounds_each_pass_and_rotates_past_blocked_dependencies() {
    let (_dir, store, _, actor, input, _) = fixture_evaluation();
    let drafts = service(&store);
    for key in 1300..1350 {
        drafts
            .create(
                &mutation(actor.clone(), key),
                &CreateQuotationDraft {
                    intent: input.clone(),
                },
            )
            .unwrap();
    }
    let customers = eitmad_customer::CustomerService::new(
        store.clone(),
        AuthorizationService::new(store.clone()),
    );
    let customer = customers
        .create(
            &mutation(actor.clone(), 1350),
            &CreateCustomer {
                name: CustomerName::parse("عميل آخر").unwrap(),
                phone: CustomerPhone::parse("777123457").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    let change = customers
        .sync_batch_for_author(&actor)
        .unwrap()
        .into_iter()
        .find(|c| c.record_id.value() == customer.id.value())
        .unwrap();
    customers
        .project_confirmed(
            &actor,
            &change,
            mutation(actor.clone(), 1351).correlation_id,
        )
        .unwrap();
    let mut ready = input;
    ready.customer = Some(QuotationCustomerIntent {
        id: customer.id,
        revision: customer.revision,
    });
    drafts
        .create(
            &mutation(actor, 1352),
            &CreateQuotationDraft { intent: ready },
        )
        .unwrap();
    let server = Server::default();
    assert_eq!(drafts.recover_pending(&server, UnixMillis(0)), Ok(0));
    assert!(server.calls.lock().unwrap().is_empty());
    assert_eq!(recover(&drafts, &server), Ok(0));
    assert_eq!(recover(&drafts, &server), Ok(1));
    assert_eq!(retained(&store), 50);
    assert_eq!(server.calls.lock().unwrap().len(), 1);
}
