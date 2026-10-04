//! Server CAS and immutable whole-YER price confirmation, with tenant isolation.
use crate::database::tenant_transaction;
use eitmad_contracts::{
    pricing::{ConfirmPrice, PublishedPrice},
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_pricing::{PricingError, validate_publication};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _};

#[derive(Clone)]
pub struct PricingServer {
    pub(super) pool: PgPool,
}
impl PricingServer {
    /// Returns only public current snapshots to scoped Manager and Receptionist relationships.
    /// # Errors
    /// Rejects invalid bounds, unrelated scopes, and unavailable storage.
    pub async fn read(
        &self,
        actor: &AuthenticatedServerSession,
        input: &eitmad_contracts::pricing::ReadPublishedPrices,
    ) -> Result<eitmad_contracts::pricing::PublishedPricePage, PricingError> {
        if input.scope.kind.as_str() != "organization" {
            return Err(PricingError::Denied);
        }
        if !(1..=100).contains(&input.limit) || input.after.as_ref().is_some_and(|v| v.len() > 100)
        {
            return Err(PricingError::Invalid);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| PricingError::Unconfirmed)?;
        let allowed: bool=sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM control.organizations o JOIN control.relationship_tuples r ON r.tenant_id=o.tenant_id WHERE o.tenant_id=$1 AND o.organization_id=$2 AND r.subject_principal_id=$3 AND r.subject_kind='user' AND r.object_kind='organization' AND r.object_id=o.organization_id AND r.relation IN ('eitmad.relation.organization.manager.v1','eitmad.relation.organization.receptionist.v1'))")
            .bind(actor.tenant_id.value()).bind(input.scope.id.value()).bind(actor.user_id.value()).fetch_one(&mut *tx).await.map_err(|_|PricingError::Unconfirmed)?;
        if !allowed {
            return Err(PricingError::Denied);
        }
        let rows=sqlx::query("SELECT kind,entry_id,variant_id,record_json FROM (SELECT DISTINCT ON (kind,entry_id,variant_id) kind,entry_id,variant_id,record_json FROM sync.price_revisions WHERE tenant_id=$1 AND organization_id=$2 ORDER BY kind,entry_id,variant_id,revision DESC) p WHERE kind || ':' || entry_id::text || ':' || variant_id::text > $3 ORDER BY kind,entry_id,variant_id LIMIT $4")
            .bind(actor.tenant_id.value()).bind(input.scope.id.value()).bind(input.after.as_deref().unwrap_or("")).bind(i64::from(input.limit)+1).fetch_all(&mut *tx).await.map_err(|_|PricingError::Unconfirmed)?;
        let mut items: Vec<PublishedPrice> = rows
            .iter()
            .take(input.limit as usize)
            .map(|row| {
                serde_json::from_slice(&row.get::<Vec<u8>, _>("record_json"))
                    .map_err(|_| PricingError::Unconfirmed)
            })
            .collect::<Result<_, _>>()?;
        let next = if rows.len() > input.limit as usize {
            items.last().map(|p| {
                let (kind, entry, variant) = p.target.identity();
                format!("{kind}:{entry}:{variant}")
            })
        } else {
            None
        };
        tx.commit().await.map_err(|_| PricingError::Unconfirmed)?;
        Ok(eitmad_contracts::pricing::PublishedPricePage {
            items: std::mem::take(&mut items),
            next,
        })
    }
    /// Resolves an exact Manager publication intent without republishing.
    /// # Errors
    /// Denies unrelated actors and rejects reuse of a key with another request.
    pub async fn status(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ConfirmPrice,
    ) -> Result<Option<PublishedPrice>, PricingError> {
        let scope = input.command.target.scope();
        if scope.kind.as_str() != "organization" {
            return Err(PricingError::Denied);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| PricingError::Unconfirmed)?;
        if !manager_allowed(&mut tx, actor, scope.id.value()).await? {
            return Err(PricingError::Denied);
        }
        let hash = request_hash(actor, input)?;
        let row = sqlx::query("SELECT request_hash,record_json FROM sync.price_receipts WHERE tenant_id=$1 AND organization_id=$2 AND idempotency_key=$3")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(input.idempotency_key.value()).fetch_optional(&mut *tx).await.map_err(|_| PricingError::Unconfirmed)?;
        let value = row
            .map(|row| {
                if row.get::<Vec<u8>, _>("request_hash") != hash {
                    return Err(PricingError::Invalid);
                }
                serde_json::from_slice(&row.get::<Vec<u8>, _>("record_json"))
                    .map_err(|_| PricingError::Unconfirmed)
            })
            .transpose()?;
        tx.commit().await.map_err(|_| PricingError::Unconfirmed)?;
        Ok(value)
    }
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    /// Authorizes the authenticated Manager and serializes compare-and-set publication.
    /// # Errors
    /// Denies Receptionists, unrelated organizations, stale input, and malformed money.
    pub async fn publish(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ConfirmPrice,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<PublishedPrice, PricingError> {
        let scope = input.command.target.scope();
        if scope.kind.as_str() != "organization" {
            return Err(PricingError::Denied);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| PricingError::Unconfirmed)?;
        if let Err(error) = preflight(&mut tx, actor, input, correlation, now).await {
            tx.commit().await.map_err(|_| PricingError::Unconfirmed)?;
            return Err(error);
        }
        // Prevent concurrent first revisions and retry-key races, including across devices.
        sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
            .bind(actor.tenant_id.value())
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| PricingError::Unconfirmed)?;
        let hash = request_hash(actor, input)?;
        let retry=sqlx::query("SELECT request_hash,record_json FROM sync.price_receipts WHERE tenant_id=$1 AND organization_id=$2 AND idempotency_key=$3")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(input.idempotency_key.value()).fetch_optional(&mut *tx).await.map_err(|_|PricingError::Unconfirmed)?;
        if let Some(row) = retry {
            if row.get::<Vec<u8>, _>("request_hash") != hash {
                return Err(PricingError::Invalid);
            }
            return serde_json::from_slice(&row.get::<Vec<u8>, _>("record_json"))
                .map_err(|_| PricingError::Unconfirmed);
        }
        let (kind, entry, variant) = input.command.target.identity();
        if let Err(e) = crate::catalog_revision::validate_price(&mut tx, actor, input).await {
            evidence(
                &mut tx,
                actor,
                input,
                correlation,
                now,
                ServerAuditOutcome::Invalid,
                Some(eitmad_pricing::error_code(e)),
            )
            .await?;
            tx.commit().await.map_err(|_| PricingError::Unconfirmed)?;
            return Err(e);
        }
        let row = sqlx::query("SELECT revision,record_json FROM sync.price_revisions WHERE tenant_id=$1 AND organization_id=$2 AND kind=$3 AND entry_id=$4 AND variant_id=$5 ORDER BY revision DESC LIMIT 1")
            .bind(actor.tenant_id.value()).bind(scope.id.value()).bind(kind).bind(entry).bind(variant).fetch_optional(&mut *tx).await.map_err(|_| PricingError::Unconfirmed)?;
        let latest: Option<PublishedPrice> = row
            .map(|row| {
                serde_json::from_slice(&row.get::<Vec<u8>, _>("record_json"))
                    .map_err(|_| PricingError::Unconfirmed)
            })
            .transpose()?;
        let actual = latest.as_ref().map(|p| p.revision);
        if actual != input.command.expected_revision
            || latest
                .as_ref()
                .is_some_and(|p| p.target.revision() > input.command.target.revision())
        {
            evidence(
                &mut tx,
                actor,
                input,
                correlation,
                now,
                ServerAuditOutcome::Conflict,
                Some("eitmad.error.pricing-revision-conflict.v1"),
            )
            .await?;
            tx.commit().await.map_err(|_| PricingError::Unconfirmed)?;
            return Err(PricingError::Conflict {
                expected: input.command.expected_revision,
                actual,
            });
        }
        persist_price(&mut tx, actor, input, actual, &hash, now).await?;
        evidence(
            &mut tx,
            actor,
            input,
            correlation,
            now,
            ServerAuditOutcome::Succeeded,
            None,
        )
        .await?;
        let record = receipt(input, actual, now);
        tx.commit().await.map_err(|_| PricingError::Unconfirmed)?;
        Ok(record)
    }
}
async fn evidence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmPrice,
    correlation: CorrelationId,
    now: UnixMillis,
    outcome: ServerAuditOutcome,
    code: Option<&str>,
) -> Result<(), PricingError> {
    append(
        tx,
        &ServerAuditEnvelope::from_session(
            actor,
            input.command.target.scope().clone(),
            ServerAuditEvent {
                operation: if input.command.confirm_below_cost {
                    "eitmad.pricing.publish-below-cost.v1"
                } else {
                    "eitmad.pricing.publish.v1"
                },
                outcome,
                target_kind: "price",
                target_id: Some(input.command.target.identity().1),
                correlation_id: correlation,
                causation_id: None,
                idempotency_key: Some(input.idempotency_key),
                redacted_error: code,
                occurred_at: now,
            },
        ),
    )
    .await
    .map_err(|_| PricingError::Unconfirmed)
}

fn receipt(input: &ConfirmPrice, actual: Option<u64>, now: UnixMillis) -> PublishedPrice {
    PublishedPrice {
        target: input.command.target.clone(),
        currency: "YER".into(),
        selling_price_yer: input.command.selling_price_yer,
        colors: input.colors.clone(),
        handles: input.handles.clone(),
        revision: actual.unwrap_or(0) + 1,
        confirmed_at: now,
    }
}
async fn persist_price(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmPrice,
    actual: Option<u64>,
    hash: &[u8],
    now: UnixMillis,
) -> Result<(), PricingError> {
    let record = receipt(input, actual, now);
    let scope = input.command.target.scope();
    let (kind, entry, variant) = input.command.target.identity();
    let bytes = serde_json::to_vec(&record).map_err(|_| PricingError::Unconfirmed)?;
    sqlx::query("INSERT INTO sync.price_revisions VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(actor.tenant_id.value())
        .bind(scope.id.value())
        .bind(kind)
        .bind(entry)
        .bind(variant)
        .bind(i64::try_from(record.revision).map_err(|_| PricingError::Invalid)?)
        .bind(&bytes)
        .execute(&mut **tx)
        .await
        .map_err(|_| PricingError::Unconfirmed)?;
    sqlx::query("INSERT INTO sync.price_receipts VALUES($1,$2,$3,$4,$5)")
        .bind(actor.tenant_id.value())
        .bind(scope.id.value())
        .bind(input.idempotency_key.value())
        .bind(hash)
        .bind(&bytes)
        .execute(&mut **tx)
        .await
        .map_err(|_| PricingError::Unconfirmed)?;
    Ok(())
}

fn request_hash(
    actor: &AuthenticatedServerSession,
    input: &ConfirmPrice,
) -> Result<Vec<u8>, PricingError> {
    Ok(Sha256::digest(
        serde_json::to_vec(&(actor.user_id, input)).map_err(|_| PricingError::Invalid)?,
    )
    .to_vec())
}
/// Checks the authenticated user's Manager relationship to an organization in the current tenant.
pub(super) async fn manager_allowed(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    organization: uuid::Uuid,
) -> Result<bool, PricingError> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM control.organizations o JOIN control.relationship_tuples r ON r.tenant_id=o.tenant_id WHERE o.tenant_id=$1 AND o.organization_id=$2 AND r.subject_principal_id=$3 AND r.subject_kind='user' AND r.object_kind='organization' AND r.object_id=o.organization_id AND r.relation='eitmad.relation.organization.manager.v1')")
        .bind(actor.tenant_id.value()).bind(organization).bind(actor.user_id.value()).fetch_one(&mut **tx).await.map_err(|_| PricingError::Unconfirmed)
}

/// Authorizes and validates proposal shape before catalog lookup or tenant serialization.
async fn preflight(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmPrice,
    correlation: CorrelationId,
    now: UnixMillis,
) -> Result<(), PricingError> {
    let scope = input.command.target.scope();
    let allowed = manager_allowed(tx, actor, scope.id.value()).await?;
    if !allowed {
        evidence(
            tx,
            actor,
            input,
            correlation,
            now,
            ServerAuditOutcome::Denied,
            Some("eitmad.error.authorization-denied.v1"),
        )
        .await?;
        return Err(PricingError::Denied);
    }
    let mut shape = input.clone();
    shape.cost_yer = 0;
    if let Err(e) = validate_publication(&shape) {
        evidence(
            tx,
            actor,
            input,
            correlation,
            now,
            ServerAuditOutcome::Invalid,
            Some(eitmad_pricing::error_code(e)),
        )
        .await?;
        return Err(e);
    }
    Ok(())
}
