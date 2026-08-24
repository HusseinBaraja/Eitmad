//! Versioned, redacted contracts for least-privilege server administration.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    identity::{DeviceId, PrincipalId, TenantId},
    transport::{CorrelationId, UnixMillis},
};

uuid_id!(SupportWorkflowId);
uuid_id!(AdministrativeAuditId);
open_id!(ServiceComponentId, "server component identifier");
open_id!(AdministrativeFailureCode, "administrative failure code");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ServiceHealthState {
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealth {
    pub component: ServiceComponentId,
    pub state: ServiceHealthState,
    pub checked_at: UnixMillis,
    pub failure_code: Option<AdministrativeFailureCode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum BackupState {
    Current,
    Stale,
    Running,
    Failed,
    NotConfigured,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub state: BackupState,
    pub last_success_at: Option<UnixMillis>,
    pub last_verified_at: Option<UnixMillis>,
    pub next_scheduled_at: Option<UnixMillis>,
    pub recovery_point_age_ms: Option<u64>,
    pub failure_code: Option<AdministrativeFailureCode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MigrationState {
    Current,
    Pending,
    Running,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MigrationStatus {
    pub state: MigrationState,
    pub current_version: u32,
    pub required_version: u32,
    pub pending_migration_ids: Vec<String>,
    pub failure_code: Option<AdministrativeFailureCode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSummary {
    pub generated_at: UnixMillis,
    pub correlation_id: CorrelationId,
    pub services: Vec<ServiceHealth>,
    pub active_relay_sessions: u32,
    pub pending_support_workflows: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TenantVisibility {
    pub tenant_id: TenantId,
    pub enabled: bool,
    pub active_device_count: u32,
    pub active_session_count: u32,
    pub last_seen_at: Option<UnixMillis>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceVisibility {
    pub tenant_id: TenantId,
    pub device_id: DeviceId,
    pub label: String,
    pub revoked: bool,
    pub last_seen_at: Option<UnixMillis>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdministrativeAuditRecord {
    pub audit_id: AdministrativeAuditId,
    pub tenant_id: TenantId,
    pub principal_id: PrincipalId,
    pub operation: String,
    pub outcome: String,
    pub target_kind: String,
    pub correlation_id: CorrelationId,
    pub occurred_at: UnixMillis,
    pub redacted_error: Option<AdministrativeFailureCode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SupportAction {
    CollectDiagnostics,
    VerifyBackup,
    RetryMigration,
    DisconnectRelaySession {
        relay_session_id: crate::relay::RelaySessionId,
    },
    RevokeDeviceSessions {
        device_id: DeviceId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartSupportWorkflow {
    pub tenant_id: TenantId,
    pub action: SupportAction,
    pub reason_code: String,
    pub correlation_id: CorrelationId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SupportWorkflowState {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SupportWorkflow {
    pub workflow_id: SupportWorkflowId,
    pub tenant_id: TenantId,
    pub action: SupportAction,
    pub reason_code: String,
    pub state: SupportWorkflowState,
    pub requested_at: UnixMillis,
    pub completed_at: Option<UnixMillis>,
    pub failure_code: Option<AdministrativeFailureCode>,
}
