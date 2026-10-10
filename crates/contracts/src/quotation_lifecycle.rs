use crate::{
    identity::{PrincipalId, ScopeRef},
    quotation_approval::DiscountRequestId,
    quotation_draft::{QuotationDraftId, QuotationDraftSnapshot},
    transport::{IdempotencyKey, UnixMillis},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QuotationState {
    Draft,
    PendingApproval,
    Issued,
    Accepted,
    Converted,
    Expired,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum QuotationPermittedAction {
    Edit,
    RequestApproval,
    Issue,
    Accept,
    Convert,
    ManageValidity,
    Revise,
    Cancel,
    Print,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IssueQuotation {
    pub draft_id: QuotationDraftId,
    pub expected_revision: u64,
    pub expected_draft_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetQuotationValidity {
    pub draft_id: QuotationDraftId,
    pub expected_revision: u64,
    pub validity_days: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelQuotation {
    pub draft_id: QuotationDraftId,
    pub expected_revision: u64,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationRecord {
    pub scope: ScopeRef,
    pub organization_id: uuid::Uuid,
    pub revision: u64,
    pub document_revision: u64,
    pub state: QuotationState,
    pub quotation: QuotationDraftSnapshot,
    pub number: Option<String>,
    pub validity_days: u32,
    pub issued_at: Option<UnixMillis>,
    pub valid_until: Option<UnixMillis>,
    pub approval_request_id: Option<DiscountRequestId>,
    pub approval_fingerprint: Option<String>,
    pub changed_at: UnixMillis,
    pub changed_by: PrincipalId,
    pub cancellation_reason: Option<String>,
    #[serde(default)]
    pub acceptance: Option<crate::order::QuotationAcceptance>,
    /// Derived from the authenticated actor and current server state, never client role flags.
    pub permitted_actions: Vec<QuotationPermittedAction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListQuotations {
    pub after: Option<QuotationDraftId>,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationPage {
    pub server_available: bool,
    pub items: Vec<QuotationRecord>,
    pub next: Option<QuotationDraftId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuotationChanges {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationNotice {
    pub scope: ScopeRef,
    pub draft_id: QuotationDraftId,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum QuotationAction {
    Issue(IssueQuotation),
    Accept(crate::order::AcceptQuotation),
    SetValidity(SetQuotationValidity),
    Revise(SetQuotationValidity),
    Cancel(CancelQuotation),
}
impl QuotationAction {
    #[must_use]
    pub const fn draft_id(&self) -> QuotationDraftId {
        match self {
            Self::Issue(c) => c.draft_id,
            Self::Accept(c) => c.draft_id,
            Self::SetValidity(c) | Self::Revise(c) => c.draft_id,
            Self::Cancel(c) => c.draft_id,
        }
    }
    #[must_use]
    pub const fn expected_revision(&self) -> u64 {
        match self {
            Self::Issue(c) => c.expected_revision,
            Self::Accept(c) => c.expected_revision,
            Self::SetValidity(c) | Self::Revise(c) => c.expected_revision,
            Self::Cancel(c) => c.expected_revision,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmQuotation {
    pub scope: ScopeRef,
    pub idempotency_key: IdempotencyKey,
    pub action: QuotationAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadQuotations {
    pub scope: ScopeRef,
    pub query: ListQuotations,
}

/// Customer-only saved document. Internal intent, price policy and approval data are excluded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomerDocument {
    pub number: Option<String>,
    pub document_revision: u64,
    pub status: String,
    pub is_draft: bool,
    pub can_print: bool,
    pub saved_at: UnixMillis,
    pub issued_at: Option<UnixMillis>,
    pub valid_until: Option<UnixMillis>,
    pub validity_days: Option<u32>,
    pub customer: crate::quotation::QuotationCustomerSnapshot,
    pub lines: Vec<CustomerDocumentLine>,
    pub discount_basis_points: u32,
    pub subtotal_yer: i64,
    pub discount_yer: i64,
    pub total_yer: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomerDocumentLine {
    pub name: String,
    pub description: String,
    pub variant_name: String,
    pub color_name: Option<String>,
    pub handle_name: Option<String>,
    pub dimensions: Option<crate::furniture::FurnitureDimensions>,
    pub quantity: u32,
    pub unit_price_yer: i64,
    pub total_yer: i64,
}
