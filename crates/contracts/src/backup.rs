//! Privileged local server-backup boundary v1. No credentials cross this boundary.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerBackupConfiguration {
    pub version: u32,
    pub deployment_id: uuid::Uuid,
    pub host_root: String,
    pub service_name: String,
    pub recovery_directory: String,
    pub local_destination: String,
    pub external_destination: Option<String>,
    #[serde(default = "default_interval")]
    pub interval_hours: u32,
    #[serde(default = "default_retention")]
    pub retention_count: usize,
    #[serde(default = "default_reserve")]
    pub reserve_bytes: u64,
}
const fn default_interval() -> u32 {
    24
}
const fn default_retention() -> usize {
    7
}
const fn default_reserve() -> u64 {
    1_073_741_824
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ServerBackupOutcome {
    NeverRun,
    Running,
    Succeeded,
    DestinationUnavailable,
    InsufficientSpace,
    CaptureFailed,
    Interrupted,
    RetentionFailed,
    ScheduleFailed,
}

/// Safe for later onboarding UI. Paths, credentials, and tool output are excluded.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerBackupStatus {
    pub version: u32,
    pub deployment_id: uuid::Uuid,
    pub last_attempt_at: Option<u64>,
    pub last_local_success_at: Option<u64>,
    pub last_external_success_at: Option<u64>,
    pub local_outcome: ServerBackupOutcome,
    pub external_outcome: ServerBackupOutcome,
    pub next_due_at: u64,
    pub overdue: bool,
    pub external_configured: bool,
    /// Only a copy verified on a different physical disk can set this value.
    pub disk_loss_protected: bool,
    pub schedule_outcome: ServerBackupOutcome,
}
