use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use eitmad_contracts::{
    server::AuthenticatedServerSession,
    sync::{
        ChangeRecord, Checkpoint, MAX_SYNC_BATCH_RECORDS, SnapshotChunk, SnapshotId,
        SnapshotManifest,
    },
    transport::{CorrelationId, SchemaId, UnixMillis},
};
use eitmad_server_audit::{
    ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append as append_audit,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use sqlx::Row as _;
use uuid::Uuid;

use crate::{
    database::tenant_transaction,
    domain::SyncIntent,
    operations::{OperationError, SyncCoordinator},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotBundle {
    pub manifest: SnapshotManifest,
    pub chunks: Vec<SnapshotChunk>,
}

/// One authorized snapshot or compaction target.
#[derive(Clone, Copy, Debug)]
pub struct SnapshotRequest<'a> {
    pub session: &'a AuthenticatedServerSession,
    pub scope: &'a eitmad_contracts::identity::ScopeRef,
    pub schema_id: &'a SchemaId,
    pub schema_version: u32,
}

struct SnapshotScope<'a> {
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: &'a eitmad_contracts::identity::ScopeRef,
    schema_id: &'a SchemaId,
    checkpoint: Checkpoint,
    generation: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot is denied")]
    Denied,
    #[error("snapshot domain is unavailable")]
    Domain,
    #[error("snapshot source is empty")]
    Empty,
    #[error("snapshot authority is unavailable")]
    Unavailable,
}

impl SyncCoordinator {
    async fn authorize_snapshot(
        &self,
        request: SnapshotRequest<'_>,
        intent: SyncIntent,
        operation: &'static str,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), SnapshotError> {
        let Ok(handler) = self.registry.get(request.schema_id, request.schema_version) else {
            crate::boundary_audit::record(
                &self.pool,
                &snapshot_audit(
                    request,
                    operation,
                    ServerAuditOutcome::Invalid,
                    correlation_id,
                    now,
                    Some("eitmad.error.server-client-incompatible.v1"),
                ),
            )
            .await
            .map_err(|_| SnapshotError::Unavailable)?;
            return Err(SnapshotError::Domain);
        };
        if handler
            .authorize(request.session, request.scope, intent)
            .await
        {
            return Ok(());
        }
        crate::boundary_audit::record(
            &self.pool,
            &snapshot_audit(
                request,
                operation,
                ServerAuditOutcome::Denied,
                correlation_id,
                now,
                Some("eitmad.error.authorization-denied.v1"),
            ),
        )
        .await
        .map_err(|_| SnapshotError::Unavailable)?;
        Err(SnapshotError::Denied)
    }

    /// Builds and stores a consistent, chunked snapshot at the current checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an authorization, domain, empty-scope, or storage error.
    pub async fn create_snapshot(
        &self,
        request: SnapshotRequest<'_>,
        correlation_id: CorrelationId,
        now: UnixMillis,
        valid_for_ms: i64,
    ) -> Result<SnapshotBundle, SnapshotError> {
        let session = request.session;
        let scope = request.scope;
        let schema_id = request.schema_id;
        self.authorize_snapshot(
            request,
            SyncIntent::Read,
            "eitmad.server.sync.create-snapshot.v1",
            correlation_id,
            now,
        )
        .await?;
        let mut transaction = tenant_transaction(&self.pool, session.tenant_id)
            .await
            .map_err(|_| SnapshotError::Unavailable)?;
        let scope_row = sqlx::query(
            "SELECT head_checkpoint, server_generation
             FROM sync.scopes
             WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3 AND schema_id = $4",
        )
        .bind(session.tenant_id.value())
        .bind(scope.kind.as_str())
        .bind(scope.id.value())
        .bind(schema_id.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| SnapshotError::Unavailable)?
        .ok_or(SnapshotError::Empty)?;
        let checkpoint = Checkpoint::new(
            scope_row
                .get::<Option<Uuid>, _>("head_checkpoint")
                .ok_or(SnapshotError::Empty)?,
        );
        let generation = u64::try_from(scope_row.get::<i64, _>("server_generation"))
            .map_err(|_| SnapshotError::Unavailable)?;
        let rows = sqlx::query(
            "SELECT change_json FROM sync.records
             WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3 AND schema_id = $4
             ORDER BY record_id",
        )
        .bind(session.tenant_id.value())
        .bind(scope.kind.as_str())
        .bind(scope.id.value())
        .bind(schema_id.as_str())
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| SnapshotError::Unavailable)?;
        let records = rows
            .into_iter()
            .map(|row| serde_json::from_value(row.get("change_json")))
            .collect::<Result<Vec<ChangeRecord>, _>>()
            .map_err(|_| SnapshotError::Unavailable)?;
        let snapshot = store_snapshot(
            &mut transaction,
            &SnapshotScope {
                tenant_id: session.tenant_id,
                scope,
                schema_id,
                checkpoint,
                generation,
            },
            &records,
            now,
            valid_for_ms,
        )
        .await?;
        append_audit(
            &mut transaction,
            &snapshot_audit(
                request,
                "eitmad.server.sync.create-snapshot.v1",
                ServerAuditOutcome::Succeeded,
                correlation_id,
                now,
                None,
            ),
        )
        .await
        .map_err(|_| SnapshotError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| SnapshotError::Unavailable)?;
        Ok(snapshot)
    }

    /// Deletes operation history only after an authorized caller, the
    /// retention floor, a covering snapshot, and the absence of an open
    /// conflict.
    ///
    /// # Errors
    ///
    /// Returns an authorization or storage error without deleting uncovered history.
    pub async fn compact_history(
        &self,
        request: SnapshotRequest<'_>,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<u64, SnapshotError> {
        let session = request.session;
        let scope = request.scope;
        let schema_id = request.schema_id;
        self.authorize_snapshot(
            request,
            SyncIntent::Write,
            "eitmad.server.sync.compact-history.v1",
            correlation_id,
            now,
        )
        .await?;
        let mut transaction = tenant_transaction(&self.pool, session.tenant_id)
            .await
            .map_err(|_| SnapshotError::Unavailable)?;
        let deleted = sqlx::query(
            "DELETE FROM sync.operations o
             WHERE o.tenant_id = $1 AND o.scope_kind = $2 AND o.scope_id = $3
               AND o.schema_id = $4 AND o.retention_until < $5
               AND NOT EXISTS (
                   SELECT 1 FROM sync.conflicts c
                   WHERE c.tenant_id = o.tenant_id AND c.scope_kind = o.scope_kind
                     AND c.scope_id = o.scope_id AND c.schema_id = o.schema_id
                     AND c.status = 'open'
               )
               AND EXISTS (
                   SELECT 1 FROM sync.snapshots s
                   JOIN sync.operations covered
                     ON covered.tenant_id = s.tenant_id
                    AND covered.scope_kind = s.scope_kind
                    AND covered.scope_id = s.scope_id
                    AND covered.schema_id = s.schema_id
                    AND covered.checkpoint = s.checkpoint
                   WHERE s.tenant_id = o.tenant_id AND s.scope_kind = o.scope_kind
                     AND s.scope_id = o.scope_id AND s.schema_id = o.schema_id
                     AND covered.sequence > o.sequence
                )",
        )
        .bind(session.tenant_id.value())
        .bind(scope.kind.as_str())
        .bind(scope.id.value())
        .bind(schema_id.as_str())
        .bind(now.0)
        .execute(&mut *transaction)
        .await
        .map_err(|_| SnapshotError::Unavailable)?;
        append_audit(
            &mut transaction,
            &snapshot_audit(
                request,
                "eitmad.server.sync.compact-history.v1",
                ServerAuditOutcome::Succeeded,
                correlation_id,
                now,
                None,
            ),
        )
        .await
        .map_err(|_| SnapshotError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| SnapshotError::Unavailable)?;
        Ok(deleted.rows_affected())
    }
}

fn snapshot_audit<'a>(
    request: SnapshotRequest<'_>,
    operation: &'a str,
    outcome: ServerAuditOutcome,
    correlation_id: CorrelationId,
    now: UnixMillis,
    redacted_error: Option<&'a str>,
) -> ServerAuditEnvelope<'a> {
    ServerAuditEnvelope::from_session(
        request.session,
        request.scope.clone(),
        ServerAuditEvent {
            operation,
            outcome,
            target_kind: "sync-snapshot",
            target_id: Some(request.scope.id.value()),
            correlation_id,
            causation_id: None,
            idempotency_key: None,
            redacted_error,
            occurred_at: now,
        },
    )
}

async fn store_snapshot(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: &SnapshotScope<'_>,
    records: &[ChangeRecord],
    now: UnixMillis,
    valid_for_ms: i64,
) -> Result<SnapshotBundle, SnapshotError> {
    let snapshot_id = SnapshotId::new(Uuid::new_v4());
    let chunks = build_chunks(snapshot_id, records)?;
    let snapshot_checksum = checksum(records)?;
    let manifest = SnapshotManifest {
        snapshot_id,
        scope: context.scope.clone(),
        checkpoint: context.checkpoint,
        server_generation: context.generation,
        created_at: now,
        valid_until: UnixMillis(now.0.saturating_add(valid_for_ms.max(1))),
        total_records: u64::try_from(records.len()).map_err(|_| SnapshotError::Unavailable)?,
        total_chunks: u32::try_from(chunks.len()).map_err(|_| SnapshotError::Unavailable)?,
        checksum: snapshot_checksum.clone(),
    };
    sqlx::query(
        "INSERT INTO sync.snapshots
            (tenant_id, scope_kind, scope_id, schema_id, snapshot_id, checkpoint,
             server_generation, manifest_json, chunks_json, checksum,
             created_at, valid_until)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(context.tenant_id.value())
    .bind(context.scope.kind.as_str())
    .bind(context.scope.id.value())
    .bind(context.schema_id.as_str())
    .bind(snapshot_id.value())
    .bind(context.checkpoint.value())
    .bind(i64::try_from(context.generation).map_err(|_| SnapshotError::Unavailable)?)
    .bind(serde_json::to_value(&manifest).map_err(|_| SnapshotError::Unavailable)?)
    .bind(serde_json::to_value(&chunks).map_err(|_| SnapshotError::Unavailable)?)
    .bind(&snapshot_checksum)
    .bind(now.0)
    .bind(manifest.valid_until.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| SnapshotError::Unavailable)?;
    Ok(SnapshotBundle { manifest, chunks })
}

fn build_chunks(
    snapshot_id: SnapshotId,
    records: &[ChangeRecord],
) -> Result<Vec<SnapshotChunk>, SnapshotError> {
    records
        .chunks(MAX_SYNC_BATCH_RECORDS)
        .enumerate()
        .map(|(index, records)| {
            Ok(SnapshotChunk {
                snapshot_id,
                chunk_index: u32::try_from(index).map_err(|_| SnapshotError::Unavailable)?,
                checksum: checksum(records)?,
                records: records.to_vec(),
            })
        })
        .collect()
}

fn checksum(value: &(impl Serialize + ?Sized)) -> Result<String, SnapshotError> {
    let bytes = serde_json::to_vec(value).map_err(|_| SnapshotError::Unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(bytes)))
}

impl From<OperationError> for SnapshotError {
    fn from(_value: OperationError) -> Self {
        Self::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::{
        identity::{ScopeId, ScopeKind, ScopeRef},
        sync::{ChangeId, ChangeOperation, RecordId},
        transport::IdempotencyKey,
    };

    use super::*;

    fn record(value: u128) -> ChangeRecord {
        ChangeRecord {
            change_id: ChangeId::new(Uuid::from_u128(value)),
            record_id: RecordId::new(Uuid::from_u128(value + 1)),
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(Uuid::from_u128(10)),
            },
            operation: ChangeOperation::Upsert,
            base_revision: None,
            revision: 1,
            changed_at: UnixMillis(1),
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(value + 2)),
            payload: None,
            merge: None,
        }
    }

    #[test]
    fn snapshots_are_bounded_and_checksums_are_stable() {
        let records = (1..=501).map(record).collect::<Vec<_>>();
        let chunks = build_chunks(SnapshotId::new(Uuid::from_u128(5)), &records).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].records.len(), 500);
        assert_eq!(chunks[1].records.len(), 1);
        assert_eq!(checksum(&records).unwrap(), checksum(&records).unwrap());
    }
}
