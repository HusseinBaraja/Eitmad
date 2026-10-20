//! Real TLS/server decisions, two durable clients and explicit held-revision review.
use super::*;
use eitmad_contracts::quotation_lifecycle::QuotationRecord;
use eitmad_pricing::{DiscountApprovalServer, QuotationDraftError as E, QuotationServer};

fn decision(
    record: &QuotationRecord,
    clients: &ApprovalClients,
) -> ConfirmQuotationDraftResolution {
    let payload = record.quotation.clone();
    let branch = lifecycle::current(&*clients.manager, &clients.manager_actor, payload.id)
        .scope
        .id;
    ConfirmQuotationDraftResolution {
        scope: clients.manager_actor.scope.clone(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        command: ResolveQuotationDraft {
            draft_id: payload.id,
            branch_id: branch,
            expected_server_revision: payload.revision,
            choice: QuotationDraftResolutionChoice::UseInput,
            source: QuotationDraftResolutionSource::PendingReview(Box::new(input_change(
                &payload,
                &record.scope,
            ))),
        },
    }
}
fn input_change(snapshot: &QuotationDraftSnapshot, scope: &ScopeRef) -> ChangeRecord {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    ChangeRecord {
        change_id: ChangeId::new(Uuid::new_v4()),
        record_id: RecordId::new(snapshot.id.value()),
        scope: scope.clone(),
        operation: ChangeOperation::Upsert,
        base_revision: Some(snapshot.revision - 1),
        revision: snapshot.revision,
        changed_at: eitmad_authorization::now(),
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        merge: None,
        payload: Some(EncodedDomainPayload {
            schema_id: SchemaId::parse(QUOTATION_DRAFT_SCHEMA).unwrap(),
            schema_version: 1,
            base64: STANDARD.encode(serde_json::to_vec(snapshot).unwrap()),
        }),
    }
}

pub(super) fn assert_final_unchanged(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    record: &QuotationRecord,
) {
    let before = lifecycle::current(&*clients.reception, actor, record.quotation.id);
    assert_eq!(before.quotation, record.quotation);
    let request = decision(record, clients);
    assert_eq!(
        clients
            .manager
            .resolve_draft(&clients.manager_actor, &request, UnixMillis(i64::MAX)),
        Err(E::UnresolvedConflict)
    );
    assert_eq!(
        lifecycle::current(&*clients.reception, actor, record.quotation.id),
        before
    );
}

pub(super) fn dependencies(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    record: &QuotationRecord,
) {
    for mutation in 0..4 {
        let mut snapshot = record.quotation.clone();
        match mutation {
            0 => snapshot.evaluation.customer.as_mut().unwrap().name = "forged contact".into(),
            1 => {
                snapshot.intent.lines[0]
                    .configuration
                    .selection
                    .price_revision = 999;
            }
            2 => match &mut snapshot.intent.lines[0].configuration.selection.target {
                PriceTarget::Product(v) => v.revision = 999,
                PriceTarget::Furniture(v) => v.revision = 999,
            },
            _ => snapshot.evaluation.totals.as_mut().unwrap().total_yer += 1,
        }
        let mut request = decision(record, clients);
        request.command.source = QuotationDraftResolutionSource::PendingReview(Box::new(
            input_change(&snapshot, &record.scope),
        ));
        assert_eq!(
            clients
                .manager
                .resolve_draft(&clients.manager_actor, &request, UnixMillis(i64::MAX)),
            Err(E::Invalid)
        );
        assert_eq!(
            lifecycle::current(&*clients.reception, actor, record.quotation.id),
            *record
        );
    }
    let request = decision(record, clients);
    clients
        .manager
        .resolve_draft(&clients.manager_actor, &request, UnixMillis(i64::MAX))
        .unwrap();
    let retained = lifecycle::current(&*clients.reception, actor, record.quotation.id);
    assert_eq!(retained.quotation.evaluation, record.quotation.evaluation);
    assert_eq!(retained.quotation.intent, record.quotation.intent);
    assert!(retained.number.is_none());
}

pub(super) fn approval_and_audit(
    clients: &ApprovalClients,
    actor: &AuthorizationContext,
    pending: &eitmad_contracts::quotation_approval::DiscountApproval,
    pool: &sqlx::PgPool,
) {
    use eitmad_contracts::quotation_approval::*;
    let record = lifecycle::current(&*clients.reception, actor, pending.quotation.id);
    let mut request = decision(&record, clients);
    request.command.choice = QuotationDraftResolutionChoice::KeepCurrent;
    assign_recovery_branch(clients, request.command.branch_id, pool);
    let query = GetQuotationDraftReview {
        draft_id: record.quotation.id,
        branch_id: record.scope.id,
        after: None,
    };
    assert_eq!(
        clients
            .pure_reception
            .draft_review(&clients.pure_actor, &query, UnixMillis(i64::MAX)),
        Err(E::Denied)
    );
    assert_eq!(
        clients
            .pure_reception
            .resolve_draft(&clients.pure_actor, &request, UnixMillis(i64::MAX)),
        Err(E::Denied)
    );
    let runtime = tokio::runtime::Handle::current();
    runtime.block_on(sqlx::raw_sql("CREATE FUNCTION audit.fail_resolution_test() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.operation='eitmad.quotation-draft.resolve.v1' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_resolution_test BEFORE INSERT ON audit.server_records FOR EACH ROW EXECUTE FUNCTION audit.fail_resolution_test();").execute(pool)).unwrap();
    let failed =
        clients
            .manager
            .resolve_draft(&clients.manager_actor, &request, UnixMillis(i64::MAX));
    runtime.block_on(sqlx::raw_sql("DROP TRIGGER fail_resolution_test ON audit.server_records; DROP FUNCTION audit.fail_resolution_test();").execute(pool)).unwrap();
    assert_eq!(failed, Err(E::Unavailable));
    assert_eq!(
        lifecycle::current(&*clients.reception, actor, record.quotation.id),
        record
    );
    let confirmed = clients
        .manager
        .resolve_draft(&clients.manager_actor, &request, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(
        clients
            .manager
            .resolve_draft(&clients.manager_actor, &request, UnixMillis(i64::MAX))
            .unwrap(),
        confirmed
    );
    let recovery = clients
        .pure_reception
        .draft_recovery(
            &clients.pure_actor,
            record.quotation.id,
            UnixMillis(i64::MAX),
        )
        .unwrap();
    assert_eq!(recovery.last().unwrap().revision, confirmed.revision);
    assert!(recovery.last().unwrap().merge.is_some());
    let approvals = clients
        .manager
        .list(
            &clients.manager_actor,
            &ListDiscountApprovals {
                after: None,
                limit: 100,
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    assert_eq!(
        approvals
            .items
            .iter()
            .find(|a| a.quotation.id == record.quotation.id)
            .unwrap()
            .state,
        DiscountApprovalState::Invalidated
    );
    let changed = lifecycle::current(&*clients.reception, actor, record.quotation.id);
    assert_eq!(
        clients.reception.quotation_transition(
            actor,
            &lifecycle::issue_request(&changed, &actor.scope),
            UnixMillis(i64::MAX)
        ),
        Err(eitmad_pricing::QuotationError::ApprovalRequired)
    );
}

fn assign_recovery_branch(clients: &ApprovalClients, branch: ScopeId, pool: &sqlx::PgPool) {
    tokio::runtime::Handle::current().block_on(async {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
            .bind(clients.pure_actor.tenant_id.value().to_string())
            .execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.receptionist.v1','branch',$3,1)")
            .bind(clients.pure_actor.tenant_id.value())
            .bind(clients.pure_actor.identity.principal_id.value())
            .bind(branch.value()).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
    });
}

pub(super) fn boundary_scenarios(
    reception: &DraftTestClient,
    customer: CustomerId,
    target: PriceTarget,
    clients: &ApprovalClients,
    pool: &sqlx::PgPool,
) {
    use eitmad_contracts::quotation_approval::*;
    let actor = &reception.client.actor;
    let mut intent = draft_intent(customer, target);
    intent.discount_basis_points = 600;
    let saved = reception
        .drafts
        .create(
            &reception.client.mutation(),
            &CreateQuotationDraft { intent },
        )
        .unwrap();
    let pending = clients
        .reception
        .transition(
            actor,
            &ConfirmDiscountApproval {
                scope: actor.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                action: DiscountApprovalAction::Request(saved.snapshot),
            },
            UnixMillis(i64::MAX),
        )
        .unwrap()
        .unwrap();
    clients
        .manager
        .transition(
            &clients.manager_actor,
            &ConfirmDiscountApproval {
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
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    approval_and_audit(clients, actor, &pending, pool);
}

pub(super) fn run(
    first: &mut DraftTestClient,
    second: &mut DraftTestClient,
    conflict: &QuotationDraft,
    server: &eitmad_server_connection::DirectDiscountApprovalClient,
) {
    let id = conflict.snapshot.id;
    let actor = second.client.actor.clone();
    let query = GetQuotationDraftReview {
        draft_id: id,
        branch_id: actor.scope.id,
        after: None,
    };
    let page = second
        .drafts
        .review(&actor, &query, server, UnixMillis(i64::MAX))
        .unwrap();
    assert_eq!(page.current.revision, 2);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.pending_reviews.len(), 1);
    let command = ResolveQuotationDraft {
        draft_id: id,
        branch_id: actor.scope.id,
        expected_server_revision: 2,
        source: QuotationDraftResolutionSource::Conflict(QuotationDraftConflictSource {
            conflict_id: page.items[0].conflict.conflict_id,
        }),
        choice: QuotationDraftResolutionChoice::UseInput,
    };
    let context = second.client.mutation();
    let stale = ResolveQuotationDraft {
        expected_server_revision: 1,
        ..command.clone()
    };
    assert!(matches!(
        second.drafts.resolve(
            &second.client.mutation(),
            &stale,
            server,
            UnixMillis(i64::MAX)
        ),
        Err(E::UnresolvedConflict | E::Conflict { .. })
    ));
    let confirmed = server
        .resolve_draft(
            &actor,
            &ConfirmQuotationDraftResolution {
                scope: actor.scope.clone(),
                idempotency_key: context.idempotency_key,
                command: command.clone(),
            },
            UnixMillis(i64::MAX),
        )
        .unwrap();
    assert_eq!(confirmed.revision, 3);
    let reopened = AuthorityStore::open(second.client.directory.path()).unwrap();
    let restarted =
        QuotationDraftService::new(reopened.clone(), AuthorizationService::new(reopened));
    assert_eq!(
        restarted
            .recover_pending(server, UnixMillis(i64::MAX))
            .unwrap(),
        0
    );
    assert_eq!(second.get(id).snapshot.intent.discount_basis_points, 200);
    let retry = ConfirmQuotationDraftResolution {
        scope: actor.scope.clone(),
        idempotency_key: context.idempotency_key,
        command: command.clone(),
    };
    assert_eq!(
        server
            .resolve_draft(&actor, &retry, UnixMillis(i64::MAX))
            .unwrap(),
        confirmed
    );
    let changed = ConfirmQuotationDraftResolution {
        command: ResolveQuotationDraft {
            choice: QuotationDraftResolutionChoice::KeepCurrent,
            ..command
        },
        ..retry
    };
    assert_eq!(
        server.resolve_draft(&actor, &changed, UnixMillis(i64::MAX)),
        Err(E::Invalid)
    );
    first.engine = draft_engine(&first.client);
    second.engine = draft_engine(&second.client);
    first.run();
    second.run();
    assert_eq!(
        first.get(id).snapshot.intent,
        second.get(id).snapshot.intent
    );
    assert_eq!(second.get(id).snapshot.intent.discount_basis_points, 200);
    review_later(first, second, server, id);
}
fn review_later(
    first: &mut DraftTestClient,
    second: &mut DraftTestClient,
    server: &eitmad_server_connection::DirectDiscountApprovalClient,
    id: QuotationDraftId,
) {
    let actor = second.client.actor.clone();
    let query = GetQuotationDraftReview {
        draft_id: id,
        branch_id: actor.scope.id,
        after: None,
    };
    let reopened = AuthorityStore::open(second.client.directory.path()).unwrap();
    let drafts = QuotationDraftService::new(reopened.clone(), AuthorizationService::new(reopened));
    let page = drafts
        .review(&actor, &query, server, UnixMillis(i64::MAX))
        .unwrap();
    assert!(page.items.is_empty());
    assert_eq!(page.pending_reviews.len(), 1);
    assert_eq!(
        page.pending_reviews[0]
            .snapshot
            .intent
            .discount_basis_points,
        300
    );
    let later = ResolveQuotationDraft {
        draft_id: id,
        branch_id: actor.scope.id,
        expected_server_revision: 3,
        source: QuotationDraftResolutionSource::PendingReview(Box::new(
            page.pending_reviews[0].change.clone(),
        )),
        choice: QuotationDraftResolutionChoice::UseInput,
    };
    drafts
        .resolve(
            &second.client.mutation(),
            &later,
            server,
            UnixMillis(i64::MAX),
        )
        .unwrap();
    second.run();
    first.run();
    assert_eq!(
        first.get(id).snapshot.intent,
        second.get(id).snapshot.intent
    );
    assert_eq!(second.get(id).snapshot.revision, 4);
    assert_eq!(second.get(id).snapshot.intent.discount_basis_points, 300);
    assert!(
        drafts
            .review(&actor, &query, server, UnixMillis(i64::MAX))
            .unwrap()
            .pending_reviews
            .is_empty()
    );
}
