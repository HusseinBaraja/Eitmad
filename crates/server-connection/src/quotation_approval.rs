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
fn error(e: HttpError) -> E {
    match e {
        HttpError::Denied => E::Denied,
        HttpError::Invalid => E::Invalid,
        HttpError::Conflict => E::Conflict,
        _ => E::Unavailable,
    }
}
impl DiscountApprovalServer for DirectDiscountApprovalClient {
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
            .map_err(error)?;
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
            .map_err(error)?;
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
        let mut driver = http.driver.lock().map_err(|_| E::Unavailable)?;
        let mut credential = driver
            .load_credential(&self.credential)
            .map_err(|_| E::Denied)?;
        if credential.user_id.value() != actor.identity.principal_id.value()
            || credential.tenant_id != actor.tenant_id
        {
            return Err(E::Denied);
        }
        driver
            .refresh_if_due(
                &self.credential,
                &mut credential,
                Instant::now() + Duration::from_secs(10),
            )
            .map_err(|_| E::Unavailable)?;
        driver
            .open_socket(&credential)
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
                        || event.change.scope != self.remote(actor)?
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
        e => quotation_error(error(e)),
    }
}

impl eitmad_orders::OrderServer for DirectDiscountApprovalClient {
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
            .map_err(|e| order_error(error(e)))?;
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
            .map_err(|e| order_error(error(e)))?;
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
