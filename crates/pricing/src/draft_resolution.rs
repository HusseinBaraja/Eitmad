//! Manager-only recovery of preserved quotation drafts and held later revisions.
use crate::{DiscountApprovalServer, QuotationDraftError as E, QuotationDraftService};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_authorization::{MutationContext, QUOTATION_DRAFT_RESOLVE_PERMISSION};
use eitmad_contracts::{
    identity::{AuthorizationContext, ScopeId, UserId},
    quotation_draft::{
        ConfirmQuotationDraftResolution, GetQuotationDraftReview, QuotationDraftResolutionSource,
        QuotationDraftReview, QuotationDraftReviewInput, QuotationDraftSnapshot,
        ResolveQuotationDraft,
    },
    sync::ChangeRecord,
    transport::UnixMillis,
};
use eitmad_observability_audit::AuditOutcome;

impl QuotationDraftService {
    /// Reads server conflicts and local held inputs without evaluating their prices.
    /// # Errors
    /// Requires current Manager authority, branch containment and an available server.
    pub fn review(
        &self,
        actor: &AuthorizationContext,
        query: &GetQuotationDraftReview,
        server: &dyn DiscountApprovalServer,
        deadline: UnixMillis,
    ) -> Result<QuotationDraftReview, E> {
        self.authorize_resolution(actor)?;
        self.reconcile_prior_resolution(actor, query.draft_id, query.branch_id, server, deadline)?;
        let mut page = server.draft_review(actor, query, deadline)?;
        if let Some(local) = self.resolution_actor(actor, query.branch_id, server)? {
            let held = self
                .store
                .quotation_draft_pending_reviews(&local.scope, query.draft_id)?;
            let mut bytes = 0;
            for change in held {
                if page
                    .items
                    .iter()
                    .any(|item| item.conflict.local.change_id == change.change_id)
                {
                    continue;
                }
                let payload = change.payload.as_ref().ok_or(E::Unavailable)?;
                let snapshot = serde_json::from_slice(
                    &STANDARD
                        .decode(&payload.base64)
                        .map_err(|_| E::Unavailable)?,
                )
                .map_err(|_| E::Unavailable)?;
                let input = QuotationDraftReviewInput { change, snapshot };
                let length = serde_json::to_vec(&input)
                    .map_err(|_| E::Unavailable)?
                    .len();
                if bytes > 0 && bytes + length > 2 * 1024 * 1024 {
                    break;
                }
                bytes += length;
                page.pending_reviews.push(input);
            }
        }
        page.pending_resolution =
            self.store
                .quotation_draft_resolution_intent(actor, query.draft_id, query.branch_id)?;
        self.authorize_resolution(actor)?;
        Ok(page)
    }

    /// Freezes the exact decision before network I/O and projects only its confirmed receipt.
    /// # Errors
    /// Unknown outcomes remain durable for exact retry after shell or engine restart.
    pub fn resolve(
        &self,
        context: &MutationContext,
        command: &ResolveQuotationDraft,
        server: &dyn DiscountApprovalServer,
        deadline: UnixMillis,
    ) -> Result<ChangeRecord, E> {
        let operation = "eitmad.quotation-draft.resolve.v1";
        if let Err(e) = self.authorize_resolution(&context.authorization) {
            self.audit_failure(context, operation, command.draft_id, &Err(e.clone()))?;
            return Err(e);
        }
        let actor = &context.authorization;
        self.reconcile_prior_resolution(
            actor,
            command.draft_id,
            command.branch_id,
            server,
            deadline,
        )?;
        let local = self.resolution_actor(actor, command.branch_id, server)?;
        if let QuotationDraftResolutionSource::PendingReview(change) = &command.source {
            let local = local.as_ref().ok_or(E::Denied)?;
            if change.record_id.value() != command.draft_id.value()
                || !self
                    .store
                    .has_quotation_draft_review_input(&local.scope, change)?
            {
                return Err(E::Denied);
            }
        }
        let input = ConfirmQuotationDraftResolution {
            scope: actor.scope.clone(),
            idempotency_key: context.idempotency_key,
            command: command.clone(),
        };
        self.store.stage_quotation_draft_resolution(
            actor,
            &input,
            &super::drafts::audit(
                context,
                "eitmad.quotation-draft.resolution-request.v1",
                Some(command.draft_id),
            ),
        )?;
        let result = server.resolve_draft(actor, &input, deadline);
        let change = match result {
            Ok(change) => change,
            Err(e) => {
                if e != E::Unavailable {
                    self.store.finish_quotation_draft_resolution(
                        actor,
                        command.draft_id,
                        command.branch_id,
                        &super::drafts::audit(context, operation, Some(command.draft_id))
                            .with_outcome(
                                if e == E::Denied {
                                    AuditOutcome::Denied
                                } else if matches!(e, E::Conflict { .. } | E::UnresolvedConflict) {
                                    AuditOutcome::Conflict
                                } else {
                                    AuditOutcome::Invalid
                                },
                                Some(
                                    if e == E::Denied {
                                        "eitmad.error.authorization-denied.v1"
                                    } else if matches!(
                                        e,
                                        E::Conflict { .. } | E::UnresolvedConflict
                                    ) {
                                        "eitmad.error.quotation-draft-conflict.v1"
                                    } else {
                                        "eitmad.error.quotation-draft-invalid.v1"
                                    }
                                    .into(),
                                ),
                            ),
                    )?;
                }
                return Err(e);
            }
        };
        validate_receipt(&change, &input)?;
        self.authorize_resolution(actor)?;
        if let Some(local) = local {
            self.project_resolution(&local, &change, context.correlation_id)?;
        }
        self.store.finish_quotation_draft_resolution(
            actor,
            command.draft_id,
            command.branch_id,
            &super::drafts::audit(context, operation, Some(command.draft_id))
                .with_outcome(AuditOutcome::Succeeded, None),
        )?;
        Ok(change)
    }
    fn reconcile_prior_resolution(
        &self,
        actor: &AuthorizationContext,
        draft: eitmad_contracts::quotation_draft::QuotationDraftId,
        branch: ScopeId,
        server: &dyn DiscountApprovalServer,
        deadline: UnixMillis,
    ) -> Result<(), E> {
        let Some(recovery) = self
            .store
            .pending_quotation_draft_resolution(actor, draft, branch)?
        else {
            return Ok(());
        };
        if recovery.original_user.value() == actor.identity.principal_id.value() {
            return Ok(());
        }
        self.authorize_resolution(actor)?;
        let receipt = server.reconcile_draft_resolution(actor, &recovery, deadline)?;
        if let Some(change) = &receipt {
            validate_receipt(change, &recovery.request)?;
        }
        self.authorize_resolution(actor)?;
        let context = MutationContext {
            authorization: actor.clone(),
            correlation_id: eitmad_contracts::transport::CorrelationId::new(uuid::Uuid::new_v4()),
            causation_id: None,
            idempotency_key: recovery.request.idempotency_key,
            occurred_at: eitmad_authorization::now(),
        };
        if let (Some(change), Some(local)) =
            (&receipt, self.resolution_actor(actor, branch, server)?)
        {
            self.project_resolution(&local, change, context.correlation_id)?;
        }
        self.store.reconcile_quotation_draft_resolution(
            actor,
            &recovery,
            receipt.as_ref(),
            &super::drafts::audit(
                &context,
                "eitmad.quotation-draft.reconcile-resolution.v1",
                Some(draft),
            ),
        )?;
        Ok(())
    }

    fn project_resolution(
        &self,
        local: &AuthorizationContext,
        change: &ChangeRecord,
        correlation: eitmad_contracts::transport::CorrelationId,
    ) -> Result<(), E> {
        let mut projected = change.clone();
        projected.scope = local.scope.clone();
        // Transport maps catalog identities; only the installed branch differs here.
        let payload = projected.payload.as_mut().ok_or(E::Unavailable)?;
        let mut snapshot: QuotationDraftSnapshot = serde_json::from_slice(
            &STANDARD
                .decode(&payload.base64)
                .map_err(|_| E::Unavailable)?,
        )
        .map_err(|_| E::Unavailable)?;
        snapshot.evaluation.scope = local.scope.clone();
        payload.base64 =
            STANDARD.encode(serde_json::to_vec(&snapshot).map_err(|_| E::Unavailable)?);
        self.project_confirmed(local, &projected, correlation)?;
        Ok(())
    }
    fn authorize_resolution(&self, actor: &AuthorizationContext) -> Result<(), E> {
        self.authorization
            .authorize(actor, QUOTATION_DRAFT_RESOLVE_PERMISSION)
            .map_err(|_| E::Denied)
    }
    fn resolution_actor(
        &self,
        actor: &AuthorizationContext,
        branch: ScopeId,
        server: &dyn DiscountApprovalServer,
    ) -> Result<Option<AuthorizationContext>, E> {
        if actor.scope.kind.as_str() == "branch" {
            return if actor.scope.id == branch {
                Ok(Some(actor.clone()))
            } else {
                Err(E::Denied)
            };
        }
        if !server.is_connected_draft_branch(branch) {
            return Ok(None);
        }
        let scope = self.store.desktop_customer_branch(
            actor.tenant_id,
            UserId::new(actor.identity.principal_id.value()),
        )?;
        Ok(scope.map(|scope| AuthorizationContext {
            scope,
            ..actor.clone()
        }))
    }
}

fn validate_receipt(
    change: &ChangeRecord,
    input: &ConfirmQuotationDraftResolution,
) -> Result<(), E> {
    if change.record_id.value() != input.command.draft_id.value()
        || change.scope.kind.as_str() != "branch"
        || change.scope.id != input.command.branch_id
        || change.base_revision != Some(input.command.expected_server_revision)
        || change.idempotency_key != input.idempotency_key
        || Some(change.revision) != input.command.expected_server_revision.checked_add(1)
        || change.merge.is_none()
    {
        return Err(E::Unavailable);
    }
    Ok(())
}
