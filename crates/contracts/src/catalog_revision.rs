//! Immutable catalog revisions transferred before server price confirmation.
use crate::{
    furniture::{Furniture, FurnitureCategory},
    identity::ScopeRef,
    material::{Material, MaterialUnit},
    part::Part,
    product::{Product, ProductCategory},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One scoped definition. Calculated costs are checked against stored dependencies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum CatalogRevision {
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

/// A dependency-ordered bounded batch; replay never changes an accepted revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SynchronizeCatalogRevisions {
    pub scope: ScopeRef,
    pub records: Vec<CatalogRevision>,
}
