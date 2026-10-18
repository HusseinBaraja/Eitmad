//! Customer projections preserve branch identity and enforce current read grants.
use super::*;
use eitmad_contracts::customer::*;

#[derive(Debug)]
struct Directory(Customer);
impl eitmad_customer::CustomerDirectory for Directory {
    fn read(
        &self,
        _: &AuthorizationContext,
        query: &CustomerRead,
    ) -> Result<CustomerPage, CustomerError> {
        let items = match query {
            CustomerRead::Get(q) if q.branch_id != Some(self.0.scope.id) => vec![],
            _ => vec![self.0.clone()],
        };
        Ok(CustomerPage { items, next: None })
    }
    fn watch(
        &self,
        _: &AuthorizationContext,
        _: &AtomicBool,
        _: &mut dyn FnMut(CustomerChangeNotice),
    ) -> Result<(), CustomerError> {
        Err(CustomerError::Unavailable)
    }
}
fn manager(dispatcher: &ProductDispatcher) {
    let mutation = ProductDispatcher::mutation_context(&context(7000)).unwrap();
    dispatcher
        .authorization
        .grant_relationship(
            &mutation,
            &GrantScopeRelationship {
                expected_policy_version: 1,
                subject: RelationshipSubject {
                    principal_id: authorization().identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
                relation: RelationId::parse(eitmad_authorization::MANAGER_RELATION).unwrap(),
            },
        )
        .unwrap();
}
fn contact() -> Customer {
    Customer {
        id: CustomerId::new(Uuid::new_v4()),
        scope: branch_authorization().scope,
        name: CustomerName::parse("عميل تجريبي").unwrap(),
        phone: CustomerPhone::parse("777123456").unwrap(),
        address: None,
        notes: None,
        status: CustomerStatus::Active,
        revision: 1,
        updated_at: UnixMillis(1),
        sync_state: CustomerSyncState::Confirmed,
    }
}
#[tokio::test]
async fn manager_home_customer_count_and_navigation_use_authorized_branch_identity() {
    let (_directory, dispatcher, _) = dispatcher();
    manager(&dispatcher);
    let customer = contact();
    let dispatcher = dispatcher.with_customer_directory(Arc::new(Directory(customer.clone())));
    let QueryResult::Home(page) = dispatcher
        .dispatch_query(
            context(7001),
            Query::Home(eitmad_contracts::home::ReadHome {
                term: "عميل".into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("home")
    };
    assert_eq!(page.customers.count, 1);
    assert_eq!(page.customers.items[0].branch_id, Some(customer.scope.id));
    assert_eq!(page.customers.items[0].id, customer.id.value());
    let mut denied = context(7002);
    denied.authorization.identity.principal_id = PrincipalId::new(Uuid::new_v4());
    let QueryResult::Home(page) = dispatcher
        .dispatch_query(
            denied,
            Query::Home(eitmad_contracts::home::ReadHome {
                term: "عميل".into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("home")
    };
    assert_eq!(
        page.customers.availability,
        eitmad_contracts::home::HomeAvailability::Denied
    );
    assert_eq!(page.customers.count, 0);
    assert!(page.customers.items.is_empty());
    assert!(
        dispatcher
            .authorization
            .authorize(
                &authorization(),
                eitmad_authorization::CUSTOMER_WRITE_PERMISSION
            )
            .is_err()
    );
    assert!(
        dispatcher
            .authorization
            .authorize(
                &authorization(),
                eitmad_authorization::QUOTATION_DRAFT_WRITE_PERMISSION
            )
            .is_err()
    );
}

#[tokio::test]
async fn customer_history_filters_cached_records_by_customer_and_branch() {
    let (_directory, dispatcher, _) = dispatcher();
    manager(&dispatcher);
    let customer = contact();
    let mut matching = orders::document_source();
    matching.organization_id = authorization().tenant_id.value();
    matching.quotation.evaluation.customer.as_mut().unwrap().id = customer.id;
    let mut other_customer = matching.clone();
    other_customer.quotation.id =
        eitmad_contracts::quotation_draft::QuotationDraftId::new(Uuid::new_v4());
    other_customer
        .quotation
        .evaluation
        .customer
        .as_mut()
        .unwrap()
        .id = CustomerId::new(Uuid::new_v4());
    let mut other_branch = matching.clone();
    other_branch.scope.id = ScopeId::new(Uuid::new_v4());
    other_branch.quotation.id =
        eitmad_contracts::quotation_draft::QuotationDraftId::new(Uuid::new_v4());
    for record in [&matching, &other_customer, &other_branch] {
        dispatcher
            .drafts
            .cache_quotation(&authorization(), record, CorrelationId::new(Uuid::new_v4()))
            .unwrap();
    }
    let dispatcher = dispatcher.with_customer_directory(Arc::new(Directory(customer.clone())));
    let query = GetCustomerHistory {
        customer: GetCustomer {
            customer_id: customer.id,
            branch_id: Some(customer.scope.id),
        },
        quotations_after: None,
        orders_after: None,
        include_quotations: true,
        include_orders: false,
    };
    let QueryResult::CustomerHistory(page) = dispatcher
        .dispatch_query(context(7003), Query::CustomerHistory(query.clone()))
        .await
        .unwrap()
    else {
        panic!("history")
    };
    assert_eq!(page.quotations.len(), 1);
    assert_eq!(page.quotations[0].quotation.id, matching.quotation.id);
    assert!(!page.server_available);
    let mut denied = context(7004);
    denied.authorization.identity.principal_id = PrincipalId::new(Uuid::new_v4());
    assert_eq!(
        dispatcher
            .dispatch_query(denied, Query::CustomerHistory(query))
            .await
            .unwrap_err()
            .code
            .as_str(),
        "eitmad.error.authorization-denied.v1"
    );
}
