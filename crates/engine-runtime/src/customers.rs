//! Customer reads and organization subscription lifecycle.
use super::{
    ContractError, CustomerError, DispatchContext, Event, ProductDispatcher, Query, QueryResult,
    customer_error,
};
use eitmad_contracts::{customer::CustomerChangeNotice, identity::AuthorizationContext};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
pub(super) struct CustomerWatch {
    actor: String,
    cancel: Arc<AtomicBool>,
}
impl Drop for CustomerWatch {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}
impl ProductDispatcher {
    #[must_use]
    pub fn with_customer_directory(
        mut self,
        directory: Arc<dyn eitmad_customer::CustomerDirectory>,
    ) -> Self {
        self.customers = self.customers.with_directory(directory);
        self
    }
    pub(super) async fn customer_history(
        &self,
        context: &DispatchContext,
        query: eitmad_contracts::customer::GetCustomerHistory,
    ) -> Result<QueryResult, ContractError> {
        let QueryResult::Customer(customer) = self
            .customer_query(context, Query::Customer(query.customer.clone()))
            .await?
        else {
            return Err(super::unsupported(context));
        };
        let mut result = eitmad_contracts::customer::CustomerHistoryPage {
            quotations: vec![],
            orders: vec![],
            quotations_next: None,
            orders_next: None,
            server_available: true,
        };
        if query.include_quotations {
            let QueryResult::Quotations(page) = self
                .quotation_list(
                    context,
                    eitmad_contracts::quotation_lifecycle::ListQuotations {
                        after: query.quotations_after,
                        limit: 100,
                    },
                )
                .await?
            else {
                return Err(super::unsupported(context));
            };
            result.quotations_next = page.next;
            result.server_available &= page.server_available;
            result.quotations = page
                .items
                .into_iter()
                .filter(|r| {
                    r.scope == customer.scope
                        && r.quotation
                            .evaluation
                            .customer
                            .as_ref()
                            .is_some_and(|c| c.id == customer.id)
                })
                .collect();
        }
        if query.include_orders {
            let QueryResult::Orders(page) = self
                .order_list(
                    context,
                    eitmad_contracts::order::ListOrders {
                        after: query.orders_after,
                        limit: 100,
                    },
                    None,
                )
                .await?
            else {
                return Err(super::unsupported(context));
            };
            result.orders_next = page.next;
            result.server_available &= page.server_available;
            result.orders = page
                .items
                .into_iter()
                .filter(|r| {
                    r.scope == customer.scope
                        && r.source
                            .quotation
                            .evaluation
                            .customer
                            .as_ref()
                            .is_some_and(|c| c.id == customer.id)
                })
                .collect();
        }
        self.authorization
            .authorize(
                &context.authorization,
                eitmad_authorization::CUSTOMER_READ_PERMISSION,
            )
            .map_err(|_| customer_error(CustomerError::Denied, context))?;
        Ok(QueryResult::CustomerHistory(result))
    }
    pub(super) async fn customer_query(
        &self,
        context: &DispatchContext,
        query: Query,
    ) -> Result<QueryResult, ContractError> {
        let service = self.customers.clone();
        let actor = context.authorization.clone();
        tokio::task::spawn_blocking(move || match query {
            Query::Customer(q) => service.get(&actor, &q).map(QueryResult::Customer),
            Query::Customers(q) => service.search(&actor, &q).map(QueryResult::Customers),
            Query::CustomerConflicts(q) => service
                .conflicts(&actor, &q)
                .map(|page| QueryResult::CustomerConflicts(Box::new(page))),
            _ => Err(CustomerError::Unavailable),
        })
        .await
        .map_err(|_| customer_error(CustomerError::Unavailable, context))?
        .map_err(|e| customer_error(e, context))
    }
}
impl ProductDispatcher {
    pub(super) async fn resolve_customer_conflict(
        &self,
        context: &DispatchContext,
        mutation: eitmad_authorization::MutationContext,
        command: eitmad_contracts::customer::ResolveCustomerConflict,
    ) -> Result<eitmad_contracts::commands::CommandResult, ContractError> {
        let service = self.customers.clone();
        let result = tokio::task::spawn_blocking(move || service.resolve(&mutation, &command))
            .await
            .map_err(|_| customer_error(CustomerError::Unavailable, context))?
            .map_err(|e| customer_error(e, context))?;
        self.drain_pending_publications()
            .map_err(|_| customer_error(CustomerError::Unavailable, context))?;
        Ok(eitmad_contracts::commands::CommandResult::CustomerConflictResolved(result))
    }
}
impl ProductDispatcher {
    pub(super) fn start_customer_watch(
        &self,
        actor: &AuthorizationContext,
    ) -> Result<(), CustomerError> {
        let server = self.customers.clone();
        let key = format!(
            "{}:{}:{}",
            actor.session_id.value(),
            actor.scope.kind.as_str(),
            actor.scope.id.value()
        );
        let mut watch = self
            .customer_watch
            .lock()
            .map_err(|_| CustomerError::Unavailable)?;
        if watch
            .as_ref()
            .is_some_and(|w| w.actor == key && !w.cancel.load(Ordering::Acquire))
        {
            return Ok(());
        }
        *watch = None;
        let cancel = Arc::new(AtomicBool::new(false));
        *watch = Some(CustomerWatch {
            actor: key,
            cancel: cancel.clone(),
        });
        let actor = actor.clone();
        let authorization = self.authorization.clone();
        let events = self.events.clone();
        std::thread::Builder::new()
            .name("eitmad-customers".into())
            .spawn(move || {
                let mut delay = 1;
                let mut unavailable = false;
                while !cancel.load(Ordering::Acquire) {
                    let result = server.watch_directory(&actor, &cancel, &mut |notice| {
                        delay = 1;
                        unavailable = false;
                        if authorization
                            .authorize(&actor, eitmad_authorization::CUSTOMER_READ_PERMISSION)
                            .is_ok()
                        {
                            let _ =
                                events.publish(actor.scope.clone(), Event::CustomerChanged(notice));
                        } else {
                            cancel.store(true, Ordering::Release);
                        }
                    });
                    if result == Err(CustomerError::Denied) {
                        break;
                    }
                    if result.is_err()
                        && !unavailable
                        && authorization
                            .authorize(&actor, eitmad_authorization::CUSTOMER_READ_PERMISSION)
                            .is_ok()
                    {
                        let _ = events.publish(
                            actor.scope.clone(),
                            Event::CustomerChanged(CustomerChangeNotice {
                                scope: actor.scope.clone(),
                                customer_id: eitmad_contracts::customer::CustomerId::new(
                                    uuid::Uuid::nil(),
                                ),
                                changed_at: eitmad_authorization::now(),
                                change_id: eitmad_contracts::sync::ChangeId::new(
                                    uuid::Uuid::new_v4(),
                                ),
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
            .map_err(|_| CustomerError::Unavailable)?;
        Ok(())
    }
}
