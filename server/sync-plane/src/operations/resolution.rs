//! Persistence for explicit domain conflict decisions within the caller's transaction.
use eitmad_contracts::{
    identity::{ScopeRef, TenantId},
    sync::{ChangeRecord, ConflictRecord},
    transport::{IdempotencyKey, UnixMillis},
};
use sqlx::{Postgres, Row as _, Transaction};

use super::OperationError as E;

pub(crate) async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    scope: &ScopeRef,
    schema: &str,
    key: IdempotencyKey,
    fingerprint: &[u8],
) -> Result<Option<ChangeRecord>, E> {
    let row = sqlx::query("SELECT request_fingerprint,result_json FROM sync.idempotency_results
        WHERE tenant_id=$1 AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3 AND idempotency_key=$4")
        .bind(tenant.value()).bind(scope.id.value()).bind(schema).bind(key.value())
        .fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.get::<Vec<u8>, _>("request_fingerprint") != fingerprint {
        return Err(E::IdempotencyMismatch);
    }
    let value: serde_json::Value = row.get("result_json");
    if value.get("status").and_then(serde_json::Value::as_str) == Some("resolutionClosed") {
        return Err(E::IdempotencyMismatch);
    }
    serde_json::from_value(value.get("value").cloned().ok_or(E::Unavailable)?)
        .map(Some)
        .map_err(|_| E::Unavailable)
}

/// Caller holds the tenant and schema locks used by resolution submission.
/// A closed key is permanent, so a late original request cannot become a new decision.
pub(crate) async fn reconcile(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    scope: &ScopeRef,
    schema: &str,
    key: IdempotencyKey,
    fingerprint: &[u8],
    now: UnixMillis,
) -> Result<Option<ChangeRecord>, E> {
    let row = sqlx::query("SELECT request_fingerprint,result_json FROM sync.idempotency_results
        WHERE tenant_id=$1 AND scope_kind='branch' AND scope_id=$2 AND schema_id=$3 AND idempotency_key=$4")
        .bind(tenant.value()).bind(scope.id.value()).bind(schema).bind(key.value())
        .fetch_optional(&mut **tx).await.map_err(|_| E::Unavailable)?;
    if let Some(row) = row {
        if row.get::<Vec<u8>, _>("request_fingerprint") != fingerprint {
            return Err(E::IdempotencyMismatch);
        }
        let value: serde_json::Value = row.get("result_json");
        if value.get("status").and_then(serde_json::Value::as_str) == Some("resolutionClosed") {
            return Ok(None);
        }
        return serde_json::from_value(value.get("value").cloned().ok_or(E::Unavailable)?)
            .map(Some)
            .map_err(|_| E::Unavailable);
    }
    sqlx::query("INSERT INTO sync.idempotency_results
        (tenant_id,scope_kind,scope_id,schema_id,idempotency_key,request_fingerprint,result_json,created_at)
        VALUES ($1,'branch',$2,$3,$4,$5,'{\"status\":\"resolutionClosed\"}'::jsonb,$6)")
        .bind(tenant.value()).bind(scope.id.value()).bind(schema).bind(key.value())
        .bind(fingerprint).bind(now.0).execute(&mut **tx).await.map_err(|_| E::Unavailable)?;
    Ok(None)
}

pub(crate) async fn save(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    schema: &str,
    conflict: &ConflictRecord,
    now: UnixMillis,
) -> Result<(), E> {
    sqlx::query("INSERT INTO sync.conflicts (tenant_id,scope_kind,scope_id,schema_id,conflict_id,record_id,conflict_json,status,detected_at,resolved_at)
        VALUES ($1,'branch',$2,$3,$4,$5,$6,'resolved',$7,$8) ON CONFLICT(tenant_id,conflict_id)
        DO UPDATE SET conflict_json=excluded.conflict_json,status='resolved',resolved_at=excluded.resolved_at")
        .bind(tenant.value()).bind(conflict.scope.id.value()).bind(schema).bind(conflict.conflict_id.value())
        .bind(conflict.record_id.value()).bind(serde_json::to_value(conflict).map_err(|_| E::Unavailable)?)
        .bind(conflict.detected_at.0).bind(now.0).execute(&mut **tx).await.map_err(|_| E::Unavailable)?;
    Ok(())
}
