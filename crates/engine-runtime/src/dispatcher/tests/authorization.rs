//! Negative requests reach Rust directly; no shell role flag participates.
use super::*;
use eitmad_contracts::{
    accounts::*, commands::RevokeScopeRelationship, order::*, quotation_approval::*,
    quotation_draft::*, quotation_lifecycle::*,
};
use eitmad_orders::{OrderError, OrderServer};
use eitmad_pricing::{ApprovalError, DiscountApprovalServer, QuotationError, QuotationServer};

struct RevokingReadServer(AuthorityStore);

impl RevokingReadServer {
    fn revoke(&self, actor: &AuthorizationContext) {
        let relationship = self
            .0
            .relationships_for_subject(
                &actor.scope,
                &RelationshipSubject {
                    principal_id: actor.identity.principal_id,
                    principal_kind: actor.identity.principal_kind,
                },
            )
            .unwrap()
            .into_iter()
            .find(|r| r.relation.as_str() == eitmad_authorization::RECEPTIONIST_RELATION)
            .unwrap();
        let mut mutation = ProductDispatcher::mutation_context(&branch_context(9001)).unwrap();
        mutation.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
        AuthorizationService::new(self.0.clone())
            .revoke_relationship(
                &mutation,
                &RevokeScopeRelationship {
                    expected_policy_version: self.0.policy_version(&actor.scope).unwrap(),
                    relationship_id: relationship.relationship_id,
                },
            )
            .unwrap();
    }
}

impl OrderServer for RevokingReadServer {
    fn orders(
        &self,
        actor: &AuthorizationContext,
        _: &ListOrders,
        _: Option<Uuid>,
        _: UnixMillis,
    ) -> Result<OrderPage, OrderError> {
        self.revoke(actor);
        Err(OrderError::Unavailable)
    }
    fn work_orders(
        &self,
        _: &AuthorizationContext,
        _: &eitmad_contracts::work_order::ListWorkOrders,
        _: UnixMillis,
    ) -> Result<eitmad_contracts::work_order::WorkOrderPage, OrderError> {
        unreachable!("only commercial list reads are exercised")
    }
    fn transition(
        &self,
        _: &AuthorizationContext,
        _: &ConfirmOrder,
        _: UnixMillis,
    ) -> Result<OrderRecord, OrderError> {
        unreachable!("only list reads are exercised")
    }
    fn watch(
        &self,
        _: &AuthorizationContext,
        _: &AtomicBool,
        _: &mut dyn FnMut(OrderNotice),
    ) -> Result<(), OrderError> {
        unreachable!("only list reads are exercised")
    }
}

impl QuotationServer for RevokingReadServer {
    fn quotations(
        &self,
        actor: &AuthorizationContext,
        _: &ListQuotations,
        _: UnixMillis,
    ) -> Result<QuotationPage, QuotationError> {
        self.revoke(actor);
        Err(QuotationError::Unavailable)
    }
    fn quotation_transition(
        &self,
        _: &AuthorizationContext,
        _: &ConfirmQuotation,
        _: UnixMillis,
    ) -> Result<QuotationRecord, QuotationError> {
        unreachable!("only list reads are exercised")
    }
    fn watch_quotations(
        &self,
        _: &AuthorizationContext,
        _: &AtomicBool,
        _: &mut dyn FnMut(QuotationNotice),
    ) -> Result<(), QuotationError> {
        unreachable!("only list reads are exercised")
    }
}

impl DiscountApprovalServer for RevokingReadServer {
    fn list(
        &self,
        actor: &AuthorizationContext,
        _: &ListDiscountApprovals,
        _: UnixMillis,
    ) -> Result<DiscountApprovalPage, ApprovalError> {
        self.revoke(actor);
        Err(ApprovalError::Unavailable)
    }
    fn transition(
        &self,
        _: &AuthorizationContext,
        _: &ConfirmDiscountApproval,
        _: UnixMillis,
    ) -> Result<Option<DiscountApproval>, ApprovalError> {
        unreachable!("only list reads are exercised")
    }
    fn watch(
        &self,
        _: &AuthorizationContext,
        _: &AtomicBool,
        _: &mut dyn FnMut(DiscountApprovalNotice),
    ) -> Result<(), ApprovalError> {
        unreachable!("only list reads are exercised")
    }
}

#[tokio::test]
async fn revoked_during_remote_read_cannot_receive_an_empty_or_cached_success() {
    for query in [
        Query::Orders(ListOrders {
            after: None,
            limit: 10,
        }),
        Query::Quotations(ListQuotations {
            after: None,
            limit: 10,
        }),
        Query::DiscountApprovals(ListDiscountApprovals {
            after: None,
            limit: 10,
        }),
    ] {
        let (_directory, dispatcher, _) = fixture();
        let server = Arc::new(RevokingReadServer(dispatcher.store.clone()));
        let dispatcher = dispatcher
            .with_orders(server.clone())
            .with_quotations(server.clone())
            .with_discount_approvals(server);
        let error = dispatcher
            .dispatch_query(role_context(4, true), query)
            .await
            .unwrap_err();
        assert_eq!(error.code.as_str(), "eitmad.error.authorization-denied.v1");
    }
}

fn role_context(principal: u128, branch: bool) -> DispatchContext {
    let mut actor = if branch {
        branch_context(9000)
    } else {
        context(9000)
    };
    actor.authorization.identity.principal_id = PrincipalId::new(Uuid::from_u128(principal));
    actor
}

fn fixture() -> (TempDir, ProductDispatcher, EventBroker) {
    let (directory, dispatcher, broker) = dispatcher();
    grant_material_roles(&dispatcher);
    let owner = branch_context(9001);
    dispatcher
        .authorization
        .bootstrap_owner(
            &ProductDispatcher::mutation_context(&owner).unwrap(),
            &RelationshipSubject {
                principal_id: owner.authorization.identity.principal_id,
                principal_kind: PrincipalKind::User,
            },
        )
        .unwrap();
    for (principal, role, version) in [
        (3, eitmad_authorization::MANAGER_RELATION, 1),
        (4, eitmad_authorization::RECEPTIONIST_RELATION, 2),
    ] {
        let mut mutation = ProductDispatcher::mutation_context(&owner).unwrap();
        mutation.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
        dispatcher
            .authorization
            .grant_relationship(
                &mutation,
                &GrantScopeRelationship {
                    expected_policy_version: version,
                    subject: RelationshipSubject {
                        principal_id: PrincipalId::new(Uuid::from_u128(principal)),
                        principal_kind: PrincipalKind::User,
                    },
                    relation: RelationId::parse(role).unwrap(),
                },
            )
            .unwrap();
    }
    (directory, dispatcher, broker)
}

async fn deny_commands(
    dispatcher: &ProductDispatcher,
    actor: DispatchContext,
    commands: Vec<Command>,
) {
    for command in commands {
        let operation = command.kind();
        let error = dispatcher
            .dispatch_command(actor.clone(), command)
            .await
            .unwrap_err();
        assert_eq!(
            error.code.as_str(),
            "eitmad.error.authorization-denied.v1",
            "{operation}"
        );
    }
    assert!(
        dispatcher
            .store
            .pending_orders(&actor.authorization)
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn manager_cannot_perform_receptionist_commercial_mutations() {
    let (_directory, dispatcher, _) = fixture();
    let draft_id = QuotationDraftId::new(Uuid::new_v4());
    let intent = eitmad_contracts::quotation::EvaluateQuotation {
        customer: None,
        lines: vec![],
        discount_basis_points: 0,
    };
    deny_commands(
        &dispatcher,
        role_context(3, true),
        vec![
            Command::CreateQuotationDraft(CreateQuotationDraft {
                intent: intent.clone(),
            }),
            Command::UpdateQuotationDraft(UpdateQuotationDraft {
                draft_id,
                expected_revision: 1,
                intent,
            }),
            Command::CancelQuotationDraft(CancelQuotationDraft {
                draft_id,
                expected_revision: 1,
            }),
            Command::RequestDiscountApproval(RequestDiscountApproval {
                draft_id,
                expected_revision: 1,
            }),
            Command::IssueQuotation(IssueQuotation {
                draft_id,
                expected_revision: 1,
                expected_draft_revision: 1,
            }),
            Command::AcceptQuotation(AcceptQuotation {
                draft_id,
                expected_revision: 1,
                method: AcceptanceMethod::InPerson,
                note: None,
            }),
            Command::ConvertQuotation(ConvertQuotation {
                draft_id,
                expected_revision: 1,
            }),
            Command::RecordOrderDelivery(RecordOrderDelivery {
                order_id: Uuid::new_v4(),
                expected_revision: 1,
                recipient: "مستلم تجريبي".into(),
                method: AcceptanceMethod::InPerson,
                note: None,
            }),
        ],
    )
    .await;
}

#[tokio::test]
async fn receptionist_cannot_decide_discounts_manage_validity_or_production() {
    let (_directory, dispatcher, _) = fixture();
    let draft_id = QuotationDraftId::new(Uuid::new_v4());
    let work = TransitionOrderWork {
        order_id: Uuid::new_v4(),
        expected_revision: 1,
        work_id: Uuid::new_v4(),
        due_at: None,
        assignment: None,
    };
    deny_commands(
        &dispatcher,
        role_context(4, true),
        vec![
            Command::DecideDiscountApproval(DecideDiscountApproval {
                draft_id,
                request_id: DiscountRequestId::new(Uuid::new_v4()),
                quotation_revision: 1,
                expected_revision: 1,
                fingerprint: "synthetic".into(),
                decision: DiscountDecision::Approve,
                reason: None,
            }),
            Command::SetQuotationValidity(SetQuotationValidity {
                draft_id,
                expected_revision: 1,
                validity_days: 30,
            }),
            Command::CancelQuotation(CancelQuotation {
                draft_id,
                expected_revision: 1,
                reason: "سبب تجريبي".into(),
            }),
            Command::CancelOrder(CancelOrder {
                order_id: work.order_id,
                expected_revision: 1,
                reason: "سبب تجريبي".into(),
            }),
            Command::EditOrderFulfillment(EditOrderFulfillment {
                order_id: work.order_id,
                expected_revision: 1,
                note: Some("ملاحظة تجريبية".into()),
            }),
            Command::StartOrderWork(work.clone()),
            Command::CompleteOrderWork(work),
        ],
    )
    .await;
}

#[tokio::test]
async fn receptionist_cannot_administer_accounts_or_read_private_catalog() {
    let (_directory, dispatcher, _) = fixture();
    let actor = role_context(4, false);
    deny_commands(
        &dispatcher,
        actor.clone(),
        vec![
            Command::CreateDesktopAccount(CreateDesktopAccount {
                display_name: "حساب تجريبي".into(),
                username: "synthetic".into(),
                password: AccountPassword::new("synthetic-password-123"),
                role: DesktopAccountRole::Manager,
            }),
            Command::UpdateDesktopAccount(UpdateDesktopAccount {
                account_id: eitmad_contracts::identity::AccountId::new(Uuid::new_v4()),
                expected_revision: 1,
                display_name: "حساب تجريبي".into(),
                role: DesktopAccountRole::Manager,
            }),
            Command::DeactivateDesktopAccount(DeactivateDesktopAccount {
                account_id: eitmad_contracts::identity::AccountId::new(Uuid::new_v4()),
                expected_revision: 1,
            }),
        ],
    )
    .await;
    let queries = vec![
        Query::DesktopAccounts(ListDesktopAccounts {}),
        Query::WorkOrders(eitmad_contracts::work_order::ListWorkOrders {
            after: None,
            limit: 10,
            order_id: None,
        }),
        Query::Products(eitmad_contracts::product::ListProducts {
            term: String::new(),
            after: None,
            limit: 10,
            selectable_only: false,
        }),
        Query::ProductCategories(eitmad_contracts::product::ListProductCategories {
            after: None,
            limit: 10,
        }),
        Query::Materials(ListMaterials::new(String::new(), None, 10).unwrap()),
        Query::Parts(ListParts {
            term: String::new(),
            after: None,
            limit: 10,
        }),
        Query::Furnitures(eitmad_contracts::furniture::ListFurnitures {
            term: String::new(),
            after: None,
            limit: 10,
            selectable_only: false,
        }),
    ];
    for query in queries {
        let operation = query.kind();
        let error = dispatcher
            .dispatch_query(actor.clone(), query)
            .await
            .unwrap_err();
        assert_eq!(
            error.code.as_str(),
            "eitmad.error.authorization-denied.v1",
            "{operation}"
        );
    }
    for subscription in [
        Subscription::Materials(MaterialChanges {}),
        Subscription::Parts(PartChanges {}),
        Subscription::Furnitures(eitmad_contracts::furniture::FurnitureChanges {}),
        Subscription::Products(eitmad_contracts::product::ProductChanges {}),
    ] {
        let error = dispatcher
            .authorize_subscription(
                SubscriptionContext {
                    authorization: actor.authorization.clone(),
                    correlation_id: actor.correlation_id,
                    protocol_version: PROTOCOL_VERSION,
                },
                &subscription,
            )
            .await
            .unwrap_err();
        assert_eq!(error.code.as_str(), "eitmad.error.authorization-denied.v1");
    }
}

#[tokio::test]
async fn foreign_tenant_cannot_reuse_assigned_branch_for_reads_counts_or_events() {
    let (_directory, dispatcher, _) = fixture();
    let mut actor = role_context(4, true);
    actor.authorization.tenant_id = TenantId::new(Uuid::new_v4());
    for query in [
        Query::Customers(
            SearchCustomers::new(CustomerSearchTerm::parse("عميل").unwrap(), None, 10).unwrap(),
        ),
        Query::QuotationDrafts(ListQuotationDrafts {
            after: None,
            limit: 10,
        }),
        Query::Orders(ListOrders {
            after: None,
            limit: 10,
        }),
        Query::OrderCustomerDocument(GetOrder {
            order_id: Uuid::new_v4(),
        }),
    ] {
        assert_eq!(
            dispatcher
                .dispatch_query(actor.clone(), query)
                .await
                .unwrap_err()
                .code
                .as_str(),
            "eitmad.error.authorization-denied.v1"
        );
    }
    let QueryResult::Home(home) = dispatcher
        .dispatch_query(
            actor.clone(),
            Query::Home(eitmad_contracts::home::ReadHome {
                term: "عميل".into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("home")
    };
    assert!(
        home.orders.items.is_empty()
            && home.quotations.items.is_empty()
            && home.customers.items.is_empty()
    );
    assert_eq!(home.orders.count, 0);
    assert_eq!(home.quotations.count, 0);
    for section in [
        &home.orders,
        &home.quotations,
        &home.approvals,
        &home.customers,
    ] {
        assert_eq!(
            section.availability,
            eitmad_contracts::home::HomeAvailability::Denied
        );
    }
    let error = dispatcher
        .authorize_subscription(
            SubscriptionContext {
                authorization: actor.authorization,
                correlation_id: actor.correlation_id,
                protocol_version: PROTOCOL_VERSION,
            },
            &Subscription::Customers(CustomerChanges {}),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code.as_str(), "eitmad.error.authorization-denied.v1");
}
