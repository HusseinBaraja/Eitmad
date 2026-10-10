//! Order lifecycle. Commercial snapshots never change after conversion.
#[cfg(test)]
mod tests;
use eitmad_contracts::{
    identity::AuthorizationContext,
    order::{
        ConfirmOrder, ListOrders, OrderAction, OrderDelivery, OrderNotice, OrderPage,
        OrderPermittedAction, OrderRecord, OrderState, OrderWork, WorkState,
    },
    transport::UnixMillis,
};

pub const ORDER_SCHEMA: &str = "eitmad.schema.order.v1";
pub const WORK_ORDER_SCHEMA: &str = "eitmad.schema.work-order.v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderError {
    Denied,
    Invalid,
    Conflict,
    Unavailable,
}
#[must_use]
pub const fn error_code(error: OrderError) -> &'static str {
    match error {
        OrderError::Denied => "eitmad.error.authorization-denied.v1",
        OrderError::Invalid => "eitmad.error.order-invalid.v1",
        OrderError::Conflict => "eitmad.error.order-conflict.v1",
        OrderError::Unavailable => "eitmad.error.order-unavailable.v1",
    }
}
#[must_use]
pub const fn permission(action: &OrderAction) -> &'static str {
    match action {
        OrderAction::Convert(_) => "eitmad.permission.quotation.convert.v1",
        OrderAction::Cancel(_) => "eitmad.permission.order.cancel.v1",
        OrderAction::EditFulfillment(_) => "eitmad.permission.order.fulfillment.write.v1",
        OrderAction::Deliver(_) => "eitmad.permission.delivery.record.v1",
        OrderAction::StartWork(_) | OrderAction::CompleteWork(_) => {
            "eitmad.permission.work-order.transition.v1"
        }
    }
}
#[must_use]
pub const fn target(action: &OrderAction) -> uuid::Uuid {
    match action {
        OrderAction::Convert(c) => c.draft_id.value(),
        OrderAction::Cancel(c) => c.order_id,
        OrderAction::EditFulfillment(c) => c.order_id,
        OrderAction::Deliver(c) => c.order_id,
        OrderAction::StartWork(c) | OrderAction::CompleteWork(c) => c.order_id,
    }
}
#[must_use]
pub const fn expected_revision(action: &OrderAction) -> u64 {
    match action {
        OrderAction::Convert(c) => c.expected_revision,
        OrderAction::Cancel(c) => c.expected_revision,
        OrderAction::EditFulfillment(c) => c.expected_revision,
        OrderAction::Deliver(c) => c.expected_revision,
        OrderAction::StartWork(c) | OrderAction::CompleteWork(c) => c.expected_revision,
    }
}
/// # Errors
/// Rejects empty required values, controls, and unbounded notes.
pub fn validate_text(text: &str, required: bool) -> Result<(), OrderError> {
    if required && text.trim().is_empty()
        || text.chars().count() > 240
        || text.chars().any(char::is_control)
    {
        Err(OrderError::Invalid)
    } else {
        Ok(())
    }
}
#[must_use]
pub fn actions(value: &OrderRecord, reception: bool, manager: bool) -> Vec<OrderPermittedAction> {
    use OrderPermittedAction as A;
    let mut result = vec![];
    if matches!(value.state, OrderState::Delivered | OrderState::Cancelled) {
        return result;
    }
    if manager {
        result.extend([A::Cancel, A::EditFulfillment]);
        if value.work.iter().any(|w| w.state == WorkState::Planned) {
            result.push(A::StartWork);
        }
        if value.work.iter().any(|w| w.state == WorkState::InProgress) {
            result.push(A::CompleteWork);
        }
    }
    if reception && value.state == OrderState::Ready {
        result.push(A::Deliver);
    }
    result
}
#[must_use]
pub fn derived_state(work: &[OrderWork]) -> OrderState {
    if work.iter().all(|w| w.state == WorkState::Completed) {
        OrderState::Ready
    } else if work.iter().any(|w| w.state == WorkState::InProgress) {
        OrderState::InProduction
    } else {
        OrderState::Confirmed
    }
}
/// Validates complete Furniture coverage and the accepted manufacturing specifications.
/// # Errors
/// Rejects missing, duplicate, or ready-made lines and incomplete accepted specifications.
pub fn validate_production(value: &OrderRecord) -> Result<(), OrderError> {
    use eitmad_contracts::pricing::PriceTarget;
    let source = &value.source.quotation;
    let required: Vec<_> = source
        .intent
        .lines
        .iter()
        .filter(|l| matches!(l.configuration.selection.target, PriceTarget::Furniture(_)))
        .map(|l| l.id)
        .collect();
    let mut included = std::collections::HashSet::new();
    let mut work_ids = std::collections::HashSet::new();
    for work in &value.work {
        if work.line_ids.is_empty() || !work_ids.insert(work.id) {
            return Err(OrderError::Conflict);
        }
        for id in &work.line_ids {
            if !required.contains(id) || !included.insert(*id) {
                return Err(OrderError::Conflict);
            }
            let line = source
                .evaluation
                .lines
                .iter()
                .find(|l| l.id == *id)
                .ok_or(OrderError::Conflict)?;
            let intent = source
                .intent
                .lines
                .iter()
                .find(|l| l.id == *id)
                .ok_or(OrderError::Conflict)?;
            let selection = &intent.configuration.selection;
            if line.quantity != selection.quantity
                || line.dimensions != intent.configuration.dimensions
                || line.color_id != selection.color_id
                || line.handle_id != selection.handle_id
                || line.price.snapshot.target != selection.target
            {
                return Err(OrderError::Conflict);
            }
            if line.quantity == 0
                || line
                    .dimensions
                    .as_ref()
                    .is_none_or(|d| [d.width_mm, d.height_mm, d.depth_mm].contains(&0))
            {
                return Err(OrderError::Conflict);
            }
        }
    }
    if included.len() != required.len() {
        return Err(OrderError::Conflict);
    }
    Ok(())
}
/// Applies authorized fulfillment intent without changing accepted commercial content.
/// # Errors
/// Rejects stale revisions, terminal states, invalid text and invalid work/delivery transitions.
pub fn apply(
    value: &mut OrderRecord,
    action: &OrderAction,
    now: UnixMillis,
) -> Result<(), OrderError> {
    if value.revision != expected_revision(action)
        || matches!(value.state, OrderState::Delivered | OrderState::Cancelled)
    {
        return Err(OrderError::Conflict);
    }
    if matches!(
        action,
        OrderAction::StartWork(_) | OrderAction::CompleteWork(_) | OrderAction::Deliver(_)
    ) {
        validate_production(value)?;
    }
    match action {
        OrderAction::Convert(_) => return Err(OrderError::Invalid),
        OrderAction::Cancel(c) => {
            validate_text(&c.reason, true)?;
            value.cancellation_reason = Some(c.reason.clone());
            for w in &mut value.work {
                if matches!(w.state, WorkState::Planned | WorkState::InProgress) {
                    w.state = WorkState::Cancelled;
                }
            }
            value.state = OrderState::Cancelled;
        }
        OrderAction::EditFulfillment(c) => {
            validate_text(c.note.as_deref().unwrap_or(""), false)?;
            value.fulfillment_note.clone_from(&c.note);
        }
        OrderAction::Deliver(c) => {
            if value.state != OrderState::Ready || value.delivery.is_some() {
                return Err(OrderError::Conflict);
            }
            validate_text(&c.recipient, true)?;
            validate_text(c.note.as_deref().unwrap_or(""), false)?;
            value.delivery = Some(OrderDelivery {
                id: uuid::Uuid::new_v4(),
                recipient: c.recipient.clone(),
                method: c.method,
                note: c.note.clone(),
                delivered_at: now,
                actor: value.changed_by,
            });
            value.state = OrderState::Delivered;
        }
        OrderAction::StartWork(c) | OrderAction::CompleteWork(c) => {
            let starting = matches!(action, OrderAction::StartWork(_));
            let work = value
                .work
                .iter_mut()
                .find(|w| w.id == c.work_id)
                .ok_or(OrderError::Conflict)?;
            if starting {
                if work.state != WorkState::Planned {
                    return Err(OrderError::Conflict);
                }
                if c.due_at.is_none_or(|d| d.0 < now.0) {
                    return Err(OrderError::Invalid);
                }
                validate_text(c.assignment.as_deref().unwrap_or(""), true)?;
                work.due_at = c.due_at;
                work.assignment.clone_from(&c.assignment);
                work.state = WorkState::InProgress;
            } else {
                if work.state != WorkState::InProgress || work.line_ids.is_empty() {
                    return Err(OrderError::Conflict);
                }
                work.state = WorkState::Completed;
            }
            value.state = derived_state(&value.work);
        }
    }
    value.revision = value
        .revision
        .checked_add(1)
        .ok_or(OrderError::Unavailable)?;
    value.changed_at = now;
    Ok(())
}
/// Authenticated server transport; every write requires central confirmation.
pub trait OrderServer: Send + Sync {
    /// Reads full production details only for an authorized Manager.
    /// # Errors
    /// Rejects foreign scopes, denied actors, and invalid bounds.
    fn work_orders(
        &self,
        actor: &AuthorizationContext,
        query: &eitmad_contracts::work_order::ListWorkOrders,
        deadline: UnixMillis,
    ) -> Result<eitmad_contracts::work_order::WorkOrderPage, OrderError>;
    /// # Errors
    /// Denies unauthorized or invalid requests; uncertain outcomes remain retryable.
    fn transition(
        &self,
        actor: &AuthorizationContext,
        request: &ConfirmOrder,
        deadline: UnixMillis,
    ) -> Result<OrderRecord, OrderError>;
    /// # Errors
    /// Denies foreign scopes and invalid bounds.
    fn orders(
        &self,
        actor: &AuthorizationContext,
        query: &ListOrders,
        order_id: Option<uuid::Uuid>,
        deadline: UnixMillis,
    ) -> Result<OrderPage, OrderError>;
    /// # Errors
    /// Ends on revocation, cancellation, or connection failure.
    fn watch(
        &self,
        actor: &AuthorizationContext,
        cancel: &std::sync::atomic::AtomicBool,
        notify: &mut dyn FnMut(OrderNotice),
    ) -> Result<(), OrderError>;
}
