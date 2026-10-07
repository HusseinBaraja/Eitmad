use super::*;
use crate::approval::{
    ApprovalError, approval_fingerprint, decide_approval, same_commercial_terms,
};
use eitmad_contracts::{
    quotation_approval::*,
    quotation_draft::{QuotationDraftId, QuotationDraftSnapshot},
};

pub(super) fn request() -> DiscountApproval {
    let (_dir, _, service, actor, mut intent, _) = fixture_evaluation();
    intent.discount_basis_points = 501;
    let evaluation = service.evaluate_quotation(&actor, &intent).unwrap();
    let quotation = QuotationDraftSnapshot {
        cancelled: false,
        id: QuotationDraftId::new(Uuid::from_u128(90)),
        revision: 1,
        intent,
        evaluation,
    };
    let expiry = UnixMillis(2_000_000_000_000);
    DiscountApproval {
        scope: actor.scope,
        organization_id: actor.tenant_id.value(),
        request_id: DiscountRequestId::new(Uuid::from_u128(91)),
        revision: 1,
        fingerprint: approval_fingerprint(
            actor.tenant_id,
            actor.tenant_id.value(),
            &quotation,
            30,
            expiry,
        )
        .unwrap(),
        quotation,
        validity_days: 30,
        proposed_valid_until: expiry,
        requester: actor.identity.principal_id,
        requested_at: UnixMillis(1),
        state: DiscountApprovalState::Pending,
        decider: None,
        decided_at: None,
        reason: None,
        correlation_id: CorrelationId::new(Uuid::from_u128(92)),
    }
}
fn command(a: &DiscountApproval, decision: DiscountDecision) -> DecideDiscountApproval {
    DecideDiscountApproval {
        draft_id: a.quotation.id,
        request_id: a.request_id,
        quotation_revision: a.quotation.revision,
        expected_revision: a.revision,
        fingerprint: a.fingerprint.clone(),
        decision,
        reason: None,
    }
}
#[test]
fn discount_decisions_bind_revision_terms_actor_and_required_rejection_reason() {
    let pending = request();
    let manager = PrincipalId::new(Uuid::from_u128(999));
    let approve = command(&pending, DiscountDecision::Approve);
    assert_eq!(
        decide_approval(
            &pending,
            &pending.quotation,
            pending.requester,
            &approve,
            UnixMillis(2)
        ),
        Err(ApprovalError::Denied)
    );
    let approved = decide_approval(
        &pending,
        &pending.quotation,
        manager,
        &approve,
        UnixMillis(2),
    )
    .unwrap();
    assert_eq!(approved.state, DiscountApprovalState::Approved);
    assert_eq!(approved.decider, Some(manager));
    assert_eq!(approved.revision, 2);
    assert_eq!(
        decide_approval(
            &approved,
            &approved.quotation,
            manager,
            &approve,
            UnixMillis(3)
        ),
        Err(ApprovalError::Conflict)
    );
    for field in 0..4 {
        let mut stale = approve.clone();
        match field {
            0 => stale.request_id = DiscountRequestId::new(Uuid::new_v4()),
            1 => stale.quotation_revision += 1,
            2 => stale.expected_revision += 1,
            _ => stale.fingerprint.push('0'),
        }
        assert_eq!(
            decide_approval(&pending, &pending.quotation, manager, &stale, UnixMillis(2)),
            Err(ApprovalError::Conflict)
        );
    }
    let mut reject = command(&pending, DiscountDecision::Reject);
    for reason in [
        None,
        Some("   ".into()),
        Some("x".repeat(241)),
        Some("سبب\nآخر".into()),
    ] {
        reject.reason = reason;
        assert_eq!(
            decide_approval(
                &pending,
                &pending.quotation,
                manager,
                &reject,
                UnixMillis(2)
            ),
            Err(ApprovalError::Invalid)
        );
    }
    reject.reason = Some("  الخصم مرتفع  ".into());
    let rejected = decide_approval(
        &pending,
        &pending.quotation,
        manager,
        &reject,
        UnixMillis(2),
    )
    .unwrap();
    assert_eq!(rejected.state, DiscountApprovalState::Rejected);
    assert_eq!(rejected.reason.as_deref(), Some("الخصم مرتفع"));
}
#[test]
fn discount_commercial_changes_invalidate_while_contact_and_display_changes_retain_binding() {
    let a = request();
    let manager = PrincipalId::new(Uuid::from_u128(999));
    let decision = command(&a, DiscountDecision::Approve);
    let mut contact = a.quotation.clone();
    contact.revision += 1;
    contact.intent.customer.as_mut().unwrap().revision += 1;
    let customer = contact.evaluation.customer.as_mut().unwrap();
    customer.revision += 1;
    customer.name = "اسم العرض".into();
    customer.phone = "777000000".into();
    customer.address = Some("عنوان جديد".into());
    assert!(same_commercial_terms(&a.quotation, &contact));
    assert!(decide_approval(&a, &contact, manager, &decision, UnixMillis(2)).is_ok());
    assert_ne!(
        approval_fingerprint(
            TenantId::new(a.organization_id),
            a.organization_id,
            &contact,
            30,
            a.proposed_valid_until
        )
        .unwrap(),
        a.fingerprint
    );
    for field in 0..7 {
        let mut changed = a.quotation.clone();
        match field {
            0 => {
                changed.intent.customer.as_mut().unwrap().id =
                    eitmad_contracts::customer::CustomerId::new(Uuid::new_v4());
            }
            1 => changed.intent.lines[0].configuration.selection.quantity += 1,
            2 => changed.intent.discount_basis_points += 1,
            3 => {
                changed.intent.lines[0]
                    .configuration
                    .selection
                    .price_revision += 1;
            }
            4 => changed.evaluation.lines[0].price.snapshot.selling_price_yer += 1,
            5 => changed.evaluation.totals.as_mut().unwrap().total_yer += 1,
            _ => changed.evaluation.scope.id = ScopeId::new(Uuid::new_v4()),
        }
        assert!(!same_commercial_terms(&a.quotation, &changed));
        assert_eq!(
            decide_approval(&a, &changed, manager, &decision, UnixMillis(2)),
            Err(ApprovalError::Conflict)
        );
    }
    for (org, days, expiry) in [
        (Uuid::new_v4(), 30, a.proposed_valid_until),
        (a.organization_id, 31, a.proposed_valid_until),
        (
            a.organization_id,
            30,
            UnixMillis(a.proposed_valid_until.0 + 1),
        ),
    ] {
        assert_ne!(
            approval_fingerprint(
                TenantId::new(a.organization_id),
                org,
                &a.quotation,
                days,
                expiry
            )
            .unwrap(),
            a.fingerprint
        );
    }
}
