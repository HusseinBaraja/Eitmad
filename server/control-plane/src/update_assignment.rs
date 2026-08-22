use eitmad_contracts::{
    identity::{DeviceId, PrincipalId, TenantId},
    server::{
        AuthenticatedServerSession, EffectiveUpdateAssignment, UpdateAssignmentSource,
        UpdateChannelId,
    },
    transport::UnixMillis,
};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

use crate::{
    audit::{self, AuditEntry},
    database::tenant_transaction,
    identity::{IdentityError, require_tenant_owner},
};

#[derive(Clone)]
pub struct UpdateAssignmentService {
    pool: PgPool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UpdateAssignmentError {
    #[error("update assignment is invalid")]
    Invalid,
    #[error("update assignment is denied")]
    Denied,
    #[error("update assignment authority is unavailable")]
    Unavailable,
}

impl UpdateAssignmentService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Resolves device override, tenant default, then global stable.
    ///
    /// # Errors
    ///
    /// Returns a sanitized storage or identifier error.
    pub async fn effective(
        &self,
        tenant_id: TenantId,
        device_id: DeviceId,
    ) -> Result<EffectiveUpdateAssignment, UpdateAssignmentError> {
        let mut transaction = tenant_transaction(&self.pool, tenant_id)
            .await
            .map_err(|_| UpdateAssignmentError::Unavailable)?;
        let row = sqlx::query(
            "SELECT assignment_kind, channel, revision
             FROM control.update_assignments
             WHERE tenant_id = $1
               AND ((assignment_kind = 'device' AND device_id = $2)
                    OR (assignment_kind = 'tenant' AND device_id = $3))
             ORDER BY CASE assignment_kind WHEN 'device' THEN 0 ELSE 1 END
             LIMIT 1",
        )
        .bind(tenant_id.value())
        .bind(device_id.value())
        .bind(Uuid::nil())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| UpdateAssignmentError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| UpdateAssignmentError::Unavailable)?;
        let Some(row) = row else {
            return Ok(global_default());
        };
        let assignment_kind: String = row.get("assignment_kind");
        Ok(EffectiveUpdateAssignment {
            channel: UpdateChannelId::parse(row.get::<String, _>("channel"))
                .map_err(|_| UpdateAssignmentError::Invalid)?,
            source: if assignment_kind == "device" {
                UpdateAssignmentSource::DeviceOverride
            } else {
                UpdateAssignmentSource::TenantDefault
            },
            revision: u64::try_from(row.get::<i64, _>("revision"))
                .map_err(|_| UpdateAssignmentError::Invalid)?,
        })
    }

    /// Assigns the tenant default update channel.
    ///
    /// # Errors
    ///
    /// Denies non-owners and returns a sanitized persistence error.
    pub async fn assign_tenant(
        &self,
        actor: &AuthenticatedServerSession,
        channel: &UpdateChannelId,
        now: UnixMillis,
    ) -> Result<EffectiveUpdateAssignment, UpdateAssignmentError> {
        self.assign(actor, None, channel, now).await
    }

    /// Assigns an update channel override to one registered tenant device.
    ///
    /// # Errors
    ///
    /// Denies non-owners and rejects a device outside the tenant.
    pub async fn assign_device(
        &self,
        actor: &AuthenticatedServerSession,
        device_id: DeviceId,
        channel: &UpdateChannelId,
        now: UnixMillis,
    ) -> Result<EffectiveUpdateAssignment, UpdateAssignmentError> {
        self.assign(actor, Some(device_id), channel, now).await
    }

    async fn assign(
        &self,
        actor: &AuthenticatedServerSession,
        device_id: Option<DeviceId>,
        channel: &UpdateChannelId,
        now: UnixMillis,
    ) -> Result<EffectiveUpdateAssignment, UpdateAssignmentError> {
        let mut transaction = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| UpdateAssignmentError::Unavailable)?;
        require_tenant_owner(&mut transaction, actor)
            .await
            .map_err(|error| match error {
                IdentityError::Denied => UpdateAssignmentError::Denied,
                _ => UpdateAssignmentError::Unavailable,
            })?;
        let assignment_kind = if device_id.is_some() {
            "device"
        } else {
            "tenant"
        };
        let assignment_device = device_id.map_or_else(Uuid::nil, DeviceId::value);
        if let Some(device_id) = device_id {
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
            .map_err(|_| UpdateAssignmentError::Unavailable)?;
            if !registered {
                return Err(UpdateAssignmentError::Invalid);
            }
        }
        let revision: i64 = sqlx::query_scalar(
            "INSERT INTO control.update_assignments
                (tenant_id, assignment_kind, device_id, channel, revision, updated_at)
             VALUES ($1, $2, $3, $4, 1, $5)
             ON CONFLICT (tenant_id, assignment_kind, device_id)
             DO UPDATE SET channel = EXCLUDED.channel,
                           revision = control.update_assignments.revision + 1,
                           updated_at = EXCLUDED.updated_at
             RETURNING revision",
        )
        .bind(actor.tenant_id.value())
        .bind(assignment_kind)
        .bind(assignment_device)
        .bind(channel.as_str())
        .bind(now.0)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| UpdateAssignmentError::Unavailable)?;
        audit::append(
            &mut transaction,
            AuditEntry {
                tenant_id: actor.tenant_id,
                session_id: actor.session_id,
                device_id: Some(actor.device_id),
                principal_id: PrincipalId::new(actor.user_id.value()),
                operation: "eitmad.server.update-channel.assign.v1",
                outcome: "succeeded",
                target_kind: assignment_kind,
                now,
            },
        )
        .await
        .map_err(|_| UpdateAssignmentError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| UpdateAssignmentError::Unavailable)?;
        Ok(EffectiveUpdateAssignment {
            channel: channel.clone(),
            source: if device_id.is_some() {
                UpdateAssignmentSource::DeviceOverride
            } else {
                UpdateAssignmentSource::TenantDefault
            },
            revision: u64::try_from(revision).map_err(|_| UpdateAssignmentError::Invalid)?,
        })
    }
}

fn global_default() -> EffectiveUpdateAssignment {
    EffectiveUpdateAssignment {
        channel: UpdateChannelId::parse("stable").expect("stable is a valid channel"),
        source: UpdateAssignmentSource::GlobalDefault,
        revision: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_fallback_is_stable_and_revision_zero() {
        let assignment = global_default();
        assert_eq!(assignment.channel.as_str(), "stable");
        assert_eq!(assignment.source, UpdateAssignmentSource::GlobalDefault);
        assert_eq!(assignment.revision, 0);
    }
}
