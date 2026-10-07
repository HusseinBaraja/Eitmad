use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    accounts::{DesktopAccountPage, ListDesktopAccounts},
    authorization::{RelationshipId, RelationshipPage},
    config::ConfigSnapshot,
    customer::{Customer, CustomerPage, GetCustomer, SearchCustomers},
    furniture::{
        CheckFurnitureSelection, Furniture, FurnitureCategories, FurniturePage, FurnitureReview,
        FurnitureSelection, GetFurnitureRevision, ListFurnitureCategories, ListFurnitures,
        SaveFurniture,
    },
    material::{ListMaterialReferences, ListMaterials, MaterialPage, MaterialReferences},
    part::{
        CalculatePartCost, GetPartComposition, ListPartCategories, ListParts, Part, PartCategories,
        PartCost, PartPage,
    },
    permissions::EffectivePermissions,
    product::{
        GetProductRevision, ListProductCategories, ListProducts, Product, ProductCategories,
        ProductPage,
    },
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetConfiguration {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetEffectivePermissions {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListScopeRelationships {
    pub after: Option<RelationshipId>,
    #[schemars(range(min = 1, max = 500))]
    limit: u32,
}

impl ListScopeRelationships {
    /// Creates a bounded relationship page query.
    ///
    /// # Errors
    ///
    /// Returns [`crate::transport::PageSizeError`] for a zero or oversized page.
    pub fn new(
        after: Option<RelationshipId>,
        limit: u32,
    ) -> Result<Self, crate::transport::PageSizeError> {
        crate::transport::PageRequest::new(None, limit)?;
        Ok(Self { after, limit })
    }

    #[must_use]
    pub const fn limit(&self) -> u32 {
        self.limit
    }
}

impl<'de> Deserialize<'de> for ListScopeRelationships {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawQuery {
            after: Option<RelationshipId>,
            limit: u32,
        }

        let query = RawQuery::deserialize(deserializer)?;
        Self::new(query.after, query.limit).map_err(serde::de::Error::custom)
    }
}

tagged_contract! {
    /// Authorized read-only requests.
    pub enum Query {
        QuotationDraft(crate::quotation_draft::GetQuotationDraft) => "eitmad.quotation-draft.get.v1",
        DiscountApprovals(crate::quotation_approval::ListDiscountApprovals) => "eitmad.quotation-approval.list.v1",
        QuotationDrafts(crate::quotation_draft::ListQuotationDrafts) => "eitmad.quotation-draft.list.v1",
        QuotationEvaluation(crate::quotation::EvaluateQuotation) => "eitmad.quotation.evaluate.v1",
        SalesCatalog(crate::sales_catalog::ListSalesCatalog) => "eitmad.sales-catalog.list.v1",
        SalesCatalogItem(crate::sales_catalog::GetSalesCatalogItem) => "eitmad.sales-catalog.get.v1",
        SalesConfiguration(crate::sales_catalog::CheckSalesConfiguration) => "eitmad.sales-catalog.check.v1",
        Prices(crate::pricing::ListPrices) => "eitmad.pricing.list.v1",
        PriceReview(crate::pricing::ReviewPrice) => "eitmad.pricing.review.v1",
        SellingPrice(crate::pricing::PriceSelection) => "eitmad.pricing.selection.v1",
        DiscountTotal(crate::pricing::CalculateDiscount) => "eitmad.pricing.discount.v1",
        CatalogImage(crate::catalog_image::GetCatalogImage) => "eitmad.catalog-image.get.v1",
        Configuration(GetConfiguration) => "eitmad.config.get.v1",
        EffectivePermissions(GetEffectivePermissions) => "eitmad.permissions.get-effective.v1",
        ScopeRelationships(ListScopeRelationships) => "eitmad.authorization.relationships.list.v1",
        Customer(GetCustomer) => "eitmad.customer.get.v1",
        Customers(SearchCustomers) => "eitmad.customer.search.v1",
        Furnitures(ListFurnitures) => "eitmad.furniture.list.v1",
        FurnitureCategories(ListFurnitureCategories) => "eitmad.furniture-category.list.v1",
        FurnitureRevision(GetFurnitureRevision) => "eitmad.furniture-revision.get.v1",
        FurnitureReview(SaveFurniture) => "eitmad.furniture.review.v1",
        FurnitureSelection(CheckFurnitureSelection) => "eitmad.furniture-selection.check.v1",
        Products(ListProducts) => "eitmad.product.list.v1",
        ProductCategories(ListProductCategories) => "eitmad.product-category.list.v1",
        ProductRevision(GetProductRevision) => "eitmad.product-revision.get.v1",
        Parts(ListParts) => "eitmad.part.list.v1",
        PartCategories(ListPartCategories) => "eitmad.part-category.list.v1",
        PartCost(CalculatePartCost) => "eitmad.part.cost.v1",
        PartComposition(GetPartComposition) => "eitmad.part-composition.get.v1",
        Materials(ListMaterials) => "eitmad.material.list.v1",
        MaterialReferences(ListMaterialReferences) => "eitmad.material-reference.list.v1",
        DesktopAccounts(ListDesktopAccounts) => "eitmad.desktop-account.list.v1"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum QueryResult {
    DiscountApprovals(crate::quotation_approval::DiscountApprovalPage),
    QuotationDraft(Box<crate::quotation_draft::QuotationDraft>),
    QuotationDrafts(crate::quotation_draft::QuotationDraftPage),
    QuotationEvaluation(crate::quotation::QuotationEvaluation),
    SalesCatalog(crate::sales_catalog::SalesCatalogPage),
    SalesCatalogItem(crate::sales_catalog::SalesCatalogDetails),
    SalesConfiguration(Box<crate::sales_catalog::SalesConfiguration>),
    Prices(crate::pricing::PricePage),
    PriceReview(crate::pricing::PriceReview),
    SellingPrice(crate::pricing::SellingPrice),
    DiscountTotal(crate::pricing::DiscountTotal),
    CatalogImage(crate::catalog_image::CatalogImageChunk),
    Configuration(ConfigSnapshot),
    EffectivePermissions(EffectivePermissions),
    ScopeRelationships(RelationshipPage),
    Customer(Customer),
    Customers(CustomerPage),
    Furnitures(FurniturePage),
    FurnitureCategories(FurnitureCategories),
    FurnitureRevision(Furniture),
    FurnitureReview(FurnitureReview),
    FurnitureSelection(FurnitureSelection),
    Products(ProductPage),
    ProductCategories(ProductCategories),
    ProductRevision(Product),
    Parts(PartPage),
    PartCategories(PartCategories),
    PartCost(PartCost),
    PartComposition(Part),
    Materials(MaterialPage),
    MaterialReferences(MaterialReferences),
    DesktopAccounts(DesktopAccountPage),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_pages_are_bounded_during_deserialization() {
        assert!(
            serde_json::from_str::<ListScopeRelationships>(r#"{"after":null,"limit":1}"#).is_ok()
        );
        assert!(
            serde_json::from_str::<ListScopeRelationships>(r#"{"after":null,"limit":0}"#).is_err()
        );
        assert!(
            serde_json::from_str::<ListScopeRelationships>(r#"{"after":null,"limit":501}"#)
                .is_err()
        );
    }
}
