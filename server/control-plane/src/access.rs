use eitmad_contracts::{
    identity::{DeviceId, TenantId},
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_server_audit::{
    ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append as append_audit,
};
use sqlx::PgPool;

use crate::database::tenant_transaction;

const OWNER_RELATION: &str = "eitmad.relation.organization.owner.v1";
const MEMBER_RELATION: &str = "eitmad.relation.organization.member.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessRequirement {
    TenantMember,
    TenantOwner,
    TenantPermission(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AccessError {
    #[error("server access is denied")]
    Denied,
    #[error("server access authority is unavailable")]
    Unavailable,
}

#[derive(Clone)]
pub struct ServerAccessService {
    pool: PgPool,
}

impl ServerAccessService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Checks an exact tenant relationship and optional registered device.
    ///
    /// # Errors
    ///
    /// Returns a denial for cross-tenant, missing relationship, or unknown
    /// target-device access. Returns unavailable for storage failure.
    pub async fn authorize(
        &self,
        actor: &AuthenticatedServerSession,
        tenant_id: TenantId,
        target_device_id: Option<DeviceId>,
        requirement: AccessRequirement,
    ) -> Result<(), AccessError> {
        if actor.tenant_id != tenant_id {
            return Err(AccessError::Denied);
        }
        let mut transaction = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| AccessError::Unavailable)?;
        let relations = match requirement {
            AccessRequirement::TenantMember => vec![MEMBER_RELATION, OWNER_RELATION],
            AccessRequirement::TenantOwner => vec![OWNER_RELATION],
            AccessRequirement::TenantPermission(permission) => vec![permission],
        };
        let authorized: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM control.relationship_tuples
                WHERE tenant_id = $1 AND subject_principal_id = $2
                  AND relation = ANY($3) AND object_kind = 'tenant' AND object_id = $1
             )",
        )
        .bind(actor.tenant_id.value())
        .bind(actor.user_id.value())
        .bind(&relations)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| AccessError::Unavailable)?;
        if !authorized {
            return Err(AccessError::Denied);
        }
        if let Some(device_id) = target_device_id {
            let registered: bool = sqlx::query_scalar(
                "SELECT EXISTS (
                    SELECT 1 FROM control.account_devices
                    WHERE tenant_id = $1 AND device_id = $2 AND revoked_at IS NULL
                 )",
            )
            .bind(actor.tenant_id.value())
            .bind(device_id.value())
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| AccessError::Unavailable)?;
            if !registered {
                return Err(AccessError::Denied);
            }
        }
        transaction
            .commit()
            .await
            .map_err(|_| AccessError::Unavailable)
    }

    /// Appends one redacted server-plane audit outcome.
    ///
    /// # Errors
    ///
    /// Returns unavailable when the append-only audit store cannot commit.
    pub async fn audit(&self, entry: &ServerAuditEnvelope<'_>) -> Result<(), AccessError> {
        let mut transaction = tenant_transaction(&self.pool, entry.actor.tenant_id)
            .await
            .map_err(|_| AccessError::Unavailable)?;
        append_audit(&mut transaction, entry)
            .await
            .map_err(|_| AccessError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AccessError::Unavailable)
    }

    /// Revokes all active sessions for one tenant device with owner
    /// authorization and an audit record in the same transaction.
    ///
    /// # Errors
    ///
    /// Returns denied for a non-owner or foreign device and unavailable when
    /// the transaction cannot commit.
    pub async fn revoke_device_sessions(
        &self,
        actor: &AuthenticatedServerSession,
        device_id: DeviceId,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), AccessError> {
        let mut transaction = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| AccessError::Unavailable)?;
        let owner: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM control.relationship_tuples
                WHERE tenant_id = $1 AND subject_principal_id = $2
                  AND relation = $3 AND object_kind = 'tenant' AND object_id = $1
             )",
        )
        .bind(actor.tenant_id.value())
        .bind(actor.user_id.value())
        .bind(OWNER_RELATION)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| AccessError::Unavailable)?;
        let device_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM control.account_devices
                WHERE tenant_id = $1 AND device_id = $2
             )",
        )
        .bind(actor.tenant_id.value())
        .bind(device_id.value())
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| AccessError::Unavailable)?;
        if !owner || !device_exists {
            return Err(AccessError::Denied);
        }
        sqlx::query(
            "UPDATE control.sessions SET revoked_at = $3
             WHERE tenant_id = $1 AND device_id = $2 AND revoked_at IS NULL",
        )
        .bind(actor.tenant_id.value())
        .bind(device_id.value())
        .bind(now.0)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AccessError::Unavailable)?;
        append_audit(
            &mut transaction,
            &ServerAuditEnvelope::for_tenant_session(
                actor,
                ServerAuditEvent {
                    operation: "eitmad.admin.device-sessions.revoke.v1",
                    outcome: ServerAuditOutcome::Succeeded,
                    target_kind: "device_sessions",
                    target_id: Some(device_id.value()),
                    correlation_id,
                    causation_id: None,
                    idempotency_key: None,
                    redacted_error: None,
                    occurred_at: now,
                },
            ),
        )
        .await
        .map_err(|_| AccessError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AccessError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_requirements_keep_members_and_owners_distinct() {
        assert_ne!(
            AccessRequirement::TenantMember,
            AccessRequirement::TenantOwner
        );
        assert_eq!(ServerAuditOutcome::Denied.as_str(), "denied");
    }
}
