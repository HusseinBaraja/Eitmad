//! Ready-made definitions, fixed supplier options, and immutable historical references.
use crate::{identity::ScopeRef, transport::UnixMillis};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

uuid_id!(ProductId);
uuid_id!(ProductCategoryId);
uuid_id!(ProductVariantId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductCategory {
    pub id: ProductCategoryId,
    pub scope: ScopeRef,
    pub name: String,
    pub archived: bool,
    pub revision: u64,
    pub updated_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveProductCategory {
    pub id: Option<ProductCategoryId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveProductVariant {
    pub id: ProductVariantId,
    pub name: String,
    pub purchase_cost_yer: i64,
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductVariant {
    pub id: ProductVariantId,
    pub name: String,
    /// Omitted entirely when the caller cannot read internal purchase costs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_cost_yer: Option<i64>,
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveProduct {
    pub image: Option<Box<crate::catalog_image::CatalogImageRef>>,
    pub id: Option<ProductId>,
    pub expected_revision: Option<u64>,
    pub name: String,
    pub category_id: ProductCategoryId,
    pub description: String,
    pub notes: String,
    pub variants: Vec<SaveProductVariant>,
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    #[serde(default)]
    pub image: Option<Box<crate::catalog_image::CatalogImageRef>>,
    pub id: ProductId,
    pub scope: ScopeRef,
    pub name: String,
    pub category_id: ProductCategoryId,
    /// Category name at this revision, retained for historical reads.
    pub category_name: String,
    pub description: String,
    /// Internal notes are withheld with purchase costs.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    pub variants: Vec<ProductVariant>,
    pub archived: bool,
    pub revision: u64,
    pub updated_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductReference {
    pub scope: ScopeRef,
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub revision: u64,
    pub schema_version: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetProductRevision {
    pub reference: ProductReference,
    /// New work requires the current revision and an active product, category, and variant.
    pub for_new_work: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListProducts {
    pub term: String,
    pub after: Option<ProductId>,
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
    pub selectable_only: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductPage {
    pub can_manage: bool,
    pub can_read_costs: bool,
    pub items: Vec<Product>,
    pub next: Option<ProductId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListProductCategories {
    pub after: Option<ProductCategoryId>,
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductCategories {
    pub items: Vec<ProductCategory>,
    pub next: Option<ProductCategoryId>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProductChanges {}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductChangeNotice {
    pub scope: ScopeRef,
    pub id: uuid::Uuid,
    pub category: bool,
    pub revision: u64,
    pub changed_at: UnixMillis,
}
