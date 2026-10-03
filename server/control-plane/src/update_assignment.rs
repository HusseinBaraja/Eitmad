use eitmad_contracts::{
    identity::{DeviceId, TenantId},
    server::{EffectiveUpdateAssignment, UpdateAssignmentSource, UpdateChannelId},
};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

use crate::database::tenant_transaction;

#[derive(Clone)]
pub struct UpdateAssignmentService {
    pool: PgPool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UpdateAssignmentError {
    #[error("update assignment is invalid")]
    Invalid,
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
}

fn global_default() -> EffectiveUpdateAssignment {
    EffectiveUpdateAssignment {
        channel: UpdateChannelId::parse("stable").expect("stable is a valid channel"),
        source: UpdateAssignmentSource::GlobalDefault,
        revision: 0,
    }
}
