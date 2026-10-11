//! Order IPC and confirmed cache; offline clients cannot create confirmed transitions.
use super::{
    Arc, Command, CommandResult, ContractError, DispatchContext, Event, MutationContext,
    ProductDispatcher, QueryResult, RetryDisposition, contract_error,
};
use eitmad_contracts::{
    identity::AuthorizationContext,
    order::{ConfirmOrder, ListOrders, OrderAction, OrderNotice, OrderRecord},
};
use eitmad_observability_audit::{AuditTarget, MutationAuditRecord};
use eitmad_orders::{OrderError as E, OrderServer};
use std::sync::atomic::{AtomicBool, Ordering};

impl ProductDispatcher {
    async fn work_order_list(
        &self,
        context: &DispatchContext,
        query: eitmad_contracts::work_order::ListWorkOrders,
    ) -> Result<QueryResult, ContractError> {
        self.require_approval(context, eitmad_authorization::WORK_READ_PERMISSION)
            .map_err(|e| *e)?;
        if !(1..=100).contains(&query.limit) {
            return Err(failure(E::Invalid, context));
        }
        let actor = context.authorization.clone();
        let deadline = context.deadline;
        let copy = query.clone();
        let result = if let Some(server) = self.order_server.clone() {
            tokio::task::spawn_blocking(move || server.work_orders(&actor, &copy, deadline))
                .await
                .map_err(|_| failure(E::Unavailable, context))?
        } else {
            Err(E::Unavailable)
        };
        self.authorization
            .authorize(
                &context.authorization,
                eitmad_authorization::WORK_READ_PERMISSION,
            )
            .map_err(|_| failure(E::Denied, context))?;
        let mut page = match result {
            Ok(page) => {
                for value in &page.items {
                    if context.authorization.scope.kind.as_str() == "branch"
                        && value.scope != context.authorization.scope
                    {
                        return Err(failure(E::Denied, context));
                    }
                    let audit = MutationAuditRecord::from_authorization(
                        &context.authorization,
                        eitmad_authorization::now(),
                        context.correlation_id,
                        "eitmad.work-order.cache.v1",
                        AuditTarget {
                            kind: "work-order".into(),
                            identifiers: vec![value.id.to_string()],
                        },
                    );
                    self.store
                        .transact_pricing(true, |tx| tx.cache_work_order(value, &audit))
                        .map_err(|_| failure(E::Unavailable, context))?;
                }
                self.drain_pending_publications()
                    .map_err(|_| failure(E::Unavailable, context))?;
                page
            }
            Err(E::Unavailable) => self
                .store
                .cached_work_orders(&context.authorization, &query)
                .map_err(|_| failure(E::Unavailable, context))?,
            Err(e) => return Err(failure(e, context)),
        };
        page.pending = self
            .store
            .pending_orders(&context.authorization)
            .map_err(|_| failure(E::Unavailable, context))?;
        Ok(QueryResult::WorkOrders(page))
    }
    #[must_use]
    pub fn with_orders(mut self, server: Arc<dyn OrderServer>) -> Self {
        self.order_server = Some(server);
        self
    }
    pub(super) async fn order_command(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, ContractError> {
        let action = match command {
            Command::ConvertQuotation(c) => OrderAction::Convert(c),
            Command::CancelOrder(c) => OrderAction::Cancel(c),
            Command::EditOrderFulfillment(c) => OrderAction::EditFulfillment(c),
            Command::RecordOrderDelivery(c) => OrderAction::Deliver(c),
            Command::StartOrderWork(c) => OrderAction::StartWork(c),
            Command::CompleteOrderWork(c) => OrderAction::CompleteWork(c),
            _ => return Err(super::unsupported(context)),
        };
        self.require_approval(context, eitmad_orders::permission(&action))
            .map_err(|e| *e)?;

        let actor = context.authorization.clone();
        let deadline = context.deadline;
        let request = ConfirmOrder {
            scope: actor.scope.clone(),
            idempotency_key: mutation.idempotency_key,
            action,
        };
        self.store
            .stage_order(&context.authorization, &request)
            .map_err(|_| failure(E::Invalid, context))?;
        let server = self
            .order_server
            .clone()
            .ok_or_else(|| failure(E::Unavailable, context))?;
        let outcome =
            tokio::task::spawn_blocking(move || server.transition(&actor, &request, deadline))
                .await
                .map_err(|_| failure(E::Unavailable, context))?;
        let value = match outcome {
            Ok(value) => value,
            Err(e) => {
                if e != E::Unavailable {
                    self.store
                        .finish_order(
                            &context.authorization,
                            mutation.idempotency_key,
                            Some(eitmad_orders::error_code(e)),
                        )
                        .map_err(|_| failure(E::Unavailable, context))?;
                }
                return Err(failure(e, context));
            }
        };
        self.cache_order(context, &value)
            .map_err(|e| failure(e, context))?;
        self.store
            .finish_order(&context.authorization, mutation.idempotency_key, None)
            .map_err(|_| failure(E::Unavailable, context))?;
        self.drain_pending_publications()
            .map_err(|_| failure(E::Unavailable, context))?;
        Ok(CommandResult::Order(Box::new(value)))
    }
    fn cache_order(&self, context: &DispatchContext, value: &OrderRecord) -> Result<(), E> {
        self.authorization
            .authorize(
                &context.authorization,
                eitmad_authorization::ORDER_READ_PERMISSION,
            )
            .map_err(|_| E::Denied)?;
        let scope = &context.authorization.scope;
        if scope.kind.as_str() == "branch" && value.scope != *scope {
            return Err(E::Denied);
        }
        let audit = MutationAuditRecord::from_authorization(
            &context.authorization,
            value.changed_at,
            context.correlation_id,
            "eitmad.order.cache.v1",
            AuditTarget {
                kind: "order".into(),
                identifiers: vec![value.id.to_string()],
            },
        );
        self.store
            .transact_pricing(true, |tx| tx.cache_order(value, &audit))
            .map_err(|_| E::Unavailable)
    }
    pub(super) async fn order_query(
        &self,
        context: &DispatchContext,
        query: eitmad_contracts::queries::Query,
    ) -> Result<QueryResult, ContractError> {
        match query {
            eitmad_contracts::queries::Query::OrderCustomerDocument(query) => {
                self.order_document(context, query, false).await
            }
            eitmad_contracts::queries::Query::OrderQuotationCustomerDocument(query) => {
                self.order_document(context, query, true).await
            }
            eitmad_contracts::queries::Query::WorkOrders(query) => {
                self.work_order_list(context, query).await
            }
            eitmad_contracts::queries::Query::Orders(query) => {
                self.order_list(context, query, None).await
            }
            eitmad_contracts::queries::Query::Order(query) => {
                self.order_list(
                    context,
                    ListOrders {
                        after: None,
                        limit: 1,
                    },
                    Some(query.order_id),
                )
                .await
            }
            _ => Err(super::unsupported(context)),
        }
    }
    pub(super) async fn order_list(
        &self,
        context: &DispatchContext,
        query: ListOrders,
        order_id: Option<uuid::Uuid>,
    ) -> Result<QueryResult, ContractError> {
        self.require_approval(context, eitmad_authorization::ORDER_READ_PERMISSION)
            .map_err(|e| *e)?;
        if !(1..=100).contains(&query.limit) {
            return Err(failure(E::Invalid, context));
        }
        let actor = context.authorization.clone();
        let deadline = context.deadline;
        let copy = query.clone();
        let result = if let Some(server) = self.order_server.clone() {
            tokio::task::spawn_blocking(move || server.orders(&actor, &copy, order_id, deadline))
                .await
                .map_err(|_| failure(E::Unavailable, context))?
        } else {
            Err(E::Unavailable)
        };
        self.require_approval(context, eitmad_authorization::ORDER_READ_PERMISSION)
            .map_err(|e| *e)?;
        let mut page = match result {
            Ok(page) => {
                for value in &page.items {
                    self.cache_order(context, value)
                        .map_err(|e| failure(e, context))?;
                }
                self.drain_pending_publications()
                    .map_err(|_| failure(E::Unavailable, context))?;
                Ok(page)
            }
            Err(E::Unavailable) => self
                .store
                .cached_orders(&context.authorization, query.after, query.limit, order_id)
                .map_err(|_| failure(E::Unavailable, context)),
            Err(e) => Err(failure(e, context)),
        }?;
        page.pending = self
            .store
            .pending_orders(&context.authorization)
            .map_err(|_| failure(E::Unavailable, context))?;
        Ok(QueryResult::Orders(page))
    }
}
fn failure(e: E, context: &DispatchContext) -> ContractError {
    let code = eitmad_orders::error_code(e);
    contract_error(
        code,
        &code.replace(".error.", ".message."),
        context.correlation_id,
        if e == E::Unavailable {
            RetryDisposition::SafeAfterDelay(1000)
        } else {
            RetryDisposition::Never
        },
        None,
    )
}

pub(super) struct OrderWatch {
    actor: String,
    cancel: Arc<AtomicBool>,
}
impl Drop for OrderWatch {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl ProductDispatcher {
    pub(super) fn start_order_watch(&self, actor: &AuthorizationContext) -> Result<(), E> {
        let server = self.order_server.clone().ok_or(E::Unavailable)?;
        let key = format!(
            "{}:{}:{}",
            actor.session_id.value(),
            actor.scope.kind.as_str(),
            actor.scope.id.value()
        );
        let mut watch = self.order_watch.lock().map_err(|_| E::Unavailable)?;
        if watch
            .as_ref()
            .is_some_and(|w| w.actor == key && !w.cancel.load(Ordering::Acquire))
        {
            return Ok(());
        }
        *watch = None;
        let cancel = Arc::new(AtomicBool::new(false));
        *watch = Some(OrderWatch {
            actor: key,
            cancel: cancel.clone(),
        });
        let actor = actor.clone();
        let authorization = self.authorization.clone();
        let events = self.events.clone();
        std::thread::Builder::new()
            .name("eitmad-orders".into())
            .spawn(move || {
                let mut delay = 1;
                let mut unavailable = false;
                while !cancel.load(Ordering::Acquire) {
                    let result = server.watch(&actor, &cancel, &mut |notice| {
                        delay = 1;
                        unavailable = false;
                        if authorization
                            .authorize(&actor, eitmad_authorization::ORDER_READ_PERMISSION)
                            .is_ok()
                        {
                            let _ =
                                events.publish(actor.scope.clone(), Event::OrderChanged(notice));
                        } else {
                            cancel.store(true, Ordering::Release);
                        }
                    });
                    if result == Err(E::Denied) {
                        break;
                    }
                    if result.is_err()
                        && !unavailable
                        && authorization
                            .authorize(&actor, eitmad_authorization::ORDER_READ_PERMISSION)
                            .is_ok()
                    {
                        let _ = events.publish(
                            actor.scope.clone(),
                            Event::OrderChanged(OrderNotice {
                                scope: actor.scope.clone(),
                                order_id: uuid::Uuid::nil(),
                                revision: 0,
                            }),
                        );
                        unavailable = true;
                    }
                    for _ in 0..delay * 10 {
                        if cancel.load(Ordering::Acquire) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    delay = (delay * 2).min(30);
                }
                cancel.store(true, Ordering::Release);
            })
            .map_err(|_| E::Unavailable)?;
        Ok(())
    }
}

impl ProductDispatcher {
    pub(super) async fn order_document(
        &self,
        context: &DispatchContext,
        query: eitmad_contracts::order::GetOrder,
        source: bool,
    ) -> Result<QueryResult, ContractError> {
        let QueryResult::Orders(page) = self
            .order_list(
                context,
                ListOrders {
                    after: None,
                    limit: 1,
                },
                Some(query.order_id),
            )
            .await?
        else {
            return Err(failure(E::Unavailable, context));
        };
        self.authorization
            .authorize(
                &context.authorization,
                eitmad_authorization::ORDER_READ_PERMISSION,
            )
            .map_err(|_| failure(E::Denied, context))?;
        let record = page
            .items
            .iter()
            .find(|r| r.id == query.order_id)
            .ok_or_else(|| failure(E::Invalid, context))?;
        let mut document = eitmad_pricing::customer_document(&record.source)
            .map_err(|_| failure(E::Invalid, context))?;
        if document.is_draft || document.number.is_none() {
            return Err(failure(E::Invalid, context));
        }
        document.can_print = true;
        if !source {
            document.number = Some(record.number.clone());
            document.document_revision = record.revision;
            document.status = match record.state {
                eitmad_contracts::order::OrderState::Confirmed => "مؤكد",
                eitmad_contracts::order::OrderState::InProduction => "قيد الإنتاج",
                eitmad_contracts::order::OrderState::Ready => "جاهز",
                eitmad_contracts::order::OrderState::Delivered => "تم التسليم",
                eitmad_contracts::order::OrderState::Cancelled => "ملغي",
            }
            .into();
            document.saved_at = record.changed_at;
            document.issued_at = Some(record.created_at);
            document.valid_until = None;
            document.validity_days = None;
        }
        Ok(QueryResult::CustomerDocument(Box::new(document)))
    }
}
