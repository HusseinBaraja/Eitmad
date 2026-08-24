use eitmad_contracts::{
    identity::{DeviceId, PrincipalId, SessionId, TenantId},
    transport::{CorrelationId, UnixMillis},
};
use uuid::Uuid;

pub(crate) struct AuditEntry<'a> {
    pub tenant_id: TenantId,
    pub session_id: SessionId,
    pub device_id: Option<DeviceId>,
    pub principal_id: PrincipalId,
    pub operation: &'a str,
    pub outcome: &'a str,
    pub target_kind: &'a str,
    pub correlation_id: CorrelationId,
    pub now: UnixMillis,
}

pub(crate) async fn append(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    entry: AuditEntry<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit.server_records
            (audit_id, tenant_id, session_id, device_id, principal_id, operation,
             outcome, target_kind, correlation_id, occurred_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(Uuid::new_v4())
    .bind(entry.tenant_id.value())
    .bind(entry.session_id.value())
    .bind(entry.device_id.map(DeviceId::value))
    .bind(entry.principal_id.value())
    .bind(entry.operation)
    .bind(entry.outcome)
    .bind(entry.target_kind)
    .bind(entry.correlation_id.value())
    .bind(entry.now.0)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
