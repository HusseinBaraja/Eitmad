//! Branch-scoped draft transfer. Historical published prices remain draft snapshots.
use crate::{
    DomainDescriptor, DomainSyncHandler, DomainValidationError, LocalOperationDraft,
    OperationError, SyncIntent, database::tenant_transaction,
};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    catalog_revision::CatalogRevision,
    identity::ScopeRef,
    pricing::PublishedPrice,
    quotation_draft::QuotationDraftSnapshot,
    server::AuthenticatedServerSession,
    sync::{ChangeOperation, SyncMode},
    transport::{SchemaId, UnixMillis},
};
use eitmad_pricing::QUOTATION_DRAFT_SCHEMA;
use sqlx::PgPool;

pub struct QuotationDraftSyncHandler {
    pool: PgPool,
}
impl QuotationDraftSyncHandler {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
pub(super) fn snapshot(
    draft: &LocalOperationDraft,
) -> Result<QuotationDraftSnapshot, DomainValidationError> {
    if draft.schema_id.as_str() != QUOTATION_DRAFT_SCHEMA
        || draft.schema_version != 1
        || draft.scope.kind.as_str() != "branch"
        || draft.operation != ChangeOperation::Upsert
    {
        return Err(DomainValidationError::Invalid);
    }
    let p = draft
        .payload
        .as_ref()
        .ok_or(DomainValidationError::Invalid)?;
    if p.schema_id != draft.schema_id || p.schema_version != 1 || p.base64.len() > 700_000 {
        return Err(DomainValidationError::Invalid);
    }
    let value: QuotationDraftSnapshot = serde_json::from_slice(
        &STANDARD
            .decode(&p.base64)
            .map_err(|_| DomainValidationError::Invalid)?,
    )
    .map_err(|_| DomainValidationError::Invalid)?;
    if value.id.value() != draft.record_id.value()
        || value.id.value().is_nil()
        || value.evaluation.scope != draft.scope
        || draft.base_revision.unwrap_or(0).checked_add(1) != Some(value.revision)
        || value.intent.lines.len() > 1000
        || value.intent.lines.is_empty()
    {
        return Err(DomainValidationError::Invalid);
    }
    Ok(value)
}
#[async_trait]
impl DomainSyncHandler for QuotationDraftSyncHandler {
    fn descriptor(&self) -> DomainDescriptor {
        DomainDescriptor {
            schema_id: SchemaId::parse(QUOTATION_DRAFT_SCHEMA).expect("static draft schema"),
            minimum_schema_version: 1,
            maximum_schema_version: 1,
            mode: SyncMode::LocalFirst,
        }
    }
    async fn authorize(
        &self,
        session: &AuthenticatedServerSession,
        scope: &ScopeRef,
        intent: SyncIntent,
    ) -> bool {
        if scope.kind.as_str() != "branch" {
            return false;
        }
        let Ok(mut tx) = tenant_transaction(&self.pool, session.tenant_id).await else {
            return false;
        };
        let read = intent == SyncIntent::Read;
        matches!(sqlx::query_scalar::<_,bool>("SELECT EXISTS (SELECT 1 FROM control.branches b JOIN control.relationship_tuples r ON r.tenant_id=b.tenant_id AND r.subject_principal_id=$2 AND r.subject_kind='user' WHERE b.tenant_id=$1 AND b.branch_id=$3 AND ((r.relation='eitmad.relation.organization.receptionist.v1' AND r.object_kind='branch' AND r.object_id=b.branch_id) OR ($4 AND r.relation='eitmad.relation.organization.manager.v1' AND r.object_kind='organization' AND r.object_id=b.organization_id)))")
            .bind(session.tenant_id.value()).bind(session.user_id.value()).bind(scope.id.value()).bind(read).fetch_one(&mut *tx).await,Ok(true))
    }
    fn validate_local(&self, draft: &LocalOperationDraft) -> Result<(), DomainValidationError> {
        snapshot(draft).map(|_| ())
    }
    async fn retain_local(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session: &AuthenticatedServerSession,
        draft: &LocalOperationDraft,
        _now: UnixMillis,
    ) -> Result<(), OperationError> {
        retain_snapshot(tx, session, draft, _now).await
    }
}

async fn validate_customer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: &AuthenticatedServerSession,
    scope: &ScopeRef,
    snapshot: &QuotationDraftSnapshot,
) -> Result<(), OperationError> {
    let reference = snapshot
        .intent
        .customer
        .as_ref()
        .ok_or(OperationError::Invalid)?;
    // A dependent customer revision may still be in the device outbox. Keep the draft retryable.
    let change: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT change_json FROM (
           SELECT change_json FROM sync.records WHERE tenant_id=$1 AND scope_kind='branch'
             AND scope_id=$2 AND schema_id='eitmad.schema.customer.v1' AND record_id=$3
             AND NOT tombstone AND revision=$4
           UNION ALL
           SELECT change_json FROM sync.operations WHERE tenant_id=$1 AND scope_kind='branch'
             AND scope_id=$2 AND schema_id='eitmad.schema.customer.v1'
             AND change_json->>'recordId'=$3::text AND change_json->>'revision'=$4::text
         ) retained LIMIT 1",
    )
    .bind(session.tenant_id.value())
    .bind(scope.id.value())
    .bind(reference.id.value())
    .bind(i64::try_from(reference.revision).map_err(|_| OperationError::Invalid)?)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| OperationError::Unavailable)?;
    let change: eitmad_contracts::sync::ChangeRecord =
        serde_json::from_value(change.ok_or(OperationError::Unavailable)?)
            .map_err(|_| OperationError::Unavailable)?;
    let payload = change.payload.ok_or(OperationError::Unavailable)?;
    let customer: eitmad_contracts::customer::CustomerSyncPayload = serde_json::from_slice(
        &STANDARD
            .decode(payload.base64)
            .map_err(|_| OperationError::Unavailable)?,
    )
    .map_err(|_| OperationError::Unavailable)?;
    let saved = snapshot
        .evaluation
        .customer
        .as_ref()
        .ok_or(OperationError::Invalid)?;
    if saved.id != customer.customer_id
        || saved.revision != customer.revision
        || saved.name != customer.name.as_str()
        || saved.phone != customer.phone.as_str()
        || saved.address.as_deref()
            != customer
                .address
                .as_ref()
                .map(eitmad_contracts::customer::CustomerAddress::as_str)
    {
        return Err(OperationError::Invalid);
    }
    Ok(())
}

pub(super) async fn retain_snapshot(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: &AuthenticatedServerSession,
    draft: &LocalOperationDraft,
    now: UnixMillis,
) -> Result<(), OperationError> {
    let value = snapshot(draft)?;
    let organization: uuid::Uuid = sqlx::query_scalar(
        "SELECT organization_id FROM control.branches WHERE tenant_id=$1 AND branch_id=$2",
    )
    .bind(session.tenant_id.value())
    .bind(draft.scope.id.value())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| OperationError::Denied)?;
    let customer = value
        .intent
        .customer
        .as_ref()
        .ok_or(OperationError::Invalid)?;
    validate_customer(tx, session, &draft.scope, &value).await?;
    let mut entries = Vec::with_capacity(value.intent.lines.len());
    for line in &value.intent.lines {
        let selection = &line.configuration.selection;
        let target = &selection.target;
        if target.scope().kind.as_str() != "organization"
            || target.scope().id.value() != organization
        {
            return Err(OperationError::Denied);
        }
        let (kind, id, variant) = target.identity();
        let definition:Vec<u8>=sqlx::query_scalar("SELECT record_json FROM sync.catalog_revisions WHERE tenant_id=$1 AND organization_id=$2 AND kind=$3 AND entry_id=$4 AND revision=$5").bind(session.tenant_id.value()).bind(organization).bind(kind).bind(id).bind(i64::try_from(target.revision()).map_err(|_| OperationError::Invalid)?).fetch_optional(&mut **tx).await.map_err(|_| OperationError::Unavailable)?.ok_or(OperationError::Invalid)?;
        let price:Vec<u8>=sqlx::query_scalar("SELECT record_json FROM sync.price_revisions WHERE tenant_id=$1 AND organization_id=$2 AND kind=$3 AND entry_id=$4 AND variant_id=$5 AND revision=$6").bind(session.tenant_id.value()).bind(organization).bind(kind).bind(id).bind(variant).bind(i64::try_from(selection.price_revision).map_err(|_| OperationError::Invalid)?).fetch_optional(&mut **tx).await.map_err(|_| OperationError::Unavailable)?.ok_or(OperationError::Invalid)?;
        let definition: CatalogRevision =
            serde_json::from_slice(&definition).map_err(|_| OperationError::Unavailable)?;
        let price: PublishedPrice =
            serde_json::from_slice(&price).map_err(|_| OperationError::Unavailable)?;
        entries.push(
            eitmad_pricing::public_entry(&definition, &price)
                .map_err(|_| OperationError::Invalid)?,
        );
    }
    eitmad_pricing::validate_draft_snapshot(&value, &entries)
        .map_err(|_| OperationError::Invalid)?;
    sqlx::query("INSERT INTO sync.quotation_draft_revisions(tenant_id,branch_id,draft_id,revision,customer_id,snapshot_json) VALUES($1,$2,$3,$4,$5,$6)").bind(session.tenant_id.value()).bind(draft.scope.id.value()).bind(value.id.value()).bind(i64::try_from(value.revision).map_err(|_| OperationError::Invalid)?).bind(customer.id.value()).bind(serde_json::to_vec(&value).map_err(|_| OperationError::Invalid)?).execute(&mut **tx).await.map_err(|_| OperationError::Unavailable)?;
    crate::quotation_approval::invalidate_for_draft(tx, session, &draft.scope, &value, now)
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}
