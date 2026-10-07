use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    accounts::{
        CreateDesktopAccount, DeactivateDesktopAccount, DesktopAccountSummary, UpdateDesktopAccount,
    },
    authorization::{RelationId, RelationshipId, RelationshipMutationResult, RelationshipSubject},
    config::{ConfigChange, ConfigSnapshot},
    customer::{
        CustomerAddress, CustomerId, CustomerMutationResult, CustomerName, CustomerNotes,
        CustomerPhone,
    },
    furniture::{Furniture, FurnitureCategory, SaveFurniture, SaveFurnitureCategory},
    material::{
        Material, MaterialCategory, MaterialUnit, SaveMaterial, SaveMaterialCategory,
        SaveMaterialUnit,
    },
    part::{Part, PartCategory, SavePart, SavePartCategory},
    product::{Product, ProductCategory, SaveProduct, SaveProductCategory},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfiguration {
    pub expected_revision: u64,
    pub changes: Vec<ConfigChange>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrantScopeRelationship {
    pub expected_policy_version: u64,
    pub subject: RelationshipSubject,
    pub relation: RelationId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevokeScopeRelationship {
    pub expected_policy_version: u64,
    pub relationship_id: RelationshipId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomer {
    pub name: CustomerName,
    pub phone: CustomerPhone,
    pub address: Option<CustomerAddress>,
    pub notes: Option<CustomerNotes>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCustomer {
    pub customer_id: CustomerId,
    pub expected_revision: u64,
    pub name: CustomerName,
    pub phone: CustomerPhone,
    pub address: Option<CustomerAddress>,
    pub notes: Option<CustomerNotes>,
}

tagged_contract! {
    /// Authoritative state-changing requests.
    pub enum Command {
        RequestDiscountApproval(crate::quotation_approval::RequestDiscountApproval) => "eitmad.quotation-approval.request.v1",
        DecideDiscountApproval(crate::quotation_approval::DecideDiscountApproval) => "eitmad.quotation-approval.decide.v1",
        PublishPrice(crate::pricing::PublishPrice) => "eitmad.pricing.publish.v1",
        ImportCatalogImage(crate::catalog_image::ImportCatalogImage) => "eitmad.catalog-image.import.v1",
        UpdateConfiguration(UpdateConfiguration) => "eitmad.config.update.v1",
        GrantScopeRelationship(GrantScopeRelationship) => "eitmad.authorization.relationship.grant.v1",
        RevokeScopeRelationship(RevokeScopeRelationship) => "eitmad.authorization.relationship.revoke.v1",
        CreateQuotationDraft(crate::quotation_draft::CreateQuotationDraft) => "eitmad.quotation-draft.create.v1",
        UpdateQuotationDraft(crate::quotation_draft::UpdateQuotationDraft) => "eitmad.quotation-draft.update.v1",
        CreateCustomer(CreateCustomer) => "eitmad.customer.create.v1",
        UpdateCustomer(UpdateCustomer) => "eitmad.customer.update.v1",
        SaveMaterialCategory(SaveMaterialCategory) => "eitmad.material-category.save.v1",
        SaveMaterialUnit(SaveMaterialUnit) => "eitmad.material-unit.save.v1",
        SaveMaterial(SaveMaterial) => "eitmad.material.save.v1",
        SaveFurniture(SaveFurniture) => "eitmad.furniture.save.v1",
        SaveFurnitureCategory(SaveFurnitureCategory) => "eitmad.furniture-category.save.v1",
        SaveProduct(SaveProduct) => "eitmad.product.save.v1",
        SaveProductCategory(SaveProductCategory) => "eitmad.product-category.save.v1",
        SavePart(SavePart) => "eitmad.part.save.v1",
        SavePartCategory(SavePartCategory) => "eitmad.part-category.save.v1",
        CreateDesktopAccount(CreateDesktopAccount) => "eitmad.desktop-account.create.v1",
        UpdateDesktopAccount(UpdateDesktopAccount) => "eitmad.desktop-account.update.v1",
        DeactivateDesktopAccount(DeactivateDesktopAccount) => "eitmad.desktop-account.deactivate.v1"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum CommandResult {
    DiscountApproval(Box<crate::quotation_approval::DiscountApproval>),
    PricePublished(crate::pricing::PublishedPrice),
    CatalogImageImported(crate::catalog_image::CatalogImageRef),
    ConfigurationUpdated(ConfigSnapshot),
    RelationshipGranted(RelationshipMutationResult),
    RelationshipRevoked(RelationshipMutationResult),
    QuotationDraftCreated(Box<crate::quotation_draft::QuotationDraft>),
    QuotationDraftUpdated(Box<crate::quotation_draft::QuotationDraft>),
    CustomerCreated(CustomerMutationResult),
    CustomerUpdated(CustomerMutationResult),
    MaterialCategorySaved(MaterialCategory),
    MaterialUnitSaved(MaterialUnit),
    MaterialSaved(Material),
    FurnitureSaved(Furniture),
    FurnitureCategorySaved(FurnitureCategory),
    ProductSaved(Product),
    ProductCategorySaved(ProductCategory),
    PartSaved(Part),
    PartCategorySaved(PartCategory),
    DesktopAccountCreated(DesktopAccountSummary),
    DesktopAccountUpdated(DesktopAccountSummary),
    DesktopAccountDeactivated(DesktopAccountSummary),
}
