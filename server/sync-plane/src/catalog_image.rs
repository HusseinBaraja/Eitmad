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

#[derive(Clone)]
pub struct CatalogImageServer {
    pool: PgPool,
}
impl CatalogImageServer {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

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
        let permission = match (kind, write) {
            (CatalogImageKind::Product, true) => "eitmad.permission.product.write.v1",
            (CatalogImageKind::Product, false) => "eitmad.permission.product.read.v1",
            (CatalogImageKind::Furniture, true) => "eitmad.permission.furniture.write.v1",
            (CatalogImageKind::Furniture, false) => "eitmad.permission.furniture.read.v1",
        };
        let allowed: bool = sqlx::query_scalar("SELECT EXISTS (
            SELECT 1 FROM control.organizations o JOIN control.relationship_tuples r ON r.tenant_id=o.tenant_id
            WHERE o.tenant_id=$1 AND o.organization_id=$2 AND r.subject_principal_id=$3 AND r.subject_kind='user'
              AND ((r.object_kind='organization' AND r.object_id=o.organization_id AND r.relation IN ('eitmad.relation.organization.manager.v1','eitmad.relation.organization.owner.v1',$4))
                OR (r.object_kind='tenant' AND r.object_id=o.tenant_id AND r.relation='eitmad.relation.organization.owner.v1')))")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(actor.user_id.value()).bind(permission)
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
fn kind(kind: CatalogImageKind) -> &'static str {
    match kind {
        CatalogImageKind::Product => "product",
        CatalogImageKind::Furniture => "furniture",
    }
}
