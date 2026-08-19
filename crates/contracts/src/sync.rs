use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    identity::{AuthorizationContext, ScopeRef},
    transport::{IdempotencyKey, SchemaId, UnixMillis},
    versioning::PeerHello,
};

pub const MAX_SYNC_BATCH_RECORDS: usize = 500;

uuid_id!(Checkpoint);
uuid_id!(RecordId);
uuid_id!(ChangeId);
uuid_id!(SnapshotId);
uuid_id!(PendingCommandId);
uuid_id!(ConflictId);
uuid_id!(DeliveryId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncMode {
    LocalFirst,
    ServerAuthoritative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChangeOperation {
    Upsert,
    Tombstone,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EncodedDomainPayload {
    pub schema_id: SchemaId,
    pub schema_version: u32,
    pub base64: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MergeStrategy {
    KeepLocal,
    KeepRemote,
    DomainMerge,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MergeMetadata {
    pub strategy: MergeStrategy,
    pub common_ancestor_revision: Option<u64>,
    pub source_changes: Vec<ChangeId>,
    pub merged_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeRecord {
    pub change_id: ChangeId,
    pub record_id: RecordId,
    pub scope: ScopeRef,
    pub operation: ChangeOperation,
    pub base_revision: Option<u64>,
    pub revision: u64,
    pub changed_at: UnixMillis,
    pub idempotency_key: IdempotencyKey,
    pub payload: Option<EncodedDomainPayload>,
    pub merge: Option<MergeMetadata>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecordChangeNotice {
    pub record_id: RecordId,
    pub scope: ScopeRef,
    pub schema_id: SchemaId,
    pub operation: ChangeOperation,
    pub revision: u64,
    pub changed_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeBatch {
    pub delivery_id: DeliveryId,
    pub idempotency_key: IdempotencyKey,
    pub from_checkpoint: Option<Checkpoint>,
    pub checkpoint: Checkpoint,
    pub records: Vec<ChangeRecord>,
    pub has_more: bool,
}

impl ChangeBatch {
    /// Creates a bounded synchronization batch.
    ///
    /// # Errors
    ///
    /// Returns [`SyncBatchSizeError`] when the record count exceeds
    /// [`MAX_SYNC_BATCH_RECORDS`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        delivery_id: DeliveryId,
        idempotency_key: IdempotencyKey,
        from_checkpoint: Option<Checkpoint>,
        checkpoint: Checkpoint,
        records: Vec<ChangeRecord>,
        has_more: bool,
    ) -> Result<Self, SyncBatchSizeError> {
        if records.len() <= MAX_SYNC_BATCH_RECORDS {
            Ok(Self {
                delivery_id,
                idempotency_key,
                from_checkpoint,
                checkpoint,
                records,
                has_more,
            })
        } else {
            Err(SyncBatchSizeError {
                records: records.len(),
            })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncBatchSizeError {
    pub records: usize,
}

impl std::fmt::Display for SyncBatchSizeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "sync batches may contain at most {MAX_SYNC_BATCH_RECORDS} records"
        )
    }
}

impl std::error::Error for SyncBatchSizeError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncSnapshot {
    pub snapshot_id: SnapshotId,
    pub scope: ScopeRef,
    pub checkpoint: Checkpoint,
    pub server_generation: u64,
    pub created_at: UnixMillis,
    pub valid_until: UnixMillis,
    pub records: Vec<ChangeRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PendingCommand {
    pub command_id: PendingCommandId,
    pub scope: ScopeRef,
    pub actor: AuthorizationContext,
    pub idempotency_key: IdempotencyKey,
    pub submitted_at: UnixMillis,
    pub command_schema: SchemaId,
    pub command_schema_version: u32,
    pub base_revision: Option<u64>,
    pub base64: String,
    pub optimistic_change: Option<ChangeRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConflictStatus {
    Open,
    Resolved,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConflictRecord {
    pub conflict_id: ConflictId,
    pub scope: ScopeRef,
    pub record_id: RecordId,
    pub local: ChangeRecord,
    pub remote: ChangeRecord,
    pub detected_at: UnixMillis,
    pub status: ConflictStatus,
    pub resolution: Option<MergeMetadata>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Offline,
    Connected,
    Incompatible,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncMetadata {
    pub mode: SyncMode,
    pub connection: ConnectionState,
    pub checkpoint: Option<Checkpoint>,
    pub last_successful_sync_at: Option<UnixMillis>,
    pub server_generation: Option<u64>,
    pub cache_valid_until: Option<UnixMillis>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncNegotiation {
    pub mode: SyncMode,
    pub peer: PeerHello,
    pub checkpoint: Option<Checkpoint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub after: Option<Checkpoint>,
    #[schemars(range(min = 1, max = 500))]
    pub maximum_records: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchAcknowledgement {
    pub delivery_id: DeliveryId,
    pub checkpoint: Checkpoint,
    pub accepted_records: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", content = "payload", rename_all = "camelCase")]
pub enum CommandDisposition {
    Accepted {
        authoritative_change: Option<Box<ChangeRecord>>,
    },
    Denied {
        reason: ErrorCodeRef,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub command_id: PendingCommandId,
    pub disposition: CommandDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationDelivery {
    pub delivery_id: DeliveryId,
    pub idempotency_key: IdempotencyKey,
    pub checkpoint: Checkpoint,
    pub received_at: UnixMillis,
    pub snapshot: Option<SyncSnapshot>,
    pub changes: Vec<ChangeRecord>,
    pub command_results: Vec<CommandResult>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConflictNotice {
    pub conflict_id: ConflictId,
    pub record_id: RecordId,
    pub local_revision: u64,
    pub remote_revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetryAfter {
    pub delay_ms: u64,
    pub reason: ErrorCodeRef,
}

open_id!(ErrorCodeRef, "sync retry reason");

tagged_contract! {
    pub enum SyncMessage {
        Negotiate(SyncNegotiation) => "eitmad.sync.negotiate.v1",
        Pull(PullRequest) => "eitmad.sync.pull.v1",
        Changes(ChangeBatch) => "eitmad.sync.changes.v1",
        Reconcile(ReconciliationDelivery) => "eitmad.sync.reconcile.v1",
        Acknowledge(BatchAcknowledgement) => "eitmad.sync.acknowledge.v1",
        Conflict(ConflictNotice) => "eitmad.sync.conflict.v1",
        Backpressure(RetryAfter) => "eitmad.sync.backpressure.v1"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum SyncStatus {
    Offline,
    Current { checkpoint: Checkpoint },
    Queued { records: u64 },
    Syncing { completed: u64, total: Option<u64> },
    Conflicted { records: u64 },
    Failed { reason: ErrorCodeRef },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CacheFreshness {
    Fresh,
    Stale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RecordAuthority {
    LocalDurable,
    ServerConfirmed,
    Optimistic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecordView {
    pub record: ChangeRecord,
    pub authority: RecordAuthority,
    pub freshness: CacheFreshness,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum SyncEvent {
    ConnectionChanged(ConnectionState),
    LocalChangeQueued(ChangeRecord),
    CommandQueued(PendingCommand),
    ConflictDetected(ConflictRecord),
    ConflictResolved(ConflictRecord),
    CommandDenied {
        command_id: PendingCommandId,
        reason: ErrorCodeRef,
    },
    SnapshotApplied(SyncSnapshot),
    DuplicateDeliveryIgnored(DeliveryId),
    StatusChanged(SyncStatus),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{ScopeId, ScopeKind};
    use uuid::Uuid;

    #[test]
    fn sync_batches_are_bounded() {
        let fake_record = ChangeRecord {
            change_id: ChangeId::new(Uuid::from_u128(1)),
            record_id: RecordId::new(Uuid::from_u128(2)),
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(Uuid::from_u128(3)),
            },
            operation: ChangeOperation::Tombstone,
            base_revision: None,
            revision: 1,
            changed_at: UnixMillis(0),
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(4)),
            payload: None,
            merge: None,
        };
        let result = ChangeBatch::new(
            DeliveryId::new(Uuid::from_u128(5)),
            IdempotencyKey::new(Uuid::from_u128(6)),
            None,
            Checkpoint::new(Uuid::from_u128(7)),
            vec![fake_record; MAX_SYNC_BATCH_RECORDS + 1],
            false,
        );
        assert!(result.is_err());
    }
}
