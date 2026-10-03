//! Customer contact validation and exact branch authorization for server sync.

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use eitmad_contracts::{
    customer::CustomerSyncPayload,
    identity::ScopeRef,
    server::AuthenticatedServerSession,
    sync::{ChangeOperation, ConflictRecord, RecordId, SyncMode},
    transport::SchemaId,
};
use sqlx::PgPool;

use crate::{
    database::tenant_transaction,
    domain::{
        AuthoritativeChangeDraft, DomainDescriptor, DomainSyncHandler, DomainValidationError,
        LocalOperationDraft, SyncIntent,
    },
};

const SCHEMA: &str = "eitmad.schema.customer.v1";

pub struct CustomerSyncHandler {
    pool: PgPool,
}

impl CustomerSyncHandler {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DomainSyncHandler for CustomerSyncHandler {
    fn descriptor(&self) -> DomainDescriptor {
        DomainDescriptor {
            schema_id: SchemaId::parse(SCHEMA).expect("static customer schema"),
            minimum_schema_version: 1,
            maximum_schema_version: 1,
            mode: SyncMode::LocalFirst,
        }
    }

    async fn authorize(
        &self,
        session: &AuthenticatedServerSession,
        scope: &ScopeRef,
        _intent: SyncIntent,
    ) -> bool {
        if scope.kind.as_str() != "branch" {
            return false;
        }
        let Ok(mut transaction) = tenant_transaction(&self.pool, session.tenant_id).await else {
            return false;
        };
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1 FROM control.branches b
                JOIN control.relationship_tuples r
                  ON r.tenant_id = b.tenant_id
                 AND r.subject_principal_id = $2 AND r.subject_kind = 'user'
                WHERE b.tenant_id = $1 AND b.branch_id = $3
                  AND ((r.relation = 'eitmad.relation.organization.manager.v1'
                        AND r.object_kind = 'organization'
                        AND r.object_id = b.organization_id)
                    OR (r.relation = 'eitmad.relation.organization.receptionist.v1'
                        AND r.object_kind = 'branch'
                        AND r.object_id = b.branch_id))
            )",
        )
        .bind(session.tenant_id.value())
        .bind(session.user_id.value())
        .bind(scope.id.value())
        .fetch_one(&mut *transaction)
        .await;
        matches!(result, Ok(true))
    }

    fn validate_local(&self, draft: &LocalOperationDraft) -> Result<(), DomainValidationError> {
        if draft.schema_id.as_str() != SCHEMA
            || draft.schema_version != 1
            || draft.scope.kind.as_str() != "branch"
            || draft.operation != ChangeOperation::Upsert
        {
            return Err(DomainValidationError::Invalid);
        }
        let payload = draft
            .payload
            .as_ref()
            .ok_or(DomainValidationError::Invalid)?;
        if payload.schema_id != draft.schema_id || payload.schema_version != 1 {
            return Err(DomainValidationError::Invalid);
        }
        let decoded = STANDARD
            .decode(&payload.base64)
            .map_err(|_| DomainValidationError::Invalid)?;
        let customer: CustomerSyncPayload =
            serde_json::from_slice(&decoded).map_err(|_| DomainValidationError::Invalid)?;
        if RecordId::new(customer.customer_id.value()) != draft.record_id
            || customer.revision == 0
            || draft.base_revision.unwrap_or(0).checked_add(1) != Some(customer.revision)
        {
            return Err(DomainValidationError::Invalid);
        }
        Ok(())
    }

    fn resolve_conflict(
        &self,
        _conflict: &ConflictRecord,
    ) -> Result<Option<AuthoritativeChangeDraft>, DomainValidationError> {
        // A later domain decision must inspect the common ancestor and changed
        // fields. Until then, preserve both edits as an open conflict.
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::{
        customer::{CustomerId, CustomerName, CustomerPhone},
        identity::{ScopeId, ScopeKind},
        sync::EncodedDomainPayload,
        transport::IdempotencyKey,
    };
    use uuid::Uuid;

    #[tokio::test]
    async fn rejects_payload_with_a_different_customer_identity() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgresql://unreachable.invalid/eitmad")
            .unwrap();
        let handler = CustomerSyncHandler::new(pool);
        let customer_id = CustomerId::new(Uuid::new_v4());
        let payload = CustomerSyncPayload {
            customer_id,
            name: CustomerName::parse("عميل اختباري").unwrap(),
            phone: CustomerPhone::parse("+967777123456").unwrap(),
            address: None,
            notes: None,
            revision: 1,
        };
        let mut draft = LocalOperationDraft {
            change_id: eitmad_contracts::sync::ChangeId::new(Uuid::new_v4()),
            scope: ScopeRef {
                kind: ScopeKind::parse("branch").unwrap(),
                id: ScopeId::new(Uuid::new_v4()),
            },
            schema_id: SchemaId::parse(SCHEMA).unwrap(),
            schema_version: 1,
            record_id: RecordId::new(customer_id.value()),
            operation: ChangeOperation::Upsert,
            base_revision: None,
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            payload: Some(EncodedDomainPayload {
                schema_id: SchemaId::parse(SCHEMA).unwrap(),
                schema_version: 1,
                base64: STANDARD.encode(serde_json::to_vec(&payload).unwrap()),
            }),
        };
        assert!(handler.validate_local(&draft).is_ok());
        draft.record_id = RecordId::new(Uuid::new_v4());
        assert_eq!(
            handler.validate_local(&draft),
            Err(DomainValidationError::Invalid)
        );
        draft.record_id = RecordId::new(customer_id.value());
        draft.operation = ChangeOperation::Tombstone;
        assert_eq!(
            handler.validate_local(&draft),
            Err(DomainValidationError::Invalid)
        );
    }
}
