//! Immutable catalog revisions transferred before server price confirmation.
use crate::{
    furniture::{Furniture, FurnitureCategory},
    identity::ScopeRef,
    material::{Material, MaterialCategory, MaterialUnit},
    part::{Part, PartCategory},
    product::{Product, ProductCategory},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One scoped definition. Calculated costs are checked against stored dependencies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum CatalogRevision {
    MaterialCategory(Box<MaterialCategory>),
    PartCategory(Box<PartCategory>),
    Unit(Box<MaterialUnit>),
    Material(Box<Material>),
    Part(Box<Part>),
    ProductCategory(Box<ProductCategory>),
    FurnitureCategory(Box<FurnitureCategory>),
    Product(Box<Product>),
    Furniture(Box<Furniture>),
}

impl CatalogRevision {
    /// Returns the immutable storage identity and its explicit organization scope.
    #[must_use]
    pub fn identity(&self) -> (&'static str, uuid::Uuid, u64, &ScopeRef) {
        match self {
            Self::MaterialCategory(v) => ("material-category", v.id.value(), v.revision, &v.scope),
            Self::PartCategory(v) => ("part-category", v.id.value(), v.revision, &v.scope),
            Self::Unit(v) => ("unit", v.id.value(), v.revision, &v.scope),
            Self::Material(v) => ("material", v.id.value(), v.revision, &v.scope),
            Self::Part(v) => ("part", v.id.value(), v.revision, &v.scope),
            Self::ProductCategory(v) => ("product-category", v.id.value(), v.revision, &v.scope),
            Self::FurnitureCategory(v) => {
                ("furniture-category", v.id.value(), v.revision, &v.scope)
            }
            Self::Product(v) => ("product", v.id.value(), v.revision, &v.scope),
            Self::Furniture(v) => ("furniture", v.id.value(), v.revision, &v.scope),
        }
    }
}

/// Server-confirmed sales projection. Private definitions and cost dependencies never occur here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub price: crate::pricing::PublishedPrice,
    pub name: String,
    pub category_name: String,
    pub description: String,
    pub variant_name: String,
    pub image: Option<Box<crate::catalog_image::CatalogImageRef>>,
    pub dimensions: Option<crate::furniture::FurnitureDimensions>,
    pub customization: Option<crate::furniture::FurnitureCustomization>,
    pub colors: Vec<crate::furniture::FurnitureOption>,
    pub handles: Vec<crate::furniture::FurnitureOption>,
}

/// A dependency-ordered bounded batch; replay never changes an accepted revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SynchronizeCatalogRevisions {
    pub scope: ScopeRef,
    pub records: Vec<CatalogRevision>,
}

/// Repair information for a Manager. The rejected payload and server response stay in Rust storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSyncIssue {
    pub kind: String,
    pub id: uuid::Uuid,
    pub revision: u64,
    pub name: String,
    pub conflicted: bool,
}
