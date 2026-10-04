//! Optional, immutable catalog images. Paths are import input, never shared references.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
pub const IMAGE_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CatalogImageKind {
    Product,
    Furniture,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogImageRef {
    pub id: uuid::Uuid,
    pub kind: CatalogImageKind,
    pub sha256: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportCatalogImage {
    pub kind: CatalogImageKind,
    pub source_path: String,
}
impl std::fmt::Debug for ImportCatalogImage {
    /// Keeps the selected private filesystem path out of diagnostics.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportCatalogImage")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetCatalogImage {
    pub reference: CatalogImageRef,
    pub offset: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogImageChunk {
    pub reference: CatalogImageRef,
    pub offset: u32,
    pub total_bytes: u32,
    pub base64: String,
}
impl std::fmt::Debug for CatalogImageChunk {
    /// Includes chunk coordinates without exposing encoded image content.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CatalogImageChunk")
            .field("reference", &self.reference)
            .field("offset", &self.offset)
            .finish_non_exhaustive()
    }
}

/// Rust-only network boundary; the native shell cannot submit image content.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadCatalogImage {
    pub scope: crate::identity::ScopeRef,
    pub reference: CatalogImageRef,
    pub base64: String,
}
impl std::fmt::Debug for UploadCatalogImage {
    /// Omits encoded bytes while retaining safe transfer metadata.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadCatalogImage")
            .field("scope", &self.scope)
            .field("reference", &self.reference)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCatalogImage {
    pub scope: crate::identity::ScopeRef,
    pub image: GetCatalogImage,
}
