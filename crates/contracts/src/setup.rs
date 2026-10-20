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
