//! IPC composition for the server-owned quotation lifecycle and confirmed cache.
use super::{
    Arc, Command, CommandResult, ContractError, DispatchContext, Event, MutationContext,
    ProductDispatcher, QueryResult, RetryDisposition, contract_error,
};
use eitmad_contracts::{
    identity::AuthorizationContext,
    quotation_lifecycle::{ConfirmQuotation, ListQuotations, QuotationAction, QuotationNotice},
};
use eitmad_pricing::{QuotationError as E, QuotationServer};
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct QuotationWatch {
    actor: String,
    cancel: Arc<AtomicBool>,
}
impl Drop for QuotationWatch {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl ProductDispatcher {
    #[must_use]
    pub fn with_quotations(mut self, server: Arc<dyn QuotationServer>) -> Self {
        self.quotation_server = Some(server);
        self
    }

    pub(super) async fn quotation_command(
        &self,
        context: &DispatchContext,
        mutation: &MutationContext,
        command: Command,
    ) -> Result<CommandResult, ContractError> {
        let (permission, action) = match command {
            Command::AcceptQuotation(c) => (
                eitmad_authorization::QUOTATION_ACCEPT_PERMISSION,
                QuotationAction::Accept(c),
            ),
            Command::IssueQuotation(c) => (
                eitmad_authorization::QUOTATION_ISSUE_PERMISSION,
                QuotationAction::Issue(c),
            ),
            Command::SetQuotationValidity(c) => (
                eitmad_authorization::QUOTATION_VALIDITY_PERMISSION,
                QuotationAction::SetValidity(c),
            ),
            Command::ReviseQuotation(c) => (
                eitmad_authorization::QUOTATION_VALIDITY_PERMISSION,
                QuotationAction::Revise(c),
            ),
            Command::CancelQuotation(c) => (
                eitmad_authorization::QUOTATION_CANCEL_PERMISSION,
                QuotationAction::Cancel(c),
            ),
            _ => return Err(super::unsupported(context)),
        };
        self.require_approval(context, permission).map_err(|e| *e)?;
        if matches!(action, QuotationAction::Issue(_)) {
            self.refresh_approval_draft(context).await;
        }
        let server = self
            .quotation_server
            .clone()
            .ok_or_else(|| failure(E::Unavailable, context))?;
        let actor = context.authorization.clone();
        let deadline = context.deadline;
        let request = ConfirmQuotation {
            scope: actor.scope.clone(),
            idempotency_key: mutation.idempotency_key,
            action,
        };
        let value = tokio::task::spawn_blocking(move || {
            server.quotation_transition(&actor, &request, deadline)
        })
        .await
        .map_err(|_| failure(E::Unavailable, context))?
        .map_err(|e| failure(e, context))?;
        self.drafts
            .cache_quotation(&context.authorization, &value, context.correlation_id)
            .map_err(|_| failure(E::Unavailable, context))?;
        self.events
            .publish(
                context.authorization.scope.clone(),
                Event::QuotationChanged(QuotationNotice {
                    scope: context.authorization.scope.clone(),
                    draft_id: value.quotation.id,
                    revision: value.revision,
                }),
            )
            .map_err(|()| failure(E::Unavailable, context))?;
        Ok(CommandResult::Quotation(Box::new(value)))
    }

    pub(super) async fn quotation_list(
        &self,
        context: &DispatchContext,
        query: ListQuotations,
    ) -> Result<QueryResult, ContractError> {
        self.require_approval(context, eitmad_authorization::QUOTATION_READ_PERMISSION)
            .map_err(|e| *e)?;
        let actor = context.authorization.clone();
        let deadline = context.deadline;
        let copy = query.clone();
        let result = if let Some(server) = self.quotation_server.clone() {
            tokio::task::spawn_blocking(move || server.quotations(&actor, &copy, deadline))
                .await
                .map_err(|_| failure(E::Unavailable, context))?
        } else {
            Err(E::Unavailable)
        };
        match result {
            Ok(page) => {
                for value in &page.items {
                    self.drafts
                        .cache_quotation(&context.authorization, value, context.correlation_id)
                        .map_err(|_| failure(E::Unavailable, context))?;
                }
                Ok(QueryResult::Quotations(page))
            }
            Err(E::Unavailable) => {
                let mut page = self
                    .store
                    .cached_quotations(
                        &context.authorization.scope,
                        query.after,
                        query.limit,
                        eitmad_authorization::now(),
                    )
                    .map_err(|_| failure(E::Unavailable, context))?;
                for value in &mut page.items {
                    value.permitted_actions.clear();
                    if context.authorization.scope.kind.as_str() == "branch" && value.scope == context.authorization.scope
                        && matches!(value.state, eitmad_contracts::quotation_lifecycle::QuotationState::Draft | eitmad_contracts::quotation_lifecycle::QuotationState::PendingApproval) {
                        if let Ok(draft) = self.drafts.get(&context.authorization, &eitmad_contracts::quotation_draft::GetQuotationDraft { draft_id: value.quotation.id }) {
                            value.permitted_actions = draft.permitted_actions;
                        }
                    }
                    if value.number.is_some()
                        && value.issued_at.is_some()
                        && matches!(
                            value.state,
                            eitmad_contracts::quotation_lifecycle::QuotationState::Issued
                                | eitmad_contracts::quotation_lifecycle::QuotationState::Expired
                                | eitmad_contracts::quotation_lifecycle::QuotationState::Cancelled
                        )
                    {
                        value.permitted_actions.push(
                            eitmad_contracts::quotation_lifecycle::QuotationPermittedAction::Print,
                        );
                    }
                }
                Ok(QueryResult::Quotations(page))
            }
            Err(e) => Err(failure(e, context)),
        }
    }

    pub(super) fn start_quotation_watch(&self, actor: &AuthorizationContext) -> Result<(), E> {
        let server = self.quotation_server.clone().ok_or(E::Unavailable)?;
        let key = format!(
            "{}:{}:{}",
            actor.session_id.value(),
            actor.scope.kind.as_str(),
            actor.scope.id.value()
        );
        let mut watch = self.quotation_watch.lock().map_err(|_| E::Unavailable)?;
        if watch
            .as_ref()
            .is_some_and(|w| w.actor == key && !w.cancel.load(Ordering::Acquire))
        {
            return Ok(());
        }
        *watch = None;
        let cancel = Arc::new(AtomicBool::new(false));
        *watch = Some(QuotationWatch {
            actor: key,
            cancel: cancel.clone(),
        });
        let actor = actor.clone();
        let authorization = self.authorization.clone();
        let events = self.events.clone();
        std::thread::Builder::new()
            .name("eitmad-quotations".into())
            .spawn(move || {
                let mut delay = 1;
                let mut unavailable = false;
                while !cancel.load(Ordering::Acquire) {
                    let result = server.watch_quotations(&actor, &cancel, &mut |notice| {
                        delay = 1;
                        unavailable = false;
                        if authorization
                            .authorize(&actor, eitmad_authorization::QUOTATION_READ_PERMISSION)
                            .is_ok()
                        {
                            let _ = events
                                .publish(actor.scope.clone(), Event::QuotationChanged(notice));
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
                            .authorize(&actor, eitmad_authorization::QUOTATION_READ_PERMISSION)
                            .is_ok()
                    {
                        let _ = events.publish(
                            actor.scope.clone(),
                            Event::QuotationChanged(QuotationNotice {
                                scope: actor.scope.clone(),
                                draft_id: eitmad_contracts::quotation_draft::QuotationDraftId::new(
                                    uuid::Uuid::nil(),
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
            .map_err(|_| E::Unavailable)?;
        Ok(())
    }
}
fn failure(e: E, context: &DispatchContext) -> ContractError {
    let code = eitmad_pricing::quotation_error_code(e);
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
