use eitmad_contracts::{
    identity::ScopeRef,
    server::{AuthenticatedServerSession, ServerEventId, ServerSubscriptionEvent},
    sync::RecordChangeNotice,
    transport::{CorrelationId, EventCursor, SchemaId, UnixMillis},
};
use eitmad_server_audit::{
    ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append as append_audit,
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
        scope: &ScopeRef,
        schema_id: &SchemaId,
        schema_version: u32,
        resume_after: Option<EventCursor>,
        maximum_events: u32,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<SubscriptionPage, SubscriptionError> {
        let audit = |outcome, redacted_error| {
            ServerAuditEnvelope::from_session(
                session,
                scope.clone(),
                ServerAuditEvent {
                    operation: "eitmad.server.sync.subscription-page.v1",
                    outcome,
                    target_kind: "sync-subscription",
                    target_id: Some(scope.id.value()),
                    correlation_id,
                    causation_id: None,
                    idempotency_key: None,
                    redacted_error,
                    occurred_at: now,
                },
            )
        };
        let limit = usize::try_from(maximum_events).map_err(|_| SubscriptionError::Invalid)?;
        if limit == 0 || limit > eitmad_contracts::sync::MAX_SYNC_BATCH_RECORDS {
            crate::boundary_audit::record(
                &self.pool,
                &audit(
                    ServerAuditOutcome::Invalid,
                    Some("eitmad.error.contract-invalid.v1"),
                ),
            )
            .await
            .map_err(|_| SubscriptionError::Unavailable)?;
            return Err(SubscriptionError::Invalid);
        }
        let handler = match self.registry.get(schema_id, schema_version) {
            Ok(handler) => handler,
            Err(_) => {
                crate::boundary_audit::record(
                    &self.pool,
                    &audit(
                        ServerAuditOutcome::Invalid,
                        Some("eitmad.error.server-client-incompatible.v1"),
                    ),
                )
                .await
                .map_err(|_| SubscriptionError::Unavailable)?;
                return Err(SubscriptionError::Invalid);
            }
        };
        if !handler.authorize(session, scope, SyncIntent::Read).await {
            crate::boundary_audit::record(
                &self.pool,
                &audit(
                    ServerAuditOutcome::Denied,
                    Some("eitmad.error.authorization-denied.v1"),
                ),
            )
            .await
            .map_err(|_| SubscriptionError::Unavailable)?;
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
        append_audit(
            &mut transaction,
            &audit(ServerAuditOutcome::Succeeded, None),
        )
        .await
        .map_err(|_| SubscriptionError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| SubscriptionError::Unavailable)?;
        Ok(SubscriptionPage { events, has_more })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eitmad_contracts::identity::{
        AccountId, DeviceId, ScopeId, ScopeKind, SessionId, TenantId, UserId,
    };
    use eitmad_contracts::sync::SyncMode;
    use uuid::Uuid;

    use super::*;
    use crate::domain::{
        AuthoritativeChangeDraft, CommandSubmission, DomainDescriptor, DomainRegistry,
        DomainSyncHandler, DomainValidationError, LocalOperationDraft,
    };

    struct DenyingHandler;

    #[async_trait::async_trait]
    impl DomainSyncHandler for DenyingHandler {
        fn descriptor(&self) -> DomainDescriptor {
            DomainDescriptor {
                schema_id: schema_id(),
                minimum_schema_version: 1,
                maximum_schema_version: 1,
                mode: SyncMode::LocalFirst,
            }
        }

        async fn authorize(
            &self,
            _session: &AuthenticatedServerSession,
            _scope: &ScopeRef,
            _intent: SyncIntent,
        ) -> bool {
            false
        }

        fn validate_local(
            &self,
            _draft: &LocalOperationDraft,
        ) -> Result<(), DomainValidationError> {
            unreachable!()
        }

        fn execute_command(
            &self,
            _session: &AuthenticatedServerSession,
            _command: &CommandSubmission,
        ) -> Result<AuthoritativeChangeDraft, DomainValidationError> {
            unreachable!()
        }
    }

    fn schema_id() -> SchemaId {
        SchemaId::parse("eitmad.schema.test.notes.v1").unwrap()
    }

    fn scope() -> ScopeRef {
        ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(Uuid::from_u128(1)),
        }
    }

    fn session() -> AuthenticatedServerSession {
        AuthenticatedServerSession {
            session_id: SessionId::new(Uuid::from_u128(2)),
            account_id: AccountId::new(Uuid::from_u128(3)),
            user_id: UserId::new(Uuid::from_u128(4)),
            device_id: DeviceId::new(Uuid::from_u128(5)),
            tenant_id: TenantId::new(Uuid::from_u128(6)),
            issued_at: UnixMillis(1),
            expires_at: UnixMillis(2),
        }
    }

    fn coordinator() -> SyncCoordinator {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgresql://unreachable.invalid/eitmad")
            .unwrap();
        let registry =
            DomainRegistry::new([Arc::new(DenyingHandler) as Arc<dyn DomainSyncHandler>]).unwrap();
        SyncCoordinator { pool, registry }
    }

    #[tokio::test]
    async fn denied_subscription_fails_closed_when_audit_is_unavailable() {
        assert_eq!(
            coordinator()
                .subscription_page(
                    &session(),
                    &scope(),
                    &schema_id(),
                    1,
                    None,
                    1,
                    CorrelationId::new(Uuid::from_u128(7)),
                    UnixMillis(1),
                )
                .await,
            Err(SubscriptionError::Unavailable)
        );
    }

    #[tokio::test]
    async fn invalid_subscription_fails_closed_when_audit_is_unavailable() {
        assert_eq!(
            coordinator()
                .subscription_page(
                    &session(),
                    &scope(),
                    &schema_id(),
                    1,
                    None,
                    0,
                    CorrelationId::new(Uuid::from_u128(7)),
                    UnixMillis(1),
                )
                .await,
            Err(SubscriptionError::Unavailable)
        );
    }
}
