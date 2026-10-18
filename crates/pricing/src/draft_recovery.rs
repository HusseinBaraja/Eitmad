//! Bounded recovery of saved intent; never evaluates prices or advances commercial state.
use crate::{ApprovalError, DiscountApprovalServer, QuotationDraftService};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_authorization::{QUOTATION_DRAFT_WRITE_PERMISSION, now};
use eitmad_contracts::{
    quotation_approval::{ConfirmDiscountApproval, DiscountApprovalAction},
    quotation_draft::{QuotationDraftSnapshot, QuotationDraftSyncState},
    transport::{CorrelationId, UnixMillis},
};

impl QuotationDraftService {
    /// Attempts at most 50 earliest revisions, for at most ten seconds per pass.
    /// Uses committed authors, exact payloads and keys, with current authority. The cursor
    /// rotates past blocked dependencies and denied authors. Clones serialize recovery
    /// with action-triggered transfer; server receipts cover interrupted acknowledgements.
    /// # Errors
    /// Returns unavailable storage, projection or temporary transport failures. Terminal
    /// server responses retain the outbox and mark the draft rejected or conflicted.
    pub fn recover_pending(
        &self,
        server: &dyn DiscountApprovalServer,
        deadline: UnixMillis,
    ) -> Result<usize, ApprovalError> {
        let mut cursor = match self.recovery_cursor.try_lock() {
            Ok(cursor) => cursor,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(0),
            Err(std::sync::TryLockError::Poisoned(_)) => return Err(ApprovalError::Unavailable),
        };
        let deadline = UnixMillis(deadline.0.min(now().0.saturating_add(10_000)));
        let pending = self
            .store
            .pending_quotation_draft_submissions(*cursor)
            .map_err(|_| ApprovalError::Unavailable)?;
        if pending.is_empty() {
            *cursor = 0;
        }
        let mut count = 0;
        let mut failure = None;
        for eitmad_storage::PendingQuotationDraft {
            position,
            author: actor,
            change,
        } in pending
        {
            if now().0 >= deadline.0 {
                break;
            }
            *cursor = position;
            if self
                .authorization
                .authorize(&actor, QUOTATION_DRAFT_WRITE_PERMISSION)
                .is_err()
            {
                continue;
            }
            let draft_id =
                eitmad_contracts::quotation_draft::QuotationDraftId::new(change.record_id.value());
            if self
                .store
                .get_quotation_draft(&actor.scope, draft_id)
                .map_err(|_| ApprovalError::Unavailable)?
                .is_some_and(|d| d.sync_state != QuotationDraftSyncState::Pending)
            {
                self.recover_blocked(&actor, draft_id, server, deadline)?;
                continue;
            }
            let payload = change.payload.as_ref().ok_or(ApprovalError::Invalid)?;
            let snapshot: QuotationDraftSnapshot = serde_json::from_slice(
                &STANDARD
                    .decode(&payload.base64)
                    .map_err(|_| ApprovalError::Invalid)?,
            )
            .map_err(|_| ApprovalError::Invalid)?;
            let customer = snapshot
                .intent
                .customer
                .as_ref()
                .ok_or(ApprovalError::Invalid)?;
            if !self
                .store
                .quotation_draft_customer_delivered(&actor.scope, customer)
                .map_err(|_| ApprovalError::Unavailable)?
            {
                continue;
            }
            let correlation = CorrelationId::new(uuid::Uuid::new_v4());
            let request = ConfirmDiscountApproval {
                scope: actor.scope.clone(),
                idempotency_key: change.idempotency_key,
                action: DiscountApprovalAction::Refresh(snapshot.clone()),
            };
            match server.transition(&actor, &request, deadline) {
                Ok(_) => {
                    self.project_confirmed(&actor, &change, correlation)
                        .map_err(|_| ApprovalError::Unavailable)?;
                    count += 1;
                }
                Err(ApprovalError::Unavailable) => {
                    failure.get_or_insert(ApprovalError::Unavailable);
                }
                Err(error) => {
                    self.mark_sync_exception(
                        &actor,
                        snapshot.id,
                        if error == ApprovalError::Conflict {
                            QuotationDraftSyncState::Conflicted
                        } else {
                            QuotationDraftSyncState::Rejected
                        },
                        None,
                        correlation,
                        now(),
                    )
                    .map_err(|_| ApprovalError::Unavailable)?;
                }
            }
        }
        failure.map_or(Ok(count), Err)
    }

    fn recover_blocked(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
        draft_id: eitmad_contracts::quotation_draft::QuotationDraftId,
        server: &dyn DiscountApprovalServer,
        deadline: UnixMillis,
    ) -> Result<(), ApprovalError> {
        match server.draft_recovery(actor, draft_id, deadline) {
            Ok(changes) => {
                for confirmed in changes {
                    self.project_confirmed(
                        actor,
                        &confirmed,
                        CorrelationId::new(uuid::Uuid::new_v4()),
                    )
                    .map_err(|_| ApprovalError::Unavailable)?;
                }
            }
            Err(crate::QuotationDraftError::Unavailable | crate::QuotationDraftError::Denied) => {}
            Err(_) => return Err(ApprovalError::Invalid),
        }
        Ok(())
    }
}
