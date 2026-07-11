use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    config::ConfigSnapshot, permissions::EffectivePermissions, sync::SyncStatus,
    updates::UpdateState,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ConfigurationChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PermissionChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateStateChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SyncStatusChanges {}

tagged_contract! {
    /// Resumable streams requested by clients.
    pub enum Subscription {
        Configuration(ConfigurationChanges) => "eitmad.config.changed.subscribe.v1",
        Permissions(PermissionChanges) => "eitmad.permissions.changed.subscribe.v1",
        UpdateState(UpdateStateChanges) => "eitmad.update.state.subscribe.v1",
        SyncStatus(SyncStatusChanges) => "eitmad.sync.status.subscribe.v1"
    }
}

tagged_contract! {
    /// Ordered values emitted by subscriptions.
    pub enum Event {
        ConfigurationChanged(ConfigSnapshot) => "eitmad.config.changed.event.v1",
        PermissionsChanged(EffectivePermissions) => "eitmad.permissions.changed.event.v1",
        UpdateStateChanged(UpdateState) => "eitmad.update.state.event.v1",
        SyncStatusChanged(SyncStatus) => "eitmad.sync.status.event.v1"
    }
}
