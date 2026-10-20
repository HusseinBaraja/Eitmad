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

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
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
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum WindowsInstallStatus {
    Running,
    Succeeded,
    Failed,
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
}
