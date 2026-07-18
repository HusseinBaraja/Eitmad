use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    background_jobs::BackgroundJobStatus,
    config::ConfigSnapshot,
    errors::ContractError,
    identity::ScopeRef,
    notifications::Notification,
    permissions::EffectivePermissions,
    sync::{RecordChangeNotice, SyncStatus},
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RecordChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BackgroundJobChanges {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Notifications {}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Errors {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScopedError {
    pub scope: ScopeRef,
    pub error: ContractError,
}

tagged_contract! {
    /// Resumable streams requested by clients.
    pub enum Subscription {
        Configuration(ConfigurationChanges) => "eitmad.config.changed.subscribe.v1",
        Permissions(PermissionChanges) => "eitmad.permissions.changed.subscribe.v1",
        UpdateState(UpdateStateChanges) => "eitmad.update.state.subscribe.v1",
        SyncStatus(SyncStatusChanges) => "eitmad.sync.status.subscribe.v1",
        RecordChanges(RecordChanges) => "eitmad.record.changed.subscribe.v1",
        BackgroundJobs(BackgroundJobChanges) => "eitmad.background-job.status.subscribe.v1",
        Notifications(Notifications) => "eitmad.notification.subscribe.v1",
        Errors(Errors) => "eitmad.error.subscribe.v1"
    }
}

tagged_contract! {
    /// Ordered values emitted by subscriptions.
    pub enum Event {
        ConfigurationChanged(ConfigSnapshot) => "eitmad.config.changed.event.v1",
        PermissionsChanged(EffectivePermissions) => "eitmad.permissions.changed.event.v1",
        UpdateStateChanged(UpdateState) => "eitmad.update.state.event.v1",
        SyncStatusChanged(SyncStatus) => "eitmad.sync.status.event.v1",
        RecordChanged(RecordChangeNotice) => "eitmad.record.changed.event.v1",
        BackgroundJobChanged(BackgroundJobStatus) => "eitmad.background-job.status.event.v1",
        NotificationRaised(Notification) => "eitmad.notification.event.v1",
        ErrorRaised(ScopedError) => "eitmad.error.event.v1"
    }
}

impl Event {
    #[must_use]
    pub const fn is_coalescible(&self) -> bool {
        matches!(
            self,
            Self::ConfigurationChanged(_)
                | Self::PermissionsChanged(_)
                | Self::UpdateStateChanged(_)
                | Self::SyncStatusChanged(_)
        )
    }

    #[must_use]
    pub const fn subscription_kind(&self) -> &'static str {
        match self {
            Self::ConfigurationChanged(_) => "eitmad.config.changed.subscribe.v1",
            Self::PermissionsChanged(_) => "eitmad.permissions.changed.subscribe.v1",
            Self::UpdateStateChanged(_) => "eitmad.update.state.subscribe.v1",
            Self::SyncStatusChanged(_) => "eitmad.sync.status.subscribe.v1",
            Self::RecordChanged(_) => "eitmad.record.changed.subscribe.v1",
            Self::BackgroundJobChanged(_) => "eitmad.background-job.status.subscribe.v1",
            Self::NotificationRaised(_) => "eitmad.notification.subscribe.v1",
            Self::ErrorRaised(_) => "eitmad.error.subscribe.v1",
        }
    }
}

impl Subscription {
    #[must_use]
    pub const fn is_coalescible(&self) -> bool {
        matches!(
            self,
            Self::Configuration(_)
                | Self::Permissions(_)
                | Self::UpdateState(_)
                | Self::SyncStatus(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_job_stream_is_discrete() {
        assert!(!Subscription::BackgroundJobs(BackgroundJobChanges {}).is_coalescible());
    }
}
