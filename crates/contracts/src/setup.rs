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
