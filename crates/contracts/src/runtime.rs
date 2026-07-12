use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    errors::ContractError,
    transport::{PROTOCOL_VERSION, UnixMillis},
    updates::ReleaseVersion,
    versioning::ProtocolVersion,
};

uuid_id!(EngineInstanceId);
open_id!(HealthCheckId, "health check identifier");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EngineMode {
    SupervisedDesktop,
    Headless,
    Diagnostic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleState {
    Starting,
    Ready,
    Stopping,
    Stopped,
    Failed,
}

impl LifecycleState {
    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Starting | Self::Ready | Self::Stopping)
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HealthCheckImpact {
    RequiredForReadiness,
    Advisory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleStage {
    ProcessIdentity,
    AuthorityLock,
    ComponentStartup,
    ReadinessCheck,
    ComponentShutdown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EngineProcessIdentity {
    pub instance_id: EngineInstanceId,
    pub process_id: u32,
    pub mode: EngineMode,
    pub started_at: UnixMillis,
    pub product_version: ReleaseVersion,
    pub protocol_version: ProtocolVersion,
}

impl EngineProcessIdentity {
    #[must_use]
    pub fn new(
        instance_id: EngineInstanceId,
        process_id: u32,
        mode: EngineMode,
        started_at: UnixMillis,
        product_version: ReleaseVersion,
    ) -> Self {
        Self {
            instance_id,
            process_id,
            mode,
            started_at,
            product_version,
            protocol_version: PROTOCOL_VERSION,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckResult {
    pub id: HealthCheckId,
    pub status: HealthStatus,
    pub impact: HealthCheckImpact,
    pub observed_at: UnixMillis,
    pub error: Option<ContractError>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleSnapshot {
    pub identity: EngineProcessIdentity,
    pub state: LifecycleState,
    pub live: bool,
    pub ready: bool,
    pub health: HealthStatus,
    pub checks: Vec<HealthCheckResult>,
    pub observed_at: UnixMillis,
    pub error: Option<ContractError>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub identity: EngineProcessIdentity,
    pub status: HealthStatus,
    pub ready_to_start: bool,
    pub checks: Vec<HealthCheckResult>,
    pub observed_at: UnixMillis,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_active_lifecycle_states_are_live() {
        assert!(LifecycleState::Starting.is_live());
        assert!(LifecycleState::Ready.is_live());
        assert!(LifecycleState::Stopping.is_live());
        assert!(!LifecycleState::Stopped.is_live());
        assert!(!LifecycleState::Failed.is_live());
    }
}
