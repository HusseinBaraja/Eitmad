//! Versioned metadata for authenticated WAN relay coordination.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    identity::{DeviceId, TenantId},
    transport::{CorrelationId, UnixMillis},
};

uuid_id!(RelaySessionId);
uuid_id!(RelayFailureId);
open_id!(RelayServerId, "relay server identifier");
open_id!(RelayFailureCode, "relay failure code");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum RelayRoute {
    Peer { target_device_id: DeviceId },
    Server { target_server_id: RelayServerId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RelaySessionState {
    Connecting,
    Active,
    Reconnecting,
    Closed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelaySessionMetadata {
    pub relay_session_id: RelaySessionId,
    pub tenant_id: TenantId,
    pub source_device_id: DeviceId,
    pub route: RelayRoute,
    pub state: RelaySessionState,
    pub connected_at: UnixMillis,
    pub last_seen_at: UnixMillis,
    pub expires_at: UnixMillis,
    pub reconnect_attempt: u32,
    pub next_reconnect_at: Option<UnixMillis>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenRelaySession {
    pub tenant_id: TenantId,
    pub source_device_id: DeviceId,
    pub route: RelayRoute,
    pub requested_ttl_ms: u64,
    pub correlation_id: CorrelationId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RelayFailurePhase {
    Authorization,
    Connect,
    Route,
    Heartbeat,
    Reconnect,
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelayFailureReport {
    pub failure_id: RelayFailureId,
    pub relay_session_id: Option<RelaySessionId>,
    pub tenant_id: TenantId,
    pub source_device_id: DeviceId,
    pub phase: RelayFailurePhase,
    pub code: RelayFailureCode,
    pub retryable: bool,
    pub retry_after_ms: Option<u64>,
    pub occurred_at: UnixMillis,
    pub correlation_id: CorrelationId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RelayHealthState {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelayHealth {
    pub tenant_id: TenantId,
    pub state: RelayHealthState,
    pub checked_at: UnixMillis,
    pub active_sessions: u32,
    pub reconnecting_sessions: u32,
    pub failed_sessions: u32,
    pub accepting_sessions: bool,
}
