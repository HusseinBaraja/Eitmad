use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

open_id!(PermissionId, "permission identifier");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PermissionDecision {
    Granted,
    Denied,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EffectivePermission {
    pub permission: PermissionId,
    pub decision: PermissionDecision,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EffectivePermissions {
    pub policy_version: u64,
    pub permissions: Vec<EffectivePermission>,
}
