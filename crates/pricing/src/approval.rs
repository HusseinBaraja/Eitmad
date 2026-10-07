//! Discount authority and immutable commercial binding shared by desktop and server.
use eitmad_contracts::{
    identity::{AuthorizationContext, PrincipalId, TenantId},
    quotation_approval::{
        ConfirmDiscountApproval, DecideDiscountApproval, DiscountApproval, DiscountApprovalNotice,
        DiscountApprovalPage, DiscountApprovalState, DiscountDecision, ListDiscountApprovals,
    },
    quotation_draft::QuotationDraftSnapshot,
    transport::UnixMillis,
};
use sha2::{Digest as _, Sha256};

pub const DISCOUNT_APPROVAL_SCHEMA: &str = "eitmad.schema.quotation-approval.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalError {
    Denied,
    Invalid,
    Conflict,
    Unavailable,
}

#[must_use]
pub const fn approval_error_code(e: ApprovalError) -> &'static str {
    match e {
        ApprovalError::Denied => "eitmad.error.authorization-denied.v1",
        ApprovalError::Invalid => "eitmad.error.quotation-approval-invalid.v1",
        ApprovalError::Conflict => "eitmad.error.quotation-approval-conflict.v1",
        ApprovalError::Unavailable => "eitmad.error.quotation-approval-unavailable.v1",
    }
}

fn terms(snapshot: &QuotationDraftSnapshot) -> Result<Vec<u8>, ApprovalError> {
    let customer = snapshot
        .intent
        .customer
        .as_ref()
        .ok_or(ApprovalError::Invalid)?;
    let lines: Vec<_> = snapshot
        .intent
        .lines
        .iter()
        .zip(&snapshot.evaluation.lines)
        .map(|(intent, evaluated)| (intent, &evaluated.price))
        .collect();
    serde_json::to_vec(&(
        snapshot.id,
        &snapshot.evaluation.scope,
        customer.id,
        lines,
        &snapshot.evaluation.currency,
        snapshot.intent.discount_basis_points,
        &snapshot.evaluation.totals,
    ))
    .map_err(|_| ApprovalError::Invalid)
}

/// Display contact revisions do not replace a commercial revision or its grant.
#[must_use]
pub fn same_commercial_terms(a: &QuotationDraftSnapshot, b: &QuotationDraftSnapshot) -> bool {
    matches!((terms(a), terms(b)), (Ok(a), Ok(b)) if a == b)
}

/// Includes the frozen quotation revision and all scope and validity identities.
/// # Errors
/// Rejects incomplete snapshots.
pub fn approval_fingerprint(
    tenant: TenantId,
    organization: uuid::Uuid,
    snapshot: &QuotationDraftSnapshot,
    validity_days: u32,
    valid_until: UnixMillis,
) -> Result<String, ApprovalError> {
    let bytes = serde_json::to_vec(&(
        1,
        tenant,
        organization,
        snapshot.revision,
        terms(snapshot)?,
        validity_days,
        valid_until,
    ))
    .map_err(|_| ApprovalError::Invalid)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// Applies a CAS decision only to the original request and commercial snapshot.
/// # Errors
/// Denies self-decision and rejects stale, competing, and malformed decisions.
pub fn decide_approval(
    current: &DiscountApproval,
    draft: &QuotationDraftSnapshot,
    actor: PrincipalId,
    command: &DecideDiscountApproval,
    now: UnixMillis,
) -> Result<DiscountApproval, ApprovalError> {
    if actor == current.requester {
        return Err(ApprovalError::Denied);
    }
    if current.state != DiscountApprovalState::Pending
        || command.draft_id != current.quotation.id
        || command.request_id != current.request_id
        || command.expected_revision != current.revision
        || command.quotation_revision != current.quotation.revision
        || command.fingerprint != current.fingerprint
        || !same_commercial_terms(&current.quotation, draft)
    {
        return Err(ApprovalError::Conflict);
    }
    let reason = command.reason.as_ref().map(|r| r.trim());
    if reason
        .is_some_and(|r| r.is_empty() || r.chars().count() > 240 || r.chars().any(char::is_control))
        || command.decision == DiscountDecision::Reject && reason.is_none()
    {
        return Err(ApprovalError::Invalid);
    }
    let mut result = current.clone();
    result.revision = result
        .revision
        .checked_add(1)
        .ok_or(ApprovalError::Unavailable)?;
    result.state = match command.decision {
        DiscountDecision::Approve => DiscountApprovalState::Approved,
        DiscountDecision::Reject => DiscountApprovalState::Rejected,
    };
    result.reason = reason.map(str::to_owned);
    result.decider = Some(actor);
    result.decided_at = Some(now);
    Ok(result)
}

/// Rust transport boundary: authenticated server results and a cancellable live subscription.
pub trait DiscountApprovalServer: Send + Sync {
    /// # Errors
    /// Requires server confirmation; exact keys retain exact results.
    fn transition(
        &self,
        actor: &AuthorizationContext,
        request: &ConfirmDiscountApproval,
        deadline: UnixMillis,
    ) -> Result<Option<DiscountApproval>, ApprovalError>;
    /// # Errors
    /// Requires an authorized bounded server read.
    fn list(
        &self,
        actor: &AuthorizationContext,
        query: &ListDiscountApprovals,
        deadline: UnixMillis,
    ) -> Result<DiscountApprovalPage, ApprovalError>;
    /// # Errors
    /// Ends on revoked authority, cancellation, or unavailable transport.
    fn watch(
        &self,
        actor: &AuthorizationContext,
        cancel: &std::sync::atomic::AtomicBool,
        notify: &mut dyn FnMut(DiscountApprovalNotice),
    ) -> Result<(), ApprovalError>;
}
