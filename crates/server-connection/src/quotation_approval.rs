use crate::{
    DirectServerConfig,
    authenticated_http::{AuthenticatedHttpClient, HttpError},
};
use eitmad_contracts::{
    identity::{AuthorizationContext, ScopeId, ScopeKind, ScopeRef},
    quotation_approval::{
        ConfirmDiscountApproval, DiscountApproval, DiscountApprovalAction, DiscountApprovalNotice,
        DiscountApprovalPage, ListDiscountApprovals, ReadDiscountApprovals,
    },
    quotation_draft::QuotationDraftSnapshot,
    secrets::SecretId,
    server::{ServerClientMessage, ServerMessage, ServerSubscriptionRequest},
    transport::{SchemaId, UnixMillis},
};
use eitmad_pricing::{ApprovalError as E, DISCOUNT_APPROVAL_SCHEMA, DiscountApprovalServer};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub struct DirectDiscountApprovalClient {
    http: AuthenticatedHttpClient,
    config: DirectServerConfig,
    secrets: eitmad_secret_storage::SecretStore,
    credential: SecretId,
    branch: ScopeRef,
}
impl DirectDiscountApprovalClient {
    #[must_use]
    pub fn from_config(
        config: DirectServerConfig,
        secrets: eitmad_secret_storage::SecretStore,
        credential: SecretId,
        branch: ScopeRef,
    ) -> Self {
        Self {
            http: AuthenticatedHttpClient::from_config(
                config.clone(),
                secrets.clone(),
                credential.clone(),
                "eitmad.capability.quotation-approval.v1",
                20,
            ),
            config,
            secrets,
            credential,
            branch,
        }
    }
    fn remote(&self, actor: &AuthorizationContext) -> Result<ScopeRef, E> {
        match actor.scope.kind.as_str() {
            "branch" => Ok(self.branch.clone()),
            "organization" if actor.scope.id.value() == actor.tenant_id.value() => {
                Ok(self.config.scope.clone())
            }
            _ => Err(E::Denied),
        }
    }
    fn map_snapshot(s: &mut QuotationDraftSnapshot, branch: &ScopeRef, organization: &ScopeRef) {
        use eitmad_contracts::pricing::PriceTarget;
        s.evaluation.scope = branch.clone();
        for line in &mut s.intent.lines {
            match &mut line.configuration.selection.target {
                PriceTarget::Product(v) => v.scope = organization.clone(),
                PriceTarget::Furniture(v) => v.scope = organization.clone(),
            }
        }
        for line in &mut s.evaluation.lines {
            match &mut line.price.snapshot.target {
                PriceTarget::Product(v) => v.scope = organization.clone(),
                PriceTarget::Furniture(v) => v.scope = organization.clone(),
            }
        }
    }
    fn localize(actor: &AuthorizationContext, a: &mut DiscountApproval) {
        let branch = if actor.scope.kind.as_str() == "branch" {
            actor.scope.clone()
        } else {
            a.scope.clone()
        };
        let organization = ScopeRef {
            kind: ScopeKind::parse("organization").expect("scope"),
            id: ScopeId::new(actor.tenant_id.value()),
        };
        a.scope = branch.clone();
        Self::map_snapshot(&mut a.quotation, &branch, &organization);
    }
}
fn budget(deadline: UnixMillis) -> Result<Instant, E> {
    let ms = deadline.0 - eitmad_authorization::now().0;
    if ms <= 0 {
        return Err(E::Unavailable);
    }
    Ok(Instant::now()
        + Duration::from_millis(u64::try_from(ms.min(10_000)).map_err(|_| E::Unavailable)?))
}
fn error(e: &HttpError) -> E {
    match e {
        HttpError::Denied => E::Denied,
        HttpError::Invalid => E::Invalid,
        HttpError::Conflict => E::Conflict,
        _ => E::Unavailable,
    }
}
impl DiscountApprovalServer for DirectDiscountApprovalClient {
    fn draft_recovery(
        &self,
        actor: &AuthorizationContext,
        draft_id: eitmad_contracts::quotation_draft::QuotationDraftId,
        deadline: UnixMillis,
    ) -> Result<Vec<eitmad_contracts::sync::ChangeRecord>, eitmad_pricing::QuotationDraftError>
    {
        let mut changes: Vec<eitmad_contracts::sync::ChangeRecord> = self
            .draft_resolution_http()
            .request(
                actor,
                "/v1/quotation-drafts/recovery",
                &eitmad_contracts::quotation_draft::ReadQuotationDraftRecovery {
                    scope: self.remote(actor).map_err(draft_resolution_error)?,
                    draft_id,
                },
                budget(deadline).map_err(draft_resolution_error)?,
            )
            .map_err(|error| draft_resolution_http_error(&error))?;
        let catalog = ScopeRef {
            kind: ScopeKind::parse("organization").expect("scope"),
            id: ScopeId::new(actor.tenant_id.value()),
        };
        for change in &mut changes {
            Self::map_draft_change(change, &actor.scope, &catalog)?;
        }
        Ok(changes)
    }
    fn is_connected_draft_branch(&self, branch: ScopeId) -> bool {
        branch == self.branch.id
    }
    fn draft_review(
        &self,
        actor: &AuthorizationContext,
        query: &eitmad_contracts::quotation_draft::GetQuotationDraftReview,
        deadline: UnixMillis,
    ) -> Result<
        eitmad_contracts::quotation_draft::QuotationDraftReview,
        eitmad_pricing::QuotationDraftError,
    > {
        let mut query = query.clone();
        if actor.scope.kind.as_str() == "branch" {
            query.branch_id = self.branch.id;
        }
        let mut page: eitmad_contracts::quotation_draft::QuotationDraftReview = self
            .draft_resolution_http()
            .request(
                actor,
                "/v1/quotation-drafts/review",
                &eitmad_contracts::quotation_draft::ReadQuotationDraftReview {
                    scope: self.remote(actor).map_err(draft_resolution_error)?,
                    query,
                },
                budget(deadline).map_err(draft_resolution_error)?,
            )
            .map_err(|error| draft_resolution_http_error(&error))?;
        let branch = if actor.scope.kind.as_str() == "branch" {
            actor.scope.clone()
        } else {
            page.current.evaluation.scope.clone()
        };
        let catalog = ScopeRef {
            kind: ScopeKind::parse("organization").expect("scope"),
            id: ScopeId::new(actor.tenant_id.value()),
        };
        Self::map_snapshot(&mut page.current, &branch, &catalog);
        for item in &mut page.items {
            Self::map_snapshot(&mut item.local, &branch, &catalog);
            Self::map_snapshot(&mut item.remote, &branch, &catalog);
        }
        Ok(page)
    }
    fn resolve_draft(
        &self,
        actor: &AuthorizationContext,
        input: &eitmad_contracts::quotation_draft::ConfirmQuotationDraftResolution,
        deadline: UnixMillis,
    ) -> Result<eitmad_contracts::sync::ChangeRecord, eitmad_pricing::QuotationDraftError> {
        let mut input = input.clone();
        input.scope = self.remote(actor).map_err(draft_resolution_error)?;
        if actor.scope.kind.as_str() == "branch" {
            input.command.branch_id = self.branch.id;
        }
        if let eitmad_contracts::quotation_draft::QuotationDraftResolutionSource::PendingReview(
            change,
        ) = &mut input.command.source
        {
            let branch = ScopeRef {
                kind: ScopeKind::parse("branch").expect("scope"),
                id: input.command.branch_id,
            };
            Self::map_draft_change(change, &branch, &self.config.scope)?;
        }
        let mut change: eitmad_contracts::sync::ChangeRecord = self
            .draft_resolution_http()
            .request(
                actor,
                "/v1/quotation-drafts/resolve",
                &input,
                budget(deadline).map_err(draft_resolution_error)?,
            )
            .map_err(|error| draft_resolution_http_error(&error))?;
        let branch = if actor.scope.kind.as_str() == "branch" {
            actor.scope.clone()
        } else {
            change.scope.clone()
        };
        let catalog = ScopeRef {
            kind: ScopeKind::parse("organization").expect("scope"),
            id: ScopeId::new(actor.tenant_id.value()),
        };
        Self::map_draft_change(&mut change, &branch, &catalog)?;
        Ok(change)
    }

    fn transition(
        &self,
        actor: &AuthorizationContext,
        input: &ConfirmDiscountApproval,
        deadline: UnixMillis,
    ) -> Result<Option<DiscountApproval>, E> {
        let mut input = input.clone();
        input.scope = self.remote(actor)?;
        match &mut input.action {
            DiscountApprovalAction::Request(s) | DiscountApprovalAction::Refresh(s) => {
                Self::map_snapshot(s, &input.scope, &self.config.scope);
            }
            DiscountApprovalAction::Decide(_) => {}
        }
        let mut result: Option<DiscountApproval> = self
            .http
            .request(
                actor,
                "/v1/quotation-approvals/transition",
                &input,
                budget(deadline)?,
            )
            .map_err(|e| error(&e))?;
        if let Some(a) = &mut result {
            Self::localize(actor, a);
        }
        Ok(result)
    }
    fn list(
        &self,
        actor: &AuthorizationContext,
        query: &ListDiscountApprovals,
        deadline: UnixMillis,
    ) -> Result<DiscountApprovalPage, E> {
        let mut page: DiscountApprovalPage = self
            .http
            .request(
                actor,
                "/v1/quotation-approvals/read",
                &ReadDiscountApprovals {
                    scope: self.remote(actor)?,
                    query: query.clone(),
                },
                budget(deadline)?,
            )
            .map_err(|e| error(&e))?;
        for a in &mut page.items {
            Self::localize(actor, a);
        }
        Ok(page)
    }
    fn watch(
        &self,
        actor: &AuthorizationContext,
        cancel: &AtomicBool,
        notify: &mut dyn FnMut(DiscountApprovalNotice),
    ) -> Result<(), E> {
        self.watch_domain(
            actor,
            cancel,
            notify,
            DISCOUNT_APPROVAL_SCHEMA,
            "eitmad.capability.quotation-approval.v1",
            20,
        )
    }
}
impl DirectDiscountApprovalClient {
    fn watch_domain(
        &self,
        actor: &AuthorizationContext,
        cancel: &AtomicBool,
        notify: &mut dyn FnMut(DiscountApprovalNotice),
        schema: &str,
        capability: &str,
        minor: u16,
    ) -> Result<(), E> {
        let mut config = self.config.clone();
        config.scope = self.remote(actor)?;
        config.schema_id = SchemaId::parse(schema).expect("schema");
        let http = AuthenticatedHttpClient::from_config(
            config,
            self.secrets.clone(),
            self.credential.clone(),
            capability,
            minor,
        );
        let mut driver = http.driver;
        let deadline = Instant::now() + Duration::from_secs(10);
        let (_, credential) = driver
            .account_credential(&self.credential, actor, deadline)
            .map_err(|failure| match failure.kind {
                eitmad_sync::TransportFailureKind::AuthenticationFailed => E::Denied,
                _ => E::Unavailable,
            })?;
        driver
            .open_socket(&credential, deadline)
            .map_err(|_| E::Unavailable)?;
        let socket = driver.socket.as_mut().ok_or(E::Unavailable)?;
        crate::write_message(
            socket,
            &ServerClientMessage::Subscribe(ServerSubscriptionRequest {
                schema_id: SchemaId::parse(schema).expect("schema"),
                resume_after: None,
            }),
        )
        .map_err(|_| E::Unavailable)?;
        while !cancel.load(Ordering::Acquire) {
            if credential.access_expires_at.0 <= eitmad_authorization::now().0 + 60_000 {
                return Err(E::Unavailable);
            }
            match crate::read_message(socket, eitmad_sync::FailurePhase::Receive) {
                Ok(ServerMessage::Event(event)) => {
                    if event.change.schema_id.as_str() != schema
                        || (event.change.scope != self.remote(actor)?
                            && !(schema == "eitmad.schema.customer.v1"
                                && actor.scope.kind.as_str() == "organization"
                                && event.change.scope.kind.as_str() == "branch"))
                    {
                        return Err(E::Unavailable);
                    }
                    notify(DiscountApprovalNotice {
                        scope: actor.scope.clone(),
                        draft_id: eitmad_contracts::quotation_draft::QuotationDraftId::new(
                            event.change.record_id.value(),
                        ),
                        revision: event.change.revision,
                    });
                }
                Err(e) if e.kind == eitmad_sync::TransportFailureKind::RetryNotReady => {}
                Ok(ServerMessage::Failure(f))
                    if matches!(
                        f.code.as_str(),
                        "eitmad.error.authorization-denied.v1"
                            | "eitmad.error.server-authentication-failed.v1"
                    ) =>
                {
                    return Err(E::Denied);
                }
                _ => return Err(E::Unavailable),
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for DirectDiscountApprovalClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectDiscountApprovalClient")
            .finish_non_exhaustive()
    }
}

impl eitmad_customer::CustomerDirectory for DirectDiscountApprovalClient {
    fn is_connected_branch(&self, branch: ScopeId) -> bool {
        branch == self.branch.id
    }
    fn conflicts(
        &self,
        actor: &AuthorizationContext,
        query: &eitmad_contracts::customer::GetCustomerConflicts,
    ) -> Result<eitmad_contracts::customer::CustomerConflictPage, eitmad_customer::CustomerError>
    {
        let mut query = query.clone();
        if actor.scope.kind.as_str() == "branch" {
            query.customer.branch_id = Some(self.branch.id);
        }
        let mut page: eitmad_contracts::customer::CustomerConflictPage = self
            .customer_http()
            .request(
                actor,
                "/v1/customers/conflicts",
                &eitmad_contracts::customer::ReadCustomerConflicts {
                    scope: self
                        .remote(actor)
                        .map_err(|_| eitmad_customer::CustomerError::Denied)?,
                    query,
                },
                Instant::now() + Duration::from_secs(10),
            )
            .map_err(|e| customer_resolution_error(&e))?;
        if actor.scope.kind.as_str() == "branch" {
            page.current.scope = actor.scope.clone();
            for v in &mut page.items {
                v.conflict.scope = actor.scope.clone();
                v.conflict.local.scope = actor.scope.clone();
                v.conflict.remote.scope = actor.scope.clone();
            }
        }
        Ok(page)
    }
    fn resolve(
        &self,
        actor: &AuthorizationContext,
        input: &eitmad_contracts::customer::ConfirmCustomerResolution,
    ) -> Result<eitmad_contracts::sync::ChangeRecord, eitmad_customer::CustomerError> {
        let mut input = input.clone();
        input.scope = self
            .remote(actor)
            .map_err(|_| eitmad_customer::CustomerError::Denied)?;
        if actor.scope.kind.as_str() == "branch" {
            input.command.customer.branch_id = Some(self.branch.id);
        }
        if let eitmad_contracts::customer::CustomerResolutionSource::PendingReview(v) =
            &mut input.command.source
        {
            v.scope = ScopeRef {
                kind: ScopeKind::parse("branch").expect("scope"),
                id: input
                    .command
                    .customer
                    .branch_id
                    .ok_or(eitmad_customer::CustomerError::Denied)?,
            };
        }
        let mut change: eitmad_contracts::sync::ChangeRecord = self
            .customer_http()
            .request(
                actor,
                "/v1/customers/resolve",
                &input,
                Instant::now() + Duration::from_secs(10),
            )
            .map_err(|e| customer_resolution_error(&e))?;
        if actor.scope.kind.as_str() == "branch" {
            change.scope = actor.scope.clone();
        }
        Ok(change)
    }
    fn read(
        &self,
        actor: &AuthorizationContext,
        query: &eitmad_contracts::customer::CustomerRead,
    ) -> Result<eitmad_contracts::customer::CustomerPage, eitmad_customer::CustomerError> {
        self.customer_http()
            .request(
                actor,
                "/v1/customers/read",
                &eitmad_contracts::customer::ReadCustomers {
                    scope: self
                        .remote(actor)
                        .map_err(|_| eitmad_customer::CustomerError::Denied)?,
                    query: query.clone(),
                },
                Instant::now() + Duration::from_secs(10),
            )
            .map_err(|e| match e {
                HttpError::Denied => eitmad_customer::CustomerError::Denied,
                _ => eitmad_customer::CustomerError::Unavailable,
            })
    }
    fn watch(
        &self,
        actor: &AuthorizationContext,
        cancel: &AtomicBool,
        notify: &mut dyn FnMut(eitmad_contracts::customer::CustomerChangeNotice),
    ) -> Result<(), eitmad_customer::CustomerError> {
        self.watch_domain(
            actor,
            cancel,
            &mut |notice| {
                notify(eitmad_contracts::customer::CustomerChangeNotice {
                    customer_id: eitmad_contracts::customer::CustomerId::new(
                        notice.draft_id.value(),
                    ),
                    scope: actor.scope.clone(),
                    revision: notice.revision,
                    changed_at: eitmad_authorization::now(),
                    change_id: eitmad_contracts::sync::ChangeId::new(uuid::Uuid::new_v4()),
                });
            },
            "eitmad.schema.customer.v1",
            "eitmad.capability.customer.v1",
            27,
        )
        .map_err(|e| {
            if e == E::Denied {
                eitmad_customer::CustomerError::Denied
            } else {
                eitmad_customer::CustomerError::Unavailable
            }
        })
    }
}

fn customer_resolution_error(error: &HttpError) -> eitmad_customer::CustomerError {
    use eitmad_customer::CustomerError as E;
    match error {
        HttpError::Denied => E::Denied,
        HttpError::Conflict => E::RevisionConflict {
            expected_revision: None,
            actual_revision: None,
        },
        HttpError::NotFound => E::NotFound,
        HttpError::Invalid => E::IdempotencyMismatch,
        _ => E::Unavailable,
    }
}

impl eitmad_pricing::QuotationServer for DirectDiscountApprovalClient {
    fn quotation_transition(
        &self,
        actor: &AuthorizationContext,
        request: &eitmad_contracts::quotation_lifecycle::ConfirmQuotation,
        deadline: UnixMillis,
    ) -> Result<
        eitmad_contracts::quotation_lifecycle::QuotationRecord,
        eitmad_pricing::QuotationError,
    > {
        let mut request = request.clone();
        request.scope = self.remote(actor).map_err(quotation_error)?;
        let http = AuthenticatedHttpClient::from_config(
            self.config.clone(),
            self.secrets.clone(),
            self.credential.clone(),
            "eitmad.capability.quotation-lifecycle.v1",
            22,
        );
        let mut value: eitmad_contracts::quotation_lifecycle::QuotationRecord = http
            .request(
                actor,
                "/v1/quotations/transition",
                &request,
                budget(deadline).map_err(quotation_error)?,
            )
            .map_err(quotation_http_error)?;
        Self::localize_quotation(actor, &mut value);
        Ok(value)
    }
    fn quotations(
        &self,
        actor: &AuthorizationContext,
        query: &eitmad_contracts::quotation_lifecycle::ListQuotations,
        deadline: UnixMillis,
    ) -> Result<eitmad_contracts::quotation_lifecycle::QuotationPage, eitmad_pricing::QuotationError>
    {
        let http = AuthenticatedHttpClient::from_config(
            self.config.clone(),
            self.secrets.clone(),
            self.credential.clone(),
            "eitmad.capability.quotation-lifecycle.v1",
            22,
        );
        let mut page: eitmad_contracts::quotation_lifecycle::QuotationPage = http
            .request(
                actor,
                "/v1/quotations/read",
                &eitmad_contracts::quotation_lifecycle::ReadQuotations {
                    scope: self.remote(actor).map_err(quotation_error)?,
                    query: query.clone(),
                },
                budget(deadline).map_err(quotation_error)?,
            )
            .map_err(quotation_http_error)?;
        for value in &mut page.items {
            Self::localize_quotation(actor, value);
        }
        Ok(page)
    }
    fn watch_quotations(
        &self,
        actor: &AuthorizationContext,
        cancel: &AtomicBool,
        notify: &mut dyn FnMut(eitmad_contracts::quotation_lifecycle::QuotationNotice),
    ) -> Result<(), eitmad_pricing::QuotationError> {
        self.watch_domain(
            actor,
            cancel,
            &mut |n| {
                notify(eitmad_contracts::quotation_lifecycle::QuotationNotice {
                    scope: n.scope,
                    draft_id: n.draft_id,
                    revision: n.revision,
                });
            },
            eitmad_pricing::QUOTATION_LIFECYCLE_SCHEMA,
            "eitmad.capability.quotation-lifecycle.v1",
            22,
        )
        .map_err(quotation_error)
    }
}
impl DirectDiscountApprovalClient {
    fn localize_quotation(
        actor: &AuthorizationContext,
        value: &mut eitmad_contracts::quotation_lifecycle::QuotationRecord,
    ) {
        let branch = if actor.scope.kind.as_str() == "branch" {
            actor.scope.clone()
        } else {
            value.scope.clone()
        };
        let organization = ScopeRef {
            kind: ScopeKind::parse("organization").expect("scope"),
            id: ScopeId::new(actor.tenant_id.value()),
        };
        value.scope = branch.clone();
        Self::map_snapshot(&mut value.quotation, &branch, &organization);
    }
}
fn quotation_error(e: E) -> eitmad_pricing::QuotationError {
    use eitmad_pricing::QuotationError as Q;
    match e {
        E::Denied => Q::Denied,
        E::Invalid => Q::Invalid,
        E::Conflict => Q::Conflict,
        E::Unavailable => Q::Unavailable,
    }
}
fn quotation_http_error(e: HttpError) -> eitmad_pricing::QuotationError {
    match e {
        HttpError::StalePrice => eitmad_pricing::QuotationError::StalePrice,
        HttpError::ApprovalRequired => eitmad_pricing::QuotationError::ApprovalRequired,
        e => quotation_error(error(&e)),
    }
}

impl eitmad_orders::OrderServer for DirectDiscountApprovalClient {
    fn work_orders(
        &self,
        actor: &AuthorizationContext,
        query: &eitmad_contracts::work_order::ListWorkOrders,
        deadline: UnixMillis,
    ) -> Result<eitmad_contracts::work_order::WorkOrderPage, eitmad_orders::OrderError> {
        let http = AuthenticatedHttpClient::from_config(
            self.config.clone(),
            self.secrets.clone(),
            self.credential.clone(),
            "eitmad.capability.work-orders.v1",
            23,
        );
        let mut page: eitmad_contracts::work_order::WorkOrderPage = http
            .request(
                actor,
                "/v1/work-orders/read",
                &eitmad_contracts::work_order::ReadWorkOrders {
                    scope: self.remote(actor).map_err(order_error)?,
                    query: query.clone(),
                },
                budget(deadline).map_err(order_error)?,
            )
            .map_err(|e| order_error(error(&e)))?;
        for value in &mut page.items {
            if actor.scope.kind.as_str() == "branch" {
                value.scope = actor.scope.clone();
            }
        }
        Ok(page)
    }

    fn transition(
        &self,
        actor: &AuthorizationContext,
        input: &eitmad_contracts::order::ConfirmOrder,
        deadline: UnixMillis,
    ) -> Result<eitmad_contracts::order::OrderRecord, eitmad_orders::OrderError> {
        let mut input = input.clone();
        input.scope = self.remote(actor).map_err(order_error)?;
        let http = AuthenticatedHttpClient::from_config(
            self.config.clone(),
            self.secrets.clone(),
            self.credential.clone(),
            "eitmad.capability.orders.v1",
            22,
        );
        let mut value: eitmad_contracts::order::OrderRecord = http
            .request(
                actor,
                "/v1/orders/transition",
                &input,
                budget(deadline).map_err(order_error)?,
            )
            .map_err(|e| order_error(error(&e)))?;
        Self::localize_order(actor, &mut value);
        Ok(value)
    }
    fn orders(
        &self,
        actor: &AuthorizationContext,
        query: &eitmad_contracts::order::ListOrders,
        order_id: Option<uuid::Uuid>,
        deadline: UnixMillis,
    ) -> Result<eitmad_contracts::order::OrderPage, eitmad_orders::OrderError> {
        let http = AuthenticatedHttpClient::from_config(
            self.config.clone(),
            self.secrets.clone(),
            self.credential.clone(),
            "eitmad.capability.orders.v1",
            22,
        );
        let mut page: eitmad_contracts::order::OrderPage = http
            .request(
                actor,
                "/v1/orders/read",
                &eitmad_contracts::order::ReadOrders {
                    scope: self.remote(actor).map_err(order_error)?,
                    query: query.clone(),
                    order_id,
                },
                budget(deadline).map_err(order_error)?,
            )
            .map_err(|e| order_error(error(&e)))?;
        for value in &mut page.items {
            Self::localize_order(actor, value);
        }
        Ok(page)
    }
    fn watch(
        &self,
        actor: &AuthorizationContext,
        cancel: &AtomicBool,
        notify: &mut dyn FnMut(eitmad_contracts::order::OrderNotice),
    ) -> Result<(), eitmad_orders::OrderError> {
        self.watch_domain(
            actor,
            cancel,
            &mut |n| {
                notify(eitmad_contracts::order::OrderNotice {
                    scope: n.scope,
                    order_id: n.draft_id.value(),
                    revision: n.revision,
                });
            },
            eitmad_orders::ORDER_SCHEMA,
            "eitmad.capability.orders.v1",
            22,
        )
        .map_err(order_error)
    }
}
impl DirectDiscountApprovalClient {
    fn localize_order(
        actor: &AuthorizationContext,
        value: &mut eitmad_contracts::order::OrderRecord,
    ) {
        if actor.scope.kind.as_str() == "branch" {
            value.scope = actor.scope.clone();
        }
        Self::localize_quotation(actor, &mut value.source);
    }
}
fn order_error(e: E) -> eitmad_orders::OrderError {
    use eitmad_orders::OrderError as O;
    match e {
        E::Denied => O::Denied,
        E::Invalid => O::Invalid,
        E::Conflict => O::Conflict,
        E::Unavailable => O::Unavailable,
    }
}

impl DirectDiscountApprovalClient {
    fn customer_http(&self) -> AuthenticatedHttpClient {
        AuthenticatedHttpClient::from_config(
            self.config.clone(),
            self.secrets.clone(),
            self.credential.clone(),
            "eitmad.capability.customer.v1",
            27,
        )
    }
    fn draft_resolution_http(&self) -> AuthenticatedHttpClient {
        AuthenticatedHttpClient::from_config(
            self.config.clone(),
            self.secrets.clone(),
            self.credential.clone(),
            "eitmad.capability.quotation-draft-resolution.v1",
            28,
        )
    }
    fn map_draft_change(
        change: &mut eitmad_contracts::sync::ChangeRecord,
        branch: &ScopeRef,
        catalog: &ScopeRef,
    ) -> Result<(), eitmad_pricing::QuotationDraftError> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let p = change
            .payload
            .as_mut()
            .ok_or(eitmad_pricing::QuotationDraftError::Unavailable)?;
        let mut snapshot: QuotationDraftSnapshot = serde_json::from_slice(
            &STANDARD
                .decode(&p.base64)
                .map_err(|_| eitmad_pricing::QuotationDraftError::Unavailable)?,
        )
        .map_err(|_| eitmad_pricing::QuotationDraftError::Unavailable)?;
        Self::map_snapshot(&mut snapshot, branch, catalog);
        change.scope = branch.clone();
        p.base64 = STANDARD.encode(
            serde_json::to_vec(&snapshot)
                .map_err(|_| eitmad_pricing::QuotationDraftError::Unavailable)?,
        );
        Ok(())
    }
}
fn draft_resolution_error(error: E) -> eitmad_pricing::QuotationDraftError {
    use eitmad_pricing::QuotationDraftError as D;
    match error {
        E::Denied => D::Denied,
        E::Invalid => D::Invalid,
        E::Conflict => D::UnresolvedConflict,
        E::Unavailable => D::Unavailable,
    }
}
fn draft_resolution_http_error(error: &HttpError) -> eitmad_pricing::QuotationDraftError {
    draft_resolution_error(crate::quotation_approval::error(error))
}
