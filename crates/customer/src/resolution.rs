use super::*;
use eitmad_authorization::CUSTOMER_RESOLVE_PERMISSION;
use eitmad_contracts::{
    customer::{
        ConfirmCustomerResolution, CustomerConflictPage, CustomerResolutionSource,
        GetCustomerConflicts, ResolveCustomerConflict,
    },
    identity::{ScopeKind, UserId},
};

impl CustomerService {
    /// Reads preserved conflicts and local held edits under current Manager authority.
    /// # Errors
    /// Denies unrelated scopes and requires an available authenticated server.
    pub fn conflicts(
        &self,
        actor: &AuthorizationContext,
        query: &GetCustomerConflicts,
    ) -> Result<CustomerConflictPage, CustomerError> {
        self.authorize_resolution(actor, query.customer.customer_id)?;
        check_branch(actor, query.customer.branch_id)?;
        let directory = self.directory.as_ref().ok_or(CustomerError::Unavailable)?;
        let mut page = directory.conflicts(actor, query)?;
        if let Some(local) = self.local_resolution_actor(actor, page.current.scope.id)? {
            page.pending_reviews = self
                .store
                .customer_pending_reviews(&local.scope, query.customer.customer_id)
                .map_err(|_| CustomerError::Unavailable)?
                .into_iter()
                .map(|change| {
                    let payload = change.payload.as_ref().ok_or(CustomerError::Unavailable)?;
                    let contact = serde_json::from_slice(
                        &STANDARD
                            .decode(&payload.base64)
                            .map_err(|_| CustomerError::Unavailable)?,
                    )
                    .map_err(|_| CustomerError::Unavailable)?;
                    Ok(eitmad_contracts::customer::CustomerPendingReview { change, contact })
                })
                .collect::<Result<_, CustomerError>>()?;
        }
        page.pending_resolution = self
            .store
            .customer_resolution_intent(actor, query.customer.customer_id, page.current.scope.id)
            .map_err(|_| CustomerError::Unavailable)?;
        self.authorize_resolution(actor, query.customer.customer_id)?;
        Ok(page)
    }

    /// Freezes intent before submission and projects only the server-confirmed decision.
    /// # Errors
    /// Keeps uncertain intent for exact retry across application/engine restart.
    pub fn resolve(
        &self,
        context: &MutationContext,
        command: &ResolveCustomerConflict,
    ) -> Result<ChangeRecord, CustomerError> {
        let actor = &context.authorization;
        self.authorize_resolution(actor, command.customer.customer_id)?;
        let directory = self.directory.as_ref().ok_or(CustomerError::Unavailable)?;
        let branch = command
            .customer
            .branch_id
            .ok_or(CustomerError::UnsupportedScope)?;
        check_branch(actor, Some(branch))?;
        let local = self.local_resolution_actor(actor, branch)?;
        if let CustomerResolutionSource::PendingReview(change) = &command.source {
            let local = local.as_ref().ok_or(CustomerError::Denied)?;
            if !self
                .store
                .has_customer_review_input(&local.scope, change)
                .map_err(|_| CustomerError::Unavailable)?
            {
                return Err(CustomerError::Denied);
            }
        }
        let input = ConfirmCustomerResolution {
            scope: actor.scope.clone(),
            idempotency_key: context.idempotency_key,
            command: command.clone(),
        };
        let audit = audit_record(
            context,
            "eitmad.customer.resolution-request.v1",
            command.customer.customer_id,
        );
        self.store
            .stage_customer_resolution(actor, &input, &audit)
            .map_err(|_| CustomerError::Unavailable)?;
        let result = directory.resolve(actor, &input);
        let change = match result {
            Ok(change) => change,
            Err(error) => {
                if error != CustomerError::Unavailable {
                    self.store
                        .finish_customer_resolution(
                            actor,
                            command.customer.customer_id,
                            branch,
                            &failure_audit(context, command.customer.customer_id, error),
                        )
                        .map_err(|_| CustomerError::Unavailable)?;
                }
                return Err(error);
            }
        };
        if change.record_id.value() != command.customer.customer_id.value()
            || change.scope.id != branch
            || change.base_revision != Some(command.expected_server_revision)
            || change.merge.is_none()
        {
            return Err(CustomerError::Unavailable);
        }
        self.authorize_resolution(actor, command.customer.customer_id)?;
        if let Some(local) = local {
            let mut local_change = change.clone();
            local_change.scope = local.scope.clone();
            self.project_confirmed(&local, &local_change, context.correlation_id)?;
        }
        self.store
            .finish_customer_resolution(
                actor,
                command.customer.customer_id,
                branch,
                &audit_record(
                    context,
                    "eitmad.customer.resolve-conflict.v1",
                    command.customer.customer_id,
                )
                .with_outcome(AuditOutcome::Succeeded, None),
            )
            .map_err(|_| CustomerError::Unavailable)?;
        Ok(change)
    }

    fn authorize_resolution(
        &self,
        actor: &AuthorizationContext,
        customer: CustomerId,
    ) -> Result<(), CustomerError> {
        match self
            .authorization
            .authorize(actor, CUSTOMER_RESOLVE_PERMISSION)
        {
            Ok(()) => Ok(()),
            Err(error) => {
                let context = MutationContext {
                    authorization: actor.clone(),
                    correlation_id: CorrelationId::new(Uuid::new_v4()),
                    causation_id: None,
                    idempotency_key: eitmad_contracts::transport::IdempotencyKey::new(
                        Uuid::new_v4(),
                    ),
                    occurred_at: eitmad_authorization::now(),
                };
                self.store
                    .append_audit(
                        &audit_record(&context, "eitmad.customer.resolve-conflict.v1", customer)
                            .with_outcome(
                                AuditOutcome::Denied,
                                Some("eitmad.error.authorization-denied.v1".into()),
                            ),
                    )
                    .map_err(|_| CustomerError::Unavailable)?;
                Err(map_authorization_error(error))
            }
        }
    }

    fn local_resolution_actor(
        &self,
        actor: &AuthorizationContext,
        branch: eitmad_contracts::identity::ScopeId,
    ) -> Result<Option<AuthorizationContext>, CustomerError> {
        if actor.scope.kind.as_str() == "branch" {
            return Ok((actor.scope.id == branch).then(|| actor.clone()));
        }
        if !self
            .directory
            .as_ref()
            .is_some_and(|d| d.is_connected_branch(branch))
        {
            return Ok(None);
        }
        let scope = self
            .store
            .desktop_customer_branch(
                actor.tenant_id,
                UserId::new(actor.identity.principal_id.value()),
            )
            .map_err(|_| CustomerError::Unavailable)?;
        Ok(scope.map(|scope| AuthorizationContext {
            scope: ScopeRef {
                kind: ScopeKind::parse("branch").expect("scope"),
                id: scope.id,
            },
            ..actor.clone()
        }))
    }
}

fn check_branch(
    actor: &AuthorizationContext,
    branch: Option<eitmad_contracts::identity::ScopeId>,
) -> Result<(), CustomerError> {
    if actor.scope.kind.as_str() == "branch" && branch.is_some_and(|id| id != actor.scope.id) {
        return Err(CustomerError::Denied);
    }
    Ok(())
}

fn failure_audit(
    context: &MutationContext,
    customer: CustomerId,
    error: CustomerError,
) -> eitmad_observability_audit::MutationAuditRecord {
    let (outcome, code) = match error {
        CustomerError::Denied | CustomerError::UnsupportedScope => {
            (AuditOutcome::Denied, "eitmad.error.authorization-denied.v1")
        }
        CustomerError::IdempotencyMismatch => {
            (AuditOutcome::Invalid, "eitmad.error.contract-invalid.v1")
        }
        CustomerError::NotFound => (AuditOutcome::Invalid, "eitmad.error.customer-not-found.v1"),
        _ => (
            AuditOutcome::Conflict,
            "eitmad.error.customer-revision-conflict.v1",
        ),
    };
    audit_record(context, "eitmad.customer.resolution-failed.v1", customer)
        .with_outcome(outcome, Some(code.into()))
}
