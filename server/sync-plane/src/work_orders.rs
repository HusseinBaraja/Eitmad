//! Manufacturing snapshots share the order aggregate transaction and revision.
use crate::{database::tenant_transaction, orders};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    catalog_revision::CatalogRevision,
    identity::ScopeRef,
    order::{OrderRecord, OrderState, OrderWork, WorkState},
    pricing::PriceTarget,
    server::AuthenticatedServerSession,
    sync::{ChangeId, ChangeOperation, EncodedDomainPayload, RecordId, SyncMode},
    transport::{IdempotencyKey, SchemaId, UnixMillis},
    work_order::{
        ReadWorkOrders, WorkOrderFurniture, WorkOrderPage, WorkOrderPart, WorkOrderRecord,
    },
};
use eitmad_orders::{OrderError as E, WORK_ORDER_SCHEMA};
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct WorkOrderServer {
    pool: PgPool,
}
impl WorkOrderServer {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    /// # Errors
    /// Full manufacturing reads require the scoped Manager relationship.
    pub async fn list(
        &self,
        actor: &AuthenticatedServerSession,
        input: &ReadWorkOrders,
    ) -> Result<WorkOrderPage, E> {
        if !(1..=100).contains(&input.query.limit) {
            return Err(E::Invalid);
        }
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| E::Unavailable)?;
        orders::authorize(&mut tx, actor, &input.scope, false, true).await?;
        let rows = sqlx::query("SELECT h.record_json,w.work_id FROM (SELECT DISTINCT tenant_id,order_id,work_id FROM sync.order_work_history) w JOIN sync.orders o ON o.tenant_id=w.tenant_id AND o.order_id=w.order_id JOIN LATERAL (SELECT record_json FROM sync.order_history h WHERE h.tenant_id=o.tenant_id AND h.order_id=o.order_id ORDER BY revision DESC LIMIT 1) h ON true WHERE o.tenant_id=$1 AND (($2='branch' AND o.branch_id=$3) OR ($2='organization' AND o.organization_id=$3)) AND ($4::uuid IS NULL OR o.order_id=$4) AND ($5::uuid IS NULL OR w.work_id>$5) ORDER BY w.work_id LIMIT $6")
            .bind(actor.tenant_id.value()).bind(input.scope.kind.as_str()).bind(input.scope.id.value()).bind(input.query.order_id).bind(input.query.after).bind(i64::from(input.query.limit)+1).fetch_all(&mut *tx).await.map_err(|_| E::Unavailable)?;
        let mut items = Vec::new();
        for row in rows {
            let bytes: Vec<u8> = row.get("record_json");
            let work_id: Uuid = row.get("work_id");
            let order: OrderRecord = serde_json::from_slice(&bytes).map_err(|_| E::Unavailable)?;
            let work = order
                .work
                .iter()
                .find(|w| w.id == work_id)
                .ok_or(E::Conflict)?;
            let mut value = snapshot(&mut tx, actor, &order, work).await?;
            project(&order, &mut value);
            items.push(value);
        }
        items.sort_by_key(|w| w.id);
        let more = items.len() > input.query.limit as usize;
        items.truncate(input.query.limit as usize);
        let next = if more {
            items.last().map(|w| w.id)
        } else {
            None
        };
        Ok(WorkOrderPage {
            server_available: true,
            items,
            next,
            pending: vec![],
        })
    }
}

fn project(order: &OrderRecord, value: &mut WorkOrderRecord) {
    let active = !matches!(order.state, OrderState::Delivered | OrderState::Cancelled);
    value.can_start = active && value.state == WorkState::Planned;
    value.can_complete = active && value.state == WorkState::InProgress;
}

async fn snapshot(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    order: &OrderRecord,
    work: &OrderWork,
) -> Result<WorkOrderRecord, E> {
    let previous: Option<Vec<u8>> = sqlx::query_scalar("SELECT record_json FROM sync.work_order_history WHERE tenant_id=$1 AND work_id=$2 ORDER BY revision DESC LIMIT 1")
        .bind(actor.tenant_id.value()).bind(work.id).fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?;
    // A previous accepted snapshot is never rebuilt from the current catalog.
    let furniture = if let Some(bytes) = previous {
        let value: WorkOrderRecord = serde_json::from_slice(&bytes).map_err(|_| E::Unavailable)?;
        if value.order_id != order.id
            || value.scope != order.scope
            || value
                .furniture
                .iter()
                .map(|l| l.line_id)
                .collect::<Vec<_>>()
                != work.line_ids
        {
            return Err(E::Conflict);
        }
        value.furniture
    } else {
        manufacture(tx, actor, order, work).await?
    };
    Ok(WorkOrderRecord {
        id: work.id,
        scope: order.scope.clone(),
        organization_id: order.organization_id,
        order_id: order.id,
        order_number: order.number.clone(),
        revision: order.revision,
        number: work.number.clone(),
        customer: order
            .source
            .quotation
            .evaluation
            .customer
            .as_ref()
            .ok_or(E::Invalid)?
            .name
            .clone(),
        state: work.state,
        due_at: work.due_at,
        assignment: work.assignment.clone(),
        furniture,
        note: order.fulfillment_note.clone(),
        can_start: false,
        can_complete: false,
    })
}

async fn manufacture(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    order: &OrderRecord,
    work: &OrderWork,
) -> Result<Vec<WorkOrderFurniture>, E> {
    let source = &order.source.quotation;
    let mut result = Vec::new();
    for id in &work.line_ids {
        let intent = source
            .intent
            .lines
            .iter()
            .find(|l| l.id == *id)
            .ok_or(E::Conflict)?;
        let line = source
            .evaluation
            .lines
            .iter()
            .find(|l| l.id == *id)
            .ok_or(E::Conflict)?;
        let PriceTarget::Furniture(reference) = &intent.configuration.selection.target else {
            return Err(E::Invalid);
        };
        if reference.scope.kind.as_str() != "organization"
            || reference.scope.id.value() != order.organization_id
            || reference.schema_version != 1
            || reference.revision == 0
            || line.quantity == 0
        {
            return Err(E::Invalid);
        }
        let Some(CatalogRevision::Furniture(definition)) = crate::catalog_revision::load(
            tx,
            actor,
            &reference.scope,
            "furniture",
            reference.furniture_id.value(),
            reference.revision,
        )
        .await
        .map_err(|_| E::Unavailable)?
        else {
            return Err(E::Conflict);
        };
        let mut parts = Vec::new();
        for usage in &definition.parts {
            let r = &usage.reference;
            if r.scope != reference.scope
                || r.schema_version != 1
                || r.revision == 0
                || usage.quantity == 0
            {
                return Err(E::Invalid);
            }
            let Some(CatalogRevision::Part(part)) = crate::catalog_revision::load(
                tx,
                actor,
                &r.scope,
                "part",
                r.part_id.value(),
                r.revision,
            )
            .await
            .map_err(|_| E::Unavailable)?
            else {
                return Err(E::Conflict);
            };
            parts.push(WorkOrderPart {
                reference: r.clone(),
                name: part.name,
                quantity: u64::from(usage.quantity)
                    .checked_mul(u64::from(line.quantity))
                    .ok_or(E::Invalid)?,
            });
        }
        if parts.is_empty() || result.iter().any(|l: &WorkOrderFurniture| l.line_id == *id) {
            return Err(E::Conflict);
        }
        result.push(WorkOrderFurniture {
            line_id: *id,
            reference: reference.clone(),
            name: line.name.clone(),
            variant_name: line.variant_name.clone(),
            dimensions: line.dimensions.clone().ok_or(E::Conflict)?,
            color_id: line.color_id,
            color_name: line.color_name.clone(),
            handle_id: line.handle_id,
            handle_name: line.handle_name.clone(),
            quantity: line.quantity,
            parts,
        });
    }
    if result.is_empty() {
        return Err(E::Conflict);
    }
    Ok(result)
}

pub(super) async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    order: &OrderRecord,
    now: UnixMillis,
) -> Result<(), E> {
    for work in &order.work {
        let value = snapshot(tx, actor, order, work).await?;
        let json = serde_json::to_vec(&value).map_err(|_| E::Unavailable)?;
        sqlx::query("INSERT INTO sync.work_order_history VALUES($1,$2,$3,$4,$5,$6,$7)")
            .bind(actor.tenant_id.value())
            .bind(order.organization_id)
            .bind(order.scope.id.value())
            .bind(order.id)
            .bind(work.id)
            .bind(i64::try_from(order.revision).map_err(|_| E::Invalid)?)
            .bind(&json)
            .execute(&mut **tx)
            .await
            .map_err(|_| E::Unavailable)?;
        for scope in [
            order.scope.clone(),
            eitmad_contracts::identity::ScopeRef {
                kind: eitmad_contracts::identity::ScopeKind::parse("organization").expect("scope"),
                id: eitmad_contracts::identity::ScopeId::new(order.organization_id),
            },
        ] {
            crate::operations::append_domain_change(
                tx,
                actor,
                crate::LocalOperationDraft {
                    scope,
                    schema_id: SchemaId::parse(WORK_ORDER_SCHEMA).expect("schema"),
                    schema_version: 1,
                    record_id: RecordId::new(work.id),
                    operation: ChangeOperation::Upsert,
                    change_id: ChangeId::new(Uuid::new_v4()),
                    base_revision: None,
                    idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                    payload: Some(EncodedDomainPayload {
                        schema_id: SchemaId::parse(WORK_ORDER_SCHEMA).expect("schema"),
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
    }
    Ok(())
}

#[async_trait::async_trait]
impl crate::DomainSyncHandler for WorkOrderServer {
    fn descriptor(&self) -> crate::DomainDescriptor {
        crate::DomainDescriptor {
            schema_id: SchemaId::parse(WORK_ORDER_SCHEMA).expect("schema"),
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
        orders::authorize(&mut tx, actor, scope, false, true)
            .await
            .is_ok()
    }
    async fn project_page(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: &AuthenticatedServerSession,
        scope: &ScopeRef,
        changes: Vec<eitmad_contracts::sync::ChangeRecord>,
    ) -> Result<Vec<eitmad_contracts::sync::ChangeRecord>, crate::OperationError> {
        let organization = orders::authorize(tx, actor, scope, false, true)
            .await
            .map_err(|_| crate::OperationError::Denied)?;
        for change in &changes {
            let bytes = STANDARD
                .decode(
                    &change
                        .payload
                        .as_ref()
                        .ok_or(crate::OperationError::Invalid)?
                        .base64,
                )
                .map_err(|_| crate::OperationError::Invalid)?;
            let value: WorkOrderRecord =
                serde_json::from_slice(&bytes).map_err(|_| crate::OperationError::Invalid)?;
            if value.organization_id != organization
                || scope.kind.as_str() == "branch" && value.scope != *scope
            {
                return Err(crate::OperationError::Denied);
            }
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
