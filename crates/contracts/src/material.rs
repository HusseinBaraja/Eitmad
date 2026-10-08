use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{identity::ScopeRef, transport::UnixMillis};

uuid_id!(MaterialId);
uuid_id!(MaterialCategoryId);
uuid_id!(MaterialUnitId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UnitDimension {
    Count,
    Length,
    Area,
    Volume,
    Mass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterialCategory {
    pub id: MaterialCategoryId,
    pub scope: ScopeRef,
    pub name: String,
    pub archived: bool,
    pub revision: u64,
    pub updated_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterialUnit {
    pub id: MaterialUnitId,
    pub scope: ScopeRef,
    pub name: String,
    pub symbol: String,
    pub dimension: UnitDimension,
    pub numerator: u64,
    pub denominator: u64,
    pub archived: bool,
    pub revision: u64,
    pub updated_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    pub id: MaterialId,
    pub scope: ScopeRef,
    pub name: String,
    pub category_id: MaterialCategoryId,
    pub unit_id: MaterialUnitId,
    /// Non-negative whole Yemeni rials per selected unit.
    pub current_cost_yer: i64,
    pub archived: bool,
    pub revision: u64,
    pub updated_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveMaterialCategory {
    pub id: Option<MaterialCategoryId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveMaterialUnit {
    pub id: Option<MaterialUnitId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub symbol: String,
    pub dimension: UnitDimension,
    pub numerator: u64,
    pub denominator: u64,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveMaterial {
    pub id: Option<MaterialId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub category_id: MaterialCategoryId,
    pub unit_id: MaterialUnitId,
    pub current_cost_yer: i64,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListMaterials {
    pub term: String,
    pub after: Option<MaterialId>,
    #[schemars(range(min = 1, max = 100))]
    limit: u32,
}

impl ListMaterials {
    /// Creates a bounded query; search text is validated by the material service.
    /// # Errors
    /// Rejects a page outside 1..=100.
    pub fn new(term: String, after: Option<MaterialId>, limit: u32) -> Result<Self, &'static str> {
        (1..=100)
            .contains(&limit)
            .then_some(Self { term, after, limit })
            .ok_or("invalid page size")
    }
    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }
}

impl<'de> Deserialize<'de> for ListMaterials {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            term: String,
            after: Option<MaterialId>,
            limit: u32,
        }
        let raw = Raw::deserialize(deserializer)?;
        Self::new(raw.term, raw.after, raw.limit).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterialPage {
    pub items: Vec<Material>,
    pub next: Option<MaterialId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListMaterialReferences {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterialReferences {
    pub categories: Vec<MaterialCategory>,
    pub units: Vec<MaterialUnit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MaterialRecordKind {
    Material,
    Category,
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterialChangeNotice {
    pub scope: ScopeRef,
    pub kind: MaterialRecordKind,
    pub id: uuid::Uuid,
    pub revision: u64,
    pub changed_at: UnixMillis,
}

/// Exact positive decimal quantity with no more than six fractional digits.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct MaterialQuantity(String);

impl MaterialQuantity {
    /// Parses an invariant decimal without rounding or binary floating point.
    /// # Errors
    /// Rejects signs, exponent notation, zero, and excess precision.
    pub fn parse(value: String) -> Result<Self, &'static str> {
        let (whole, fraction) = value
            .split_once('.')
            .map_or((value.as_str(), None), |(a, b)| (a, Some(b)));
        let valid = !whole.is_empty()
            && whole.bytes().all(|b| b.is_ascii_digit())
            && fraction.is_none_or(|part| {
                !part.is_empty() && part.len() <= 6 && part.bytes().all(|b| b.is_ascii_digit())
            })
            && value.bytes().any(|b| b.is_ascii_digit() && b != b'0');
        valid
            .then_some(Self(value))
            .ok_or("invalid material quantity")
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for MaterialQuantity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
