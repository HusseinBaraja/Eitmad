//! Dispatch and live notification composition; discount rules remain in Pricing.
use super::{
    AccessAuditContext, Arc, AuditOutcome, AuthorizationError, Command, CommandResult,
    ContractError, DispatchContext, Event, MutationContext, ProductDispatcher, QueryResult,
    RetryDisposition, authorization_error, contract_error, draft_error, now, unsupported,
};
use eitmad_contracts::quotation_approval::{
    ConfirmDiscountApproval, DiscountApprovalAction, DiscountApprovalNotice, ListDiscountApprovals,
};
use eitmad_pricing::{ApprovalError, DiscountApprovalServer};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(super) struct ApprovalWatch {
    cancel: Arc<AtomicBool>,
    actor: String,
}
impl Drop for ApprovalWatch {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl ProductDispatcher {
    /// Attaches the authenticated Rust server boundary.
    #[must_use]
    pub fn with_discount_approvals(mut self, server: Arc<dyn DiscountApprovalServer>) -> Self {
        self.approval_server = Some(server);
        self
    }

    pub(super) async fn approval_command(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, ContractError> {
        let (permission, action) = match command {
            Command::RequestDiscountApproval(c) => {
                self.require_approval(context, eitmad_authorization::DISCOUNT_REQUEST_PERMISSION)
                    .map_err(|e| *e)?;
                self.drafts
                    .get(
                        &context.authorization,
                        &eitmad_contracts::quotation_draft::GetQuotationDraft {
                            draft_id: c.draft_id,
                        },
                    )
                    .map_err(|e| draft_error(e, context))?;
                self.refresh_approval_draft(context).await;
                let draft = self
                    .drafts
                    .get(
                        &context.authorization,
                        &eitmad_contracts::quotation_draft::GetQuotationDraft {
                            draft_id: c.draft_id,
                        },
                    )
                    .map_err(|e| draft_error(e, context))?;
                if draft.snapshot.revision != c.expected_revision
                    || matches!(
                        draft.sync_state,
                        eitmad_contracts::quotation_draft::QuotationDraftSyncState::Conflicted
                            | eitmad_contracts::quotation_draft::QuotationDraftSyncState::Rejected
                    )
                {
                    return Err(approval_error(ApprovalError::Conflict, context));
                }
                (
                    eitmad_authorization::DISCOUNT_REQUEST_PERMISSION,
                    DiscountApprovalAction::Request(draft.snapshot),
                )
            }
            Command::DecideDiscountApproval(c) => (
                eitmad_authorization::DISCOUNT_DECIDE_PERMISSION,
                DiscountApprovalAction::Decide(c),
            ),
            _ => return Err(unsupported(context)),
        };
        self.require_approval(context, permission).map_err(|e| *e)?;
        let server = self
            .approval_server
            .clone()
            .ok_or_else(|| approval_error(ApprovalError::Unavailable, context))?;
        let actor = context.authorization.clone();
        let deadline = context.deadline;
        let request = ConfirmDiscountApproval {
            scope: actor.scope.clone(),
            idempotency_key: mutation.idempotency_key,
            action,
        };
        let value =
            tokio::task::spawn_blocking(move || server.transition(&actor, &request, deadline))
                .await
                .map_err(|_| approval_error(ApprovalError::Unavailable, context))?
                .map_err(|e| approval_error(e, context))?
                .ok_or_else(|| approval_error(ApprovalError::Unavailable, context))?;
        self.events
            .publish(
                context.authorization.scope.clone(),
                Event::DiscountApprovalChanged(DiscountApprovalNotice {
                    scope: context.authorization.scope.clone(),
                    draft_id: value.quotation.id,
                    revision: value.revision,
                }),
            )
            .map_err(|()| approval_error(ApprovalError::Unavailable, context))?;
        Ok(CommandResult::DiscountApproval(Box::new(value)))
    }
    fn require_approval(
        &self,
        context: &DispatchContext,
        permission: &str,
    ) -> Result<(), Box<ContractError>> {
        self.authorization
            .authorize(&context.authorization, permission)
            .map_err(|e| {
                let audit = self.authorization.audit_access_result(
                    &AccessAuditContext {
                        authorization: context.authorization.clone(),
                        correlation_id: context.correlation_id,
                        causation_id: context.causation_id,
                        occurred_at: now(),
                    },
                    permission,
                    "quotation-approval",
                    AuditOutcome::Denied,
                    Some("eitmad.error.authorization-denied.v1"),
                    vec![],
                );
                Box::new(authorization_error(
                    if audit.is_err() {
                        AuthorizationError::Unavailable
                    } else {
                        e
                    },
                    context,
                ))
            })
    }
    pub(super) async fn approval_list(
        &self,
        context: &DispatchContext,
        query: ListDiscountApprovals,
    ) -> Result<QueryResult, ContractError> {
        self.require_approval(context, eitmad_authorization::DISCOUNT_READ_PERMISSION)
            .map_err(|e| *e)?;
        let server = self
            .approval_server
            .clone()
            .ok_or_else(|| approval_error(ApprovalError::Unavailable, context))?;
        let actor = context.authorization.clone();
        let deadline = context.deadline;
        tokio::task::spawn_blocking(move || server.list(&actor, &query, deadline))
            .await
            .map_err(|_| approval_error(ApprovalError::Unavailable, context))?
            .map(QueryResult::DiscountApprovals)
            .map_err(|e| approval_error(e, context))
    }
    pub(super) async fn save_approval_draft(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, ContractError> {
        let result = self
            .dispatch_draft_command(context, mutation, command)
            .map_err(|e| *e)?;
        if let CommandResult::QuotationDraftCreated(d) | CommandResult::QuotationDraftUpdated(d) =
            &result
        {
            self.refresh_approval_draft(context).await;
            if let Ok(fresh) = self.drafts.get(
                &context.authorization,
                &eitmad_contracts::quotation_draft::GetQuotationDraft {
                    draft_id: d.snapshot.id,
                },
            ) {
                return Ok(
                    if matches!(result, CommandResult::QuotationDraftCreated(_)) {
                        CommandResult::QuotationDraftCreated(Box::new(fresh))
                    } else {
                        CommandResult::QuotationDraftUpdated(Box::new(fresh))
                    },
                );
            }
        }
        Ok(result)
    }

    /// Projects online saved revisions so other clients see invalidation immediately.
    pub(super) async fn refresh_approval_draft(&self, context: &DispatchContext) {
        let Some(server) = self.approval_server.clone() else {
            return;
        };
        let actor = context.authorization.clone();
        let deadline = context.deadline;
        let drafts = self.drafts.clone();
        let correlation = context.correlation_id;
        let _ = tokio::task::spawn_blocking(move || -> Result<(), ApprovalError> {
            loop {
                let pending = drafts
                    .sync_batch(&actor, 50)
                    .map_err(|_| ApprovalError::Unavailable)?;
                if pending.is_empty() {
                    return Ok(());
                }
                for change in pending {
                    use base64::{Engine as _, engine::general_purpose::STANDARD};
                    let payload = change.payload.as_ref().ok_or(ApprovalError::Invalid)?;
                    let snapshot = serde_json::from_slice(
                        &STANDARD
                            .decode(&payload.base64)
                            .map_err(|_| ApprovalError::Invalid)?,
                    )
                    .map_err(|_| ApprovalError::Invalid)?;
                    let request = ConfirmDiscountApproval {
                        scope: actor.scope.clone(),
                        idempotency_key: change.idempotency_key,
                        action: DiscountApprovalAction::Refresh(snapshot),
                    };
                    server.transition(&actor, &request, deadline)?;
                    drafts
                        .project_confirmed(&actor, &change, correlation)
                        .map_err(|_| ApprovalError::Unavailable)?;
                }
            }
        })
        .await;
    }
    pub(super) fn start_approval_watch(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
    ) -> Result<(), ApprovalError> {
        let Some(server) = self.approval_server.clone() else {
            return Err(ApprovalError::Unavailable);
        };
        let key = format!(
            "{}:{}:{}",
            actor.session_id.value(),
            actor.scope.kind.as_str(),
            actor.scope.id.value()
        );
        let mut watch = self
            .approval_watch
            .lock()
            .map_err(|_| ApprovalError::Unavailable)?;
        if watch
            .as_ref()
            .is_some_and(|w| w.actor == key && !w.cancel.load(Ordering::Acquire))
        {
            return Ok(());
        }
        *watch = None;
        let cancel = Arc::new(AtomicBool::new(false));
        *watch = Some(ApprovalWatch {
            cancel: cancel.clone(),
            actor: key,
        });
        let actor = actor.clone();
        let authorization = self.authorization.clone();
        let events = self.events.clone();
        std::thread::Builder::new()
            .name("eitmad-discount-approvals".into())
            .spawn(move || {
                let mut unavailable = false;
                let mut delay = 1;
                while !cancel.load(Ordering::Acquire) {
                    let result = server.watch(&actor, &cancel, &mut |notice| {
                        unavailable = false;
                        delay = 1;
                        if authorization
                            .authorize(&actor, eitmad_authorization::DISCOUNT_READ_PERMISSION)
                            .is_ok()
                        {
                            let _ = events.publish(
                                actor.scope.clone(),
                                Event::DiscountApprovalChanged(notice),
                            );
                        } else {
                            cancel.store(true, Ordering::Release);
                        }
                    });
                    if result.is_err()
                        && !unavailable
                        && authorization
                            .authorize(&actor, eitmad_authorization::DISCOUNT_READ_PERMISSION)
                            .is_ok()
                    {
                        let _ = events.publish(
                            actor.scope.clone(),
                            Event::DiscountApprovalChanged(DiscountApprovalNotice {
                                scope: actor.scope.clone(),
                                draft_id: eitmad_contracts::quotation_draft::QuotationDraftId::new(
                                    uuid::Uuid::nil(),
                                ),
                                revision: 0,
                            }),
                        );
                        unavailable = true;
                    }
                    if result == Err(ApprovalError::Denied) {
                        cancel.store(true, Ordering::Release);
                        break;
                    }
                    for _ in 0..delay * 10 {
                        if cancel.load(Ordering::Acquire) {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    delay = (delay * 2).min(30);
                }
            })
            .map_err(|_| ApprovalError::Unavailable)?;
        Ok(())
    }
}
pub(super) fn approval_error(e: ApprovalError, context: &DispatchContext) -> ContractError {
    let code = eitmad_pricing::approval_error_code(e);
    contract_error(
        code,
        &code.replace(".error.", ".message."),
        context.correlation_id,
        if e == ApprovalError::Unavailable {
            RetryDisposition::SafeAfterDelay(1000)
        } else {
            RetryDisposition::Never
        },
        None,
    )
}
pub(super) fn empty_watch() -> Arc<Mutex<Option<ApprovalWatch>>> {
    Arc::new(Mutex::new(None))
}
