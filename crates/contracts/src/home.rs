//! Bounded home projections. Scope and permissions come from the authenticated session.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadHome {
    /// Search input, limited to 256 UTF-8 bytes before trimming and without Unicode control characters.
    /// Invalid input returns `eitmad.error.contract-invalid.v1` with retry disposition `Never`.
    /// Quotation and order matching uses the trimmed Arabic-normalized search form.
    /// Customer and catalog authorities receive the trimmed input and apply their own normalization.
    /// Empty or whitespace-only input selects recent activity without customer or catalog search.
    /// See `docs/developer/subsystems/manager-receptionist-workflows.md#connected-home-screens`.
    pub term: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HomeAvailability {
    Available,
    Denied,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HomeDestination {
    Quotation,
    Order,
    Customer,
    Catalog,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HomeItem {
    pub id: uuid::Uuid,
    pub destination: HomeDestination,
    pub number: Option<String>,
    pub title: String,
    pub state: String,
    pub changed_at: crate::transport::UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HomeSection {
    pub availability: HomeAvailability,
    /// False means counts and recent/search results cover a bounded subset only.
    pub complete: bool,
    pub server_available: bool,
    pub count: u32,
    pub secondary_count: u32,
    pub items: Vec<HomeItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HomeSnapshot {
    pub quotations: HomeSection,
    pub orders: HomeSection,
    pub approvals: HomeSection,
    pub customers: HomeSection,
    pub catalog: HomeSection,
    pub ready_orders: Vec<HomeItem>,
}
