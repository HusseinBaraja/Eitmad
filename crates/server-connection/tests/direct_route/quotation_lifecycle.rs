use super::*;
use eitmad_contracts::quotation_lifecycle::*;
use eitmad_pricing::{DiscountApprovalServer, QuotationError as E, QuotationServer};

pub(super) fn current(
    client: &dyn QuotationServer,
    actor: &AuthorizationContext,
    id: QuotationDraftId,
) -> QuotationRecord {
    client
        .quotations(
            actor,
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
        .unwrap()
}
pub(super) fn issue_request(record: &QuotationRecord, scope: &ScopeRef) -> ConfirmQuotation {
    ConfirmQuotation {
        scope: scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: QuotationAction::Issue(IssueQuotation {
            draft_id: record.quotation.id,
            expected_revision: record.revision,
            expected_draft_revision: record.quotation.revision,
        }),
    }
}
pub(super) fn run(
    reception: &mut DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
    pool: &sqlx::PgPool,
    server_actor: &AuthenticatedServerSession,
) {
    let original = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft {
                intent: draft_intent(customer, target.clone()),
            },
        )
        .unwrap();
    let actor = reception.client.actor.clone();
    clients
        .reception
        .transition(
            &actor,
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
    let watches = LiveWatches::new(clients, &actor);
    let pending = current(&*clients.reception, &actor, original.snapshot.id);
    let request = issue_request(&pending, &actor.scope);
    assert_eq!(
        clients.manager.quotation_transition(
            &clients.manager_actor,
            &request,
            UnixMillis(i64::MAX)
        ),
        Err(E::Denied)
    );
    let competing = ConfirmQuotation {
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        ..request.clone()
    };
    let (first, second) = std::thread::scope(|threads| {
        let one = threads.spawn(|| {
            clients
                .reception
                .quotation_transition(&actor, &request, UnixMillis(i64::MAX))
        });
        let two = threads.spawn(|| {
            clients.pure_reception.quotation_transition(
                &clients.pure_actor,
                &competing,
                UnixMillis(i64::MAX),
            )
        });
        (one.join().unwrap(), two.join().unwrap())
    });
    assert_ne!(first.is_ok(), second.is_ok());
    let issued = first.clone().or(second.clone()).unwrap();
    assert_eq!(issued.state, QuotationState::Issued);
    assert!(issued.number.as_ref().unwrap().starts_with("QT-"));
    assert_eq!(issued.quotation, original.snapshot);
    watches.wait(issued.quotation.id, issued.revision);
    verify_issue_retry(
        clients,
        &actor,
        &request,
        &competing,
        &issued,
        first.is_ok(),
    );
    assert_eq!(
        clients.reception.quotation_transition(
            &actor,
            &issue_request(&issued, &actor.scope),
            UnixMillis(i64::MAX)
        ),
        Err(E::Conflict)
    );
    let cancelled = verify_revision_and_cancellation(clients, &actor, &issued);
    watches.wait(cancelled.quotation.id, cancelled.revision);
    verify_approval_issue(
        reception,
        customer,
        target.clone(),
        clients,
        pool,
        server_actor,
    );
    verify_issue_rollback(reception, customer, target.clone(), clients, pool);
    verify_bounded_pool_and_large_document(
        reception,
        customer,
        target.clone(),
        clients,
        pool,
        server_actor,
    );
    verify_catalog_change(reception, customer, target, clients, &cancelled);
    watches.stop();
}

fn verify_issue_retry(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    request: &ConfirmQuotation,
    competing: &ConfirmQuotation,
    issued: &QuotationRecord,
    first_won: bool,
) {
    if first_won {
        assert_eq!(
            clients
                .reception
                .quotation_transition(actor, request, UnixMillis(i64::MAX))
                .unwrap(),
            *issued
        );
    } else {
        assert_eq!(
            clients
                .pure_reception
                .quotation_transition(&clients.pure_actor, competing, UnixMillis(i64::MAX))
                .unwrap(),
            *issued
        );
    }
    let changed_key = if first_won {
        ConfirmQuotation {
            action: QuotationAction::Cancel(CancelQuotation {
                draft_id: issued.quotation.id,
                expected_revision: issued.revision,
                reason: "إلغاء تجريبي".into(),
            }),
            ..request.clone()
        }
    } else {
        request.clone()
    };
    if first_won {
        assert_eq!(
            clients
                .reception
                .quotation_transition(actor, &changed_key, UnixMillis(i64::MAX)),
            Err(E::Invalid)
        );
    }
}

fn verify_revision_and_cancellation(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    issued: &QuotationRecord,
) -> QuotationRecord {
    let revise = ConfirmQuotation {
        scope: clients.manager_actor.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: QuotationAction::Revise(SetQuotationValidity {
            draft_id: issued.quotation.id,
            expected_revision: issued.revision,
            validity_days: 45,
        }),
    };
    assert_eq!(
        clients.pure_reception.quotation_transition(
            &clients.pure_actor,
            &revise,
            UnixMillis(i64::MAX)
        ),
        Err(E::Denied)
    );
    let revised = clients
        .manager
        .quotation_transition(&clients.manager_actor, &revise, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(revised.state, QuotationState::Draft);
    assert_eq!(revised.number, issued.number);
    assert_eq!(revised.document_revision, 2);
    assert_eq!(revised.validity_days, 45);
    assert!(revised.issued_at.is_none());
    let new_issue = issue_request(&revised, &actor.scope);
    let reissued = clients
        .reception
        .quotation_transition(actor, &new_issue, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(reissued.number, issued.number);
    assert_eq!(reissued.document_revision, 2);
    verify_cancellation_race(clients, actor, &reissued)
}

fn verify_cancellation_race(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    reissued: &QuotationRecord,
) -> QuotationRecord {
    let cancel = ConfirmQuotation {
        scope: clients.manager_actor.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: QuotationAction::Cancel(CancelQuotation {
            draft_id: reissued.quotation.id,
            expected_revision: reissued.revision,
            reason: "إلغاء تجريبي مع حفظ السجل".into(),
        }),
    };
    let competing = ConfirmQuotation {
        scope: clients.manager_actor.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: QuotationAction::Revise(SetQuotationValidity {
            draft_id: reissued.quotation.id,
            expected_revision: reissued.revision,
            validity_days: 60,
        }),
    };
    let (cancel_result, revise_result) = std::thread::scope(|threads| {
        let cancel_task = threads.spawn(|| {
            clients.manager.quotation_transition(
                &clients.manager_actor,
                &cancel,
                UnixMillis(i64::MAX),
            )
        });
        let revise_task = threads.spawn(|| {
            clients.manager.quotation_transition(
                &clients.manager_actor,
                &competing,
                UnixMillis(i64::MAX),
            )
        });
        (cancel_task.join().unwrap(), revise_task.join().unwrap())
    });
    assert_ne!(cancel_result.is_ok(), revise_result.is_ok());
    let (cancelled, receipt) = if let Ok(value) = cancel_result {
        assert_eq!(revise_result, Err(E::Conflict));
        (value, cancel)
    } else {
        assert_eq!(cancel_result, Err(E::Conflict));
        let revised = revise_result.unwrap();
        let issued_again = clients
            .reception
            .quotation_transition(
                actor,
                &issue_request(&revised, &actor.scope),
                UnixMillis(i64::MAX),
            )
            .unwrap();
        let receipt = ConfirmQuotation {
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            action: QuotationAction::Cancel(CancelQuotation {
                draft_id: issued_again.quotation.id,
                expected_revision: issued_again.revision,
                reason: "إلغاء تجريبي مع حفظ السجل".into(),
            }),
            ..cancel
        };
        let value = clients
            .manager
            .quotation_transition(&clients.manager_actor, &receipt, UnixMillis(i64::MAX))
            .unwrap();
        (value, receipt)
    };
    assert_eq!(cancelled.quotation.intent, reissued.quotation.intent);
    assert_eq!(
        cancelled.quotation.evaluation.lines,
        reissued.quotation.evaluation.lines
    );

    assert_eq!(cancelled.number, reissued.number);
    assert_eq!(
        current(&*clients.reception, actor, cancelled.quotation.id).state,
        QuotationState::Cancelled
    );
    assert_eq!(
        current(
            &*clients.manager,
            &clients.manager_actor,
            cancelled.quotation.id
        )
        .state,
        QuotationState::Cancelled
    );
    assert_eq!(
        clients
            .manager
            .quotation_transition(&clients.manager_actor, &receipt, UnixMillis(i64::MAX))
            .unwrap(),
        cancelled
    );
    cancelled
}

fn verify_approval_issue(
    reception: &mut DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
    pool: &sqlx::PgPool,
    server_actor: &AuthenticatedServerSession,
) {
    use eitmad_contracts::quotation_approval::*;
    let actor = reception.client.actor.clone();
    let mut intent = draft_intent(customer, target);
    intent.discount_basis_points = 600;
    let draft = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft { intent },
        )
        .unwrap();
    let pending = clients
        .reception
        .transition(
            &actor,
            &ConfirmDiscountApproval {
                scope: actor.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                action: DiscountApprovalAction::Request(draft.snapshot),
            },
            UnixMillis(i64::MAX),
        )
        .unwrap()
        .unwrap();
    let before = current(&*clients.reception, &actor, pending.quotation.id);
    assert_eq!(
        clients.reception.quotation_transition(
            &actor,
            &issue_request(&before, &actor.scope),
            UnixMillis(i64::MAX)
        ),
        Err(E::ApprovalRequired)
    );
    let decision = ConfirmDiscountApproval {
        scope: clients.manager_actor.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        action: DiscountApprovalAction::Decide(DecideDiscountApproval {
            draft_id: pending.quotation.id,
            request_id: pending.request_id,
            quotation_revision: pending.quotation.revision,
            expected_revision: pending.revision,
            fingerprint: pending.fingerprint.clone(),
            decision: DiscountDecision::Approve,
            reason: None,
        }),
    };
    clients
        .manager
        .transition(&clients.manager_actor, &decision, UnixMillis(i64::MAX))
        .unwrap();
    let before = current(&*clients.reception, &actor, pending.quotation.id);
    let issued = clients
        .reception
        .quotation_transition(
            &actor,
            &issue_request(&before, &actor.scope),
            UnixMillis(i64::MAX),
        )
        .unwrap();
    assert_eq!(issued.approval_request_id, Some(pending.request_id));
    assert_eq!(issued.quotation, pending.quotation);
    verify_expiry_and_history(clients, &actor, &issued, pool, server_actor);
}

fn verify_expiry_and_history(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    issued: &QuotationRecord,
    pool: &sqlx::PgPool,
    server_actor: &AuthenticatedServerSession,
) {
    let scope = current(
        &*clients.manager,
        &clients.manager_actor,
        issued.quotation.id,
    )
    .scope;
    let runtime = tokio::runtime::Handle::current();
    runtime.block_on(async {
        let server = eitmad_sync_plane::QuotationLifecycleServer::new(pool.clone());
        server.expire_due(server_actor,&scope,UnixMillis(issued.valid_until.unwrap().0+1)).await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)").bind(server_actor.tenant_id.value().to_string()).execute(&mut *tx).await.unwrap();
        let history: Vec<Vec<u8>> = sqlx::query_scalar("SELECT record_json FROM sync.quotation_history WHERE tenant_id=$1 AND draft_id=$2 ORDER BY revision")
            .bind(server_actor.tenant_id.value()).bind(issued.quotation.id.value()).fetch_all(&mut *tx).await.unwrap();
        assert!(history.len() >= 3);
        let stored: QuotationRecord = serde_json::from_slice(&history[history.len()-2]).unwrap();
        assert_frozen(&stored.quotation,&issued.quotation); assert_eq!(stored.number,issued.number);
        assert!(sqlx::query("UPDATE sync.quotation_history SET revision=99 WHERE tenant_id=$1").bind(server_actor.tenant_id.value()).execute(&mut *tx).await.is_err());
        tx.rollback().await.unwrap();
    });
    assert_eq!(
        current(&*clients.reception, actor, issued.quotation.id).state,
        QuotationState::Expired
    );
    assert_eq!(
        current(
            &*clients.manager,
            &clients.manager_actor,
            issued.quotation.id
        )
        .state,
        QuotationState::Expired
    );
    assert_eq!(
        clients.reception.quotation_transition(
            actor,
            &issue_request(issued, &actor.scope),
            UnixMillis(i64::MAX)
        ),
        Err(E::Conflict)
    );
}

pub(super) fn assert_frozen(
    actual: &eitmad_contracts::quotation_draft::QuotationDraftSnapshot,
    expected: &eitmad_contracts::quotation_draft::QuotationDraftSnapshot,
) {
    let mut normalized = actual.clone();
    normalized.evaluation.scope = expected.evaluation.scope.clone();
    for (line, source) in normalized
        .intent
        .lines
        .iter_mut()
        .zip(&expected.intent.lines)
    {
        normalize_scope(
            &mut line.configuration.selection.target,
            source.configuration.selection.target.scope(),
        );
    }
    for (line, source) in normalized
        .evaluation
        .lines
        .iter_mut()
        .zip(&expected.evaluation.lines)
    {
        normalize_scope(
            &mut line.price.snapshot.target,
            source.price.snapshot.target.scope(),
        );
    }
    assert_eq!(normalized, *expected);
}

fn verify_catalog_change(
    reception: &mut DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
    retained: &QuotationRecord,
) {
    let actor = reception.client.actor.clone();
    let draft = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft {
                intent: draft_intent(customer, target.clone()),
            },
        )
        .unwrap();
    clients
        .reception
        .transition(
            &actor,
            &eitmad_contracts::quotation_approval::ConfirmDiscountApproval {
                scope: actor.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                action: eitmad_contracts::quotation_approval::DiscountApprovalAction::Refresh(
                    draft.snapshot.clone(),
                ),
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    let before = current(&*clients.reception, &actor, draft.snapshot.id);
    clients
        .pricing
        .publish(
            &mutation(&clients.pricing_actor),
            &eitmad_contracts::pricing::PublishPrice {
                target,
                expected_revision: Some(1),
                selling_price_yer: 120_000,
                confirm_below_cost: false,
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    let after = current(&*clients.reception, &actor, retained.quotation.id);
    assert_frozen(&after.quotation, &retained.quotation);
    assert_eq!(after.number, retained.number);
    assert_eq!(
        clients.reception.quotation_transition(
            &actor,
            &issue_request(&before, &actor.scope),
            UnixMillis(i64::MAX)
        ),
        Err(E::StalePrice)
    );
    let failed = current(&*clients.reception, &actor, before.quotation.id);
    assert!(failed.number.is_none());
    assert_eq!(failed.revision, before.revision);
}
type LifecycleWatchHandle = (
    Arc<std::sync::atomic::AtomicBool>,
    std::thread::JoinHandle<Result<(), E>>,
);
struct LiveWatches {
    receivers: Vec<std::sync::mpsc::Receiver<QuotationNotice>>,
    handles: Vec<LifecycleWatchHandle>,
}
impl LiveWatches {
    fn new(clients: &ApprovalClients, actor: &AuthorizationContext) -> Self {
        let mut result = Self {
            receivers: Vec::new(),
            handles: Vec::new(),
        };
        for (client, actor) in [
            (clients.reception.clone(), actor.clone()),
            (clients.manager.clone(), clients.manager_actor.clone()),
        ] {
            let (send, receive) = std::sync::mpsc::channel();
            let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let stop = cancel.clone();
            let handle = std::thread::spawn(move || {
                client.watch_quotations(&actor, &stop, &mut |notice| {
                    let _ = send.send(notice);
                })
            });
            receive.recv_timeout(Duration::from_secs(10)).expect(
                "Lifecycle subscription must be ready before submitting concurrent transitions",
            );
            result.receivers.push(receive);
            result.handles.push((cancel, handle));
        }
        result
    }
    fn wait(&self, id: QuotationDraftId, revision: u64) {
        for receiver in &self.receivers {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            loop {
                let notice = receiver
                    .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .unwrap();
                if notice.draft_id == id && notice.revision >= revision {
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
impl Drop for LiveWatches {
    fn drop(&mut self) {
        for (cancel, _) in &self.handles {
            cancel.store(true, std::sync::atomic::Ordering::Release);
        }
    }
}

fn normalize_scope(target: &mut PriceTarget, scope: &ScopeRef) {
    match target {
        PriceTarget::Product(p) => p.scope = scope.clone(),
        PriceTarget::Furniture(f) => f.scope = scope.clone(),
    }
}

fn verify_issue_rollback(
    reception: &mut DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
    pool: &sqlx::PgPool,
) {
    let actor = reception.client.actor.clone();
    let draft = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft {
                intent: draft_intent(customer, target),
            },
        )
        .unwrap();
    clients
        .reception
        .transition(
            &actor,
            &eitmad_contracts::quotation_approval::ConfirmDiscountApproval {
                scope: actor.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                action: eitmad_contracts::quotation_approval::DiscountApprovalAction::Refresh(
                    draft.snapshot.clone(),
                ),
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    let before = current(&*clients.reception, &actor, draft.snapshot.id);
    let request = issue_request(&before, &actor.scope);
    let runtime = tokio::runtime::Handle::current();
    let last_number: i64 = runtime.block_on(async {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
            .bind(actor.tenant_id.value().to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query_scalar("SELECT max(last_number) FROM sync.quotation_numbers WHERE tenant_id=$1")
            .bind(actor.tenant_id.value())
            .fetch_one(&mut *tx)
            .await
            .unwrap()
    });
    runtime.block_on(sqlx::raw_sql("CREATE FUNCTION audit.fail_issue_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.quotation.issue.v1' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_issue_test BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION audit.fail_issue_test();").execute(pool)).unwrap();
    let failed = clients
        .reception
        .quotation_transition(&actor, &request, UnixMillis(i64::MAX));
    runtime.block_on(sqlx::raw_sql("DROP TRIGGER fail_issue_test ON audit.server_records; DROP FUNCTION audit.fail_issue_test();").execute(pool)).unwrap();
    assert_eq!(failed, Err(E::Unavailable));
    assert_eq!(
        current(&*clients.reception, &actor, draft.snapshot.id),
        before
    );
    let issued = clients
        .reception
        .quotation_transition(&actor, &request, UnixMillis(i64::MAX))
        .unwrap();
    assert!(
        issued
            .number
            .unwrap()
            .ends_with(&format!("-{:05}", last_number + 2))
    );
}

fn verify_bounded_pool_and_large_document(
    reception: &mut DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
    pool: &sqlx::PgPool,
    server_actor: &AuthenticatedServerSession,
) {
    let actor = reception.client.actor.clone();
    let mut intent = draft_intent(customer, target);
    let line = intent.lines[0].clone();
    intent.lines = (0..240)
        .map(|_| {
            let mut copy = line.clone();
            copy.id = Uuid::new_v4();
            copy
        })
        .collect();
    let draft = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft { intent },
        )
        .unwrap();
    clients
        .reception
        .transition(
            &actor,
            &eitmad_contracts::quotation_approval::ConfirmDiscountApproval {
                scope: actor.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                action: eitmad_contracts::quotation_approval::DiscountApprovalAction::Refresh(
                    draft.snapshot.clone(),
                ),
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    // A legitimate commercial record exceeds the old image-route response budget.
    assert!(serde_json::to_vec(&draft.snapshot).unwrap().len() > 136 * 1024);
    let before = current(&*clients.reception, &actor, draft.snapshot.id);
    let request = issue_request(&before, &actor.scope);
    let server_scope = current(&*clients.manager, &clients.manager_actor, draft.snapshot.id).scope;
    let issued = tokio::runtime::Handle::current().block_on(async {
        let single = sqlx::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(2))
            .connect_with((*pool.connect_options()).clone())
            .await
            .unwrap();
        let server = eitmad_sync_plane::QuotationLifecycleServer::new(single.clone());
        let mut locked = pool.begin().await.unwrap();
        sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
            .bind(server_actor.tenant_id.value())
            .fetch_one(&mut *locked)
            .await
            .unwrap();
        let listed = tokio::time::timeout(
            Duration::from_secs(2),
            server.list(
                server_actor,
                &ReadQuotations {
                    scope: server_scope.clone(),
                    query: ListQuotations {
                        after: None,
                        limit: 100,
                    },
                },
                eitmad_control_plane::unix_millis_now(),
            ),
        )
        .await
        .expect("Initialized read must not wait for the tenant write lock")
        .unwrap();
        assert!(
            listed
                .items
                .iter()
                .any(|r| r.quotation.id == before.quotation.id)
        );
        locked.rollback().await.unwrap();
        let issued = tokio::time::timeout(
            Duration::from_secs(10),
            server.transition(
                server_actor,
                &ConfirmQuotation {
                    scope: server_scope.clone(),
                    ..request.clone()
                },
                CorrelationId::new(Uuid::new_v4()),
                eitmad_control_plane::unix_millis_now(),
            ),
        )
        .await
        .expect("First issuance must complete with one pool connection")
        .unwrap();
        single.close().await;
        issued
    });
    assert_eq!(issued.quotation.evaluation.lines.len(), 240);
    // The retained command reply also crosses the large transition-response boundary.
    let repeated = clients
        .reception
        .quotation_transition(&actor, &request, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(repeated.revision, issued.revision);
    assert_eq!(repeated.number, issued.number);
    assert_frozen(&repeated.quotation, &draft.snapshot);
}
