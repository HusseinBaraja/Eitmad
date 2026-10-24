//! Untrusted LAN candidates. These values never establish TLS trust or device authority.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::transport::{CorrelationId, RequestId, UnixMillis};

uuid_id!(ServerInstallationId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryStatus {
    Complete,
    Cancelled,
    NetworkChanged,
    Unavailable,
    Busy,
    AuthenticationRequired,
    LimitReached,
    ServerMissing,
    IdentityConflict,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveredServer {
    pub server_id: ServerInstallationId,
    /// A hint only. Pairing must authenticate the server identity and TLS name separately.
    pub address: String,
    pub port: u16,
    pub expires_at: UnixMillis,
    /// More than one running installation claims this identity. Never select it automatically.
    pub identity_conflict: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoverServers {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    /// Revalidate this untrusted identity before pairing; never substitute another server.
    pub selected_server_id: Option<ServerInstallationId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerDiscoveryResult {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub status: DiscoveryStatus,
    pub servers: Vec<DiscoveredServer>,
}
