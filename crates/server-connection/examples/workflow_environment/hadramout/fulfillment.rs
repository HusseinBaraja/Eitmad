use super::super::super::{Client, Result, checked, deadline};
use eitmad_contracts::{
    order::{
        AcceptanceMethod, CancelOrder, ConfirmOrder, ListOrders, OrderAction, OrderRecord,
        RecordOrderDelivery, TransitionOrderWork, WorkState,
    },
    transport::{IdempotencyKey, UnixMillis},
};
use eitmad_orders::OrderServer as _;
use uuid::Uuid;

pub(super) fn run(manager: &Client, reception: &Client) -> Result<()> {
    let mut page = checked(
        manager.workflow.orders(
            &manager.branch,
            &ListOrders {
                after: None,
                limit: 100,
            },
            None,
            deadline(),
        ),
        "demo orders",
    )?;
    page.items.sort_by(|a, b| a.number.cmp(&b.number));
    for (index, order) in page.items.into_iter().enumerate() {
        progress(manager, reception, order, index)?;
    }
    Ok(())
}

fn transition(client: &Client, action: OrderAction, key: u128) -> Result<OrderRecord> {
    checked(
        client.workflow.transition(
            &client.branch,
            &ConfirmOrder {
                scope: client.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(key)),
                action,
            },
            deadline(),
        ),
        "demo fulfillment transition",
    )
}

fn progress(
    manager: &Client,
    reception: &Client,
    mut order: OrderRecord,
    index: usize,
) -> Result<()> {
    use eitmad_contracts::order::OrderState;
    if matches!(order.state, OrderState::Delivered | OrderState::Cancelled) {
        return Ok(());
    }
    let mode = index % 5;
    let base = 0x4844_7000 + index as u128 * 20;
    if mode == 4 {
        transition(manager, OrderAction::Cancel(CancelOrder {
            order_id: order.id, expected_revision: order.revision,
            reason: "الزبون أجّل تجهيز البيت وطلب إلغاء الطلب قبل موعد التسليم، بعد التواصل مع المعرض.".into(),
        }), base)?;
        return Ok(());
    }
    if mode == 0 {
        return Ok(());
    }
    for (work_index, work) in order.work.clone().into_iter().enumerate() {
        let key = base + work_index as u128 * 2 + 1;
        let mut input = TransitionOrderWork {
            order_id: order.id,
            expected_revision: order.revision,
            work_id: work.id,
            due_at: Some(UnixMillis(
                eitmad_authorization::now().0
                    + (i64::try_from(index % 12).map_err(|_| "demo due date")? + 1) * 86_400_000,
            )),
            assignment: Some(
                [
                    "المعلم عوض ـ نجارة",
                    "المعلم سالم ـ تنجيد",
                    "حسن ـ تشطيب وتركيب",
                    "أحمد ـ تجهيز وتغليف",
                ][index % 4]
                    .into(),
            ),
        };
        if work.state == WorkState::Planned {
            order = transition(manager, OrderAction::StartWork(input.clone()), key)?;
        }
        if mode >= 2 && work.state != WorkState::Completed {
            input.expected_revision = order.revision;
            order = transition(manager, OrderAction::CompleteWork(input), key + 1)?;
        }
    }
    if mode == 3 && order.state == OrderState::Ready {
        transition(reception, OrderAction::Deliver(RecordOrderDelivery {
            order_id: order.id, expected_revision: order.revision,
            recipient: order.source.quotation.evaluation.customer.as_ref().map_or_else(|| "صاحب البيت".into(), |c| c.name.clone()),
            method: AcceptanceMethod::InPerson,
            note: Some("تم التركيب في البيت، راجع الزبون المقاس والأبواب والتشطيب وقال تمام، يعطيكم العافية.".into()),
        }), base + 15)?;
    }
    Ok(())
}
