use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    identity::ScopeRef,
    transport::{SchemaId, UnixMillis},
};

pub const MAX_SYNC_BATCH_RECORDS: usize = 500;

uuid_id!(Checkpoint);
uuid_id!(RecordId);

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeRecord {
    pub record_id: RecordId,
    pub scope: ScopeRef,
    pub operation: ChangeOperation,
    pub revision: u64,
    pub changed_at: UnixMillis,
    pub payload: Option<EncodedDomainPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeBatch {
    pub from_checkpoint: Option<Checkpoint>,
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
    pub fn new(
        from_checkpoint: Option<Checkpoint>,
        records: Vec<ChangeRecord>,
        has_more: bool,
    ) -> Result<Self, SyncBatchSizeError> {
        if records.len() <= MAX_SYNC_BATCH_RECORDS {
            Ok(Self {
                from_checkpoint,
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
pub struct SyncNegotiation {
    pub mode: SyncMode,
    pub schemas: Vec<SchemaId>,
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
    pub checkpoint: Checkpoint,
    pub accepted_records: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConflictNotice {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_batches_are_bounded() {
        let oversized = vec![];
        assert!(ChangeBatch::new(None, oversized, false).is_ok());

        let fake_record = ChangeRecord {
            record_id: RecordId::new(uuid::Uuid::nil()),
            scope: crate::identity::ScopeRef {
                kind: crate::identity::ScopeKind::parse("organization").unwrap(),
                id: crate::identity::ScopeId::new(uuid::Uuid::nil()),
            },
            operation: ChangeOperation::Tombstone,
            revision: 1,
            changed_at: UnixMillis(0),
            payload: None,
        };
        assert!(
            ChangeBatch::new(None, vec![fake_record; MAX_SYNC_BATCH_RECORDS + 1], false).is_err()
        );
    }
}
