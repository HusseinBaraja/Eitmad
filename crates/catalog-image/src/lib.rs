//! Rust authority for durable, optional catalog image assets.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_authorization::{AuthorizationService, MutationContext};
use eitmad_contracts::{
    catalog_image::{
        CatalogImageChunk, CatalogImageKind, CatalogImageRef, GetCatalogImage, IMAGE_CHUNK_BYTES,
        ImportCatalogImage, MAX_IMAGE_BYTES,
    },
    identity::AuthorizationContext,
    transport::UnixMillis,
};
use eitmad_observability_audit::{AuditTarget, MutationAuditRecord};
use eitmad_storage::{AuthorityStore, DurableIdempotency};
use image::{ImageFormat, ImageReader, Limits};
use sha2::{Digest as _, Sha256};
use std::{
    fs::File,
    io::{Cursor, Read as _},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError {
    Denied,
    Invalid,
    NotFound,
    Unavailable,
}
impl From<eitmad_storage::StorageError> for ImageError {
    /// Exposes only retryable unavailability at the image boundary, without storage details.
    fn from(_: eitmad_storage::StorageError) -> Self {
        Self::Unavailable
    }
}

/// Bounded transport implemented by the authenticated Rust server client.
pub trait CatalogImageTransfer: Send + Sync {
    /// Uploads an immutable validated asset. Exact retries must be safe.
    /// # Errors
    /// Returns a redacted failure when the server denies or cannot commit it.
    fn upload(
        &self,
        actor: &AuthorizationContext,
        image: &CatalogImageRef,
        content: &[u8],
    ) -> Result<(), ImageError>;
    /// Reads one bounded chunk after server authorization.
    /// # Errors
    /// Returns a redacted failure for denied, missing, invalid, or unavailable assets.
    fn download(
        &self,
        actor: &AuthorizationContext,
        image: &CatalogImageRef,
        deadline: UnixMillis,
    ) -> Result<Vec<u8>, ImageError>;
}

#[derive(Clone)]
pub struct CatalogImageService {
    store: AuthorityStore,
    authorization: AuthorizationService,
    transfer: Option<Arc<dyn CatalogImageTransfer>>,
}
impl CatalogImageService {
    /// Creates a local image authority; network transfer remains disabled until configured.
    #[must_use]
    pub const fn new(store: AuthorityStore, authorization: AuthorizationService) -> Self {
        Self {
            store,
            authorization,
            transfer: None,
        }
    }
    /// Attaches the authenticated Rust transfer adapter without changing local persistence.
    #[must_use]
    pub fn with_transfer(mut self, transfer: Arc<dyn CatalogImageTransfer>) -> Self {
        self.transfer = Some(transfer);
        self
    }

    /// Requires the owning capability permission in an organization scope before accessing bytes.
    fn authorize(
        &self,
        actor: &AuthorizationContext,
        kind: CatalogImageKind,
        write: bool,
    ) -> Result<(), ImageError> {
        if actor.scope.kind.as_str() != "organization" {
            return Err(ImageError::Denied);
        }
        let permission = match (kind, write) {
            (CatalogImageKind::Product, false) => eitmad_authorization::PRODUCT_READ_PERMISSION,
            (CatalogImageKind::Product, true) => eitmad_authorization::PRODUCT_WRITE_PERMISSION,
            (CatalogImageKind::Furniture, false) => eitmad_authorization::FURNITURE_READ_PERMISSION,
            (CatalogImageKind::Furniture, true) => eitmad_authorization::FURNITURE_WRITE_PERMISSION,
        };
        self.authorization
            .authorize(actor, permission)
            .map_err(|e| match e {
                eitmad_authorization::AuthorizationError::Denied => ImageError::Denied,
                _ => ImageError::Unavailable,
            })
    }

    /// Imports without retaining or logging the picker path. Retry by content is immutable.
    /// # Errors
    /// Rejects denied access, non-regular files, oversized input, invalid codecs, and decoded bounds.
    pub fn import(
        &self,
        actor: &MutationContext,
        input: &ImportCatalogImage,
        deadline: UnixMillis,
    ) -> Result<CatalogImageRef, ImageError> {
        check_deadline(deadline)?;
        self.authorize(&actor.authorization, input.kind, true)?;
        self.authorize(&actor.authorization, input.kind, false)?;
        let mut hash = Sha256::new();
        hash.update(actor.authorization.identity.principal_id.value().as_bytes());
        hash.update(actor.authorization.tenant_id.value().as_bytes());
        hash.update(serde_json::to_vec(input).map_err(|_| ImageError::Invalid)?);
        let request_hash: [u8; 32] = hash.finalize().into();
        if let Some(retry) = self
            .store
            .catalog_image_retry(&actor.authorization.scope, actor.idempotency_key)?
        {
            if retry.request_hash != request_hash {
                return Err(ImageError::Invalid);
            }
            return serde_json::from_slice(&retry.response_json)
                .map_err(|_| ImageError::Unavailable);
        }
        check_deadline(deadline)?;
        let file = File::open(&input.source_path).map_err(|_| ImageError::Invalid)?;
        let metadata = file.metadata().map_err(|_| ImageError::Invalid)?;
        if !metadata.is_file() || metadata.len() > MAX_IMAGE_BYTES as u64 {
            return Err(ImageError::Invalid);
        }
        let mut bytes = Vec::new();
        file.take((MAX_IMAGE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| ImageError::Invalid)?;
        let content = normalize_before(&bytes, deadline)?;
        let reference = reference(input.kind, &content);
        let mut audit = MutationAuditRecord::from_authorization(
            &actor.authorization,
            actor.occurred_at,
            actor.correlation_id,
            "eitmad.catalog-image.import.v1",
            AuditTarget {
                kind: "catalog-image".into(),
                identifiers: vec![reference.id.to_string()],
            },
        );
        audit.causation_id = actor.causation_id;
        audit.idempotency_key = Some(actor.idempotency_key);
        let retry = DurableIdempotency {
            key: actor.idempotency_key,
            request_hash,
            response_json: serde_json::to_vec(&reference).map_err(|_| ImageError::Unavailable)?,
        };
        // Storage commits before transfer, so a temporary network failure does not lose the imported asset.
        check_deadline(deadline)?;
        self.authorize(&actor.authorization, input.kind, true)?;
        self.store.insert_catalog_image(
            &actor.authorization.scope,
            &reference,
            &content,
            &audit,
            Some(&retry),
            Some(&actor.authorization),
        )?;
        Ok(reference)
    }

    /// Checks a reference before a Product or Furniture save; transfer remains Rust-owned.
    /// # Errors
    /// Rejects wrong kind, foreign scope, missing assets, or unavailable local storage.
    pub fn attach(
        &self,
        actor: &AuthorizationContext,
        kind: CatalogImageKind,
        image: Option<&CatalogImageRef>,
    ) -> Result<(), ImageError> {
        let Some(image) = image else {
            return Ok(());
        };
        self.authorize(actor, kind, true)?;
        if image.kind != kind {
            return Err(ImageError::Invalid);
        }
        if self.store.catalog_image(&actor.scope, image)?.is_none() {
            return Err(ImageError::NotFound);
        }
        Ok(())
    }

    /// Retries a bounded page of durable uploads using current local and server authorization.
    /// # Errors
    /// Retains pending work when transport or storage is unavailable.
    pub fn retry_uploads(&self) -> Result<usize, ImageError> {
        let Some(transfer) = &self.transfer else {
            return Ok(0);
        };
        let mut uploaded = 0;
        for (actor, image) in self.store.pending_catalog_images()? {
            let outcome = self.authorize(&actor, image.kind, true).and_then(|()| {
                let bytes = self
                    .store
                    .catalog_image(&actor.scope, &image)?
                    .ok_or(ImageError::NotFound)?;
                transfer.upload(&actor, &image, &bytes)
            });
            if outcome == Err(ImageError::Unavailable) {
                break;
            }
            let audit = MutationAuditRecord::from_authorization(
                &actor,
                eitmad_authorization::now(),
                eitmad_contracts::transport::CorrelationId::new(uuid::Uuid::new_v4()),
                if outcome.is_ok() {
                    "eitmad.catalog-image.upload-acknowledged.v1"
                } else {
                    "eitmad.catalog-image.upload-deferred.v1"
                },
                AuditTarget {
                    kind: "catalog-image".into(),
                    identifiers: vec![image.id.to_string()],
                },
            );
            if outcome.is_ok() {
                self.store
                    .acknowledge_catalog_image(&actor.scope, &image, &audit)?;
                uploaded += 1;
                break;
            }
            self.store
                .defer_catalog_image(&actor.scope, &image, &audit)?;
        }
        Ok(uploaded)
    }

    /// Serves bounded chunks after local authorization, with authorized server fallback.
    /// # Errors
    /// Rejects denied, invalid, missing, or unavailable reads.
    pub fn get(
        &self,
        actor: &AuthorizationContext,
        query: &GetCatalogImage,
        deadline: UnixMillis,
    ) -> Result<CatalogImageChunk, ImageError> {
        check_deadline(deadline)?;
        self.authorize(actor, query.reference.kind, false)?;
        if let Some(content) = self.store.catalog_image(&actor.scope, &query.reference)? {
            check_deadline(deadline)?;
            return chunk(&query.reference, &content, query.offset);
        }
        let content = self
            .transfer
            .as_ref()
            .ok_or(ImageError::NotFound)?
            .download(actor, &query.reference, deadline)?;
        check_deadline(deadline)?;
        validate_asset(&query.reference, &content)?;
        check_deadline(deadline)?;
        self.authorize(actor, query.reference.kind, false)?;
        let audit = MutationAuditRecord::from_authorization(
            actor,
            eitmad_authorization::now(),
            eitmad_contracts::transport::CorrelationId::new(uuid::Uuid::new_v4()),
            "eitmad.catalog-image.cache.v1",
            AuditTarget {
                kind: "catalog-image".into(),
                identifiers: vec![query.reference.id.to_string()],
            },
        );
        self.store.insert_catalog_image(
            &actor.scope,
            &query.reference,
            &content,
            &audit,
            None,
            None,
        )?;
        chunk(&query.reference, &content, query.offset)
    }
}

/// Decodes only PNG/JPEG with independent side, pixel, allocation, and input limits.
/// # Errors
/// Returns Invalid without decoder diagnostics or content.
pub fn normalize(bytes: &[u8]) -> Result<Vec<u8>, ImageError> {
    normalize_before(bytes, UnixMillis(i64::MAX))
}

/// Checks the request budget between bounded decoder, resize, and encoder stages.
fn normalize_before(bytes: &[u8], deadline: UnixMillis) -> Result<Vec<u8>, ImageError> {
    check_deadline(deadline)?;
    let image = decode(bytes)?;
    check_deadline(deadline)?;
    let image = image.thumbnail(2048, 2048);
    check_deadline(deadline)?;
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| ImageError::Invalid)?;
    let bytes = output.into_inner();
    check_deadline(deadline)?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::Invalid);
    }
    Ok(bytes)
}

/// Stops image work at a stage boundary when its request budget has expired.
/// # Errors
/// Returns a retryable unavailable result after the deadline.
pub fn check_deadline(deadline: UnixMillis) -> Result<(), ImageError> {
    if eitmad_authorization::now().0 >= deadline.0 {
        Err(ImageError::Unavailable)
    } else {
        Ok(())
    }
}

/// Applies independent codec, input, side, pixel, and allocation bounds before decoding.
fn decode(bytes: &[u8]) -> Result<image::DynamicImage, ImageError> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::Invalid);
    }
    let format = image::guess_format(bytes).map_err(|_| ImageError::Invalid)?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return Err(ImageError::Invalid);
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| ImageError::Invalid)?;
    if u64::from(width) * u64::from(height) > 16 * 1024 * 1024 {
        return Err(ImageError::Invalid);
    }
    reader.decode().map_err(|_| ImageError::Invalid)
}

/// Derives a content identity that cannot alias across Product and Furniture capabilities.
#[must_use]
pub fn reference(kind: CatalogImageKind, bytes: &[u8]) -> CatalogImageRef {
    let digest = Sha256::digest(bytes);
    let mut id = [0; 16];
    id.copy_from_slice(&digest[..16]);
    // Different capabilities cannot alias their assets even for identical content.
    id[0] ^= match kind {
        CatalogImageKind::Product => 1,
        CatalogImageKind::Furniture => 2,
    };
    CatalogImageRef {
        id: uuid::Uuid::from_bytes(id),
        kind,
        sha256: format!("{digest:x}"),
    }
}

/// Verifies transfer integrity and decoded bounds independently at each receiving authority.
/// # Errors
/// Rejects non-canonical PNGs, forged IDs, digest mismatch, and oversized decoded images.
pub fn validate_asset(image: &CatalogImageRef, bytes: &[u8]) -> Result<(), ImageError> {
    if reference(image.kind, bytes) != *image
        || image::guess_format(bytes).ok() != Some(ImageFormat::Png)
    {
        return Err(ImageError::Invalid);
    }
    let decoded = decode(bytes)?;
    if decoded.width() > 2048 || decoded.height() > 2048 {
        return Err(ImageError::Invalid);
    }
    Ok(())
}

/// Encodes one fixed-size chunk without allowing an unbounded read response.
/// # Errors
/// Rejects invalid offsets or asset sizes.
pub fn chunk(
    image: &CatalogImageRef,
    content: &[u8],
    offset: u32,
) -> Result<CatalogImageChunk, ImageError> {
    let start = offset as usize;
    if content.is_empty() || content.len() > MAX_IMAGE_BYTES || start >= content.len() {
        return Err(ImageError::Invalid);
    }
    let end = (start + IMAGE_CHUNK_BYTES).min(content.len());
    Ok(CatalogImageChunk {
        reference: image.clone(),
        offset,
        total_bytes: u32::try_from(content.len()).map_err(|_| ImageError::Invalid)?,
        base64: STANDARD.encode(&content[start..end]),
    })
}

#[cfg(test)]
mod tests;
