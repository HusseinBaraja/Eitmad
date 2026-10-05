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
pub(super) async fn reader_allowed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
) -> Result<bool, OperationError> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM control.organizations o JOIN control.relationship_tuples r ON r.tenant_id=o.tenant_id WHERE o.tenant_id=$1 AND o.organization_id=$2 AND r.subject_principal_id=$3 AND r.subject_kind='user' AND ((r.object_kind='organization' AND r.object_id=o.organization_id AND r.relation IN ('eitmad.relation.organization.manager.v1','eitmad.relation.organization.receptionist.v1')) OR (r.object_kind='branch' AND r.relation='eitmad.relation.organization.receptionist.v1' AND EXISTS(SELECT 1 FROM control.branches b WHERE b.tenant_id=o.tenant_id AND b.organization_id=o.organization_id AND b.branch_id=r.object_id))))")
        .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(actor.user_id.value()).fetch_one(&mut **tx).await.map_err(|_| OperationError::Unavailable)
}
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
        let public = matches!(
            self.schema,
            PUBLIC_SCHEMA | "eitmad.schema.product.v1" | "eitmad.schema.pricing.v1"
        );
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
    async fn project(
        &self,
        actor: &AuthenticatedServerSession,
        mut change: ChangeRecord,
    ) -> Result<ChangeRecord, OperationError> {
        if self.schema != "eitmad.schema.product.v1" || change.payload.is_none() {
            return Ok(change);
        }
        let mut tx = crate::database::tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| OperationError::Unavailable)?;
        if crate::pricing::manager_allowed(&mut tx, actor, change.scope.id.value())
            .await
            .map_err(|_| OperationError::Unavailable)?
        {
            return Ok(change);
        }
        let payload_value = change.payload.as_ref().ok_or(OperationError::Invalid)?;
        let mut record: CatalogRevision = serde_json::from_slice(
            &STANDARD
                .decode(&payload_value.base64)
                .map_err(|_| OperationError::Invalid)?,
        )
        .map_err(|_| OperationError::Invalid)?;
        if let CatalogRevision::Product(p) = &mut record {
            p.notes.clear();
            for v in &mut p.variants {
                v.purchase_cost_yer = None;
            }
        }
        change.payload = Some(payload(self.schema, &record)?);
        Ok(change)
    }
}
