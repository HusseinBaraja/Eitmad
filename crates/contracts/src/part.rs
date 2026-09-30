//! Scoped parts and immutable composition references for commercial snapshots.
use crate::{
    identity::ScopeRef,
    material::{Material, MaterialId, MaterialQuantity, MaterialUnit, MaterialUnitId},
    transport::UnixMillis,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

uuid_id!(PartId);
uuid_id!(PartCategoryId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartCategory {
    pub id: PartCategoryId,
    pub scope: ScopeRef,
    pub name: String,
    pub archived: bool,
    pub revision: u64,
    pub updated_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavePartCategory {
    pub id: Option<PartCategoryId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartUsage {
    pub material_id: MaterialId,
    pub material_revision: u64,
    pub unit_id: MaterialUnitId,
    pub unit_revision: u64,
    pub quantity: MaterialQuantity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavePart {
    pub id: Option<PartId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub category_id: PartCategoryId,
    pub description: String,
    pub usages: Vec<PartUsage>,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompositionReference {
    pub scope: ScopeRef,
    pub part_id: PartId,
    pub revision: u64,
    pub schema_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CostedUsage {
    pub usage: PartUsage,
    pub material: Material,
    pub unit: MaterialUnit,
    pub cost_unit: MaterialUnit,
    pub cost_yer: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartCost {
    pub rows: Vec<CostedUsage>,
    pub total_cost_yer: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub id: PartId,
    pub scope: ScopeRef,
    pub name: String,
    pub category_id: PartCategoryId,
    pub description: String,
    pub archived: bool,
    pub revision: u64,
    pub updated_at: UnixMillis,
    pub composition: CompositionReference,
    pub cost: PartCost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CalculatePartCost {
    pub usages: Vec<PartUsage>,
    pub part_id: Option<PartId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetPartComposition {
    pub reference: CompositionReference,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListParts {
    pub term: String,
    pub after: Option<PartId>,
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartProjection {
    pub part: Part,
    /// Advisory current cost; commercial references resolve the immutable part snapshot instead.
    pub current_cost: PartCost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartPage {
    pub items: Vec<PartProjection>,
    pub next: Option<PartId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListPartCategories {
    pub after: Option<PartCategoryId>,
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartCategories {
    pub items: Vec<PartCategory>,
    pub next: Option<PartCategoryId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PartChanges {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartChangeNotice {
    pub scope: ScopeRef,
    pub id: uuid::Uuid,
    pub category: bool,
    pub revision: u64,
    pub changed_at: UnixMillis,
}
