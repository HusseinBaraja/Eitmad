//! Native onboarding over the authenticated local process channel. Codes remain transient.
use crate::{
    discovery::DiscoveredServer,
    transport::{CorrelationId, RequestId},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum DesktopOnboardingAction {
    State,
    Pair {
        candidate: DiscoveredServer,
        code: String,
    },
    /// Explicitly abandon only the pending exchange; retain identity, trust, and data.
    DiscardPendingPairing,
    OpenPairing,
}
impl std::fmt::Debug for DesktopOnboardingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::State => "State",
            Self::Pair { .. } => "Pair([redacted])",
            Self::DiscardPendingPairing => "DiscardPendingPairing",
            Self::OpenPairing => "OpenPairing",
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopOnboardingRequest {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub action: DesktopOnboardingAction,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopOnboardingState {
    pub paired: bool,
    /// The saved completion must be retried against this server until explicitly discarded.
    pub pending_pairing_server: Option<crate::discovery::ServerInstallationId>,
    pub code: Option<String>,
    pub expires_at: Option<crate::transport::UnixMillis>,
}
impl std::fmt::Debug for DesktopOnboardingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DesktopOnboardingState([redacted])")
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopOnboardingResponse {
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub state: Option<DesktopOnboardingState>,
    pub error: Option<crate::errors::ContractError>,
}
