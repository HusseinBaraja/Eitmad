use eitmad_contracts::{
    identity::PrincipalId, server::AuthenticatedServerSession, transport::UnixMillis,
};
use uuid::Uuid;

pub(crate) async fn append(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: &AuthenticatedServerSession,
    operation: &str,
    outcome: &str,
    now: UnixMillis,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit.server_records
            (audit_id, tenant_id, session_id, device_id, principal_id, operation,
             outcome, target_kind, correlation_id, occurred_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'sync-record', $8, $9)",
    )
    .bind(Uuid::new_v4())
    .bind(session.tenant_id.value())
    .bind(session.session_id.value())
    .bind(session.device_id.value())
    .bind(PrincipalId::new(session.user_id.value()).value())
    .bind(operation)
    .bind(outcome)
    .bind(Uuid::new_v4())
    .bind(now.0)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
