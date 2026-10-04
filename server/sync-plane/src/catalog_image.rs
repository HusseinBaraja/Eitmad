//! Authenticated organization image transfer, independent of catalog publication.
use crate::database::tenant_transaction;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_catalog_image::{ImageError, validate_asset};
use eitmad_contracts::{
    catalog_image::{
        CatalogImageChunk, CatalogImageKind, CatalogImageRef, DownloadCatalogImage,
        IMAGE_CHUNK_BYTES, MAX_IMAGE_BYTES, UploadCatalogImage,
    },
    identity::ScopeRef,
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sqlx::{PgPool, Row as _};

const MAX_ORGANIZATION_IMAGE_BYTES: i64 = 512 * 1024 * 1024;
const MAX_ORGANIZATION_IMAGES: i64 = 4096;
const MAX_TENANT_IMAGE_BYTES: i64 = 2 * 1024 * 1024 * 1024;
const MAX_TENANT_IMAGES: i64 = 16384;

#[derive(Clone)]
pub struct CatalogImageServer {
    pool: PgPool,
}
impl CatalogImageServer {
    /// Uses the sync database for tenant-isolated retained assets and mutation audit.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Opens a tenant transaction only when the user has the owning organization relationship.
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        scope: &ScopeRef,
        kind: CatalogImageKind,
        write: bool,
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, ImageError> {
        if scope.kind.as_str() != "organization" {
            return Err(ImageError::Denied);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| ImageError::Unavailable)?;
        let relations: &[&str] = match (kind, write) {
            (CatalogImageKind::Product, false) => &[
                "eitmad.relation.organization.manager.v1",
                "eitmad.relation.organization.owner.v1",
                "eitmad.relation.organization.receptionist.v1",
            ],
            _ => &[
                "eitmad.relation.organization.manager.v1",
                "eitmad.relation.organization.owner.v1",
            ],
        };
        let allowed: bool = sqlx::query_scalar("SELECT EXISTS (
            SELECT 1 FROM control.organizations o JOIN control.relationship_tuples r ON r.tenant_id=o.tenant_id
            WHERE o.tenant_id=$1 AND o.organization_id=$2 AND r.subject_principal_id=$3 AND r.subject_kind='user'
              AND ((r.object_kind='organization' AND r.object_id=o.organization_id AND r.relation=ANY($4))
                OR (r.object_kind='tenant' AND r.object_id=o.tenant_id AND r.relation='eitmad.relation.organization.owner.v1')))")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(actor.user_id.value()).bind(relations)
            .fetch_one(&mut *tx).await.map_err(|_| ImageError::Unavailable)?;
        if !allowed {
            return Err(ImageError::Denied);
        }
        Ok(tx)
    }

    /// Independently validates content and commits immutable bytes with mandatory audit.
    /// # Errors
    /// Rejects denied, malformed, forged, oversized, or unavailable requests.
    pub async fn upload(
        &self,
        actor: &AuthenticatedServerSession,
        input: UploadCatalogImage,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<CatalogImageRef, ImageError> {
        let mut tx = self
            .authorize(actor, &input.scope, input.reference.kind, true)
            .await?;
        if input.base64.len() > MAX_IMAGE_BYTES.div_ceil(3) * 4 {
            return Err(ImageError::Invalid);
        }
        let bytes = STANDARD
            .decode(&input.base64)
            .map_err(|_| ImageError::Invalid)?;
        let image = input.reference.clone();
        let bytes = tokio::task::spawn_blocking(move || {
            validate_asset(&image, &bytes)?;
            Ok::<_, ImageError>(bytes)
        })
        .await
        .map_err(|_| ImageError::Unavailable)??;
        // Serialize new retained assets across all organizations in the tenant.
        sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
            .bind(actor.tenant_id.value())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| ImageError::Unavailable)?;
        let existing: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sync.catalog_images WHERE tenant_id=$1 AND organization_id=$2 AND id=$3)")
            .bind(actor.tenant_id.value()).bind(input.scope.id.value()).bind(input.reference.id)
            .fetch_one(&mut *tx).await.map_err(|_| ImageError::Unavailable)?;
        if !existing {
            let usage = sqlx::query("SELECT COUNT(*) AS tenant_count, COALESCE(SUM(octet_length(content)),0)::bigint AS tenant_bytes,
                COUNT(*) FILTER(WHERE organization_id=$2) AS organization_count,
                COALESCE(SUM(octet_length(content)) FILTER(WHERE organization_id=$2),0)::bigint AS organization_bytes
                FROM sync.catalog_images WHERE tenant_id=$1")
                .bind(actor.tenant_id.value()).bind(input.scope.id.value()).fetch_one(&mut *tx).await
                .map_err(|_| ImageError::Unavailable)?;
            let length = i64::try_from(bytes.len()).map_err(|_| ImageError::Invalid)?;
            if !within_retention_budget(
                usage.get("tenant_count"),
                usage.get("tenant_bytes"),
                usage.get("organization_count"),
                usage.get("organization_bytes"),
                length,
            ) {
                return Err(ImageError::Invalid);
            }
        }
        let result=sqlx::query("INSERT INTO sync.catalog_images(tenant_id,organization_id,id,kind,sha256,content) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
            .bind(actor.tenant_id.value()).bind(input.scope.id.value()).bind(input.reference.id).bind(kind(input.reference.kind)).bind(&input.reference.sha256).bind(&bytes)
            .execute(&mut *tx).await.map_err(|_| ImageError::Unavailable)?;
        if result.rows_affected() > 0 {
            append(
                &mut tx,
                &ServerAuditEnvelope::from_session(
                    actor,
                    input.scope,
                    ServerAuditEvent {
                        operation: "eitmad.catalog-image.upload.v1",
                        outcome: ServerAuditOutcome::Succeeded,
                        target_kind: "catalog-image",
                        target_id: Some(input.reference.id),
                        correlation_id: correlation,
                        causation_id: None,
                        idempotency_key: None,
                        redacted_error: None,
                        occurred_at: now,
                    },
                ),
            )
            .await
            .map_err(|_| ImageError::Unavailable)?;
        }
        tx.commit().await.map_err(|_| ImageError::Unavailable)?;
        Ok(input.reference)
    }

    /// Reads only one bounded chunk in an exact tenant/organization/reference scope.
    /// # Errors
    /// Rejects denied, missing, invalid offsets, or unavailable storage.
    pub async fn download(
        &self,
        actor: &AuthenticatedServerSession,
        input: &DownloadCatalogImage,
    ) -> Result<CatalogImageChunk, ImageError> {
        let image = &input.image.reference;
        let mut tx = self
            .authorize(actor, &input.scope, image.kind, false)
            .await?;
        let row = sqlx::query("SELECT octet_length(content) AS size, substring(content FROM $6 FOR $7) AS chunk FROM sync.catalog_images WHERE tenant_id=$1 AND organization_id=$2 AND id=$3 AND kind=$4 AND sha256=$5")
            .bind(actor.tenant_id.value()).bind(input.scope.id.value()).bind(image.id).bind(kind(image.kind)).bind(&image.sha256)
            .bind(i32::try_from(input.image.offset).map_err(|_| ImageError::Invalid)?.checked_add(1).ok_or(ImageError::Invalid)?).bind(i32::try_from(IMAGE_CHUNK_BYTES).map_err(|_| ImageError::Invalid)?)
            .fetch_optional(&mut *tx).await.map_err(|_| ImageError::Unavailable)?.ok_or(ImageError::NotFound)?;
        let size: i32 = row.get("size");
        let size = u32::try_from(size).map_err(|_| ImageError::Invalid)?;
        if input.image.offset >= size {
            return Err(ImageError::Invalid);
        }
        let content: Vec<u8> = row.get("chunk");
        tx.commit().await.map_err(|_| ImageError::Unavailable)?;
        Ok(CatalogImageChunk {
            reference: image.clone(),
            offset: input.image.offset,
            total_bytes: size,
            base64: STANDARD.encode(content),
        })
    }
}
/// Uses the storage discriminator for the owning catalog capability.
fn kind(kind: CatalogImageKind) -> &'static str {
    match kind {
        CatalogImageKind::Product => "product",
        CatalogImageKind::Furniture => "furniture",
    }
}

/// Checks both aggregate scopes before charging one new immutable asset.
const fn within_retention_budget(
    tenant_count: i64,
    tenant_bytes: i64,
    organization_count: i64,
    organization_bytes: i64,
    added: i64,
) -> bool {
    tenant_count < MAX_TENANT_IMAGES
        && organization_count < MAX_ORGANIZATION_IMAGES
        && matches!(tenant_bytes.checked_add(added), Some(total) if total <= MAX_TENANT_IMAGE_BYTES)
        && matches!(organization_bytes.checked_add(added), Some(total) if total <= MAX_ORGANIZATION_IMAGE_BYTES)
}

#[cfg(test)]
mod tests {
    use super::within_retention_budget;

    #[test]
    fn new_assets_must_fit_both_tenant_and_organization_budgets() {
        let organization_bytes = 512 * 1024 * 1024;
        let tenant_bytes = 2 * 1024 * 1024 * 1024;
        assert!(within_retention_budget(
            16383,
            tenant_bytes - 4,
            4095,
            organization_bytes - 4,
            4
        ));
        assert!(!within_retention_budget(16384, 0, 0, 0, 1));
        assert!(!within_retention_budget(4096, 0, 4096, 0, 1));
        assert!(!within_retention_budget(1, tenant_bytes, 1, 0, 1));
        assert!(!within_retention_budget(1, 0, 1, organization_bytes, 1));
        assert!(!within_retention_budget(1, i64::MAX, 1, 0, 1));
    }
}
