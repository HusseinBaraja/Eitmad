use crate::{
    identity::{PrincipalId, ScopeRef},
    quotation_draft::QuotationDraftId,
    quotation_lifecycle::QuotationRecord,
    transport::{IdempotencyKey, UnixMillis},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AcceptanceMethod {
    InPerson,
    Phone,
    Written,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptQuotation {
    pub draft_id: QuotationDraftId,
    pub expected_revision: u64,
    pub method: AcceptanceMethod,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QuotationAcceptance {
    pub document_revision: u64,
    pub method: AcceptanceMethod,
    pub note: Option<String>,
    pub actor: PrincipalId,
    pub accepted_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertQuotation {
    pub draft_id: QuotationDraftId,
    pub expected_revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OrderState {
    Confirmed,
    InProduction,
    Ready,
    Delivered,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OrderPermittedAction {
    Cancel,
    Deliver,
    EditFulfillment,
    StartWork,
    CompleteWork,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WorkState {
    Planned,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderWork {
    pub id: uuid::Uuid,
    pub number: String,
    pub state: WorkState,
    pub line_ids: Vec<uuid::Uuid>,
    pub due_at: Option<UnixMillis>,
    pub assignment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderDelivery {
    pub id: uuid::Uuid,
    pub recipient: String,
    pub method: AcceptanceMethod,
    pub note: Option<String>,
    pub delivered_at: UnixMillis,
    pub actor: PrincipalId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderRecord {
    pub id: uuid::Uuid,
    pub scope: ScopeRef,
    pub organization_id: uuid::Uuid,
    pub revision: u64,
    pub number: String,
    pub state: OrderState,
    /// Exact accepted quotation, including its issued prices and customer snapshot.
    pub source: QuotationRecord,
    pub work: Vec<OrderWork>,
    pub delivery: Option<OrderDelivery>,
    pub fulfillment_note: Option<String>,
    pub cancellation_reason: Option<String>,
    pub created_at: UnixMillis,
    pub changed_at: UnixMillis,
    pub changed_by: PrincipalId,
    pub permitted_actions: Vec<OrderPermittedAction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelOrder {
    pub order_id: uuid::Uuid,
    pub expected_revision: u64,
    pub reason: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditOrderFulfillment {
    pub order_id: uuid::Uuid,
    pub expected_revision: u64,
    pub note: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordOrderDelivery {
    pub order_id: uuid::Uuid,
    pub expected_revision: u64,
    pub recipient: String,
    pub method: AcceptanceMethod,
    pub note: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransitionOrderWork {
    pub order_id: uuid::Uuid,
    pub expected_revision: u64,
    pub work_id: uuid::Uuid,
    pub due_at: Option<UnixMillis>,
    pub assignment: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum OrderAction {
    Convert(ConvertQuotation),
    Cancel(CancelOrder),
    EditFulfillment(EditOrderFulfillment),
    Deliver(RecordOrderDelivery),
    StartWork(TransitionOrderWork),
    CompleteWork(TransitionOrderWork),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfirmOrder {
    pub scope: ScopeRef,
    pub idempotency_key: IdempotencyKey,
    pub action: OrderAction,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListOrders {
    pub after: Option<uuid::Uuid>,
    pub limit: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetOrder {
    pub order_id: uuid::Uuid,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadOrders {
    pub scope: ScopeRef,
    pub query: ListOrders,
    pub order_id: Option<uuid::Uuid>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderPage {
    pub server_available: bool,
    pub items: Vec<OrderRecord>,
    pub next: Option<uuid::Uuid>,
    #[serde(default)]
    pub pending: Vec<OrderPending>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderPending {
    pub request: ConfirmOrder,
    pub rejected_code: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrderChanges {}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderNotice {
    pub scope: ScopeRef,
    pub order_id: uuid::Uuid,
    pub revision: u64,
}
