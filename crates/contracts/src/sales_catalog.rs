//! Receptionist-safe confirmed catalog browsing and configuration checks.
use crate::{
    catalog_revision::CatalogEntry,
    furniture::FurnitureDimensions,
    pricing::{PriceSelection, PriceTarget, SellingPrice},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSalesCatalog {
    pub term: String,
    pub category: Option<String>,
    pub after: Option<uuid::Uuid>,
    pub limit: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SalesCatalogPage {
    pub items: Vec<CatalogEntry>,
    pub categories: Vec<String>,
    pub next: Option<uuid::Uuid>,
    pub server_available: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetSalesCatalogItem {
    pub target: PriceTarget,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SalesCatalogDetails {
    pub variants: Vec<CatalogEntry>,
    pub server_available: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckSalesConfiguration {
    pub selection: PriceSelection,
    pub dimensions: Option<FurnitureDimensions>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SalesConfiguration {
    pub entry: CatalogEntry,
    pub dimensions: Option<FurnitureDimensions>,
    pub price: SellingPrice,
    pub additions_yer: i64,
    pub server_available: bool,
}
