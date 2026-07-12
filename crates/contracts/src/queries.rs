use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    config::ConfigSnapshot, permissions::EffectivePermissions, sync::SyncStatus,
    updates::UpdateState,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetConfiguration {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetEffectivePermissions {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetUpdateState {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GetSyncStatus {}

tagged_contract! {
    /// Authorized read-only requests.
    pub enum Query {
        Configuration(GetConfiguration) => "eitmad.config.get.v1",
        EffectivePermissions(GetEffectivePermissions) => "eitmad.permissions.get-effective.v1",
        UpdateState(GetUpdateState) => "eitmad.update.get-state.v1",
        SyncStatus(GetSyncStatus) => "eitmad.sync.get-status.v1"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum QueryResult {
    Configuration(ConfigSnapshot),
    EffectivePermissions(EffectivePermissions),
    UpdateState(UpdateState),
    SyncStatus(SyncStatus),
}
