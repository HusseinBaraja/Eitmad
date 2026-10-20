//! Local operator recovery. No HTTP route exposes this authority.
use eitmad_contracts::{
    identity::{AccountId, TenantId},
    server::TenantCode,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_server_audit::{
    ServerAuditActor, ServerAuditEnvelope, ServerAuditOutcome, append, tenant_scope,
};
use sqlx::PgPool;

use super::{IdentityError, OWNER_RELATION, accounts::revoke};
use crate::authentication::hash_password;

/// Recovers an active tenant Owner through a local `PostgreSQL` superuser connection.
/// The operator authenticates to `PostgreSQL`; ordinary application roles cannot recover accounts.
/// Password replacement, session revocation, revision, notification, and audit commit together.
/// # Errors
/// Denies remote/non-superuser connections and non-Owners. Invalid passwords or unavailable
/// storage/audit leave all account state unchanged. No password or hash is returned.
pub async fn recover_owner_password(
    pool: &PgPool,
    tenant_code: &TenantCode,
    username: &str,
    password: &str,
    correlation_id: CorrelationId,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    let mut tx = pool.begin().await.map_err(|_| IdentityError::Unavailable)?;
    let authorized: bool = sqlx::query_scalar(
        "SELECT rolsuper AND (inet_client_addr() IS NULL
            OR inet_client_addr() <<= '127.0.0.0/8'::inet OR inet_client_addr()='::1'::inet)
         FROM pg_roles WHERE rolname=current_user",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    if !authorized {
        return Err(IdentityError::Denied);
    }
    let canonical = crate::authentication::canonical_username(username)
        .map_err(|_| IdentityError::InvalidInput)?;
    let tenant_id: uuid::Uuid =
        sqlx::query_scalar("SELECT tenant_id FROM control.tenants WHERE tenant_code=$1")
            .bind(tenant_code.as_str())
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| IdentityError::Unavailable)?
            .ok_or(IdentityError::Denied)?;
    // Match activation and account administration lock ordering.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,16016))")
        .bind(tenant_id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
    let account_id = sqlx::query_scalar(
        "SELECT a.account_id FROM control.accounts a
         WHERE a.tenant_id=$1 AND a.canonical_username=$2 AND a.status='active'
           AND EXISTS(SELECT 1 FROM control.relationship_tuples r
             WHERE r.tenant_id=a.tenant_id AND r.subject_principal_id=a.user_id
               AND r.subject_kind='user' AND r.relation=$3
               AND r.object_kind='tenant' AND r.object_id=a.tenant_id)
         FOR UPDATE OF a",
    )
    .bind(tenant_id)
    .bind(canonical)
    .bind(OWNER_RELATION)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| IdentityError::Unavailable)?
    .ok_or(IdentityError::Denied)?;
    let tenant = TenantId::new(tenant_id);
    let account = AccountId::new(account_id);
    let hash = hash_password(password).map_err(|_| IdentityError::InvalidInput)?;
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(tenant.value().to_string())
        .execute(&mut *tx)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query("UPDATE control.accounts SET password_hash=$3, locked_until=NULL, revision=revision+1 WHERE tenant_id=$1 AND account_id=$2")
        .bind(tenant.value()).bind(account.value()).bind(hash).execute(&mut *tx).await
        .map_err(|_| IdentityError::Unavailable)?;
    revoke(&mut tx, tenant, account, now)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "UPDATE control.tenants SET account_revision=account_revision+1 WHERE tenant_id=$1",
    )
    .bind(tenant.value())
    .execute(&mut *tx)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query("SELECT pg_notify('eitmad_accounts_changed','')")
        .execute(&mut *tx)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
    append(
        &mut tx,
        &ServerAuditEnvelope {
            actor: ServerAuditActor::system(tenant),
            scope: tenant_scope(tenant),
            operation: "eitmad.server.authentication.owner-password-recovered.v1",
            outcome: ServerAuditOutcome::Succeeded,
            target_kind: "account",
            target_id: Some(account.value()),
            correlation_id,
            causation_id: None,
            idempotency_key: None,
            redacted_error: None,
            occurred_at: now,
        },
    )
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    tx.commit().await.map_err(|_| IdentityError::Unavailable)
}
