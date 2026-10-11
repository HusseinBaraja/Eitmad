//! Capability-owned private revision handlers and a server-confirmed public sales domain.
use crate::{
    OperationError,
    domain::{
        DomainDescriptor, DomainSyncHandler, DomainValidationError, LocalOperationDraft, SyncIntent,
    },
};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    catalog_revision::CatalogRevision,
    identity::ScopeRef,
    server::AuthenticatedServerSession,
    sync::{ChangeOperation, ChangeRecord, EncodedDomainPayload, SyncMode},
    transport::{SchemaId, UnixMillis},
};
use sqlx::PgPool;

pub const PUBLIC_SCHEMA: &str = "eitmad.schema.catalog-public.v1";

/// Extracts only a valid public image reference when its record is written.
/// Malformed retained payloads grant no image access and never break unrelated chunk reads.
pub(super) fn public_image_reference(change: &ChangeRecord) -> Option<(uuid::Uuid, String)> {
    if change.operation != ChangeOperation::Upsert {
        return None;
    }
    let encoded = change.payload.as_ref()?;
    if encoded.schema_id.as_str() != PUBLIC_SCHEMA || encoded.schema_version != 1 {
        return None;
    }
    let entry: eitmad_contracts::catalog_revision::CatalogEntry =
        serde_json::from_slice(&STANDARD.decode(&encoded.base64).ok()?).ok()?;
    if entry.price.target.scope() != &change.scope {
        return None;
    }
    let image = entry.image?;
    let expected = match entry.price.target {
        eitmad_contracts::pricing::PriceTarget::Furniture(_) => {
            eitmad_contracts::catalog_image::CatalogImageKind::Furniture
        }
        eitmad_contracts::pricing::PriceTarget::Product(_) => {
            eitmad_contracts::catalog_image::CatalogImageKind::Product
        }
    };
    (image.kind == expected).then_some((image.id, image.sha256))
}
pub struct CatalogSyncHandler {
    pool: PgPool,
    schema: &'static str,
}
impl CatalogSyncHandler {
    #[must_use]
    pub fn handlers(pool: &PgPool) -> Vec<std::sync::Arc<dyn DomainSyncHandler>> {
        [
            "eitmad.schema.material.v1",
            "eitmad.schema.part.v1",
            "eitmad.schema.product.v1",
            "eitmad.schema.furniture.v1",
            PUBLIC_SCHEMA,
            "eitmad.schema.pricing.v1",
        ]
        .into_iter()
        .map(|schema| {
            std::sync::Arc::new(Self {
                pool: pool.clone(),
                schema,
            }) as std::sync::Arc<dyn DomainSyncHandler>
        })
        .collect()
    }
}
/// Checks organization or owning-Branch Receptionist relationships in the caller's tenant transaction.
pub(super) async fn reader_allowed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
) -> Result<bool, OperationError> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM control.organizations o JOIN control.relationship_tuples r ON r.tenant_id=o.tenant_id WHERE o.tenant_id=$1 AND o.organization_id=$2 AND r.subject_principal_id=$3 AND r.subject_kind='user' AND ((r.object_kind='organization' AND r.object_id=o.organization_id AND r.relation IN ('eitmad.relation.organization.manager.v1','eitmad.relation.organization.receptionist.v1')) OR (r.object_kind='branch' AND r.relation='eitmad.relation.organization.receptionist.v1' AND EXISTS(SELECT 1 FROM control.branches b WHERE b.tenant_id=o.tenant_id AND b.organization_id=o.organization_id AND b.branch_id=r.object_id))))")
        .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(actor.user_id.value()).fetch_one(&mut **tx).await.map_err(|_| OperationError::Unavailable)
}
/// Validates schema, scoped identity, and payload bounds before retaining a private revision.
pub(super) fn decode(
    draft: &LocalOperationDraft,
) -> Result<CatalogRevision, DomainValidationError> {
    let payload = draft
        .payload
        .as_ref()
        .ok_or(DomainValidationError::Invalid)?;
    if payload.base64.len() > 700_000
        || payload.schema_id != draft.schema_id
        || payload.schema_version != 1
    {
        return Err(DomainValidationError::Invalid);
    }
    let bytes = STANDARD
        .decode(&payload.base64)
        .map_err(|_| DomainValidationError::Invalid)?;
    let record: CatalogRevision =
        serde_json::from_slice(&bytes).map_err(|_| DomainValidationError::Invalid)?;
    if record.identity().3 != &draft.scope
        || eitmad_pricing::revision_schema(&record) != draft.schema_id.as_str()
        || eitmad_pricing::revision_record_id(&record) != draft.record_id.value()
    {
        return Err(DomainValidationError::Invalid);
    }
    Ok(record)
}
pub(super) fn payload(
    schema: &str,
    value: &impl serde::Serialize,
) -> Result<EncodedDomainPayload, OperationError> {
    Ok(EncodedDomainPayload {
        schema_id: SchemaId::parse(schema).map_err(|_| OperationError::Invalid)?,
        schema_version: 1,
        base64: STANDARD.encode(serde_json::to_vec(value).map_err(|_| OperationError::Invalid)?),
    })
}
#[async_trait]
impl DomainSyncHandler for CatalogSyncHandler {
    fn descriptor(&self) -> DomainDescriptor {
        DomainDescriptor {
            schema_id: SchemaId::parse(self.schema).expect("static catalog schema"),
            minimum_schema_version: 1,
            maximum_schema_version: 1,
            mode: if matches!(self.schema, PUBLIC_SCHEMA | "eitmad.schema.pricing.v1") {
                SyncMode::ServerAuthoritative
            } else {
                SyncMode::LocalFirst
            },
        }
    }
    /// Requires Manager private access or a permitted public reader relationship for this scope and intent.
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        scope: &ScopeRef,
        intent: SyncIntent,
    ) -> bool {
        if scope.kind.as_str() != "organization" {
            return false;
        }
        let Ok(mut tx) = crate::database::tenant_transaction(&self.pool, actor.tenant_id).await
        else {
            return false;
        };
        let public = matches!(self.schema, PUBLIC_SCHEMA | "eitmad.schema.pricing.v1");
        if intent == SyncIntent::Read && public {
            return reader_allowed(&mut tx, actor, scope).await.unwrap_or(false);
        }
        if matches!(self.schema, PUBLIC_SCHEMA | "eitmad.schema.pricing.v1") {
            return false;
        }
        crate::pricing::manager_allowed(&mut tx, actor, scope.id.value())
            .await
            .unwrap_or(false)
    }
    /// Rejects non-upsert, based, or incompatible private revision submissions before storage work.
    fn validate_local(&self, draft: &LocalOperationDraft) -> Result<(), DomainValidationError> {
        if draft.schema_version != 1
            || draft.operation != ChangeOperation::Upsert
            || draft.base_revision.is_some()
        {
            return Err(DomainValidationError::Invalid);
        }
        decode(draft)?;
        Ok(())
    }
    /// Rechecks Manager access and validates immutable dependencies in the operation/audit transaction.
    async fn retain_local(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        actor: &AuthenticatedServerSession,
        draft: &LocalOperationDraft,
        now: UnixMillis,
    ) -> Result<(), OperationError> {
        let record = decode(draft).map_err(OperationError::from)?;
        if !crate::pricing::manager_allowed(tx, actor, draft.scope.id.value())
            .await
            .map_err(|_| OperationError::Unavailable)?
        {
            return Err(OperationError::Denied);
        }
        crate::catalog_revision::retain(tx, actor, &record, now)
            .await
            .map(|_| ())
            .map_err(|e| match e {
                eitmad_pricing::PricingError::Unconfirmed => OperationError::Unavailable,
                _ => OperationError::Invalid,
            })
    }
    /// Rechecks the current reader in the page transaction before returning any payload.
    async fn project_page(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        actor: &AuthenticatedServerSession,
        scope: &ScopeRef,
        changes: Vec<ChangeRecord>,
    ) -> Result<Vec<ChangeRecord>, OperationError> {
        if changes.iter().any(|change| &change.scope != scope) {
            return Err(OperationError::Denied);
        }
        let allowed = if matches!(self.schema, PUBLIC_SCHEMA | "eitmad.schema.pricing.v1") {
            reader_allowed(tx, actor, scope).await?
        } else {
            crate::pricing::manager_allowed(tx, actor, scope.id.value())
                .await
                .map_err(|_| OperationError::Unavailable)?
        };
        if !allowed {
            return Err(OperationError::Denied);
        }
        Ok(changes)
    }
}
