//! Home reads reuse each capability's authorization, cache and server boundary.
use super::{ProductDispatcher, customer_error, draft_error, unsupported};
use crate::local_ipc::DispatchContext;
use eitmad_contracts::{
    errors::ContractError,
    home::{
        HomeAvailability as Available, HomeDestination as Destination, HomeItem, HomeSection,
        HomeSnapshot, ReadHome,
    },
    order::{ListOrders, OrderState},
    queries::{Query, QueryResult},
    quotation_approval::{DiscountApprovalState, ListDiscountApprovals},
    quotation_draft::ListQuotationDrafts,
    quotation_lifecycle::{ListQuotations, QuotationState},
    transport::UnixMillis,
};
use std::collections::BTreeMap;

// Bound server calls, memory and IPC output independently of organization size.
const MAX_PAGES: usize = 10;
const ROWS: usize = 8;

fn empty(availability: Available) -> HomeSection {
    HomeSection {
        availability,
        complete: false,
        server_available: false,
        count: 0,
        secondary_count: 0,
        items: vec![],
    }
}
fn section() -> HomeSection {
    HomeSection {
        availability: Available::Available,
        complete: true,
        server_available: true,
        ..empty(Available::Available)
    }
}
fn failed(error: &ContractError) -> HomeSection {
    empty(
        if error.code.as_str() == "eitmad.error.authorization-denied.v1" {
            Available::Denied
        } else {
            Available::Unavailable
        },
    )
}
fn matches(item: &HomeItem, phone: &str, term: &str) -> bool {
    eitmad_material::normalize_search(&format!(
        "{} {} {phone}",
        item.number.as_deref().unwrap_or(""),
        item.title
    ))
    .contains(term)
        || !term.is_empty()
            && eitmad_customer::normalize_phone(phone).is_some_and(|p| p.contains(term))
}
fn finish(section: &mut HomeSection) {
    section
        .items
        .sort_by(|a, b| b.changed_at.cmp(&a.changed_at).then(a.id.cmp(&b.id)));
    section.items.truncate(ROWS);
}

fn quotation_row(
    value: eitmad_contracts::quotation_lifecycle::QuotationRecord,
) -> Option<(HomeItem, String, bool, u64)> {
    let customer = value.quotation.evaluation.customer?;
    let open = matches!(
        value.state,
        QuotationState::Draft
            | QuotationState::PendingApproval
            | QuotationState::Issued
            | QuotationState::Accepted
    );
    Some((
        HomeItem {
            id: value.quotation.id.value(),
            destination: Destination::Quotation,
            number: value.number,
            title: customer.name,
            state: quotation_state(value.state).into(),
            changed_at: value.changed_at,
        },
        customer.phone,
        open,
        value.quotation.revision,
    ))
}

impl ProductDispatcher {
    pub(super) async fn home(
        &self,
        context: &DispatchContext,
        input: ReadHome,
    ) -> Result<QueryResult, ContractError> {
        if input.term.len() > 256 || input.term.chars().any(char::is_control) {
            return Err(unsupported(context));
        }
        let term = eitmad_material::normalize_search(input.term.trim());
        let mut catalog_context = context.clone();
        // The public catalog belongs to the authenticated tenant organization.
        catalog_context.authorization.scope = eitmad_contracts::identity::ScopeRef {
            kind: eitmad_contracts::identity::ScopeKind::parse("organization")
                .expect("static scope"),
            id: eitmad_contracts::identity::ScopeId::new(context.authorization.tenant_id.value()),
        };
        let mut quotations = self.home_quotations(context, &term).await;
        let (mut orders, mut ready_orders) = self.home_orders(context, &term).await;
        let mut approvals = self.home_approvals(context).await;
        let mut customers = self.home_customers(context, input.term.trim());
        let mut catalog = self.home_catalog(&catalog_context, input.term.trim()).await;
        // A revocation during the multi-source read must also remove earlier results.
        for (value, permission) in [
            (
                &mut quotations,
                eitmad_authorization::QUOTATION_READ_PERMISSION,
            ),
            (&mut orders, eitmad_authorization::ORDER_READ_PERMISSION),
            (
                &mut approvals,
                eitmad_authorization::DISCOUNT_READ_PERMISSION,
            ),
        ] {
            if let Err(e) = self.require_approval(context, permission) {
                *value = failed(&e);
            }
        }
        if context.authorization.scope.kind.as_str() == "branch" {
            if let Err(e) = self.require_approval(
                context,
                eitmad_authorization::QUOTATION_DRAFT_READ_PERMISSION,
            ) {
                quotations = failed(&e);
            }
            if let Err(e) =
                self.require_approval(context, eitmad_customer::CUSTOMER_READ_PERMISSION)
            {
                customers = failed(&e);
            }
        }
        if let Err(e) = self.require_approval(
            &catalog_context,
            eitmad_authorization::CATALOG_READ_PERMISSION,
        ) {
            catalog = failed(&e);
        }
        if orders.availability != Available::Available {
            ready_orders.clear();
        }
        Ok(QueryResult::Home(HomeSnapshot {
            quotations,
            orders,
            approvals,
            customers,
            catalog,
            ready_orders,
        }))
    }

    async fn home_quotations(&self, context: &DispatchContext, term: &str) -> HomeSection {
        let mut result = section();
        let mut records = BTreeMap::new();
        let mut after = None;
        for _ in 0..MAX_PAGES {
            let page = match self
                .quotation_list(context, ListQuotations { after, limit: 100 })
                .await
            {
                Ok(QueryResult::Quotations(p)) => p,
                Err(e) => return failed(&e),
                _ => return empty(Available::Unavailable),
            };
            result.server_available &= page.server_available;
            for value in page.items {
                let Some(row) = quotation_row(value) else {
                    return empty(Available::Unavailable);
                };
                records.insert(row.0.id, row);
            }
            after = page.next;
            if after.is_none() {
                break;
            }
        }
        result.complete &= after.is_none();
        // Pending local edits remain visible without replacing issued history.
        after = None;
        for _ in 0..(usize::from(context.authorization.scope.kind.as_str() == "branch") * MAX_PAGES)
        {
            let page = match self.drafts.list(
                &context.authorization,
                &ListQuotationDrafts { after, limit: 100 },
            ) {
                Ok(p) => p,
                Err(e) => return failed(&draft_error(e, context)),
            };
            for value in page.items {
                let id = value.snapshot.id.value();
                if records.get(&id).is_some_and(|(item, _, _, revision)| {
                    item.number.is_some() || *revision >= value.snapshot.revision
                }) {
                    continue;
                }
                let Some(customer) = value.snapshot.evaluation.customer else {
                    return empty(Available::Unavailable);
                };
                let item = HomeItem {
                    id,
                    destination: Destination::Quotation,
                    number: None,
                    title: customer.name,
                    state: if value.snapshot.cancelled {
                        "ملغي"
                    } else {
                        "مسودة محلية"
                    }
                    .into(),
                    changed_at: value.updated_at,
                };
                records.insert(
                    id,
                    (
                        item,
                        customer.phone,
                        !value.snapshot.cancelled,
                        value.snapshot.revision,
                    ),
                );
            }
            after = page.next;
            if after.is_none() {
                break;
            }
        }
        result.complete &= after.is_none();
        for (item, phone, open, _) in records.into_values() {
            result.count += u32::from(open);
            if matches(&item, &phone, term) {
                result.items.push(item);
            }
        }
        finish(&mut result);
        result
    }

    async fn home_orders(
        &self,
        context: &DispatchContext,
        term: &str,
    ) -> (HomeSection, Vec<HomeItem>) {
        let mut result = section();
        let mut ready = vec![];
        let mut after = None;
        for _ in 0..MAX_PAGES {
            let page = match self
                .order_list(context, ListOrders { after, limit: 100 }, None)
                .await
            {
                Ok(QueryResult::Orders(p)) => p,
                Err(e) => return (failed(&e), vec![]),
                _ => return (empty(Available::Unavailable), vec![]),
            };
            result.server_available &= page.server_available;
            for value in page.items {
                result.count += u32::from(!matches!(
                    value.state,
                    OrderState::Cancelled | OrderState::Delivered
                ));
                result.secondary_count += u32::from(value.state == OrderState::Ready);
                let Some(customer) = value.source.quotation.evaluation.customer else {
                    return (empty(Available::Unavailable), vec![]);
                };
                let item = HomeItem {
                    id: value.id,
                    destination: Destination::Order,
                    number: Some(value.number),
                    title: customer.name,
                    state: order_state(value.state).into(),
                    changed_at: value.changed_at,
                };
                if value.state == OrderState::Ready {
                    ready.push(item.clone());
                }
                if matches(&item, &customer.phone, term) {
                    result.items.push(item);
                }
            }
            after = page.next;
            if after.is_none() {
                break;
            }
        }
        result.complete = after.is_none();
        finish(&mut result);
        ready.sort_by(|a, b| b.changed_at.cmp(&a.changed_at).then(a.id.cmp(&b.id)));
        ready.truncate(ROWS);
        (result, ready)
    }

    async fn home_approvals(&self, context: &DispatchContext) -> HomeSection {
        let mut result = section();
        let mut after = None;
        for _ in 0..MAX_PAGES {
            let page = match self
                .approval_list(context, ListDiscountApprovals { after, limit: 100 })
                .await
            {
                Ok(QueryResult::DiscountApprovals(p)) => p,
                Err(e) => return failed(&e),
                _ => return empty(Available::Unavailable),
            };
            result.count += u32::try_from(
                page.items
                    .iter()
                    .filter(|v| v.state == DiscountApprovalState::Pending)
                    .count(),
            )
            .unwrap_or(0);
            after = page.next;
            if after.is_none() {
                break;
            }
        }
        result.complete = after.is_none();
        result
    }

    fn home_customers(&self, context: &DispatchContext, term: &str) -> HomeSection {
        let mut result = section();
        if term.is_empty() {
            return result;
        }
        // Organization-wide customer reads are not yet implemented by the customer authority.
        if context.authorization.scope.kind.as_str() != "branch" {
            return empty(Available::Unavailable);
        }
        let query = eitmad_contracts::customer::CustomerSearchTerm::parse(term)
            .ok()
            .and_then(|t| eitmad_contracts::customer::SearchCustomers::new(t, None, 8).ok());
        let Some(query) = query else {
            return empty(Available::Unavailable);
        };
        match self.customers.search(&context.authorization, &query) {
            Ok(page) => {
                result.complete = page.next.is_none();
                result.items = page
                    .items
                    .into_iter()
                    .map(|c| HomeItem {
                        id: c.id.value(),
                        destination: Destination::Customer,
                        number: None,
                        title: c.name.as_str().into(),
                        state: "عميل".into(),
                        changed_at: c.updated_at,
                    })
                    .collect();
            }
            Err(e) => return failed(&customer_error(e, context)),
        }
        result
    }

    async fn home_catalog(&self, context: &DispatchContext, term: &str) -> HomeSection {
        let mut result = section();
        if term.is_empty() {
            return result;
        }
        let query = Query::SalesCatalog(eitmad_contracts::sales_catalog::ListSalesCatalog {
            term: term.into(),
            category: None,
            after: None,
            limit: 8,
        });
        match self.sales_catalog_query(context, query).await {
            Ok(QueryResult::SalesCatalog(page)) => {
                result.complete = page.next.is_none();
                result.server_available = page.server_available;
                result.items = page
                    .items
                    .into_iter()
                    .map(|c| HomeItem {
                        id: c.price.target.identity().1,
                        destination: Destination::Catalog,
                        number: None,
                        title: c.name,
                        state: c.variant_name,
                        changed_at: UnixMillis(0),
                    })
                    .collect();
            }
            Err(e) => return failed(&e),
            _ => return empty(Available::Unavailable),
        }
        result
    }
}

fn quotation_state(value: QuotationState) -> &'static str {
    match value {
        QuotationState::Draft => "مسودة",
        QuotationState::PendingApproval => "بانتظار الموافقة",
        QuotationState::Issued => "صادر",
        QuotationState::Accepted => "مقبول",
        QuotationState::Converted => "محوّل",
        QuotationState::Expired => "منتهي",
        QuotationState::Cancelled => "ملغي",
    }
}
fn order_state(value: OrderState) -> &'static str {
    match value {
        OrderState::Confirmed => "مؤكد",
        OrderState::InProduction => "قيد الإنتاج",
        OrderState::Ready => "جاهز",
        OrderState::Delivered => "تم التسليم",
        OrderState::Cancelled => "ملغي",
    }
}
