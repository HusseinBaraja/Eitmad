use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    sync::SyncMessage,
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
    versioning::ProtocolVersion,
};

uuid_id!(SyncFrameId);
uuid_id!(SyncStreamId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SyncCancellationReason {
    ClientRequested,
    DeadlineExceeded,
    Superseded,
    ShuttingDown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncCancellation {
    pub stream_id: SyncStreamId,
    pub last_accepted_sequence: Option<u64>,
    pub reason: SyncCancellationReason,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum SyncTransportPayload {
    Message(SyncMessage),
    Cancel(SyncCancellation),
    Heartbeat { sent_at: UnixMillis },
    HeartbeatAcknowledged { sent_at: UnixMillis },
}

/// One transport-independent sync frame used by simulation, LAN, and WAN links.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SyncTransportFrame {
    pub frame_id: SyncFrameId,
    pub idempotency_key: IdempotencyKey,
    pub protocol_version: ProtocolVersion,
    pub correlation_id: CorrelationId,
    pub stream_id: SyncStreamId,
    pub sequence: u64,
    pub end_of_stream: bool,
    pub payload: SyncTransportPayload,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::sync::{PullRequest, SyncMessage};

    #[test]
    fn transport_frame_round_trips_with_one_sync_message_shape() {
        let frame = SyncTransportFrame {
            frame_id: SyncFrameId::new(Uuid::from_u128(1)),
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(2)),
            protocol_version: ProtocolVersion { major: 1, minor: 3 },
            correlation_id: CorrelationId::new(Uuid::from_u128(3)),
            stream_id: SyncStreamId::new(Uuid::from_u128(4)),
            sequence: 7,
            end_of_stream: true,
            payload: SyncTransportPayload::Message(SyncMessage::Pull(PullRequest {
                after: None,
                maximum_records: 100,
            })),
        };

        let value = serde_json::to_value(&frame).unwrap();
        assert_eq!(value["payload"]["kind"], "message");
        assert_eq!(value["payload"]["payload"]["kind"], "eitmad.sync.pull.v1");
        assert_eq!(
            serde_json::from_value::<SyncTransportFrame>(value).unwrap(),
            frame
        );
    }
}
