//! Bounded contact reads over the authoritative customer sync records.
use crate::{DomainSyncHandler as _, SyncCoordinator, SyncIntent, database::tenant_transaction};
use eitmad_contracts::{
    customer::{Customer, CustomerCursor, CustomerPage, CustomerRead, ReadCustomers},
    identity::{ScopeId, ScopeKind, ScopeRef},
    server::AuthenticatedServerSession,
    sync::ChangeRecord,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_customer::{CustomerError as E, normalize_arabic_name, normalize_phone};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sqlx::Row as _;

impl SyncCoordinator {
    /// Reads at most 1,000 candidates and returns at most 100 matching contacts.
    /// The continuation is the last examined (customer, branch), including on empty pages.
    /// # Errors
    /// Denies other organizations/branches before reading contacts. Audit failure withholds data.
    pub async fn read_customers(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ReadCustomers,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<CustomerPage, E> {
        let allowed = crate::CustomerSyncHandler::new(self.pool.clone())
            .authorize(actor, &input.scope, SyncIntent::Read)
            .await;
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        if !allowed {
            evidence(
                &mut tx,
                actor,
                input,
                correlation,
                now,
                ServerAuditOutcome::Denied,
            )
            .await?;
            tx.commit().await.map_err(|_| E::Unavailable)?;
            return Err(E::Denied);
        }
        let (after, id, branch, limit, term) = match &input.query {
            CustomerRead::Search(q) => (q.after, None, None, q.limit(), q.term.as_str()),
            CustomerRead::Get(q) => (
                None,
                Some(q.customer_id.value()),
                q.branch_id.map(ScopeId::value),
                1,
                "",
            ),
        };
        if id.is_some() && branch.is_none() && input.scope.kind.as_str() == "organization" {
            return Err(E::UnsupportedScope);
        }
        let rows = sqlx::query("SELECT r.scope_id, r.change_json FROM sync.records r
            JOIN control.branches b ON b.tenant_id=r.tenant_id AND b.branch_id=r.scope_id
            WHERE r.tenant_id=$1 AND r.scope_kind='branch' AND r.schema_id='eitmad.schema.customer.v1'
              AND NOT r.tombstone
              AND (($2='organization' AND b.organization_id=$3) OR ($2='branch' AND b.branch_id=$3))
              AND ($4::uuid IS NULL OR (r.record_id,r.scope_id)>($4,$5))
              AND ($6::uuid IS NULL OR r.record_id=$6) AND ($7::uuid IS NULL OR r.scope_id=$7)
            ORDER BY r.record_id,r.scope_id LIMIT 1001")
            .bind(actor.tenant_id.value()).bind(input.scope.kind.as_str()).bind(input.scope.id.value())
            .bind(after.map(|c| c.customer_id.value())).bind(after.map(|c| c.branch_id.value()))
            .bind(id).bind(branch).fetch_all(&mut *tx).await.map_err(|_| E::Unavailable)?;
        let name = normalize_arabic_name(term);
        let phone = normalize_phone(term).filter(|p| p.chars().any(|c| c.is_ascii_digit()));
        let mut items = Vec::new();
        let mut next = None;
        let length = rows.len();
        for (index, row) in rows.into_iter().take(1000).enumerate() {
            let (contact, cursor) = decode_contact(&row)?;
            if term.is_empty()
                || normalize_arabic_name(contact.name.as_str()).contains(&name)
                || phone.as_ref().is_some_and(|p| {
                    normalize_phone(contact.phone.as_str()).is_some_and(|v| v.contains(p))
                })
            {
                items.push(contact);
            }
            next = (index + 1 < length).then_some(cursor);
            if items.len() == limit as usize {
                break;
            }
        }
        evidence(
            &mut tx,
            actor,
            input,
            correlation,
            now,
            ServerAuditOutcome::Succeeded,
        )
        .await?;
        tx.commit().await.map_err(|_| E::Unavailable)?;
        Ok(CustomerPage { items, next })
    }
}

fn decode_contact(row: &sqlx::PgRow) -> Result<(Customer, CustomerCursor), E> {
    let change: ChangeRecord =
        serde_json::from_value(row.try_get("change_json").map_err(|_| E::Unavailable)?)
            .map_err(|_| E::Unavailable)?;
    let scope = ScopeRef {
        kind: ScopeKind::parse("branch").map_err(|_| E::Unavailable)?,
        id: ScopeId::new(row.try_get("scope_id").map_err(|_| E::Unavailable)?),
    };
    if change.scope != scope {
        return Err(E::Unavailable);
    }
    let contact = crate::customer_resolution::contact(&change)?;
    let cursor = CustomerCursor {
        customer_id: contact.id,
        branch_id: scope.id,
    };
    Ok((contact, cursor))
}

async fn evidence(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ReadCustomers,
    correlation: CorrelationId,
    now: UnixMillis,
    outcome: ServerAuditOutcome,
) -> Result<(), E> {
    append(
        tx,
        &ServerAuditEnvelope::from_session(
            actor,
            input.scope.clone(),
            ServerAuditEvent {
                operation: "eitmad.customer.read.v1",
                outcome,
                target_kind: "customer-directory",
                target_id: None,
                correlation_id: correlation,
                causation_id: None,
                idempotency_key: None,
                redacted_error: None,
                occurred_at: now,
            },
        ),
    )
    .await
    .map_err(|_| E::Unavailable)
}
