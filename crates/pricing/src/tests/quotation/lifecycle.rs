use super::*;
use crate::{QuotationError as E, issuance_snapshot, quotation_expiry};
use eitmad_contracts::{
    quotation_approval::DiscountApprovalState,
    quotation_lifecycle::{QuotationRecord, QuotationState},
};

#[test]
fn quotation_issue_binds_approval_revision_validity_and_frozen_contact() {
    let mut approval = super::approval::request();
    let tenant = TenantId::new(approval.organization_id);
    let now = UnixMillis(1_791_417_600_000);
    approval.proposed_valid_until = quotation_expiry(now, 30).unwrap();
    approval.fingerprint = crate::approval_fingerprint(
        tenant,
        approval.organization_id,
        &approval.quotation,
        30,
        approval.proposed_valid_until,
    )
    .unwrap();
    let mut record = QuotationRecord {
        scope: approval.scope.clone(),
        organization_id: approval.organization_id,
        revision: 1,
        document_revision: 1,
        state: QuotationState::PendingApproval,
        quotation: approval.quotation.clone(),
        number: None,
        validity_days: 30,
        issued_at: None,
        valid_until: None,
        approval_request_id: None,
        approval_fingerprint: None,
        changed_at: now,
        changed_by: approval.requester,
        cancellation_reason: None,
        acceptance: None,
        permitted_actions: vec![],
    };
    assert_eq!(
        issuance_snapshot(tenant, &record, &record.quotation, Some(&approval), now),
        Err(E::ApprovalRequired)
    );
    approval.state = DiscountApprovalState::Approved;
    let original = record.quotation.clone();
    let mut contact_change = original.clone();
    contact_change.revision += 1;
    contact_change.evaluation.customer.as_mut().unwrap().phone = "777000999".into();
    assert_eq!(
        issuance_snapshot(tenant, &record, &contact_change, Some(&approval), now).unwrap(),
        original
    );
    let mut quantity_change = original.clone();
    quantity_change.intent.lines[0]
        .configuration
        .selection
        .quantity += 1;
    assert_eq!(
        issuance_snapshot(tenant, &record, &quantity_change, Some(&approval), now),
        Err(E::ApprovalRequired)
    );
    record.validity_days = 45;
    assert_eq!(
        issuance_snapshot(tenant, &record, &original, Some(&approval), now),
        Err(E::ApprovalRequired)
    );
    record.validity_days = 30;
    assert_eq!(
        issuance_snapshot(
            tenant,
            &record,
            &original,
            Some(&approval),
            UnixMillis(now.0 + 86_400_000)
        ),
        Err(E::ApprovalRequired)
    );
    record.state = QuotationState::Issued;
    assert_eq!(
        issuance_snapshot(tenant, &record, &original, Some(&approval), now),
        Err(E::Conflict)
    );
}

#[test]
fn quotation_expiry_uses_aden_calendar_date_and_checked_bounds() {
    // 2026-12-31 20:59 UTC is still December 31 in Aden; one minute later is January 1.
    assert_eq!(
        quotation_expiry(UnixMillis(1_798_750_740_000), 1).unwrap(),
        UnixMillis(1_798_837_199_999)
    );
    assert_eq!(
        quotation_expiry(UnixMillis(1_798_750_800_000), 1).unwrap(),
        UnixMillis(1_798_923_599_999)
    );
    assert_eq!(quotation_expiry(UnixMillis(0), 0), Err(E::Invalid));
    assert_eq!(quotation_expiry(UnixMillis(i64::MAX), 30), Err(E::Invalid));
}

#[test]
fn quotation_local_cancellation_replays_preserves_prices_and_blocks_edits() {
    use eitmad_contracts::quotation_draft::*;
    let (_dir, store, _, actor, intent, _) = fixture_evaluation();
    let service =
        crate::QuotationDraftService::new(store.clone(), AuthorizationService::new(store));
    let original = service
        .create(
            &mutation(actor.clone(), 800),
            &CreateQuotationDraft {
                intent: intent.clone(),
            },
        )
        .unwrap();
    let command = CancelQuotationDraft {
        draft_id: original.snapshot.id,
        expected_revision: original.snapshot.revision,
    };
    let context = mutation(actor.clone(), 801);
    let cancelled = service.cancel(&context, &command).unwrap();
    assert_eq!(service.cancel(&context, &command).unwrap(), cancelled);
    assert!(cancelled.snapshot.cancelled);
    assert_eq!(cancelled.snapshot.evaluation, original.snapshot.evaluation);
    assert!(
        service
            .update(
                &mutation(actor.clone(), 802),
                &UpdateQuotationDraft {
                    draft_id: command.draft_id,
                    expected_revision: cancelled.snapshot.revision,
                    intent
                }
            )
            .is_err()
    );
    assert!(
        service
            .get(
                &actor,
                &GetQuotationDraft {
                    draft_id: command.draft_id
                }
            )
            .unwrap()
            .permitted_actions
            .is_empty()
    );
}

#[test]
fn quotation_confirmed_issue_and_revision_projection_preserve_competing_local_work() {
    use eitmad_contracts::{
        quotation_draft::*, quotation_lifecycle::QuotationPermittedAction as A,
    };
    let (_dir, store, _, actor, intent, _) = fixture_evaluation();
    let service =
        crate::QuotationDraftService::new(store.clone(), AuthorizationService::new(store));
    let draft = service
        .create(
            &mutation(actor.clone(), 810),
            &CreateQuotationDraft {
                intent: intent.clone(),
            },
        )
        .unwrap();
    let change = service.sync_batch(&actor, 50).unwrap().remove(0);
    service
        .project_confirmed(&actor, &change, CorrelationId::new(Uuid::new_v4()))
        .unwrap();
    let mut record = issued_cache_record(&actor, &draft.snapshot);
    let correlation = CorrelationId::new(Uuid::new_v4());
    service
        .cache_quotation(&actor, &record, correlation)
        .unwrap();
    assert!(
        service
            .get(
                &actor,
                &GetQuotationDraft {
                    draft_id: draft.snapshot.id
                }
            )
            .unwrap()
            .permitted_actions
            .is_empty()
    );
    assert!(
        service
            .update(
                &mutation(actor.clone(), 812),
                &UpdateQuotationDraft {
                    draft_id: draft.snapshot.id,
                    expected_revision: 1,
                    intent: intent.clone()
                }
            )
            .is_err()
    );
    record.revision = 3;
    record.document_revision = 2;
    record.state = QuotationState::Draft;
    record.quotation.revision = 2;
    record.issued_at = None;
    record.valid_until = None;
    record.permitted_actions = vec![A::Edit];
    service
        .cache_quotation(&actor, &record, correlation)
        .unwrap();
    let reopened = service
        .get(
            &actor,
            &GetQuotationDraft {
                draft_id: draft.snapshot.id,
            },
        )
        .unwrap();
    assert_eq!(reopened.snapshot, record.quotation);
    assert_eq!(reopened.permitted_actions, vec![A::Edit]);
    let local = service
        .update(
            &mutation(actor.clone(), 813),
            &UpdateQuotationDraft {
                draft_id: draft.snapshot.id,
                expected_revision: 2,
                intent,
            },
        )
        .unwrap();
    record.revision = 4;
    record.quotation.revision = 3;
    record.quotation.intent.lines[0]
        .configuration
        .selection
        .quantity += 1;
    service
        .cache_quotation(&actor, &record, correlation)
        .unwrap();
    let preserved = service
        .get(
            &actor,
            &GetQuotationDraft {
                draft_id: draft.snapshot.id,
            },
        )
        .unwrap();
    assert_eq!(preserved.snapshot, local.snapshot);
    assert_eq!(preserved.sync_state, QuotationDraftSyncState::Conflicted);
}

fn issued_cache_record(
    actor: &AuthorizationContext,
    snapshot: &eitmad_contracts::quotation_draft::QuotationDraftSnapshot,
) -> QuotationRecord {
    use eitmad_contracts::quotation_lifecycle::QuotationPermittedAction as A;
    QuotationRecord {
        scope: actor.scope.clone(),
        organization_id: actor.tenant_id.value(),
        revision: 2,
        document_revision: 1,
        state: QuotationState::Issued,
        quotation: snapshot.clone(),
        number: Some("QT-2026-00001".into()),
        validity_days: 30,
        issued_at: Some(UnixMillis(811)),
        valid_until: Some(UnixMillis(999)),
        approval_request_id: None,
        approval_fingerprint: None,
        changed_at: UnixMillis(811),
        changed_by: actor.identity.principal_id,
        cancellation_reason: None,
        acceptance: None,
        permitted_actions: vec![A::Print],
    }
}

#[test]
fn customer_document_preserves_saved_totals_and_contacts_after_catalog_edits() {
    use eitmad_contracts::quotation_draft::{QuotationDraftId, QuotationDraftSnapshot};
    use eitmad_contracts::quotation_lifecycle::QuotationPermittedAction;
    let (_dir, store, service, actor, input, mut entry) = fixture_evaluation();
    let saved = service.evaluate_quotation(&actor, &input).unwrap();
    let record = QuotationRecord {
        scope: actor.scope.clone(),
        organization_id: actor.tenant_id.value(),
        revision: 2,
        document_revision: 1,
        state: QuotationState::Issued,
        quotation: QuotationDraftSnapshot {
            cancelled: false,
            id: QuotationDraftId::new(Uuid::new_v4()),
            revision: 1,
            intent: input.clone(),
            evaluation: saved,
        },
        number: Some("QT-2026-00001".into()),
        validity_days: 30,
        issued_at: Some(UnixMillis(1)),
        valid_until: Some(UnixMillis(1000)),
        approval_request_id: None,
        approval_fingerprint: Some("INTERNAL_APPROVAL".into()),
        changed_at: UnixMillis(1),
        changed_by: actor.identity.principal_id,
        cancellation_reason: Some("INTERNAL_REASON".into()),
        acceptance: None,
        permitted_actions: vec![QuotationPermittedAction::Print],
    };
    let before = crate::customer_document(&record).unwrap();
    assert_eq!(
        (before.subtotal_yer, before.discount_yer, before.total_yer),
        (1010, 51, 959)
    );
    assert_eq!(before.lines[0].total_yer, 1010);
    assert!(before.can_print);
    entry.name = "اسم جديد".into();
    entry.price.selling_price_yer = 5000;
    entry.price.revision += 1;
    let mut catalog_actor = actor.clone();
    catalog_actor.scope = entry.price.target.scope().clone();
    project_sales(&store, &catalog_actor, vec![(1, 2, Some(entry))]);
    assert_eq!(crate::customer_document(&record).unwrap(), before);
    let json = serde_json::to_string(&before).unwrap();
    for forbidden in [
        "INTERNAL_APPROVAL",
        "INTERNAL_REASON",
        "approvalRequired",
        "costYer",
        "snapshot",
        "intent",
    ] {
        assert!(!json.contains(forbidden));
    }
    let mut draft = record.clone();
    draft.state = QuotationState::Draft;
    let document = crate::customer_document(&draft).unwrap();
    assert!(document.is_draft);
    assert!(!document.can_print);
    draft.quotation.evaluation.totals = None;
    assert_eq!(crate::customer_document(&draft), Err(E::Invalid));
}
