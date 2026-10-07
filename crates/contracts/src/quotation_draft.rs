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
