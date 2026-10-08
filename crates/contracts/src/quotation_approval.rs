//! Server-confirmed discount requests and decisions; clients cannot supply commercial authority.
use crate::{
    identity::{PrincipalId, ScopeRef},
    quotation_draft::{QuotationDraftId, QuotationDraftSnapshot},
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

uuid_id!(DiscountRequestId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestDiscountApproval {
    pub draft_id: QuotationDraftId,
    pub expected_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DiscountDecision {
    Approve,
    Reject,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecideDiscountApproval {
    pub draft_id: QuotationDraftId,
    pub request_id: DiscountRequestId,
    pub quotation_revision: u64,
    pub expected_revision: u64,
    pub fingerprint: String,
    pub decision: DiscountDecision,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DiscountApprovalState {
    Pending,
    Approved,
    Rejected,
    Invalidated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscountApproval {
    pub scope: ScopeRef,
    pub organization_id: uuid::Uuid,
    pub request_id: DiscountRequestId,
    pub revision: u64,
    pub quotation: QuotationDraftSnapshot,
    pub fingerprint: String,
    pub validity_days: u32,
    pub proposed_valid_until: UnixMillis,
    pub requester: PrincipalId,
    pub requested_at: UnixMillis,
    pub state: DiscountApprovalState,
    pub decider: Option<PrincipalId>,
    pub decided_at: Option<UnixMillis>,
    pub reason: Option<String>,
    pub correlation_id: CorrelationId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListDiscountApprovals {
    pub after: Option<QuotationDraftId>,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscountApprovalPage {
    pub items: Vec<DiscountApproval>,
    pub next: Option<QuotationDraftId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscountApprovalChanges {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscountApprovalNotice {
    pub scope: ScopeRef,
    pub draft_id: QuotationDraftId,
    pub revision: u64,
}

/// Only the authenticated Rust engine sends a validated persisted snapshot for promotion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmDiscountApproval {
    pub scope: ScopeRef,
    pub idempotency_key: IdempotencyKey,
    pub action: DiscountApprovalAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum DiscountApprovalAction {
    Request(QuotationDraftSnapshot),
    Refresh(QuotationDraftSnapshot),
    Decide(DecideDiscountApproval),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadDiscountApprovals {
    pub scope: ScopeRef,
    pub query: ListDiscountApprovals,
}
