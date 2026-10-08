use super::*;
use eitmad_contracts::{
    identity::{PrincipalId, ScopeId, ScopeKind, ScopeRef},
    order::{
        AcceptanceMethod, CancelOrder, EditOrderFulfillment, RecordOrderDelivery,
        TransitionOrderWork,
    },
    quotation::{EvaluateQuotation, QuotationEvaluation},
    quotation_draft::{QuotationDraftId, QuotationDraftSnapshot},
    quotation_lifecycle::{QuotationRecord, QuotationState},
};
use uuid::Uuid;

fn order(work: Vec<OrderWork>) -> OrderRecord {
    let scope = ScopeRef {
        kind: ScopeKind::parse("branch").unwrap(),
        id: ScopeId::new(Uuid::new_v4()),
    };
    let actor = PrincipalId::new(Uuid::new_v4());
    OrderRecord {
        id: Uuid::new_v4(),
        scope: scope.clone(),
        organization_id: Uuid::new_v4(),
        revision: 1,
        number: "OR-2026-00001".into(),
        state: derived_state(&work),
        work,
        source: QuotationRecord {
            scope: scope.clone(),
            organization_id: Uuid::new_v4(),
            revision: 2,
            document_revision: 1,
            state: QuotationState::Accepted,
            quotation: QuotationDraftSnapshot {
                cancelled: false,
                id: QuotationDraftId::new(Uuid::new_v4()),
                revision: 1,
                intent: EvaluateQuotation {
                    customer: None,
                    lines: vec![],
                    discount_basis_points: 0,
                },
                evaluation: QuotationEvaluation {
                    scope,
                    customer: None,
                    lines: vec![],
                    currency: "YER".into(),
                    discount_basis_points: 0,
                    totals: None,
                    errors: vec![],
                    server_available: true,
                },
            },
            number: Some("QT-2026-00001".into()),
            validity_days: 30,
            issued_at: Some(UnixMillis(1)),
            valid_until: Some(UnixMillis(1000)),
            approval_request_id: None,
            approval_fingerprint: None,
            changed_at: UnixMillis(1),
            changed_by: actor,
            cancellation_reason: None,
            acceptance: None,
            permitted_actions: vec![],
        },
        delivery: None,
        fulfillment_note: None,
        cancellation_reason: None,
        created_at: UnixMillis(1),
        changed_at: UnixMillis(1),
        changed_by: actor,
        permitted_actions: vec![],
    }
}
fn planned() -> OrderWork {
    OrderWork {
        id: Uuid::new_v4(),
        number: "WO-2026-00001".into(),
        state: WorkState::Planned,
        line_ids: vec![Uuid::new_v4()],
        due_at: None,
        assignment: None,
    }
}
#[test]
fn fulfillment_progress_derives_readiness_and_preserves_commercial_snapshot() {
    let mut value = order(vec![planned(), planned()]);
    let snapshot = value.source.clone();
    for index in 0..2 {
        let input = TransitionOrderWork {
            order_id: value.id,
            expected_revision: value.revision,
            work_id: value.work[index].id,
            due_at: Some(UnixMillis(10)),
            assignment: Some("ورشة تجريبية".into()),
        };
        apply(
            &mut value,
            &OrderAction::StartWork(input.clone()),
            UnixMillis(2),
        )
        .unwrap();
        assert_eq!(value.state, OrderState::InProduction);
        let mut complete = input;
        complete.expected_revision = value.revision;
        apply(
            &mut value,
            &OrderAction::CompleteWork(complete),
            UnixMillis(3),
        )
        .unwrap();
        assert_eq!(
            value.state,
            if index == 1 {
                OrderState::Ready
            } else {
                OrderState::Confirmed
            }
        );
    }
    let edit = OrderAction::EditFulfillment(EditOrderFulfillment {
        order_id: value.id,
        expected_revision: value.revision,
        note: Some("تغليف تجريبي".into()),
    });
    apply(&mut value, &edit, UnixMillis(4)).unwrap();
    assert_eq!(value.source, snapshot);
    let delivery = OrderAction::Deliver(RecordOrderDelivery {
        order_id: value.id,
        expected_revision: value.revision,
        recipient: "مستلم تجريبي".into(),
        method: AcceptanceMethod::Written,
        note: None,
    });
    apply(&mut value, &delivery, UnixMillis(5)).unwrap();
    assert_eq!(value.state, OrderState::Delivered);
    assert_eq!(value.delivery.as_ref().unwrap().delivered_at, UnixMillis(5));
    let cancel = OrderAction::Cancel(CancelOrder {
        order_id: value.id,
        expected_revision: value.revision,
        reason: "إلغاء".into(),
    });
    assert_eq!(
        apply(&mut value, &cancel, UnixMillis(6)),
        Err(OrderError::Conflict)
    );
    assert_eq!(value.source, snapshot);
}
#[test]
fn cancellation_preserves_completed_work_and_blocks_delivery_and_stale_edits() {
    let mut work = vec![planned(), planned()];
    work[0].state = WorkState::Completed;
    let mut value = order(work);
    let source = value.source.clone();
    let delivery = OrderAction::Deliver(RecordOrderDelivery {
        order_id: value.id,
        expected_revision: 1,
        recipient: "مستلم".into(),
        method: AcceptanceMethod::Phone,
        note: None,
    });
    assert_eq!(
        apply(&mut value, &delivery, UnixMillis(2)),
        Err(OrderError::Conflict)
    );
    let stale = OrderAction::EditFulfillment(EditOrderFulfillment {
        order_id: value.id,
        expected_revision: 0,
        note: Some("تجربة".into()),
    });
    assert_eq!(
        apply(&mut value, &stale, UnixMillis(2)),
        Err(OrderError::Conflict)
    );
    let cancel = OrderAction::Cancel(CancelOrder {
        order_id: value.id,
        expected_revision: 1,
        reason: "ألغى العميل الطلب".into(),
    });
    apply(&mut value, &cancel, UnixMillis(3)).unwrap();
    assert_eq!(value.state, OrderState::Cancelled);
    assert_eq!(value.work[0].state, WorkState::Completed);
    assert_eq!(value.work[1].state, WorkState::Cancelled);
    assert_eq!(
        apply(&mut value, &delivery, UnixMillis(4)),
        Err(OrderError::Conflict)
    );
    assert_eq!(value.source, source);
    assert_eq!(derived_state(&[]), OrderState::Ready);
}
