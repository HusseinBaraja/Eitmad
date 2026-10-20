use super::*;
use crate::authentication::{
    canonical_username, decode_device_key, register_device, verify_device_proof, verify_password,
};
use eitmad_contracts::{
    identity::{AccountId, OrganizationId, ScopeId},
    server::LoginRequest,
};

impl PairingService {
    /// Enrolls a client-generated key once through the private local server CLI.
    /// Never expose this operation as a network or ordinary sign-in route.
    /// # Errors
    /// Requires the active Owner password, key possession, and an unclaimed workshop.
    pub async fn enroll_first_desktop(
        &self,
        request: &LoginRequest,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), PairingError> {
        let tenant: Uuid =
            sqlx::query_scalar("SELECT tenant_id FROM control.tenants WHERE tenant_code=$1")
                .bind(request.tenant_code.as_str())
                .fetch_optional(&self.pool)
                .await?
                .ok_or(PairingError::Denied)?;
        let tenant = TenantId::new(tenant);
        let mut tx = tenant_transaction(&self.pool, tenant).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,19))")
            .bind(tenant.value().to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("SAVEPOINT first_desktop")
            .execute(&mut *tx)
            .await?;
        let result = enroll(&mut tx, tenant, request, now).await;
        if result == Err(PairingError::Unavailable) {
            return result;
        }
        if result.is_err() {
            sqlx::query("ROLLBACK TO SAVEPOINT first_desktop")
                .execute(&mut *tx)
                .await?;
        }
        audit(
            &mut tx,
            ServerAuditActor::system(tenant),
            Some(request.device_id.value()),
            "eitmad.server.pairing.first-desktop.v1",
            result.is_ok(),
            correlation,
            now,
        )
        .await?;
        tx.commit().await?;
        result
    }
}

async fn enroll(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    request: &LoginRequest,
    now: UnixMillis,
) -> Result<(), PairingError> {
    let claimed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.first_desktop_claims WHERE tenant_id=$1) OR EXISTS(SELECT 1 FROM control.account_devices WHERE tenant_id=$1)")
        .bind(tenant.value()).fetch_one(&mut **tx).await?;
    if claimed {
        return Err(PairingError::Denied);
    }
    let username = canonical_username(&request.username).map_err(|_| PairingError::Denied)?;
    let row = sqlx::query("SELECT a.account_id,a.password_hash FROM control.accounts a JOIN control.relationship_tuples r ON r.tenant_id=a.tenant_id AND r.subject_principal_id=a.user_id WHERE a.tenant_id=$1 AND a.canonical_username=$2 AND a.status='active' AND (a.locked_until IS NULL OR a.locked_until<=$3) AND r.relation='eitmad.relation.organization.owner.v1' AND r.object_kind='tenant' AND r.object_id=$1 FOR UPDATE OF a FOR SHARE OF r")
        .bind(tenant.value()).bind(username).bind(now.0).fetch_optional(&mut **tx).await?.ok_or(PairingError::Denied)?;
    verify_password(&request.password, &row.get::<String, _>("password_hash"))
        .map_err(|_| PairingError::Denied)?;
    let key = decode_device_key(&request.device_public_key).map_err(|_| PairingError::Denied)?;
    let proof = request.device_proof.as_ref().ok_or(PairingError::Denied)?;
    if proof.device_id != request.device_id || request.device_id.value().is_nil() {
        return Err(PairingError::Denied);
    }
    verify_device_proof(proof, &key, now).map_err(|_| PairingError::Denied)?;
    let scopes = sqlx::query("SELECT organization_id,branch_id FROM control.branches WHERE tenant_id=$1 ORDER BY branch_id LIMIT 2")
        .bind(tenant.value()).fetch_all(&mut **tx).await?;
    if scopes.len() != 1 {
        return Err(PairingError::Denied);
    }
    let organization = OrganizationId::new(scopes[0].get("organization_id"));
    let branch = ScopeId::new(scopes[0].get("branch_id"));
    register_device(
        tx,
        tenant,
        AccountId::new(row.get("account_id")),
        request.device_id,
        &key,
        &request.device_label,
        now,
    )
    .await
    .map_err(|error| match error {
        crate::AuthenticationError::Unavailable => PairingError::Unavailable,
        _ => PairingError::Denied,
    })?;
    sqlx::query("INSERT INTO control.paired_devices(tenant_id,device_id,organization_id,branch_id,approved_at) VALUES($1,$2,$3,$4,$5)")
        .bind(tenant.value()).bind(request.device_id.value()).bind(organization.value()).bind(branch.value()).bind(now.0).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO control.first_desktop_claims(tenant_id,claimed_at) VALUES($1,$2)")
        .bind(tenant.value())
        .bind(now.0)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
