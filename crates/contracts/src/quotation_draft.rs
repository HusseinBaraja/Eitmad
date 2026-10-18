//! Durable branch-scoped drafts. They carry no official number or issuance authority.
use crate::{
    identity::ScopeRef,
    quotation::{EvaluateQuotation, QuotationEvaluation},
    sync::{ChangeId, ChangeRecord, ConflictId},
    transport::UnixMillis,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

uuid_id!(QuotationDraftId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateQuotationDraft {
    pub intent: EvaluateQuotation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateQuotationDraft {
    pub draft_id: QuotationDraftId,
    pub expected_revision: u64,
    pub intent: EvaluateQuotation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetQuotationDraft {
    pub draft_id: QuotationDraftId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListQuotationDrafts {
    pub after: Option<QuotationDraftId>,
    pub limit: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuotationDraftChanges {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QuotationDraftSyncState {
    Pending,
    Confirmed,
    Rejected,
    Conflicted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotationDraftSnapshot {
    #[serde(default)]
    pub cancelled: bool,
    pub id: QuotationDraftId,
    pub revision: u64,
    pub intent: EvaluateQuotation,
    pub evaluation: QuotationEvaluation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationDraftConflict {
    pub server_conflict_id: Option<ConflictId>,
    /// Retained authoritative input; the local snapshot remains visible.
    pub remote: Option<ChangeRecord>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationDraft {
    #[serde(default)]
    pub permitted_actions: Vec<crate::quotation_lifecycle::QuotationPermittedAction>,
    pub scope: ScopeRef,
    pub snapshot: QuotationDraftSnapshot,
    pub updated_at: UnixMillis,
    pub sync_state: QuotationDraftSyncState,
    pub conflict: Option<QuotationDraftConflict>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationDraftPage {
    pub items: Vec<QuotationDraft>,
    pub next: Option<QuotationDraftId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationDraftChangeNotice {
    pub draft_id: QuotationDraftId,
    pub scope: ScopeRef,
    pub revision: u64,
    pub changed_at: UnixMillis,
    pub change_id: ChangeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelQuotationDraft {
    pub draft_id: QuotationDraftId,
    pub expected_revision: u64,
}

/// A review always names the branch, including an organization Manager's review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetQuotationDraftReview {
    pub draft_id: QuotationDraftId,
    pub branch_id: crate::identity::ScopeId,
    pub after: Option<ConflictId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum QuotationDraftResolutionSource {
    Conflict(QuotationDraftConflictSource),
    PendingReview(Box<ChangeRecord>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotationDraftConflictSource {
    pub conflict_id: ConflictId,
}

/// Whole saved snapshots retain their exact prices and configurations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QuotationDraftResolutionChoice {
    KeepCurrent,
    UseInput,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveQuotationDraft {
    pub draft_id: QuotationDraftId,
    pub branch_id: crate::identity::ScopeId,
    pub expected_server_revision: u64,
    pub source: QuotationDraftResolutionSource,
    pub choice: QuotationDraftResolutionChoice,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmQuotationDraftResolution {
    pub scope: ScopeRef,
    pub idempotency_key: crate::transport::IdempotencyKey,
    pub command: ResolveQuotationDraft,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadQuotationDraftReview {
    pub scope: ScopeRef,
    pub query: GetQuotationDraftReview,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationDraftReviewInput {
    pub change: ChangeRecord,
    pub snapshot: QuotationDraftSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationDraftConflictView {
    pub conflict: crate::sync::ConflictRecord,
    pub local: QuotationDraftSnapshot,
    pub remote: QuotationDraftSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationDraftReview {
    pub current: QuotationDraftSnapshot,
    pub items: Vec<QuotationDraftConflictView>,
    pub next: Option<ConflictId>,
    pub pending_reviews: Vec<QuotationDraftReviewInput>,
    pub pending_resolution: Option<ConfirmQuotationDraftResolution>,
    pub can_resolve: bool,
}

/// Authenticated recovery read; it conveys no resolution permission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadQuotationDraftRecovery {
    pub scope: ScopeRef,
    pub draft_id: QuotationDraftId,
}
