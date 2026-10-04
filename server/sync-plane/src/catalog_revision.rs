//! Audited immutable catalog authority used by server price validation.
use crate::{
    database::tenant_transaction,
    pricing::{PricingServer, manager_allowed},
};
use eitmad_contracts::{
    catalog_revision::{CatalogRevision, SynchronizeCatalogRevisions},
    identity::ScopeRef,
    pricing::ConfirmPrice,
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_pricing::{
    PricingError, catalog_dependencies, publication_basis, validate_catalog_revision,
};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sqlx::{Postgres, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

impl PricingServer {
    /// Accepts Manager catalog revisions atomically, preserving every previous cost basis.
    /// # Errors
    /// Denies foreign scopes and rejects altered revisions, invalid costs, or missing dependencies.
    pub async fn synchronize_catalog(
        &self,
        actor: &AuthenticatedServerSession,
        input: &SynchronizeCatalogRevisions,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), PricingError> {
        if input.scope.kind.as_str() != "organization" {
            return Err(PricingError::Denied);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| PricingError::Unconfirmed)?;
        let result = synchronize(&mut tx, actor, input, correlation, now).await;
        match result {
            Ok(()) => tx.commit().await.map_err(|_| PricingError::Unconfirmed),
            Err(error) => {
                // No accepted prefix survives a failed batch; rejection evidence is separate.
                drop(tx);
                let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
                    .await
                    .map_err(|_| PricingError::Unconfirmed)?;
                evidence(
                    &mut tx,
                    actor,
                    (&input.scope, None),
                    correlation,
                    now,
                    if error == PricingError::Denied {
                        ServerAuditOutcome::Denied
                    } else {
                        ServerAuditOutcome::Invalid
                    },
                    Some(eitmad_pricing::error_code(error)),
                )
                .await?;
                tx.commit().await.map_err(|_| PricingError::Unconfirmed)?;
                Err(error)
            }
        }
    }
}

/// Holds the same tenant lock as price CAS while validating and inserting a bounded batch.
async fn synchronize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &SynchronizeCatalogRevisions,
    correlation: CorrelationId,
    now: UnixMillis,
) -> Result<(), PricingError> {
    if !manager_allowed(tx, actor, input.scope.id.value()).await? {
        return Err(PricingError::Denied);
    }
    if input.records.is_empty() || input.records.len() > 32 {
        return Err(PricingError::Invalid);
    }
    sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
        .bind(actor.tenant_id.value())
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| PricingError::Unconfirmed)?;
    for record in &input.records {
        let (kind, id, revision, scope) = record.identity();
        if scope != &input.scope {
            return Err(PricingError::Denied);
        }
        let bytes = serde_json::to_vec(record).map_err(|_| PricingError::Invalid)?;
        if bytes.len() > 512 * 1024 {
            return Err(PricingError::Invalid);
        }
        let dependencies = catalog_dependencies(record)?;
        if let Some(existing) = load(tx, actor, scope, kind, id, revision).await? {
            if existing != *record {
                return Err(PricingError::Reference);
            }
            continue;
        }
        let mut known = BTreeMap::new();
        for (kind, id, revision) in dependencies {
            let dependency = load(tx, actor, scope, kind, id, revision)
                .await?
                .ok_or(PricingError::Reference)?;
            known.insert((kind, id, revision), dependency);
        }
        validate_catalog_revision(record, &known)?;
        sqlx::query("INSERT INTO sync.catalog_revisions(tenant_id,organization_id,kind,entry_id,revision,record_json) VALUES($1,$2,$3,$4,$5,$6)")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(kind).bind(id)
            .bind(i64::try_from(revision).map_err(|_| PricingError::Invalid)?).bind(bytes)
            .execute(&mut **tx).await.map_err(|_| PricingError::Unconfirmed)?;
        evidence(
            tx,
            actor,
            (scope, Some(id)),
            correlation,
            now,
            ServerAuditOutcome::Succeeded,
            None,
        )
        .await?;
    }
    Ok(())
}

/// Loads an exact immutable dependency, or the latest revision when revision is zero.
async fn load(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    kind: &str,
    id: Uuid,
    revision: u64,
) -> Result<Option<CatalogRevision>, PricingError> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar("SELECT record_json FROM sync.catalog_revisions WHERE tenant_id=$1 AND organization_id=$2 AND kind=$3 AND entry_id=$4 AND ($5::bigint=0 OR revision=$5) ORDER BY revision DESC LIMIT 1")
        .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(kind).bind(id)
        .bind(i64::try_from(revision).map_err(|_| PricingError::Invalid)?)
        .fetch_optional(&mut **tx).await.map_err(|_| PricingError::Unconfirmed)?;
    bytes
        .map(|bytes| serde_json::from_slice(&bytes).map_err(|_| PricingError::Unconfirmed))
        .transpose()
}

/// Resolves a current active server definition and checks the proposal against its stored basis.
pub(super) async fn validate_price(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmPrice,
) -> Result<(), PricingError> {
    let target = &input.command.target;
    let (kind, id, _) = target.identity();
    let record = load(tx, actor, target.scope(), kind, id, 0)
        .await?
        .ok_or(PricingError::Reference)?;
    publication_basis(&record, target)?;
    let category_key = match &record {
        CatalogRevision::Product(p) => ("product-category", p.category_id.value()),
        CatalogRevision::Furniture(f) => ("furniture-category", f.category_id.value()),
        _ => return Err(PricingError::Reference),
    };
    let category = load(tx, actor, target.scope(), category_key.0, category_key.1, 0)
        .await?
        .ok_or(PricingError::Reference)?;
    if !matches!(category, CatalogRevision::ProductCategory(ref c) if !c.archived)
        && !matches!(category, CatalogRevision::FurnitureCategory(ref c) if !c.archived)
    {
        return Err(PricingError::Reference);
    }
    eitmad_pricing::validate_server_proposal(input, &record)
}

/// Records catalog mutation outcomes without definitions, quantities, costs, or margin values.
async fn evidence(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    target: (&ScopeRef, Option<Uuid>),
    correlation: CorrelationId,
    now: UnixMillis,
    outcome: ServerAuditOutcome,
    code: Option<&str>,
) -> Result<(), PricingError> {
    append(
        tx,
        &ServerAuditEnvelope::from_session(
            actor,
            target.0.clone(),
            ServerAuditEvent {
                operation: "eitmad.catalog-revision.synchronize.v1",
                outcome,
                target_kind: "catalog-revision",
                target_id: target.1,
                correlation_id: correlation,
                causation_id: None,
                idempotency_key: None,
                redacted_error: code,
                occurred_at: now,
            },
        ),
    )
    .await
    .map_err(|_| PricingError::Unconfirmed)
}
