//! Furniture production definitions, permitted selections, and immutable references.
use crate::{identity::ScopeRef, transport::UnixMillis};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

uuid_id!(FurnitureId);
uuid_id!(FurnitureCategoryId);
uuid_id!(FurnitureVariantId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureCategory {
    pub id: FurnitureCategoryId,
    pub scope: ScopeRef,
    pub name: String,
    pub archived: bool,
    pub revision: u64,
    pub updated_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveFurnitureCategory {
    pub id: Option<FurnitureCategoryId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureDimensions {
    pub width_mm: u32,
    pub height_mm: u32,
    pub depth_mm: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureCustomization {
    pub minimum: FurnitureDimensions,
    pub maximum: FurnitureDimensions,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureVariant {
    pub id: FurnitureVariantId,
    pub name: String,
    pub dimensions: FurnitureDimensions,
    pub customization: Option<FurnitureCustomization>,
    pub selling_price_yer: i64,
    pub archived: bool,
    /// Empty means all options of the corresponding kind are compatible.
    pub color_ids: Vec<uuid::Uuid>,
    pub handle_ids: Vec<uuid::Uuid>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureOption {
    pub id: uuid::Uuid,
    pub name: String,
    pub visual: String,
    pub price_adjustment_yer: i64,
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurniturePart {
    pub reference: crate::part::CompositionReference,
    pub quantity: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FurnitureState {
    Draft,
    Active,
    Archived,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Furniture {
    pub id: FurnitureId,
    pub scope: ScopeRef,
    pub name: String,
    pub category_id: FurnitureCategoryId,
    /// Category name at this revision, retained for historical reads.
    pub category_name: String,
    pub description: String,
    /// Internal notes are only exposed to Managers.
    pub notes: String,
    pub variants: Vec<FurnitureVariant>,
    pub parts: Vec<FurniturePart>,
    pub colors: Vec<FurnitureOption>,
    pub handles: Vec<FurnitureOption>,
    pub state: FurnitureState,
    pub parts_cost_yer: i64,
    pub revision: u64,
    pub updated_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureReference {
    pub scope: ScopeRef,
    pub furniture_id: FurnitureId,
    pub variant_id: FurnitureVariantId,
    pub revision: u64,
    pub schema_version: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetFurnitureRevision {
    pub reference: FurnitureReference,
    /// New work requires the current revision and an active furniture, category, and variant.
    pub for_new_work: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListFurnitures {
    pub term: String,
    pub after: Option<FurnitureId>,
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
    pub selectable_only: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurniturePage {
    pub can_manage: bool,
    pub can_read_costs: bool,
    pub items: Vec<Furniture>,
    pub next: Option<FurnitureId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListFurnitureCategories {
    pub after: Option<FurnitureCategoryId>,
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureCategories {
    pub items: Vec<FurnitureCategory>,
    pub next: Option<FurnitureCategoryId>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FurnitureChanges {}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureChangeNotice {
    pub scope: ScopeRef,
    pub id: uuid::Uuid,
    pub category: bool,
    pub revision: u64,
    pub changed_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureReview {
    pub parts_cost_yer: i64,
    pub row_costs_yer: Vec<i64>,
    pub margins_yer: Vec<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckFurnitureSelection {
    pub reference: FurnitureReference,
    pub dimensions: FurnitureDimensions,
    pub color_id: Option<uuid::Uuid>,
    pub handle_id: Option<uuid::Uuid>,
    pub quantity: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureSelection {
    pub definition: Furniture,
    pub unit_price_yer: i64,
    pub total_yer: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveFurniture {
    pub id: Option<FurnitureId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub category_id: FurnitureCategoryId,
    pub description: String,
    pub notes: String,
    pub parts: Vec<FurniturePart>,
    pub variants: Vec<FurnitureVariant>,
    pub colors: Vec<FurnitureOption>,
    pub handles: Vec<FurnitureOption>,
    pub state: FurnitureState,
    pub confirm_below_cost: bool,
}
