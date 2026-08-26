use eitmad_contracts::{
    server::{AuthenticatedServerSession, ServerEventId, ServerSubscriptionEvent},
    sync::RecordChangeNotice,
    transport::{EventCursor, SchemaId},
};
use sqlx::Row as _;

use crate::{database::tenant_transaction, domain::SyncIntent, operations::SyncCoordinator};

pub(crate) async fn append_event(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: eitmad_contracts::identity::ScopeRef,
    schema_id: SchemaId,
    event: serde_json::Value,
    now: eitmad_contracts::transport::UnixMillis,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sync.subscription_events
            (event_id, tenant_id, scope_kind, scope_id, schema_id, event_json, occurred_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(tenant_id.value())
    .bind(scope.kind.as_str())
    .bind(scope.id.value())
    .bind(schema_id.as_str())
    .bind(event)
    .bind(now.0)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionPage {
    pub events: Vec<ServerSubscriptionEvent>,
    pub has_more: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SubscriptionError {
    #[error("subscription is denied")]
    Denied,
    #[error("subscription cursor is not retained")]
    ResyncRequired,
    #[error("subscription request is invalid")]
    Invalid,
    #[error("subscription authority is unavailable")]
    Unavailable,
}

impl SyncCoordinator {
    /// Reads an authorized, resumable page of durable subscription events.
    ///
    /// # Errors
    ///
    /// Denies unauthorized delivery and rejects unknown resume cursors.
    pub async fn subscription_page(
        &self,
        session: &AuthenticatedServerSession,
        scope: &eitmad_contracts::identity::ScopeRef,
        schema_id: &SchemaId,
        schema_version: u32,
        resume_after: Option<EventCursor>,
        maximum_events: u32,
    ) -> Result<SubscriptionPage, SubscriptionError> {
        let limit = usize::try_from(maximum_events).map_err(|_| SubscriptionError::Invalid)?;
        if limit == 0 || limit > eitmad_contracts::sync::MAX_SYNC_BATCH_RECORDS {
            return Err(SubscriptionError::Invalid);
        }
        let handler = self
            .registry
            .get(schema_id, schema_version)
            .map_err(|_| SubscriptionError::Invalid)?;
        if !handler.authorize(session, scope, SyncIntent::Read).await {
            return Err(SubscriptionError::Denied);
        }
        let mut transaction = tenant_transaction(&self.pool, session.tenant_id)
            .await
            .map_err(|_| SubscriptionError::Unavailable)?;
        let cursor = if let Some(resume_after) = resume_after {
            sqlx::query_scalar::<_, i64>(
                "SELECT cursor FROM sync.subscription_events
                 WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
                   AND schema_id = $4 AND event_id = $5",
            )
            .bind(session.tenant_id.value())
            .bind(scope.kind.as_str())
            .bind(scope.id.value())
            .bind(schema_id.as_str())
            .bind(resume_after.value())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| SubscriptionError::Unavailable)?
            .ok_or(SubscriptionError::ResyncRequired)?
        } else {
            0
        };
        let rows = sqlx::query(
            "SELECT cursor, event_id, event_json, occurred_at
             FROM sync.subscription_events
             WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
               AND schema_id = $4 AND cursor > $5
             ORDER BY cursor LIMIT $6",
        )
        .bind(session.tenant_id.value())
        .bind(scope.kind.as_str())
        .bind(scope.id.value())
        .bind(schema_id.as_str())
        .bind(cursor)
        .bind(i64::try_from(limit + 1).map_err(|_| SubscriptionError::Invalid)?)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| SubscriptionError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| SubscriptionError::Unavailable)?;
        let has_more = rows.len() > limit;
        let events = rows
            .into_iter()
            .take(limit)
            .map(|row| {
                let event_id = row.get::<uuid::Uuid, _>("event_id");
                let change = serde_json::from_value::<RecordChangeNotice>(row.get("event_json"))
                    .map_err(|_| SubscriptionError::Unavailable)?;
                Ok(ServerSubscriptionEvent {
                    event_id: ServerEventId::new(event_id),
                    cursor: EventCursor::new(event_id),
                    occurred_at: eitmad_contracts::transport::UnixMillis(row.get("occurred_at")),
                    change,
                })
            })
            .collect::<Result<Vec<_>, SubscriptionError>>()?;
        Ok(SubscriptionPage { events, has_more })
    }
}
