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

/// One authorized source; counts, freshness and rows have independent meanings.
/// See `docs/developer/subsystems/manager-receptionist-workflows.md#connected-home-screens`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HomeSection {
    /// Denied or unavailable sections contain no rows; their zero counts mean unknown.
    pub availability: HomeAvailability,
    /// Whether source pagination ended within the read bounds. False means counts are lower bounds
    /// and rows cover a subset. True does not remove the eight-row limit or prove server freshness.
    pub complete: bool,
    /// For quotations and orders, every visited page was server-confirmed; for catalog, refresh
    /// succeeded. False permits cached data without current-server confirmation. Approvals require
    /// a successful server read. Customers are local reads: true does not prove synchronization.
    pub server_available: bool,
    /// Unfiltered count within visited records: open quotations (Draft, `PendingApproval`, Issued,
    /// Accepted, including non-cancelled local drafts), active orders (not Delivered or Cancelled),
    /// or Pending approvals. Customer and catalog sections leave this zero, not a match total.
    pub count: u32,
    /// Unfiltered Ready-order count within visited records; zero for every other section.
    pub secondary_count: u32,
    /// At most eight rows. Quotations and orders sort by `changed_at` descending, then UUID ascending.
    /// Customers retain ascending customer UUID order; catalog retains its sales-entry cursor order.
    /// Approvals have no rows. Empty searches omit customer/catalog rows; other rows match the term.
    pub items: Vec<HomeItem>,
}

/// Independently authorized home sources, not an atomic cross-source snapshot.
/// See `docs/developer/subsystems/manager-receptionist-workflows.md#connected-home-screens`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HomeSnapshot {
    /// Recent or matching quotations, with open counts independent of the search term.
    pub quotations: HomeSection,
    /// Recent or matching orders, with active and Ready counts independent of the search term.
    pub orders: HomeSection,
    /// Pending approval count without search filtering or item rows; requires server confirmation.
    pub approvals: HomeSection,
    /// Branch customer search results from local authority; organization-wide search is unavailable.
    pub customers: HomeSection,
    /// Public organization catalog search results from the authorized confirmed cache.
    pub catalog: HomeSection,
    /// At most eight Ready orders from the visited order pages, independent of the search term.
    /// Sorted by `changed_at` descending, then UUID ascending. Uses orders' completeness and freshness;
    /// empty when orders are denied or unavailable. Length is not the full Ready-order count.
    pub ready_orders: Vec<HomeItem>,
}
