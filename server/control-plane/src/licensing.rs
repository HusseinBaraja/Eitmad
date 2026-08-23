use std::collections::BTreeSet;

use eitmad_contracts::{
    identity::{PrincipalId, TenantId},
    server::{AuthenticatedServerSession, EntitlementId, LicenseId, LicenseState, LicenseStatus},
    transport::{CorrelationId, UnixMillis},
};
use sqlx::{PgPool, Row as _};

use crate::{
    audit::{self, AuditEntry},
    database::tenant_transaction,
    identity::require_tenant_owner,
};

#[derive(Clone)]
pub struct LicenseService {
    pool: PgPool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LicenseDecision {
    Granted,
    Grace,
    Denied,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LicenseError {
    #[error("license entitlement is required")]
    Required,
    #[error("license authority is unavailable")]
    Unavailable,
    #[error("license state is invalid")]
    Invalid,
}

impl LicenseService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Reads the effective tenant license state.
    ///
    /// # Errors
    ///
    /// Returns a sanitized availability or schema error.
    pub async fn state(&self, tenant_id: TenantId) -> Result<LicenseState, LicenseError> {
        let mut transaction = tenant_transaction(&self.pool, tenant_id)
            .await
            .map_err(|_| LicenseError::Unavailable)?;
        let row = sqlx::query(
            "SELECT license_id, provider_revision, status, valid_until, grace_until
             FROM control.licenses WHERE tenant_id = $1",
        )
        .bind(tenant_id.value())
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| LicenseError::Unavailable)?;
        let values = sqlx::query(
            "SELECT entitlement FROM control.license_entitlements
             WHERE tenant_id = $1 ORDER BY entitlement",
        )
        .bind(tenant_id.value())
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| LicenseError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| LicenseError::Unavailable)?;
        let status_value: String = row.get("status");
        let status = parse_status(&status_value)?;
        let entitlements = values
            .into_iter()
            .map(|value| EntitlementId::parse(value.get::<String, _>("entitlement")))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| LicenseError::Invalid)?;
        Ok(LicenseState {
            license_id: LicenseId::new(row.get("license_id")),
            tenant_id,
            provider_revision: row.get("provider_revision"),
            status,
            valid_until: row.get::<Option<i64>, _>("valid_until").map(UnixMillis),
            grace_until: row.get::<Option<i64>, _>("grace_until").map(UnixMillis),
            entitlements,
        })
    }

    /// Evaluates one entitlement without blocking core recovery operations.
    ///
    /// # Errors
    ///
    /// Returns [`LicenseError::Required`] when the entitlement is unavailable.
    pub async fn require(
        &self,
        tenant_id: TenantId,
        entitlement: &EntitlementId,
        now: UnixMillis,
    ) -> Result<LicenseDecision, LicenseError> {
        let state = self.state(tenant_id).await?;
        if !state.entitlements.contains(entitlement) {
            return Err(LicenseError::Required);
        }
        match effective_status(&state, now) {
            LicenseStatus::Active => Ok(LicenseDecision::Granted),
            LicenseStatus::Grace => Ok(LicenseDecision::Grace),
            _ => Err(LicenseError::Required),
        }
    }

    /// Persists one validated snapshot from a future license-provider adapter.
    ///
    /// # Errors
    ///
    /// Denies non-owners and rejects cross-tenant, duplicate, or invalid state.
    pub async fn record_provider_state(
        &self,
        actor: &AuthenticatedServerSession,
        state: &LicenseState,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), LicenseError> {
        validate_provider_state(actor, state)?;
        let mut transaction = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| LicenseError::Unavailable)?;
        require_tenant_owner(&mut transaction, actor)
            .await
            .map_err(|error| match error {
                crate::identity::IdentityError::Denied => LicenseError::Required,
                _ => LicenseError::Unavailable,
            })?;
        let updated = sqlx::query(
            "UPDATE control.licenses
             SET license_id = $2, provider_revision = $3, status = $4,
                 valid_until = $5, grace_until = $6, updated_at = $7
             WHERE tenant_id = $1",
        )
        .bind(actor.tenant_id.value())
        .bind(state.license_id.value())
        .bind(&state.provider_revision)
        .bind(status_name(state.status))
        .bind(state.valid_until.map(|value| value.0))
        .bind(state.grace_until.map(|value| value.0))
        .bind(now.0)
        .execute(&mut *transaction)
        .await
        .map_err(|_| LicenseError::Unavailable)?;
        if updated.rows_affected() != 1 {
            return Err(LicenseError::Invalid);
        }
        sqlx::query("DELETE FROM control.license_entitlements WHERE tenant_id = $1")
            .bind(actor.tenant_id.value())
            .execute(&mut *transaction)
            .await
            .map_err(|_| LicenseError::Unavailable)?;
        for entitlement in &state.entitlements {
            sqlx::query(
                "INSERT INTO control.license_entitlements (tenant_id, entitlement)
                 VALUES ($1, $2)",
            )
            .bind(actor.tenant_id.value())
            .bind(entitlement.as_str())
            .execute(&mut *transaction)
            .await
            .map_err(|_| LicenseError::Unavailable)?;
        }
        audit::append(
            &mut transaction,
            AuditEntry {
                tenant_id: actor.tenant_id,
                session_id: actor.session_id,
                device_id: Some(actor.device_id),
                principal_id: PrincipalId::new(actor.user_id.value()),
                operation: "eitmad.server.license.record-provider-state.v1",
                outcome: "succeeded",
                target_kind: "license",
                correlation_id,
                now,
            },
        )
        .await
        .map_err(|_| LicenseError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| LicenseError::Unavailable)
    }
}

fn validate_provider_state(
    actor: &AuthenticatedServerSession,
    state: &LicenseState,
) -> Result<(), LicenseError> {
    let unique = state
        .entitlements
        .iter()
        .map(EntitlementId::as_str)
        .collect::<BTreeSet<_>>();
    if state.tenant_id != actor.tenant_id
        || state.provider_revision.trim().is_empty()
        || state.provider_revision.len() > 256
        || state.entitlements.len() > 256
        || unique.len() != state.entitlements.len()
    {
        return Err(LicenseError::Invalid);
    }
    Ok(())
}

const fn status_name(status: LicenseStatus) -> &'static str {
    match status {
        LicenseStatus::Active => "active",
        LicenseStatus::Grace => "grace",
        LicenseStatus::Expired => "expired",
        LicenseStatus::Suspended => "suspended",
        LicenseStatus::Unknown => "unknown",
    }
}

fn effective_status(state: &LicenseState, now: UnixMillis) -> LicenseStatus {
    match state.status {
        LicenseStatus::Active
            if state
                .valid_until
                .is_none_or(|valid_until| valid_until.0 > now.0) =>
        {
            LicenseStatus::Active
        }
        LicenseStatus::Active | LicenseStatus::Grace
            if state
                .grace_until
                .is_some_and(|grace_until| grace_until.0 > now.0) =>
        {
            LicenseStatus::Grace
        }
        LicenseStatus::Suspended => LicenseStatus::Suspended,
        LicenseStatus::Unknown => LicenseStatus::Unknown,
        _ => LicenseStatus::Expired,
    }
}

fn parse_status(value: &str) -> Result<LicenseStatus, LicenseError> {
    match value {
        "active" => Ok(LicenseStatus::Active),
        "grace" => Ok(LicenseStatus::Grace),
        "expired" => Ok(LicenseStatus::Expired),
        "suspended" => Ok(LicenseStatus::Suspended),
        "unknown" => Ok(LicenseStatus::Unknown),
        _ => Err(LicenseError::Invalid),
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn state(status: LicenseStatus, valid: Option<i64>, grace: Option<i64>) -> LicenseState {
        LicenseState {
            license_id: LicenseId::new(Uuid::from_u128(1)),
            tenant_id: TenantId::new(Uuid::from_u128(2)),
            provider_revision: "synthetic".to_owned(),
            status,
            valid_until: valid.map(UnixMillis),
            grace_until: grace.map(UnixMillis),
            entitlements: Vec::new(),
        }
    }

    #[test]
    fn expiry_uses_grace_then_denies() {
        assert_eq!(
            effective_status(
                &state(LicenseStatus::Active, Some(10), Some(20)),
                UnixMillis(5)
            ),
            LicenseStatus::Active
        );
        assert_eq!(
            effective_status(
                &state(LicenseStatus::Active, Some(10), Some(20)),
                UnixMillis(15)
            ),
            LicenseStatus::Grace
        );
        assert_eq!(
            effective_status(
                &state(LicenseStatus::Active, Some(10), Some(20)),
                UnixMillis(25)
            ),
            LicenseStatus::Expired
        );
    }

    #[test]
    fn suspension_never_becomes_grace() {
        assert_eq!(
            effective_status(
                &state(LicenseStatus::Suspended, None, Some(20)),
                UnixMillis(1)
            ),
            LicenseStatus::Suspended
        );
    }
}
