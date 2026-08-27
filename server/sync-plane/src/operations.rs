//! Operation ingestion: local-first operations and server-authoritative
//! commands with idempotency, conflict durability, and event publication.

use std::sync::Arc;

use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

use eitmad_contracts::identity::ScopeRef;
use eitmad_contracts::server::AuthenticatedServerSession;
use eitmad_contracts::sync::{
    BatchAcknowledgement, ChangeBatch, ChangeId, ChangeOperation, ChangeRecord, Checkpoint,
    CommandDisposition, ConflictId, ConflictRecord, ConflictStatus, DeliveryId, RecordChangeNotice,
    RecordId, SyncMode,
};
use eitmad_contracts::transport::{CorrelationId, IdempotencyKey, SchemaId, UnixMillis};
use eitmad_server_audit::{
    ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append as append_audit,
};

use crate::database::{SyncDatabase, tenant_transaction};
use crate::domain::{
    AuthoritativeChangeDraft, CommandSubmission, DomainRegistry, DomainRegistryError,
    DomainSyncHandler, DomainValidationError, LocalOperationDraft, SyncIntent,
};

/// History floor applied to every stored operation.
pub const OPERATION_RETENTION_MS: i64 = eitmad_contracts::server::DEFAULT_SYNC_HISTORY_FLOOR_MS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OperationError {
    #[error("domain schema is unknown or incompatible")]
    UnknownDomain,
    #[error("domain denied the operation")]
    Denied,
    #[error("operation payload is invalid")]
    Invalid,
    #[error("scope mode does not accept this operation kind")]
    WrongMode,
    #[error("idempotency key was reused with different intent")]
    IdempotencyMismatch,
    #[error("the base revision refers to an unknown record")]
    UnknownRecord,
    #[error("synchronization authority is unavailable")]
    Unavailable,
    #[error("the requested operation history is no longer retained")]
    SnapshotRequired,
}

impl From<DomainRegistryError> for OperationError {
    fn from(value: DomainRegistryError) -> Self {
        match value {
            DomainRegistryError::Duplicate
            | DomainRegistryError::Unknown
            | DomainRegistryError::IncompatibleVersion => Self::UnknownDomain,
        }
    }
}

impl From<DomainValidationError> for OperationError {
    fn from(value: DomainValidationError) -> Self {
        match value {
            DomainValidationError::Denied => Self::Denied,
            DomainValidationError::Invalid | DomainValidationError::Conflict => Self::Invalid,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationResult {
    /// The operation received a new authoritative revision.
    Applied { change: Box<ChangeRecord> },
    /// An exact duplicate; the stored authoritative result is replayed.
    Replayed { change: Box<ChangeRecord> },
    /// A stale base revision created a durable open conflict.
    ConflictRecorded { conflict_id: ConflictId },
}

/// One authorized request for a page of retained synchronization history.
pub struct PullPageRequest<'a> {
    pub session: &'a AuthenticatedServerSession,
    pub scope: &'a ScopeRef,
    pub schema_id: &'a SchemaId,
    pub schema_version: u32,
    pub after: Option<Checkpoint>,
    pub maximum_records: u32,
    pub correlation_id: CorrelationId,
    pub now: UnixMillis,
}

/// One authorized device-checkpoint acknowledgement.
pub struct AcknowledgeRequest<'a> {
    pub session: &'a AuthenticatedServerSession,
    pub scope: &'a ScopeRef,
    pub schema_id: &'a SchemaId,
    pub schema_version: u32,
    pub acknowledgement: &'a BatchAcknowledgement,
    pub correlation_id: CorrelationId,
    pub now: UnixMillis,
}

#[derive(Clone)]
pub struct SyncCoordinator {
    pub(crate) pool: PgPool,
    pub(crate) registry: DomainRegistry,
}

struct SyncAuditContext<'a> {
    session: &'a AuthenticatedServerSession,
    scope: &'a ScopeRef,
    operation: &'static str,
    target_kind: &'static str,
    target_id: Uuid,
    correlation_id: CorrelationId,
    idempotency_key: Option<IdempotencyKey>,
    now: UnixMillis,
}

impl<'a> SyncAuditContext<'a> {
    fn envelope(
        &self,
        outcome: ServerAuditOutcome,
        redacted_error: Option<&'static str>,
    ) -> ServerAuditEnvelope<'a> {
        ServerAuditEnvelope::from_session(
            self.session,
            self.scope.clone(),
            ServerAuditEvent {
                operation: self.operation,
                outcome,
                target_kind: self.target_kind,
                target_id: Some(self.target_id),
                correlation_id: self.correlation_id,
                causation_id: None,
                idempotency_key: self.idempotency_key,
                redacted_error,
                occurred_at: self.now,
            },
        )
    }
}

fn local_operation_audit<'a>(
    session: &'a AuthenticatedServerSession,
    draft: &'a LocalOperationDraft,
    correlation_id: CorrelationId,
    now: UnixMillis,
) -> SyncAuditContext<'a> {
    SyncAuditContext {
        session,
        scope: &draft.scope,
        operation: "eitmad.server.sync.apply-local.v1",
        target_kind: "sync-record",
        target_id: draft.record_id.value(),
        correlation_id,
        idempotency_key: Some(draft.idempotency_key),
        now,
    }
}

fn command_audit<'a>(
    session: &'a AuthenticatedServerSession,
    command: &'a CommandSubmission,
    correlation_id: CorrelationId,
    now: UnixMillis,
) -> SyncAuditContext<'a> {
    SyncAuditContext {
        session,
        scope: &command.scope,
        operation: "eitmad.server.sync.submit-command.v1",
        target_kind: "sync-record",
        target_id: command.record_id.value(),
        correlation_id,
        idempotency_key: Some(command.idempotency_key),
        now,
    }
}

fn pull_audit<'a>(request: &PullPageRequest<'a>) -> SyncAuditContext<'a> {
    SyncAuditContext {
        session: request.session,
        scope: request.scope,
        operation: "eitmad.server.sync.pull.v1",
        target_kind: "sync-history",
        target_id: request.scope.id.value(),
        correlation_id: request.correlation_id,
        idempotency_key: None,
        now: request.now,
    }
}

fn acknowledge_audit<'a>(request: &AcknowledgeRequest<'a>) -> SyncAuditContext<'a> {
    SyncAuditContext {
        session: request.session,
        scope: request.scope,
        operation: "eitmad.server.sync.acknowledge.v1",
        target_kind: "sync-checkpoint",
        target_id: request.scope.id.value(),
        correlation_id: request.correlation_id,
        idempotency_key: None,
        now: request.now,
    }
}

const fn validation_audit(error: DomainValidationError) -> (ServerAuditOutcome, &'static str) {
    match error {
        DomainValidationError::Denied => (
            ServerAuditOutcome::Denied,
            "eitmad.error.authorization-denied.v1",
        ),
        DomainValidationError::Invalid => (
            ServerAuditOutcome::Invalid,
            "eitmad.error.contract-invalid.v1",
        ),
        DomainValidationError::Conflict => (
            ServerAuditOutcome::Conflict,
            "eitmad.error.contract-invalid.v1",
        ),
    }
}

impl SyncCoordinator {
    #[must_use]
    pub fn new(database: &SyncDatabase, registry: DomainRegistry) -> Self {
        Self {
            pool: database.pool(),
            registry,
        }
    }

    #[must_use]
    pub const fn domains(&self) -> &DomainRegistry {
        &self.registry
    }

    async fn local_handler(
        &self,
        session: &AuthenticatedServerSession,
        draft: &LocalOperationDraft,
        audit: &SyncAuditContext<'_>,
    ) -> Result<Arc<dyn DomainSyncHandler>, OperationError> {
        let handler = match self.registry.get(&draft.schema_id, draft.schema_version) {
            Ok(handler) => handler,
            Err(error) => {
                self.record_boundary_outcome(audit.envelope(
                    ServerAuditOutcome::Invalid,
                    Some("eitmad.error.server-client-incompatible.v1"),
                ))
                .await?;
                return Err(error.into());
            }
        };
        if handler.descriptor().mode != SyncMode::LocalFirst {
            self.record_boundary_outcome(audit.envelope(
                ServerAuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1"),
            ))
            .await?;
            return Err(OperationError::WrongMode);
        }
        if !handler
            .authorize(session, &draft.scope, SyncIntent::Write)
            .await
        {
            self.record_boundary_outcome(audit.envelope(
                ServerAuditOutcome::Denied,
                Some("eitmad.error.authorization-denied.v1"),
            ))
            .await?;
            return Err(OperationError::Denied);
        }
        if let Err(error) = handler.validate_local(draft) {
            let (outcome, code) = validation_audit(error);
            self.record_boundary_outcome(audit.envelope(outcome, Some(code)))
                .await?;
            return Err(error.into());
        }
        Ok(handler)
    }

    async fn command_handler(
        &self,
        session: &AuthenticatedServerSession,
        command: &CommandSubmission,
        audit: &SyncAuditContext<'_>,
    ) -> Result<Arc<dyn DomainSyncHandler>, OperationError> {
        let handler = match self
            .registry
            .get(&command.schema_id, command.schema_version)
        {
            Ok(handler) => handler,
            Err(error) => {
                self.record_boundary_outcome(audit.envelope(
                    ServerAuditOutcome::Invalid,
                    Some("eitmad.error.server-client-incompatible.v1"),
                ))
                .await?;
                return Err(error.into());
            }
        };
        if handler.descriptor().mode != SyncMode::ServerAuthoritative {
            self.record_boundary_outcome(audit.envelope(
                ServerAuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1"),
            ))
            .await?;
            return Err(OperationError::WrongMode);
        }
        if !handler
            .authorize(session, &command.scope, SyncIntent::Write)
            .await
        {
            self.record_boundary_outcome(audit.envelope(
                ServerAuditOutcome::Denied,
                Some("eitmad.error.authorization-denied.v1"),
            ))
            .await?;
            return Err(OperationError::Denied);
        }
        Ok(handler)
    }

    async fn read_handler(
        &self,
        session: &AuthenticatedServerSession,
        scope: &ScopeRef,
        schema_id: &SchemaId,
        schema_version: u32,
        audit: &SyncAuditContext<'_>,
    ) -> Result<Arc<dyn DomainSyncHandler>, OperationError> {
        let handler = match self.registry.get(schema_id, schema_version) {
            Ok(handler) => handler,
            Err(error) => {
                self.record_boundary_outcome(audit.envelope(
                    ServerAuditOutcome::Invalid,
                    Some("eitmad.error.server-client-incompatible.v1"),
                ))
                .await?;
                return Err(error.into());
            }
        };
        if !handler.authorize(session, scope, SyncIntent::Read).await {
            self.record_boundary_outcome(audit.envelope(
                ServerAuditOutcome::Denied,
                Some("eitmad.error.authorization-denied.v1"),
            ))
            .await?;
            return Err(OperationError::Denied);
        }
        Ok(handler)
    }

    /// Accepts one handler-validated local-first operation and assigns the
    /// authoritative revision and checkpoint on the server.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError`] for unknown domains, denials, wrong
    /// modes, idempotency mismatches, or an unavailable authority.
    pub async fn apply_local_operation(
        &self,
        session: &AuthenticatedServerSession,
        draft: &LocalOperationDraft,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<OperationResult, OperationError> {
        let audit = local_operation_audit(session, draft, correlation_id, now);
        let handler = self.local_handler(session, draft, &audit).await?;

        let fingerprint = fingerprint_draft(draft);
        let mut transaction = tenant_transaction(&self.pool, session.tenant_id)
            .await
            .map_err(|_| OperationError::Unavailable)?;
        ensure_scope(
            &mut transaction,
            session.tenant_id,
            draft,
            &handler.descriptor(),
        )
        .await?;
        lock_scope(
            &mut transaction,
            session.tenant_id,
            &draft.scope,
            &draft.schema_id,
        )
        .await?;

        if let Some(stored) =
            load_stored_result(&mut transaction, session.tenant_id, draft, &fingerprint).await?
        {
            return Ok(stored);
        }

        let current_revision = current_record_revision(&mut transaction, session.tenant_id, draft)
            .await
            .map_err(|_| OperationError::Unavailable)?;
        let base_is_current = match draft.base_revision {
            None => current_revision == 0,
            Some(base) => base == current_revision,
        };
        if !base_is_current {
            let conflict_id = record_conflict(
                &mut transaction,
                session.tenant_id,
                draft,
                current_revision,
                &fingerprint,
                now,
            )
            .await
            .map_err(|_| OperationError::Unavailable)?;
            let Some(conflict_id) = conflict_id else {
                return Err(OperationError::UnknownRecord);
            };
            append_audit(
                &mut transaction,
                &audit.envelope(ServerAuditOutcome::Conflict, None),
            )
            .await
            .map_err(|_| OperationError::Unavailable)?;
            transaction
                .commit()
                .await
                .map_err(|_| OperationError::Unavailable)?;
            return Ok(OperationResult::ConflictRecorded { conflict_id });
        }

        let change = commit_change(
            &mut transaction,
            session.tenant_id,
            draft,
            current_revision,
            &fingerprint,
            now,
        )
        .await
        .map_err(|_| OperationError::Unavailable)?;
        append_audit(
            &mut transaction,
            &audit.envelope(ServerAuditOutcome::Succeeded, None),
        )
        .await
        .map_err(|_| OperationError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| OperationError::Unavailable)?;
        Ok(OperationResult::Applied {
            change: Box::new(change),
        })
    }

    /// Runs one registered server-authoritative command inside the server
    /// transaction and returns the authoritative change or a durable denial.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError`] for unknown domains, denials, or an
    /// unavailable authority.
    pub async fn submit_command(
        &self,
        session: &AuthenticatedServerSession,
        command: &CommandSubmission,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<CommandDisposition, OperationError> {
        let audit = command_audit(session, command, correlation_id, now);
        let handler = self.command_handler(session, command, &audit).await?;

        let fingerprint = fingerprint_command(command);
        let mut transaction = tenant_transaction(&self.pool, session.tenant_id)
            .await
            .map_err(|_| OperationError::Unavailable)?;
        ensure_command_scope(&mut transaction, session.tenant_id, command).await?;
        lock_scope(
            &mut transaction,
            session.tenant_id,
            &command.scope,
            &command.schema_id,
        )
        .await?;

        if let Some(stored) =
            load_stored_disposition(&mut transaction, session.tenant_id, command, &fingerprint)
                .await?
        {
            return Ok(stored);
        }

        match handler.execute_command(session, command) {
            Ok(draft) => {
                let change = match commit_authoritative_change(
                    &mut transaction,
                    session.tenant_id,
                    command,
                    &fingerprint,
                    draft,
                    now,
                )
                .await
                {
                    Ok(change) => change,
                    Err(OperationError::Invalid) => {
                        append_audit(
                            &mut transaction,
                            &audit.envelope(
                                ServerAuditOutcome::Invalid,
                                Some("eitmad.error.contract-invalid.v1"),
                            ),
                        )
                        .await
                        .map_err(|_| OperationError::Unavailable)?;
                        transaction
                            .commit()
                            .await
                            .map_err(|_| OperationError::Unavailable)?;
                        return Err(OperationError::Invalid);
                    }
                    Err(error) => return Err(error),
                };
                append_audit(
                    &mut transaction,
                    &audit.envelope(ServerAuditOutcome::Succeeded, None),
                )
                .await
                .map_err(|_| OperationError::Unavailable)?;
                transaction
                    .commit()
                    .await
                    .map_err(|_| OperationError::Unavailable)?;
                Ok(CommandDisposition::Accepted {
                    authoritative_change: change.map(Box::new),
                })
            }
            Err(DomainValidationError::Denied) => {
                store_denial(
                    &mut transaction,
                    session.tenant_id,
                    command,
                    &fingerprint,
                    now,
                )
                .await
                .map_err(|_| OperationError::Unavailable)?;
                append_audit(
                    &mut transaction,
                    &audit.envelope(
                        ServerAuditOutcome::Denied,
                        Some("eitmad.error.authorization-denied.v1"),
                    ),
                )
                .await
                .map_err(|_| OperationError::Unavailable)?;
                transaction
                    .commit()
                    .await
                    .map_err(|_| OperationError::Unavailable)?;
                Ok(CommandDisposition::Denied {
                    reason: eitmad_contracts::sync::ErrorCodeRef::parse(
                        "eitmad.error.authorization-denied.v1",
                    )
                    .map_err(|_| OperationError::Invalid)?,
                })
            }
            Err(error) => {
                let outcome = match error {
                    DomainValidationError::Denied => ServerAuditOutcome::Denied,
                    DomainValidationError::Invalid => ServerAuditOutcome::Invalid,
                    DomainValidationError::Conflict => ServerAuditOutcome::Conflict,
                };
                append_audit(
                    &mut transaction,
                    &audit.envelope(outcome, Some("eitmad.error.contract-invalid.v1")),
                )
                .await
                .map_err(|_| OperationError::Unavailable)?;
                transaction
                    .commit()
                    .await
                    .map_err(|_| OperationError::Unavailable)?;
                Err(error.into())
            }
        }
    }
}

impl SyncCoordinator {
    /// Reads one authorized page of operation history after a checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`OperationError::SnapshotRequired`] when the requested
    /// checkpoint is not retained. Other failures do not expose storage data.
    pub async fn pull(&self, request: PullPageRequest<'_>) -> Result<ChangeBatch, OperationError> {
        let audit = pull_audit(&request);
        let PullPageRequest {
            session,
            scope,
            schema_id,
            schema_version,
            after,
            maximum_records,
            correlation_id: _,
            now: _,
        } = request;
        if maximum_records == 0
            || usize::try_from(maximum_records).unwrap_or(usize::MAX)
                > eitmad_contracts::sync::MAX_SYNC_BATCH_RECORDS
        {
            self.record_boundary_outcome(audit.envelope(
                ServerAuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1"),
            ))
            .await?;
            return Err(OperationError::Invalid);
        }
        self.read_handler(session, scope, schema_id, schema_version, &audit)
            .await?;

        let mut transaction = tenant_transaction(&self.pool, session.tenant_id)
            .await
            .map_err(|_| OperationError::Unavailable)?;
        let after_sequence = if let Some(checkpoint) = after {
            sqlx::query_scalar::<_, i64>(
                "SELECT sequence FROM sync.operations
                 WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
                   AND schema_id = $4 AND checkpoint = $5",
            )
            .bind(session.tenant_id.value())
            .bind(scope.kind.as_str())
            .bind(scope.id.value())
            .bind(schema_id.as_str())
            .bind(checkpoint.value())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .ok_or(OperationError::SnapshotRequired)?
        } else {
            0
        };
        let rows = sqlx::query(
            "SELECT sequence, checkpoint, change_json
             FROM sync.operations
             WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
               AND schema_id = $4 AND sequence > $5
             ORDER BY sequence
             LIMIT $6",
        )
        .bind(session.tenant_id.value())
        .bind(scope.kind.as_str())
        .bind(scope.id.value())
        .bind(schema_id.as_str())
        .bind(after_sequence)
        .bind(i64::from(maximum_records) + 1)
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| OperationError::Unavailable)?;
        let limit = usize::try_from(maximum_records).map_err(|_| OperationError::Invalid)?;
        let has_more = rows.len() > limit;
        let retained = rows.into_iter().take(limit).collect::<Vec<_>>();
        let checkpoint = retained
            .last()
            .map(|row| Checkpoint::new(row.get::<Uuid, _>("checkpoint")))
            .or(after)
            .unwrap_or_else(|| Checkpoint::new(Uuid::nil()));
        let records = retained
            .into_iter()
            .map(|row| serde_json::from_value::<ChangeRecord>(row.get("change_json")))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| OperationError::Unavailable)?;
        let batch = ChangeBatch::new(
            DeliveryId::new(Uuid::new_v4()),
            IdempotencyKey::new(Uuid::new_v4()),
            after,
            checkpoint,
            records,
            has_more,
        )
        .map_err(|_| OperationError::Invalid)?;
        append_audit(
            &mut transaction,
            &audit.envelope(ServerAuditOutcome::Succeeded, None),
        )
        .await
        .map_err(|_| OperationError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| OperationError::Unavailable)?;
        Ok(batch)
    }

    /// Stores the authenticated device checkpoint for one delivered batch.
    ///
    /// # Errors
    ///
    /// Rejects an unknown checkpoint or a count larger than the protocol batch.
    pub async fn acknowledge(&self, request: AcknowledgeRequest<'_>) -> Result<(), OperationError> {
        let audit = acknowledge_audit(&request);
        let AcknowledgeRequest {
            session,
            scope,
            schema_id,
            schema_version,
            acknowledgement,
            correlation_id: _,
            now,
        } = request;
        if usize::try_from(acknowledgement.accepted_records).unwrap_or(usize::MAX)
            > eitmad_contracts::sync::MAX_SYNC_BATCH_RECORDS
        {
            self.record_boundary_outcome(audit.envelope(
                ServerAuditOutcome::Invalid,
                Some("eitmad.error.contract-invalid.v1"),
            ))
            .await?;
            return Err(OperationError::Invalid);
        }
        self.read_handler(session, scope, schema_id, schema_version, &audit)
            .await?;
        let mut transaction = tenant_transaction(&self.pool, session.tenant_id)
            .await
            .map_err(|_| OperationError::Unavailable)?;
        let sequence = if acknowledgement.checkpoint.value().is_nil() {
            0
        } else {
            sqlx::query_scalar::<_, i64>(
                "SELECT sequence FROM sync.operations
                 WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
                   AND schema_id = $4 AND checkpoint = $5",
            )
            .bind(session.tenant_id.value())
            .bind(scope.kind.as_str())
            .bind(scope.id.value())
            .bind(schema_id.as_str())
            .bind(acknowledgement.checkpoint.value())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|_| OperationError::Unavailable)?
            .ok_or(OperationError::SnapshotRequired)?
        };
        sqlx::query(
            "INSERT INTO sync.device_checkpoints
                (tenant_id, account_id, device_id, scope_kind, scope_id,
                 schema_id, checkpoint, sequence, acknowledged_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT
                (tenant_id, account_id, device_id, scope_kind, scope_id, schema_id)
             DO UPDATE SET checkpoint = EXCLUDED.checkpoint,
                           sequence = GREATEST(sync.device_checkpoints.sequence, EXCLUDED.sequence),
                           acknowledged_at = EXCLUDED.acknowledged_at
             WHERE sync.device_checkpoints.sequence <= EXCLUDED.sequence",
        )
        .bind(session.tenant_id.value())
        .bind(session.account_id.value())
        .bind(session.device_id.value())
        .bind(scope.kind.as_str())
        .bind(scope.id.value())
        .bind(schema_id.as_str())
        .bind(acknowledgement.checkpoint.value())
        .bind(sequence)
        .bind(now.0)
        .execute(&mut *transaction)
        .await
        .map_err(|_| OperationError::Unavailable)?;
        append_audit(
            &mut transaction,
            &audit.envelope(ServerAuditOutcome::Succeeded, None),
        )
        .await
        .map_err(|_| OperationError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| OperationError::Unavailable)
    }

    async fn record_boundary_outcome(
        &self,
        envelope: ServerAuditEnvelope<'_>,
    ) -> Result<(), OperationError> {
        crate::boundary_audit::record(&self.pool, &envelope)
            .await
            .map_err(|_| OperationError::Unavailable)
    }
}

fn fingerprint_draft(draft: &LocalOperationDraft) -> [u8; 32] {
    fingerprint_json(draft)
}

fn fingerprint_command(command: &CommandSubmission) -> [u8; 32] {
    fingerprint_json(command)
}

fn fingerprint_json(value: &impl serde::Serialize) -> [u8; 32] {
    let encoded =
        serde_json::to_vec(value).expect("draft serialization cannot fail for owned data");
    Sha256::digest(encoded).into()
}

type Tx<'c> = sqlx::Transaction<'c, sqlx::Postgres>;

async fn ensure_scope(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    draft: &LocalOperationDraft,
    descriptor: &crate::domain::DomainDescriptor,
) -> Result<(), OperationError> {
    let mode = match descriptor.mode {
        SyncMode::LocalFirst => "local_first",
        SyncMode::ServerAuthoritative => "server_authoritative",
    };
    upsert_scope(transaction, tenant_id, &draft.scope, &draft.schema_id, mode).await
}

async fn ensure_command_scope(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    command: &CommandSubmission,
) -> Result<(), OperationError> {
    upsert_scope(
        transaction,
        tenant_id,
        &command.scope,
        &command.schema_id,
        "server_authoritative",
    )
    .await
}

async fn upsert_scope(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: &ScopeRef,
    schema_id: &eitmad_contracts::transport::SchemaId,
    mode: &str,
) -> Result<(), OperationError> {
    sqlx::query(
        "INSERT INTO sync.scopes
             (tenant_id, scope_kind, scope_id, schema_id, mode, head_sequence)
         VALUES ($1, $2, $3, $4, $5, 0)
         ON CONFLICT (tenant_id, scope_kind, scope_id, schema_id) DO NOTHING",
    )
    .bind(tenant_id.value())
    .bind(scope.kind.as_str())
    .bind(scope.id.value())
    .bind(schema_id.as_str())
    .bind(mode)
    .execute(&mut **transaction)
    .await
    .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}

async fn lock_scope(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: &ScopeRef,
    schema_id: &SchemaId,
) -> Result<(), OperationError> {
    sqlx::query(
        "SELECT head_sequence FROM sync.scopes
         WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
           AND schema_id = $4
         FOR UPDATE",
    )
    .bind(tenant_id.value())
    .bind(scope.kind.as_str())
    .bind(scope.id.value())
    .bind(schema_id.as_str())
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}

async fn load_stored_result(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    draft: &LocalOperationDraft,
    fingerprint: &[u8],
) -> Result<Option<OperationResult>, OperationError> {
    let row = sqlx::query(
        "SELECT request_fingerprint, result_json FROM sync.idempotency_results
         WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
           AND schema_id = $4 AND idempotency_key = $5",
    )
    .bind(tenant_id.value())
    .bind(draft.scope.kind.as_str())
    .bind(draft.scope.id.value())
    .bind(draft.schema_id.as_str())
    .bind(draft.idempotency_key.value())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| OperationError::Unavailable)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stored_fingerprint: Vec<u8> = row.get("request_fingerprint");
    if stored_fingerprint != fingerprint {
        return Err(OperationError::IdempotencyMismatch);
    }
    let stored: StoredLocalResult =
        serde_json::from_value(row.get("result_json")).map_err(|_| OperationError::Unavailable)?;
    Ok(Some(match stored {
        StoredLocalResult::Applied(change) => OperationResult::Replayed { change },
        StoredLocalResult::Conflict(conflict_id) => {
            OperationResult::ConflictRecorded { conflict_id }
        }
    }))
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
enum StoredLocalResult {
    Applied(Box<ChangeRecord>),
    Conflict(ConflictId),
}

async fn load_stored_disposition(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    command: &CommandSubmission,
    fingerprint: &[u8],
) -> Result<Option<CommandDisposition>, OperationError> {
    let row = sqlx::query(
        "SELECT request_fingerprint, result_json FROM sync.idempotency_results
         WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
           AND schema_id = $4 AND idempotency_key = $5",
    )
    .bind(tenant_id.value())
    .bind(command.scope.kind.as_str())
    .bind(command.scope.id.value())
    .bind(command.schema_id.as_str())
    .bind(command.idempotency_key.value())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| OperationError::Unavailable)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stored_fingerprint: Vec<u8> = row.get("request_fingerprint");
    if stored_fingerprint != fingerprint {
        return Err(OperationError::IdempotencyMismatch);
    }
    let disposition: CommandDisposition =
        serde_json::from_value(row.get("result_json")).map_err(|_| OperationError::Unavailable)?;
    Ok(Some(disposition))
}

async fn current_record_revision(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    draft: &LocalOperationDraft,
) -> Result<u64, sqlx::Error> {
    let revision: Option<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM sync.records
         WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
           AND schema_id = $4 AND record_id = $5",
    )
    .bind(tenant_id.value())
    .bind(draft.scope.kind.as_str())
    .bind(draft.scope.id.value())
    .bind(draft.schema_id.as_str())
    .bind(draft.record_id.value())
    .fetch_optional(&mut **transaction)
    .await?;
    Ok(u64::try_from(revision.unwrap_or(0)).unwrap_or(u64::MAX))
}

async fn record_conflict(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    draft: &LocalOperationDraft,
    current_revision: u64,
    fingerprint: &[u8],
    now: UnixMillis,
) -> Result<Option<ConflictId>, sqlx::Error> {
    let conflict_id = ConflictId::new(Uuid::new_v4());
    let remote_json: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT change_json FROM sync.records
         WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
           AND schema_id = $4 AND record_id = $5",
    )
    .bind(tenant_id.value())
    .bind(draft.scope.kind.as_str())
    .bind(draft.scope.id.value())
    .bind(draft.schema_id.as_str())
    .bind(draft.record_id.value())
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(remote_json) = remote_json else {
        return Ok(None);
    };
    let remote = serde_json::from_value::<ChangeRecord>(remote_json)
        .map_err(|_| sqlx::Error::Protocol("stored projection is invalid".to_owned()))?;
    debug_assert_eq!(remote.revision, current_revision);
    let local = ChangeRecord {
        change_id: ChangeId::new(Uuid::new_v4()),
        record_id: draft.record_id,
        scope: draft.scope.clone(),
        operation: draft.operation,
        base_revision: draft.base_revision,
        revision: draft.base_revision.unwrap_or(0).saturating_add(1),
        changed_at: now,
        idempotency_key: draft.idempotency_key,
        payload: draft.payload.clone(),
        merge: None,
    };
    let conflict = ConflictRecord {
        conflict_id,
        scope: draft.scope.clone(),
        record_id: draft.record_id,
        local,
        remote,
        detected_at: now,
        status: ConflictStatus::Open,
        resolution: None,
    };
    sqlx::query(
        "INSERT INTO sync.conflicts
             (tenant_id, scope_kind, scope_id, schema_id, conflict_id,
              record_id, conflict_json, status, detected_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'open', $8)",
    )
    .bind(tenant_id.value())
    .bind(draft.scope.kind.as_str())
    .bind(draft.scope.id.value())
    .bind(draft.schema_id.as_str())
    .bind(conflict_id.value())
    .bind(draft.record_id.value())
    .bind(
        serde_json::to_value(&conflict)
            .map_err(|_| sqlx::Error::Protocol("conflict serialization failed".to_owned()))?,
    )
    .bind(now.0)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO sync.idempotency_results
            (tenant_id, scope_kind, scope_id, schema_id, idempotency_key,
             request_fingerprint, result_json, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(tenant_id.value())
    .bind(draft.scope.kind.as_str())
    .bind(draft.scope.id.value())
    .bind(draft.schema_id.as_str())
    .bind(draft.idempotency_key.value())
    .bind(fingerprint)
    .bind(
        serde_json::to_value(StoredLocalResult::Conflict(conflict_id))
            .map_err(|_| sqlx::Error::Protocol("idempotency serialization failed".to_owned()))?,
    )
    .bind(now.0)
    .execute(&mut **transaction)
    .await?;
    Ok(Some(conflict_id))
}

async fn commit_change(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    draft: &LocalOperationDraft,
    current_revision: u64,
    fingerprint: &[u8],
    now: UnixMillis,
) -> Result<ChangeRecord, OperationError> {
    let new_revision = current_revision.saturating_add(1);
    let sequence = advance_head(transaction, tenant_id, &draft.scope, &draft.schema_id)
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let checkpoint = Uuid::new_v4();
    let change = ChangeRecord {
        change_id: ChangeId::new(Uuid::new_v4()),
        record_id: draft.record_id,
        scope: draft.scope.clone(),
        operation: draft.operation,
        base_revision: draft.base_revision,
        revision: new_revision,
        changed_at: now,
        idempotency_key: draft.idempotency_key,
        payload: draft.payload.clone(),
        merge: None,
    };

    sqlx::query(
        "INSERT INTO sync.operations
             (tenant_id, scope_kind, scope_id, schema_id, sequence, checkpoint,
              change_id, idempotency_key, request_fingerprint, change_json,
              created_at, retention_until)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(tenant_id.value())
    .bind(draft.scope.kind.as_str())
    .bind(draft.scope.id.value())
    .bind(draft.schema_id.as_str())
    .bind(i64::try_from(sequence).unwrap_or(i64::MAX))
    .bind(checkpoint)
    .bind(change.change_id.value())
    .bind(change.idempotency_key.value())
    .bind(fingerprint.to_vec())
    .bind(serde_json::to_value(&change).map_err(|_| OperationError::Unavailable)?)
    .bind(now.0)
    .bind(now.0 + OPERATION_RETENTION_MS)
    .execute(&mut **transaction)
    .await
    .map_err(|_| OperationError::Unavailable)?;

    write_projection(transaction, tenant_id, draft, &change, now)
        .await
        .map_err(|_| OperationError::Unavailable)?;

    sqlx::query(
        "INSERT INTO sync.idempotency_results
             (tenant_id, scope_kind, scope_id, schema_id, idempotency_key,
              request_fingerprint, result_json, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(tenant_id.value())
    .bind(draft.scope.kind.as_str())
    .bind(draft.scope.id.value())
    .bind(draft.schema_id.as_str())
    .bind(change.idempotency_key.value())
    .bind(fingerprint.to_vec())
    .bind(
        serde_json::to_value(StoredLocalResult::Applied(Box::new(change.clone())))
            .map_err(|_| OperationError::Unavailable)?,
    )
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| OperationError::Unavailable)?;

    mark_checkpoint(
        transaction,
        tenant_id,
        &draft.scope,
        &draft.schema_id,
        sequence,
        checkpoint,
    )
    .await
    .map_err(|_| OperationError::Unavailable)?;
    publish_notice(
        transaction,
        tenant_id,
        draft.scope.clone(),
        &draft.schema_id,
        &change,
        now,
    )
    .await
    .map_err(|_| OperationError::Unavailable)?;
    Ok(change)
}

async fn commit_authoritative_change(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    command: &CommandSubmission,
    fingerprint: &[u8],
    draft: AuthoritativeChangeDraft,
    now: UnixMillis,
) -> Result<Option<ChangeRecord>, OperationError> {
    let current_revision = current_record_revision_for(
        transaction,
        tenant_id,
        &command.scope,
        &command.schema_id,
        command.record_id,
    )
    .await
    .map_err(|_| OperationError::Unavailable)?;
    let stale = command
        .base_revision
        .is_some_and(|base| base != current_revision);
    if stale {
        return Err(OperationError::Invalid);
    }

    let new_revision = current_revision.saturating_add(1);
    let sequence = advance_head(transaction, tenant_id, &command.scope, &command.schema_id)
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let checkpoint = Uuid::new_v4();
    let change = build_authoritative_change(command, &draft, new_revision, now);

    sqlx::query(
        "INSERT INTO sync.operations
             (tenant_id, scope_kind, scope_id, schema_id, sequence, checkpoint,
              change_id, idempotency_key, request_fingerprint, change_json,
              created_at, retention_until)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(tenant_id.value())
    .bind(command.scope.kind.as_str())
    .bind(command.scope.id.value())
    .bind(command.schema_id.as_str())
    .bind(i64::try_from(sequence).unwrap_or(i64::MAX))
    .bind(checkpoint)
    .bind(change.change_id.value())
    .bind(change.idempotency_key.value())
    .bind(fingerprint.to_vec())
    .bind(serde_json::to_value(&change).map_err(|_| OperationError::Unavailable)?)
    .bind(now.0)
    .bind(now.0 + OPERATION_RETENTION_MS)
    .execute(&mut **transaction)
    .await
    .map_err(|_| OperationError::Unavailable)?;

    write_projection_from(
        transaction,
        tenant_id,
        &command.scope,
        &command.schema_id,
        &change,
        now,
    )
    .await
    .map_err(|_| OperationError::Unavailable)?;

    let disposition = CommandDisposition::Accepted {
        authoritative_change: Some(Box::new(change.clone())),
    };
    sqlx::query(
        "INSERT INTO sync.idempotency_results
             (tenant_id, scope_kind, scope_id, schema_id, idempotency_key,
              request_fingerprint, result_json, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(tenant_id.value())
    .bind(command.scope.kind.as_str())
    .bind(command.scope.id.value())
    .bind(command.schema_id.as_str())
    .bind(change.idempotency_key.value())
    .bind(fingerprint.to_vec())
    .bind(serde_json::to_value(&disposition).map_err(|_| OperationError::Unavailable)?)
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| OperationError::Unavailable)?;

    mark_checkpoint(
        transaction,
        tenant_id,
        &command.scope,
        &command.schema_id,
        sequence,
        checkpoint,
    )
    .await
    .map_err(|_| OperationError::Unavailable)?;
    publish_notice(
        transaction,
        tenant_id,
        command.scope.clone(),
        &command.schema_id,
        &change,
        now,
    )
    .await
    .map_err(|_| OperationError::Unavailable)?;
    Ok(Some(change))
}

fn build_authoritative_change(
    command: &CommandSubmission,
    draft: &AuthoritativeChangeDraft,
    revision: u64,
    now: UnixMillis,
) -> ChangeRecord {
    ChangeRecord {
        change_id: ChangeId::new(Uuid::new_v4()),
        record_id: command.record_id,
        scope: command.scope.clone(),
        operation: draft.operation,
        base_revision: command.base_revision,
        revision,
        changed_at: now,
        idempotency_key: command.idempotency_key,
        payload: draft.payload.clone(),
        merge: None,
    }
}

async fn store_denial(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    command: &CommandSubmission,
    fingerprint: &[u8],
    now: UnixMillis,
) -> Result<(), sqlx::Error> {
    let reason =
        eitmad_contracts::sync::ErrorCodeRef::parse("eitmad.error.authorization-denied.v1")
            .map_err(|_| sqlx::Error::Protocol("stable denial code missing".to_owned()))?;
    let disposition = CommandDisposition::Denied { reason };
    sqlx::query(
        "INSERT INTO sync.idempotency_results
             (tenant_id, scope_kind, scope_id, schema_id, idempotency_key,
              request_fingerprint, result_json, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(tenant_id.value())
    .bind(command.scope.kind.as_str())
    .bind(command.scope.id.value())
    .bind(command.schema_id.as_str())
    .bind(command.idempotency_key.value())
    .bind(fingerprint.to_vec())
    .bind(
        serde_json::to_value(&disposition)
            .map_err(|_| sqlx::Error::Protocol("serialize".to_owned()))?,
    )
    .bind(now.0)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn advance_head(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: &ScopeRef,
    schema_id: &eitmad_contracts::transport::SchemaId,
) -> Result<u64, sqlx::Error> {
    let next: i64 = sqlx::query_scalar(
        "UPDATE sync.scopes SET head_sequence = head_sequence + 1
         WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3 AND schema_id = $4
         RETURNING head_sequence",
    )
    .bind(tenant_id.value())
    .bind(scope.kind.as_str())
    .bind(scope.id.value())
    .bind(schema_id.as_str())
    .fetch_one(&mut **transaction)
    .await?;
    u64::try_from(next).map_err(|_| sqlx::Error::Protocol("sequence overflow".to_owned()))
}

async fn write_projection(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    draft: &LocalOperationDraft,
    change: &ChangeRecord,
    now: UnixMillis,
) -> Result<(), sqlx::Error> {
    write_projection_from(
        transaction,
        tenant_id,
        &draft.scope,
        &draft.schema_id,
        change,
        now,
    )
    .await
}

async fn write_projection_from(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: &ScopeRef,
    schema_id: &eitmad_contracts::transport::SchemaId,
    change: &ChangeRecord,
    now: UnixMillis,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sync.records
             (tenant_id, scope_kind, scope_id, schema_id, record_id,
              revision, tombstone, change_json, changed_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (tenant_id, scope_kind, scope_id, schema_id, record_id)
         DO UPDATE SET revision = EXCLUDED.revision,
                       tombstone = EXCLUDED.tombstone,
                       change_json = EXCLUDED.change_json,
                       changed_at = EXCLUDED.changed_at",
    )
    .bind(tenant_id.value())
    .bind(scope.kind.as_str())
    .bind(scope.id.value())
    .bind(schema_id.as_str())
    .bind(change.record_id.value())
    .bind(i64::try_from(change.revision).unwrap_or(i64::MAX))
    .bind(change.operation == ChangeOperation::Tombstone)
    .bind(serde_json::to_value(change).map_err(|_| sqlx::Error::Protocol("serialize".to_owned()))?)
    .bind(now.0)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn mark_checkpoint(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: &ScopeRef,
    schema_id: &eitmad_contracts::transport::SchemaId,
    sequence: u64,
    checkpoint: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE sync.scopes SET head_checkpoint = $5
         WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3 AND schema_id = $4",
    )
    .bind(tenant_id.value())
    .bind(scope.kind.as_str())
    .bind(scope.id.value())
    .bind(schema_id.as_str())
    .bind(checkpoint)
    .execute(&mut **transaction)
    .await?;
    let _ = sequence;
    Ok(())
}

async fn publish_notice(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: ScopeRef,
    schema_id: &eitmad_contracts::transport::SchemaId,
    change: &ChangeRecord,
    now: UnixMillis,
) -> Result<(), sqlx::Error> {
    crate::subscriptions::append_event(
        transaction,
        tenant_id,
        scope,
        schema_id.clone(),
        serde_json::to_value(RecordChangeNotice {
            record_id: change.record_id,
            scope: change.scope.clone(),
            schema_id: schema_id.clone(),
            operation: change.operation,
            revision: change.revision,
            changed_at: now,
        })
        .map_err(|_| sqlx::Error::Protocol("serialize".to_owned()))?,
        now,
    )
    .await
}

async fn current_record_revision_for(
    transaction: &mut Tx<'_>,
    tenant_id: eitmad_contracts::identity::TenantId,
    scope: &ScopeRef,
    schema_id: &eitmad_contracts::transport::SchemaId,
    record_id: RecordId,
) -> Result<u64, sqlx::Error> {
    let revision: Option<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM sync.records
         WHERE tenant_id = $1 AND scope_kind = $2 AND scope_id = $3
           AND schema_id = $4 AND record_id = $5",
    )
    .bind(tenant_id.value())
    .bind(scope.kind.as_str())
    .bind(scope.id.value())
    .bind(schema_id.as_str())
    .bind(record_id.value())
    .fetch_optional(&mut **transaction)
    .await?;
    Ok(u64::try_from(revision.unwrap_or(0)).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use eitmad_contracts::identity::{
        AccountId, DeviceId, ScopeId, ScopeKind, SessionId, TenantId, UserId,
    };
    use eitmad_contracts::transport::{IdempotencyKey, SchemaId};

    struct DenyingHandler;

    #[async_trait::async_trait]
    impl crate::domain::DomainSyncHandler for DenyingHandler {
        fn descriptor(&self) -> crate::domain::DomainDescriptor {
            crate::domain::DomainDescriptor {
                schema_id: SchemaId::parse("eitmad.schema.test.notes.v1").unwrap(),
                minimum_schema_version: 1,
                maximum_schema_version: 1,
                mode: SyncMode::LocalFirst,
            }
        }

        async fn authorize(
            &self,
            _session: &AuthenticatedServerSession,
            _scope: &ScopeRef,
            _intent: SyncIntent,
        ) -> bool {
            false
        }

        fn validate_local(
            &self,
            _draft: &LocalOperationDraft,
        ) -> Result<(), DomainValidationError> {
            panic!("authorization must run before validation")
        }

        fn execute_command(
            &self,
            _session: &AuthenticatedServerSession,
            _command: &CommandSubmission,
        ) -> Result<AuthoritativeChangeDraft, DomainValidationError> {
            panic!("authorization must run before command execution")
        }
    }

    fn draft() -> LocalOperationDraft {
        LocalOperationDraft {
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(Uuid::from_u128(1)),
            },
            schema_id: SchemaId::parse("eitmad.schema.test.notes.v1").unwrap(),
            schema_version: 1,
            record_id: RecordId::new(Uuid::from_u128(2)),
            operation: ChangeOperation::Upsert,
            base_revision: None,
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(3)),
            payload: None,
        }
    }

    #[test]
    fn fingerprints_are_deterministic_and_intent_sensitive() {
        let first = fingerprint_draft(&draft());
        let second = fingerprint_draft(&draft());
        assert_eq!(first, second);

        let mut changed = draft();
        changed.base_revision = Some(4);
        assert_ne!(first, fingerprint_draft(&changed));
    }

    #[test]
    fn duplicate_conflict_results_replay_the_same_conflict_id() {
        let conflict_id = ConflictId::new(Uuid::from_u128(9));
        let encoded = serde_json::to_value(StoredLocalResult::Conflict(conflict_id)).unwrap();
        let decoded = serde_json::from_value::<StoredLocalResult>(encoded).unwrap();
        assert!(matches!(
            decoded,
            StoredLocalResult::Conflict(value) if value == conflict_id
        ));
    }

    #[tokio::test]
    async fn unauthorized_local_changes_fail_closed_when_mandatory_audit_is_unavailable() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgresql://unreachable.invalid/eitmad")
            .unwrap();
        let registry = DomainRegistry::new([
            Arc::new(DenyingHandler) as Arc<dyn crate::domain::DomainSyncHandler>
        ])
        .unwrap();
        let coordinator = SyncCoordinator { pool, registry };
        let session = AuthenticatedServerSession {
            session_id: SessionId::new(Uuid::from_u128(10)),
            account_id: AccountId::new(Uuid::from_u128(11)),
            user_id: UserId::new(Uuid::from_u128(12)),
            device_id: DeviceId::new(Uuid::from_u128(13)),
            tenant_id: TenantId::new(Uuid::from_u128(14)),
            issued_at: UnixMillis(1),
            expires_at: UnixMillis(2),
        };
        assert_eq!(
            coordinator
                .apply_local_operation(
                    &session,
                    &draft(),
                    CorrelationId::new(Uuid::from_u128(15)),
                    UnixMillis(1)
                )
                .await,
            Err(OperationError::Unavailable)
        );
    }
}
