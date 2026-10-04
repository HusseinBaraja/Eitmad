//! Whole-YER pricing, separate catalog references, and server-confirmed revisions.
use crate::{
    furniture::FurnitureReference,
    identity::ScopeRef,
    product::ProductReference,
    transport::{IdempotencyKey, UnixMillis},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum PriceTarget {
    Product(ProductReference),
    Furniture(FurnitureReference),
}
impl PriceTarget {
    #[must_use]
    pub fn scope(&self) -> &ScopeRef {
        match self {
            Self::Product(r) => &r.scope,
            Self::Furniture(r) => &r.scope,
        }
    }
    #[must_use]
    pub const fn identity(&self) -> (&'static str, uuid::Uuid, uuid::Uuid) {
        match self {
            Self::Product(r) => ("product", r.product_id.value(), r.variant_id.value()),
            Self::Furniture(r) => ("furniture", r.furniture_id.value(), r.variant_id.value()),
        }
    }
    #[must_use]
    pub const fn revision(&self) -> u64 {
        match self {
            Self::Product(r) => r.revision,
            Self::Furniture(r) => r.revision,
        }
    }
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        match self {
            Self::Product(r) => r.schema_version,
            Self::Furniture(r) => r.schema_version,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublishPrice {
    pub target: PriceTarget,
    pub expected_revision: Option<u64>,
    pub selling_price_yer: i64,
    pub confirm_below_cost: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPrice {
    pub target: PriceTarget,
    pub selling_price_yer: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriceReview {
    pub cost_yer: i64,
    pub margin_yer: i64,
    pub below_cost: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriceAdjustment {
    pub id: uuid::Uuid,
    pub price_adjustment_yer: i64,
}
/// Immutable public snapshot. Contains no purchase cost, margin, BOM, or notes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublishedPrice {
    pub target: PriceTarget,
    pub currency: String,
    pub selling_price_yer: i64,
    pub colors: Vec<PriceAdjustment>,
    pub handles: Vec<PriceAdjustment>,
    pub revision: u64,
    pub confirmed_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriceItem {
    pub publication_required: bool,
    pub target: PriceTarget,
    pub name: String,
    pub variant_name: String,
    pub category_name: String,
    pub published: Option<PriceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_yer: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_yer: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriceSummary {
    pub currency: String,
    pub selling_price_yer: i64,
    pub revision: u64,
    pub confirmed_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPrices {
    pub term: String,
    pub after: Option<String>,
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PricePage {
    pub server_available: bool,
    pub items: Vec<PriceItem>,
    pub next: Option<String>,
    pub can_manage: bool,
    pub can_read_costs: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriceSelection {
    pub target: PriceTarget,
    pub price_revision: u64,
    pub color_id: Option<uuid::Uuid>,
    pub handle_id: Option<uuid::Uuid>,
    pub quantity: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SellingPrice {
    pub snapshot: PublishedPrice,
    pub unit_price_yer: i64,
    pub total_yer: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDiscount {
    pub line_totals_yer: Vec<i64>,
    pub discount_basis_points: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiscountTotal {
    pub subtotal_yer: i64,
    pub discount_yer: i64,
    pub total_yer: i64,
    pub approval_required: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PriceChanges {}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PriceChangeNotice {
    pub target: PriceTarget,
    pub revision: u64,
}
/// Authenticated engine-to-server proposal. The server assigns revision and time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPrice {
    /// Internal advisory basis from the authorized catalog revision. Never returned in public snapshots.
    pub cost_yer: i64,
    pub command: PublishPrice,
    pub colors: Vec<PriceAdjustment>,
    pub handles: Vec<PriceAdjustment>,
    pub idempotency_key: IdempotencyKey,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadPublishedPrices {
    pub scope: ScopeRef,
    pub after: Option<String>,
    pub limit: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublishedPricePage {
    pub items: Vec<PublishedPrice>,
    pub next: Option<String>,
}
