//! Version-one offline workshop initialization. Passwords travel only over private local input.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    accounts::AccountPassword,
    identity::{AccountId, OrganizationId, ScopeId, TenantId},
    server::TenantCode,
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitializeWorkshop {
    pub version: u32,
    pub workshop_name: String,
    pub owner_username: String,
    pub owner_password: AccountPassword,
}

/// Public enrollment coordinates, without credentials or an activation token.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitializedWorkshop {
    pub version: u32,
    pub tenant_code: TenantCode,
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub branch_id: ScopeId,
    pub owner_account_id: AccountId,
}

/// Local deployment input. Paths identify reviewed offline installation tools.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareWindowsWorkshop {
    pub version: u32,
    pub state_directory: String,
    pub platform_script: String,
    pub postgres_bin: String,
    pub certificate_executable: String,
    pub server_name: String,
    pub listen_address: String,
    pub server_port: u16,
    pub database_port: u16,
    pub workshop: InitializeWorkshop,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedWindowsWorkshop {
    pub version: u32,
    pub endpoint: String,
    pub workshop: InitializedWorkshop,
}

/// Private local installer input. Never persist or log this request.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum InstallWindows {
    Server(Box<InstallWindowsServer>),
    Client(InstallWindowsClient),
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowsInstallPackage {
    pub directory: String,
    pub release_sha256: String,
    pub publisher: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallWindowsClient {
    pub version: u32,
    pub progress_directory: String,
    /// The initiating Windows user's SID, checked again immediately before registration.
    pub intended_user_sid: String,
    pub desktop: WindowsInstallPackage,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallWindowsServer {
    pub version: u32,
    pub client: InstallWindowsClient,
    pub preparation: PrepareWindowsWorkshop,
    pub server: WindowsInstallPackage,
    pub service_state_directory: String,
    pub service_name: String,
    pub server_progress_directory: String,
    pub winsw_executable: String,
    pub winsw_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_desktop: Option<crate::server::LoginRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup: Option<SetupBackupDestinations>,
}

impl std::fmt::Debug for InstallWindowsServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InstallWindowsServer([redacted])")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupBackupDestinations {
    pub local: String,
    pub external: Option<String>,
}

/// Public trust returned only over the original user's authenticated elevation channel.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupDesktopHandoff {
    pub version: u32,
    pub prepared: PreparedWindowsWorkshop,
    pub public_ca_pem: String,
    pub server_id: crate::discovery::ServerInstallationId,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum GuidedSetupAction {
    Inspect,
    Retry,
    Client,
    Server,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuidedWindowsSetup {
    pub version: u32,
    pub action: GuidedSetupAction,
    pub bundle_directory: String,
    pub intended_user_sid: String,
    pub workshop: Option<InitializeWorkshop>,
    pub backup: Option<SetupBackupDestinations>,
    /// Retry credentials are transient and are never written to the recovery journal.
    pub owner_password: Option<AccountPassword>,
}

/// Fixed installation capability of an authenticated offline payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WindowsSetupRole {
    Server,
    Client,
}

/// Release metadata, emitted by offline bundle assembly. It contains no machine settings.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowsSetupBundle {
    pub version: u32,
    pub role: WindowsSetupRole,
    pub publisher: String,
    pub desktop_release_sha256: String,
    pub server_release_sha256: String,
    pub winsw_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WindowsInstallStage {
    Preflight,
    Elevation,
    PrepareWorkshop,
    ApplyMigrations,
    InitializeOwner,
    StopPreparedDatabase,
    InstallServices,
    ConfigureFirewall,
    VerifyHttps,
    RegisterDesktop,
    EnrollDesktop,
    ConfigureBackups,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WindowsInstallStatus {
    Running,
    Succeeded,
    Failed,
    /// State cannot be proved safe. Preserve it for an operator inspection.
    Blocked,
}

/// Public, ordered progress. Contains no user input, credentials, paths, or OS error text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowsInstallProgress {
    pub version: u32,
    pub sequence: u32,
    pub stage: WindowsInstallStage,
    pub status: WindowsInstallStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadWindowsInstallation {
    pub version: u32,
    pub progress_directory: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WindowsInstallationState {
    pub version: u32,
    pub progress: WindowsInstallProgress,
    pub interrupted: bool,
    pub recovery: WindowsSetupRecovery,
}

/// Rust decides which action is available; the native shell supplies localized copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WindowsSetupRecovery {
    Retry,
    OwnerPassword,
    Wait,
    InspectManually,
    Complete,
}
