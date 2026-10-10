use crate::{database::tenant_transaction, quotation_approval as approvals, quotation_lifecycle};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    identity::{PrincipalId, ScopeId, ScopeKind, ScopeRef},
    order::{
        ConfirmOrder, ConvertQuotation, OrderAction, OrderPage, OrderRecord, OrderWork, ReadOrders,
        WorkState,
    },
    quotation_lifecycle::QuotationState,
    server::AuthenticatedServerSession,
    sync::{ChangeId, ChangeOperation, EncodedDomainPayload, RecordId, SyncMode},
    transport::{CorrelationId, IdempotencyKey, SchemaId, UnixMillis},
};
use eitmad_orders::{ORDER_SCHEMA, OrderError as E};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct OrderServer {
    pool: PgPool,
}
impl OrderServer {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    /// # Errors
    /// Denies foreign scopes and invalid page bounds. Details use the same authorization.
    pub async fn list(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ReadOrders,
    ) -> Result<OrderPage, E> {
        if !(1..=100).contains(&input.query.limit) {
            return Err(E::Invalid);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        authorize(&mut tx, actor, &input.scope, false, false).await?;
        let rows: Vec<Vec<u8>> = sqlx::query_scalar("SELECT h.record_json FROM sync.orders o JOIN LATERAL (SELECT record_json FROM sync.order_history h WHERE h.tenant_id=o.tenant_id AND h.order_id=o.order_id ORDER BY revision DESC LIMIT 1) h ON true WHERE o.tenant_id=$1 AND (($2='branch' AND o.branch_id=$3) OR ($2='organization' AND o.organization_id=$3)) AND ($4::uuid IS NULL OR o.order_id>$4) AND ($5::uuid IS NULL OR o.order_id=$5) ORDER BY o.order_id LIMIT $6")
            .bind(actor.tenant_id.value()).bind(input.scope.kind.as_str()).bind(input.scope.id.value()).bind(input.query.after).bind(input.order_id).bind(i64::from(input.query.limit)+1).fetch_all(&mut *tx).await.map_err(|_| E::Unavailable)?;
        let more = rows.len() > input.query.limit as usize;
        let mut items = Vec::new();
        for bytes in rows.into_iter().take(input.query.limit as usize) {
            let mut value: OrderRecord =
                serde_json::from_slice(&bytes).map_err(|_| E::Unavailable)?;
            project(&mut tx, actor, &mut value).await?;
            items.push(value);
        }
        let next = if more {
            items.last().map(|v| v.id)
        } else {
            None
        };
        Ok(OrderPage {
            server_available: true,
            items,
            next,
            pending: vec![],
        })
    }
    async fn reserve_conversion(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ConfirmOrder,
        now: UnixMillis,
    ) -> Result<Option<OrderNumbers>, E> {
        let OrderAction::Convert(c) = &input.action else {
            return Ok(None);
        };
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        let organization = authorize(&mut tx, actor, &input.scope, true, false).await?;
        let retained:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sync.order_receipts WHERE tenant_id=$1 AND idempotency_key=$2) OR EXISTS(SELECT 1 FROM sync.orders WHERE tenant_id=$1 AND draft_id=$3)").bind(actor.tenant_id.value()).bind(input.idempotency_key.value()).bind(c.draft_id.value()).fetch_one(&mut *tx).await.map_err(|_|E::Unavailable)?;
        if retained {
            return Ok(None);
        }
        let source = conversion_source(&mut tx, actor, input, c, organization, now).await?;
        let order = number(&mut tx, actor, organization, "OR", now).await?;
        let work = if source.quotation.intent.lines.iter().any(|l| {
            matches!(
                l.configuration.selection.target,
                eitmad_contracts::pricing::PriceTarget::Furniture(_)
            )
        }) {
            Some(number(&mut tx, actor, organization, "WO", now).await?)
        } else {
            None
        };
        tx.commit().await.map_err(|_| E::Unavailable)?;
        Ok(Some(OrderNumbers { order, work }))
    }

    /// # Errors
    /// Rolls back the complete mutation if mandatory history, audit, receipt or publication fails.
    pub async fn transition(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ConfirmOrder,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<OrderRecord, E> {
        // Reserve independently so rolled-back conversions never recycle an official number.
        let numbers = self.reserve_conversion(actor, input, now).await;
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        let result = transact(&mut tx, actor, input, correlation, now, numbers).await;
        match result {
            Ok(value) => {
                tx.commit().await.map_err(|_| E::Unavailable)?;
                Ok(value)
            }
            Err(E::Unavailable) => Err(E::Unavailable),
            Err(e) => {
                tx.rollback().await.map_err(|_| E::Unavailable)?;
                let mut failed = tenant_transaction(&self.pool, actor.tenant_id)
                    .await
                    .map_err(|_| E::Unavailable)?;
                evidence(&mut failed, actor, input, correlation, now, Err(e)).await?;
                failed.commit().await.map_err(|_| E::Unavailable)?;
                Err(e)
            }
        }
    }
}
pub(super) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    scope: &ScopeRef,
    reception: bool,
    manager: bool,
) -> Result<Uuid, E> {
    approvals::authorize(tx, actor, scope, reception, manager)
        .await
        .map_err(|e| match e {
            eitmad_pricing::ApprovalError::Denied => E::Denied,
            eitmad_pricing::ApprovalError::Invalid => E::Invalid,
            eitmad_pricing::ApprovalError::Conflict => E::Conflict,
            eitmad_pricing::ApprovalError::Unavailable => E::Unavailable,
        })
}
async fn project(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    value: &mut OrderRecord,
) -> Result<(), E> {
    let reception = has_role(authorize(tx, actor, &value.scope, true, false).await)?;
    let manager = has_role(authorize(tx, actor, &value.scope, false, true).await)?;
    value.permitted_actions = eitmad_orders::actions(value, reception, manager);
    if !manager {
        for work in &mut value.work {
            work.assignment = None;
        }
    }
    Ok(())
}
fn has_role(result: Result<Uuid, E>) -> Result<bool, E> {
    match result {
        Ok(_) => Ok(true),
        Err(E::Denied) => Ok(false),
        Err(e) => Err(e),
    }
}

async fn latest(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    id: Uuid,
) -> Result<Option<OrderRecord>, E> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar("SELECT record_json FROM sync.order_history WHERE tenant_id=$1 AND order_id=$2 ORDER BY revision DESC LIMIT 1")
        .bind(actor.tenant_id.value()).bind(id).fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?;
    bytes
        .map(|b| serde_json::from_slice(&b).map_err(|_| E::Unavailable))
        .transpose()
}
struct OrderNumbers {
    order: String,
    work: Option<String>,
}
async fn transact(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmOrder,
    correlation: CorrelationId,
    now: UnixMillis,
    numbers: Result<Option<OrderNumbers>, E>,
) -> Result<OrderRecord, E> {
    let reception = matches!(
        input.action,
        OrderAction::Convert(_) | OrderAction::Deliver(_)
    );
    let organization = authorize(tx, actor, &input.scope, reception, !reception).await?;
    // Shared tenant lock serializes with quotation revision/cancellation and catalog writes.
    sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_id=$1 FOR UPDATE")
        .bind(actor.tenant_id.value())
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    let hash = Sha256::digest(serde_json::to_vec(&(actor.user_id, input)).map_err(|_| E::Invalid)?)
        .to_vec();
    if let Some(row) = sqlx::query("SELECT request_hash,response_json FROM sync.order_receipts WHERE tenant_id=$1 AND idempotency_key=$2")
        .bind(actor.tenant_id.value()).bind(input.idempotency_key.value()).fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)? {
        if row.get::<Vec<u8>,_>("request_hash") != hash { return Err(E::Invalid); }
        let mut value: OrderRecord = serde_json::from_slice(&row.get::<Vec<u8>,_>("response_json")).map_err(|_| E::Unavailable)?;
        check_scope(&value, &input.scope, organization)?;
        project(tx,actor,&mut value).await?; return Ok(value);
    }
    let mut value = if let OrderAction::Convert(c) = &input.action {
        // Check the unique source before revision/state: a competing conversion returns the winner.
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT order_id FROM sync.orders WHERE tenant_id=$1 AND draft_id=$2",
        )
        .bind(actor.tenant_id.value())
        .bind(c.draft_id.value())
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
        if let Some(id) = id {
            let value = latest(tx, actor, id).await?.ok_or(E::Unavailable)?;
            check_scope(&value, &input.scope, organization)?;
            value
        } else {
            convert(
                tx,
                actor,
                input,
                c,
                organization,
                now,
                numbers?.ok_or(E::Unavailable)?,
            )
            .await?
        }
    } else {
        let mut value = latest(tx, actor, eitmad_orders::target(&input.action))
            .await?
            .ok_or(E::Conflict)?;
        check_scope(&value, &input.scope, organization)?;
        value.changed_by = PrincipalId::new(actor.user_id.value());
        eitmad_orders::apply(&mut value, &input.action, now)?;
        persist(tx, actor, &value, now).await?;
        value
    };
    evidence(tx, actor, input, correlation, now, Ok(())).await?;
    value.permitted_actions.clear();
    sqlx::query("INSERT INTO sync.order_receipts VALUES($1,$2,$3,$4)")
        .bind(actor.tenant_id.value())
        .bind(input.idempotency_key.value())
        .bind(hash)
        .bind(serde_json::to_vec(&value).map_err(|_| E::Unavailable)?)
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    project(tx, actor, &mut value).await?;
    Ok(value)
}
fn check_scope(value: &OrderRecord, scope: &ScopeRef, organization: Uuid) -> Result<(), E> {
    if value.organization_id != organization
        || scope.kind.as_str() == "branch" && value.scope != *scope
    {
        Err(E::Denied)
    } else {
        Ok(())
    }
}
async fn conversion_source(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmOrder,
    c: &ConvertQuotation,
    organization: Uuid,
    now: UnixMillis,
) -> Result<eitmad_contracts::quotation_lifecycle::QuotationRecord, E> {
    let source = quotation_lifecycle::latest(tx, actor, c.draft_id)
        .await
        .map_err(|_| E::Unavailable)?
        .ok_or(E::Conflict)?;
    if source.scope != input.scope || source.organization_id != organization {
        return Err(E::Denied);
    }
    if source.revision != c.expected_revision
        || source.state != QuotationState::Accepted
        || source.valid_until.is_none_or(|v| now.0 > v.0)
        || source
            .acceptance
            .as_ref()
            .is_none_or(|a| a.document_revision != source.document_revision)
    {
        return Err(E::Conflict);
    }
    if source.number.is_none()
        || source.quotation.evaluation.totals.is_none()
        || !source.quotation.evaluation.errors.is_empty()
    {
        return Err(E::Invalid);
    }
    Ok(source)
}
async fn convert(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmOrder,
    c: &ConvertQuotation,
    organization: Uuid,
    now: UnixMillis,
    numbers: OrderNumbers,
) -> Result<OrderRecord, E> {
    let mut source = conversion_source(tx, actor, input, c, organization, now).await?;
    source.permitted_actions.clear();
    let mut work = vec![];
    let furniture: Vec<Uuid> = source
        .quotation
        .intent
        .lines
        .iter()
        .filter(|l| {
            matches!(
                l.configuration.selection.target,
                eitmad_contracts::pricing::PriceTarget::Furniture(_)
            )
        })
        .map(|l| l.id)
        .collect();
    if !furniture.is_empty() {
        work.push(OrderWork {
            id: Uuid::new_v4(),
            number: numbers.work.ok_or(E::Unavailable)?,
            state: WorkState::Planned,
            line_ids: furniture,
            due_at: None,
            assignment: None,
        });
    }
    let value = OrderRecord {
        id: Uuid::new_v4(),
        scope: source.scope.clone(),
        organization_id: organization,
        revision: 1,
        number: numbers.order,
        state: eitmad_orders::derived_state(&work),
        source: source.clone(),
        work,
        delivery: None,
        fulfillment_note: None,
        cancellation_reason: None,
        created_at: now,
        changed_at: now,
        changed_by: PrincipalId::new(actor.user_id.value()),
        permitted_actions: vec![],
    };
    sqlx::query("INSERT INTO sync.orders VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(actor.tenant_id.value())
        .bind(organization)
        .bind(value.scope.id.value())
        .bind(value.id)
        .bind(c.draft_id.value())
        .bind(i64::try_from(source.document_revision).map_err(|_| E::Invalid)?)
        .bind(&value.number)
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    persist(tx, actor, &value, now).await?;
    source.state = QuotationState::Converted;
    source.revision = source.revision.checked_add(1).ok_or(E::Unavailable)?;
    source.changed_at = now;
    source.changed_by = value.changed_by;
    quotation_lifecycle::persist(tx, actor, &source, now)
        .await
        .map_err(|_| E::Unavailable)?;
    Ok(value)
}
async fn number(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    organization: Uuid,
    kind: &str,
    now: UnixMillis,
) -> Result<String, E> {
    let year: i32 = sqlx::query_scalar("SELECT extract(year FROM to_timestamp($1::double precision/1000) AT TIME ZONE 'Asia/Aden')::integer").bind(now.0).fetch_one(&mut **tx).await.map_err(|_| E::Unavailable)?;
    let seq: i64 = sqlx::query_scalar("INSERT INTO sync.order_numbers VALUES($1,$2,$3,$4,1) ON CONFLICT(tenant_id,organization_id,record_type,calendar_year) DO UPDATE SET last_number=sync.order_numbers.last_number+1 RETURNING last_number")
        .bind(actor.tenant_id.value()).bind(organization).bind(kind).bind(year).fetch_one(&mut **tx).await.map_err(|_| E::Unavailable)?;
    Ok(format!("{kind}-{year}-{seq:05}"))
}
async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    value: &OrderRecord,
    now: UnixMillis,
) -> Result<(), E> {
    eitmad_orders::validate_production(value)?;
    crate::work_orders::persist(tx, actor, value, now).await?;
    let mut retained = value.clone();
    retained.permitted_actions.clear();
    let json = serde_json::to_vec(&retained).map_err(|_| E::Unavailable)?;
    let revision = i64::try_from(value.revision).map_err(|_| E::Unavailable)?;
    sqlx::query("INSERT INTO sync.order_history VALUES($1,$2,$3,$4)")
        .bind(actor.tenant_id.value())
        .bind(value.id)
        .bind(revision)
        .bind(&json)
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    for work in &value.work {
        sqlx::query("INSERT INTO sync.order_work_history VALUES($1,$2,$3,$4,$5,$6)")
            .bind(actor.tenant_id.value())
            .bind(value.id)
            .bind(work.id)
            .bind(revision)
            .bind(&work.number)
            .bind(serde_json::to_vec(work).map_err(|_| E::Unavailable)?)
            .execute(&mut **tx)
            .await
            .map_err(|_| E::Unavailable)?;
    }
    if let Some(delivery) = &value.delivery {
        sqlx::query("INSERT INTO sync.order_deliveries VALUES($1,$2,$3,$4)")
            .bind(actor.tenant_id.value())
            .bind(value.id)
            .bind(delivery.id)
            .bind(serde_json::to_vec(delivery).map_err(|_| E::Unavailable)?)
            .execute(&mut **tx)
            .await
            .map_err(|_| E::Unavailable)?;
    }
    for scope in [
        value.scope.clone(),
        ScopeRef {
            kind: ScopeKind::parse("organization").expect("scope"),
            id: ScopeId::new(value.organization_id),
        },
    ] {
        crate::operations::append_domain_change(
            tx,
            actor,
            crate::LocalOperationDraft {
                scope,
                schema_id: SchemaId::parse(ORDER_SCHEMA).expect("schema"),
                schema_version: 1,
                record_id: RecordId::new(value.id),
                operation: ChangeOperation::Upsert,
                change_id: ChangeId::new(Uuid::new_v4()),
                base_revision: None,
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                payload: Some(EncodedDomainPayload {
                    schema_id: SchemaId::parse(ORDER_SCHEMA).expect("schema"),
                    schema_version: 1,
                    base64: STANDARD.encode(&json),
                }),
            },
            SyncMode::ServerAuthoritative,
            now,
        )
        .await
        .map_err(|_| E::Unavailable)?;
    }
    sqlx::query("SELECT pg_notify('eitmad_quotation_approvals','changed')")
        .execute(&mut **tx)
        .await
        .map_err(|_| E::Unavailable)?;
    Ok(())
}
async fn evidence(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    input: &ConfirmOrder,
    correlation: CorrelationId,
    now: UnixMillis,
    result: Result<(), E>,
) -> Result<(), E> {
    let operation = match input.action {
        OrderAction::Convert(_) => "eitmad.order.convert.v1",
        OrderAction::Cancel(_) => "eitmad.order.cancel.v1",
        OrderAction::EditFulfillment(_) => "eitmad.order.fulfillment.v1",
        OrderAction::Deliver(_) => "eitmad.order.deliver.v1",
        OrderAction::StartWork(_) => "eitmad.order.work-start.v1",
        OrderAction::CompleteWork(_) => "eitmad.order.work-complete.v1",
    };
    append(
        tx,
        &ServerAuditEnvelope::from_session(
            actor,
            input.scope.clone(),
            ServerAuditEvent {
                operation,
                outcome: match result {
                    Ok(()) => ServerAuditOutcome::Succeeded,
                    Err(E::Denied) => ServerAuditOutcome::Denied,
                    Err(E::Conflict) => ServerAuditOutcome::Conflict,
                    _ => ServerAuditOutcome::Invalid,
                },
                target_kind: "order",
                target_id: Some(eitmad_orders::target(&input.action)),
                correlation_id: correlation,
                causation_id: None,
                idempotency_key: Some(input.idempotency_key),
                redacted_error: result.err().map(eitmad_orders::error_code),
                occurred_at: now,
            },
        ),
    )
    .await
    .map_err(|_| E::Unavailable)
}
#[async_trait::async_trait]
impl crate::DomainSyncHandler for OrderServer {
    fn descriptor(&self) -> crate::DomainDescriptor {
        crate::DomainDescriptor {
            schema_id: SchemaId::parse(ORDER_SCHEMA).expect("schema"),
            minimum_schema_version: 1,
            maximum_schema_version: 1,
            mode: SyncMode::ServerAuthoritative,
        }
    }
    async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        scope: &ScopeRef,
        intent: crate::SyncIntent,
    ) -> bool {
        if intent != crate::SyncIntent::Read {
            return false;
        }
        let Ok(mut tx) = tenant_transaction(&self.pool, actor.tenant_id).await else {
            return false;
        };
        authorize(&mut tx, actor, scope, false, false).await.is_ok()
    }
    async fn project_page(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: &AuthenticatedServerSession,
        scope: &ScopeRef,
        mut changes: Vec<eitmad_contracts::sync::ChangeRecord>,
    ) -> Result<Vec<eitmad_contracts::sync::ChangeRecord>, crate::OperationError> {
        let organization = authorize(tx, actor, scope, false, false)
            .await
            .map_err(|_| crate::OperationError::Denied)?;
        for change in &mut changes {
            let payload = change
                .payload
                .as_mut()
                .ok_or(crate::OperationError::Invalid)?;
            let mut value: OrderRecord = serde_json::from_slice(
                &STANDARD
                    .decode(&payload.base64)
                    .map_err(|_| crate::OperationError::Invalid)?,
            )
            .map_err(|_| crate::OperationError::Invalid)?;
            check_scope(&value, scope, organization).map_err(|_| crate::OperationError::Denied)?;
            project(tx, actor, &mut value)
                .await
                .map_err(|_| crate::OperationError::Unavailable)?;
            payload.base64 = STANDARD.encode(
                serde_json::to_vec(&value).map_err(|_| crate::OperationError::Unavailable)?,
            );
        }
        Ok(changes)
    }
    fn validate_local(
        &self,
        _: &crate::LocalOperationDraft,
    ) -> Result<(), crate::DomainValidationError> {
        Err(crate::DomainValidationError::Denied)
    }
}
