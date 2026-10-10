//! Manager-only manufacturing projection. Commercial prices never enter this contract.
use crate::{
    furniture::{FurnitureDimensions, FurnitureReference},
    identity::ScopeRef,
    order::{OrderPending, WorkState},
    part::CompositionReference,
    transport::UnixMillis,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderPart {
    pub reference: CompositionReference,
    pub name: String,
    /// Total count for this accepted Furniture line, calculated in Rust.
    pub quantity: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderFurniture {
    pub line_id: uuid::Uuid,
    pub reference: FurnitureReference,
    pub name: String,
    pub variant_name: String,
    pub dimensions: FurnitureDimensions,
    pub color_id: Option<uuid::Uuid>,
    pub color_name: Option<String>,
    pub handle_id: Option<uuid::Uuid>,
    pub handle_name: Option<String>,
    pub quantity: u32,
    pub parts: Vec<WorkOrderPart>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderRecord {
    pub id: uuid::Uuid,
    pub scope: ScopeRef,
    pub organization_id: uuid::Uuid,
    pub order_id: uuid::Uuid,
    pub order_number: String,
    /// Order aggregate revision used by all production commands.
    pub revision: u64,
    pub number: String,
    pub customer: String,
    pub state: WorkState,
    pub due_at: Option<UnixMillis>,
    pub assignment: Option<String>,
    pub furniture: Vec<WorkOrderFurniture>,
    pub note: Option<String>,
    pub can_start: bool,
    pub can_complete: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListWorkOrders {
    pub after: Option<uuid::Uuid>,
    pub limit: u32,
    pub order_id: Option<uuid::Uuid>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadWorkOrders {
    pub scope: ScopeRef,
    pub query: ListWorkOrders,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderPage {
    pub server_available: bool,
    pub items: Vec<WorkOrderRecord>,
    pub next: Option<uuid::Uuid>,
    pub pending: Vec<OrderPending>,
}
