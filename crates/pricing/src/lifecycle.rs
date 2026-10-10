use crate::{approval_fingerprint, same_commercial_terms};
use eitmad_contracts::{
    identity::TenantId,
    quotation_approval::{DiscountApproval, DiscountApprovalState},
    quotation_draft::QuotationDraftSnapshot,
    quotation_lifecycle::{QuotationPermittedAction as A, QuotationRecord, QuotationState as S},
    transport::UnixMillis,
};

pub const QUOTATION_LIFECYCLE_SCHEMA: &str = "eitmad.schema.quotation-lifecycle.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuotationError {
    Denied,
    Invalid,
    Conflict,
    StalePrice,
    ApprovalRequired,
    Unavailable,
}
#[must_use]
pub const fn quotation_error_code(e: QuotationError) -> &'static str {
    match e {
        QuotationError::Denied => "eitmad.error.authorization-denied.v1",
        QuotationError::Invalid => "eitmad.error.quotation-invalid.v1",
        QuotationError::Conflict => "eitmad.error.quotation-state-conflict.v1",
        QuotationError::StalePrice => "eitmad.error.quotation-stale-price.v1",
        QuotationError::ApprovalRequired => "eitmad.error.quotation-approval-required.v1",
        QuotationError::Unavailable => "eitmad.error.quotation-unavailable.v1",
    }
}

/// End of the resulting Asia/Aden calendar date, with checked integer arithmetic.
/// # Errors
/// Rejects zero validity and times outside the supported integer range.
pub fn quotation_expiry(now: UnixMillis, days: u32) -> Result<UnixMillis, QuotationError> {
    if days == 0 {
        return Err(QuotationError::Invalid);
    }
    now.0
        .checked_add(10_800_000)
        .map(|v| v.div_euclid(86_400_000))
        .and_then(|v| v.checked_add(i64::from(days) + 1))
        .and_then(|v| v.checked_mul(86_400_000))
        .and_then(|v| v.checked_sub(10_800_001))
        .map(UnixMillis)
        .ok_or(QuotationError::Invalid)
}

/// A grant binds its original commercial revision, even after contact-only edits.
/// # Errors
/// Requires an unconsumed exact approval, including issue-day validity.
pub fn issuance_snapshot(
    tenant: TenantId,
    record: &QuotationRecord,
    draft: &QuotationDraftSnapshot,
    approval: Option<&DiscountApproval>,
    now: UnixMillis,
) -> Result<QuotationDraftSnapshot, QuotationError> {
    if !matches!(record.state, S::Draft | S::PendingApproval) || draft.cancelled {
        return Err(QuotationError::Conflict);
    }
    let totals = draft
        .evaluation
        .totals
        .as_ref()
        .filter(|_| draft.evaluation.errors.is_empty())
        .ok_or(QuotationError::Invalid)?;
    if !totals.approval_required {
        return Ok(draft.clone());
    }
    let a = approval.ok_or(QuotationError::ApprovalRequired)?;
    let expiry = quotation_expiry(now, record.validity_days)?;
    if a.state != DiscountApprovalState::Approved
        || a.scope != record.scope
        || a.organization_id != record.organization_id
        || a.quotation.id != draft.id
        || a.validity_days != record.validity_days
        || a.proposed_valid_until != expiry
        || !same_commercial_terms(&a.quotation, draft)
        || approval_fingerprint(
            tenant,
            record.organization_id,
            &a.quotation,
            record.validity_days,
            expiry,
        )
        .map_err(|_| QuotationError::ApprovalRequired)?
            != a.fingerprint
    {
        return Err(QuotationError::ApprovalRequired);
    }
    Ok(a.quotation.clone())
}

#[must_use]
pub fn quotation_actions(record: &QuotationRecord, reception: bool, manager: bool) -> Vec<A> {
    let mut actions = Vec::new();
    match record.state {
        S::Draft | S::PendingApproval => {
            if reception {
                actions.push(A::Edit);
                if record.number.is_none() {
                    actions.push(A::Cancel);
                }
                if record
                    .quotation
                    .evaluation
                    .totals
                    .as_ref()
                    .is_some_and(|t| t.approval_required)
                {
                    actions.push(A::RequestApproval);
                } else {
                    actions.push(A::Issue);
                }
            }
            if manager {
                actions.push(A::ManageValidity);
            }
        }
        S::Issued => {
            actions.push(A::Print);
            if reception {
                actions.push(A::Accept);
            }
            if manager {
                actions.extend([A::Revise, A::Cancel]);
            }
        }
        S::Accepted => {
            actions.push(A::Print);
            if reception {
                actions.push(A::Convert);
            }
            if manager {
                actions.push(A::Cancel);
            }
        }
        S::Converted => actions.push(A::Print),
        S::Expired => {
            actions.push(A::Print);
            if manager {
                actions.push(A::Revise);
            }
        }
        S::Cancelled => {
            if record.number.is_some() {
                actions.push(A::Print);
            }
        }
    }
    actions
}

pub trait QuotationServer: Send + Sync {
    /// # Errors
    /// Requires current server authority and exact retry identity.
    fn quotation_transition(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
        request: &eitmad_contracts::quotation_lifecycle::ConfirmQuotation,
        deadline: UnixMillis,
    ) -> Result<QuotationRecord, QuotationError>;
    /// # Errors
    /// Returns only authorized branch or organization records.
    fn quotations(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
        query: &eitmad_contracts::quotation_lifecycle::ListQuotations,
        deadline: UnixMillis,
    ) -> Result<eitmad_contracts::quotation_lifecycle::QuotationPage, QuotationError>;
    /// # Errors
    /// Ends on revoked authority, unavailable transport, or cancellation.
    fn watch_quotations(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
        cancel: &std::sync::atomic::AtomicBool,
        notify: &mut dyn FnMut(eitmad_contracts::quotation_lifecycle::QuotationNotice),
    ) -> Result<(), QuotationError>;
}

/// Projects saved commercial values only. The caller must authorize the source read.
/// # Errors
/// Rejects an incomplete saved evaluation instead of inventing amounts or customer data.
pub fn customer_document(
    record: &QuotationRecord,
) -> Result<eitmad_contracts::quotation_lifecycle::CustomerDocument, QuotationError> {
    use eitmad_contracts::quotation_lifecycle::{CustomerDocument, CustomerDocumentLine};
    let e = &record.quotation.evaluation;
    let totals = e
        .totals
        .as_ref()
        .filter(|_| e.errors.is_empty())
        .ok_or(QuotationError::Invalid)?;
    let customer = e.customer.clone().ok_or(QuotationError::Invalid)?;
    let is_draft =
        matches!(record.state, S::Draft | S::PendingApproval) || record.issued_at.is_none();
    Ok(CustomerDocument {
        number: record.number.clone(),
        document_revision: record.document_revision,
        status: match record.state {
            S::Draft => "مسودة",
            S::PendingApproval => "بانتظار الموافقة",
            S::Issued => "صادر",
            S::Accepted => "مقبول",
            S::Converted => "محوّل",
            S::Expired => "منتهي",
            S::Cancelled => "ملغي",
        }
        .into(),
        is_draft,
        can_print: !is_draft
            && record.number.is_some()
            && record.permitted_actions.contains(&A::Print),
        saved_at: record.changed_at,
        issued_at: record.issued_at,
        valid_until: record.valid_until,
        validity_days: Some(record.validity_days),
        customer,
        lines: e
            .lines
            .iter()
            .map(|l| CustomerDocumentLine {
                name: l.name.clone(),
                description: l.description.clone(),
                variant_name: l.variant_name.clone(),
                color_name: l.color_name.clone(),
                handle_name: l.handle_name.clone(),
                dimensions: l.dimensions.clone(),
                quantity: l.quantity,
                unit_price_yer: l.price.unit_price_yer,
                total_yer: l.price.total_yer,
            })
            .collect(),
        discount_basis_points: e.discount_basis_points,
        subtotal_yer: totals.subtotal_yer,
        discount_yer: totals.discount_yer,
        total_yer: totals.total_yer,
    })
}
