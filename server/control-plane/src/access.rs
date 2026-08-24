use eitmad_contracts::{
    identity::{DeviceId, PrincipalId, TenantId},
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use sqlx::PgPool;

use crate::{
    audit::{self, AuditEntry},
    database::tenant_transaction,
};

const OWNER_RELATION: &str = "eitmad.relation.organization.owner.v1";
const MEMBER_RELATION: &str = "eitmad.relation.organization.member.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessRequirement {
    TenantMember,
    TenantOwner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerAuditOutcome {
    Succeeded,
    Denied,
    Invalid,
    Failed,
}

impl ServerAuditOutcome {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Denied => "denied",
            Self::Invalid => "invalid",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AccessError {
    #[error("server access is denied")]
    Denied,
    #[error("server access authority is unavailable")]
    Unavailable,
}

#[derive(Clone, Copy, Debug)]
pub struct ServerAuditEntry<'a> {
    pub operation: &'a str,
    pub outcome: ServerAuditOutcome,
    pub target_kind: &'a str,
    pub redacted_error: Option<&'a str>,
    pub correlation_id: CorrelationId,
    pub now: UnixMillis,
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
            AccessRequirement::TenantMember => [MEMBER_RELATION, OWNER_RELATION].as_slice(),
            AccessRequirement::TenantOwner => [OWNER_RELATION].as_slice(),
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
        .bind(relations)
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
    pub async fn audit(
        &self,
        actor: &AuthenticatedServerSession,
        entry: &ServerAuditEntry<'_>,
    ) -> Result<(), AccessError> {
        let mut transaction = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| AccessError::Unavailable)?;
        audit::append(
            &mut transaction,
            AuditEntry {
                tenant_id: actor.tenant_id,
                session_id: actor.session_id,
                device_id: Some(actor.device_id),
                principal_id: PrincipalId::new(actor.user_id.value()),
                operation: entry.operation,
                outcome: entry.outcome.as_str(),
                target_kind: entry.target_kind,
                correlation_id: entry.correlation_id,
                redacted_error: entry.redacted_error,
                now: entry.now,
            },
        )
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
        audit::append(
            &mut transaction,
            AuditEntry {
                tenant_id: actor.tenant_id,
                session_id: actor.session_id,
                device_id: Some(actor.device_id),
                principal_id: PrincipalId::new(actor.user_id.value()),
                operation: "eitmad.admin.device-sessions.revoke.v1",
                outcome: "succeeded",
                target_kind: "device_sessions",
                correlation_id,
                redacted_error: None,
                now,
            },
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
