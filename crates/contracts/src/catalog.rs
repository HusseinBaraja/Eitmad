use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    commands::Command,
    events::{Event, Subscription},
    queries::Query,
    sync::SyncMessage,
};

pub const CAPABILITIES: &[&str] = &[
    "eitmad.capability.engine-lifecycle.v1",
    "eitmad.capability.local-ipc.v1",
    "eitmad.capability.config.v1",
    "eitmad.capability.permissions.v1",
    "eitmad.capability.sync.v1",
    "eitmad.capability.update.v1",
];

pub const PERMISSIONS: &[&str] = &[
    "eitmad.permission.config.read.v1",
    "eitmad.permission.config.write.v1",
    "eitmad.permission.permissions.read.v1",
    "eitmad.permission.sync.read.v1",
    "eitmad.permission.update.read.v1",
    "eitmad.permission.update.report-installer.v1",
];

pub const ERROR_CODES: &[&str] = &[
    "eitmad.error.authorization-denied.v1",
    "eitmad.error.config-revision-conflict.v1",
    "eitmad.error.contract-invalid.v1",
    "eitmad.error.engine-already-running.v1",
    "eitmad.error.engine-health-check-failed.v1",
    "eitmad.error.engine-shutdown-failed.v1",
    "eitmad.error.engine-startup-failed.v1",
    "eitmad.error.engine-supervisor-invalid.v1",
    "eitmad.error.ipc-engine-stopping.v1",
    "eitmad.error.ipc-payload-too-large.v1",
    "eitmad.error.ipc-session-invalid.v1",
    "eitmad.error.ipc-deadline-exceeded.v1",
    "eitmad.error.protocol-incompatible.v1",
    "eitmad.error.sync-backpressure.v1",
    "eitmad.error.update-installer-failed.v1",
];

pub const MESSAGE_IDS: &[&str] = &[
    "eitmad.message.authorization-denied.v1",
    "eitmad.message.config-revision-conflict.v1",
    "eitmad.message.contract-invalid.v1",
    "eitmad.message.engine-already-running.v1",
    "eitmad.message.engine-health-check-failed.v1",
    "eitmad.message.engine-shutdown-failed.v1",
    "eitmad.message.engine-startup-failed.v1",
    "eitmad.message.engine-supervisor-invalid.v1",
    "eitmad.message.ipc-engine-stopping.v1",
    "eitmad.message.ipc-payload-too-large.v1",
    "eitmad.message.ipc-session-invalid.v1",
    "eitmad.message.ipc-deadline-exceeded.v1",
    "eitmad.message.protocol-incompatible.v1",
    "eitmad.message.sync-backpressure.v1",
    "eitmad.message.update-installer-failed.v1",
];

pub const ERROR_PARAMETER_NAMES: &[&str] = &[
    "actual-revision",
    "expected-revision",
    "required-capability",
    "retry-after-ms",
    "maximum-payload-bytes",
];

pub const CONFIG_KEYS: &[&str] = &["eitmad.config.locale.primary.v1"];
pub const DOMAIN_SCHEMA_IDS: &[&str] = &["eitmad.schema.protocol.v1"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolCatalog {
    pub commands: Vec<String>,
    pub queries: Vec<String>,
    pub subscriptions: Vec<String>,
    pub events: Vec<String>,
    pub sync_messages: Vec<String>,
    pub capabilities: Vec<String>,
    pub permissions: Vec<String>,
    pub config_keys: Vec<String>,
    pub schema_ids: Vec<String>,
    pub error_codes: Vec<String>,
    pub message_ids: Vec<String>,
    pub error_parameter_names: Vec<String>,
}

impl ProtocolCatalog {
    #[must_use]
    pub fn current() -> Self {
        Self {
            commands: strings(Command::IDS),
            queries: strings(Query::IDS),
            subscriptions: strings(Subscription::IDS),
            events: strings(Event::IDS),
            sync_messages: strings(SyncMessage::IDS),
            capabilities: strings(CAPABILITIES),
            permissions: strings(PERMISSIONS),
            config_keys: strings(CONFIG_KEYS),
            schema_ids: strings(DOMAIN_SCHEMA_IDS),
            error_codes: strings(ERROR_CODES),
            message_ids: strings(MESSAGE_IDS),
            error_parameter_names: strings(ERROR_PARAMETER_NAMES),
        }
    }

    #[must_use]
    pub fn duplicate_identifiers(&self) -> Vec<String> {
        let mut identifiers = self
            .commands
            .iter()
            .chain(&self.queries)
            .chain(&self.subscriptions)
            .chain(&self.events)
            .chain(&self.sync_messages)
            .chain(&self.capabilities)
            .chain(&self.permissions)
            .chain(&self.config_keys)
            .chain(&self.schema_ids)
            .chain(&self.error_codes)
            .chain(&self.message_ids)
            .cloned()
            .collect::<Vec<_>>();
        identifiers.sort_unstable();
        identifiers
            .windows(2)
            .filter(|window| window[0] == window[1])
            .map(|window| window[0].clone())
            .collect()
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_no_duplicate_protocol_identifiers() {
        assert!(
            ProtocolCatalog::current()
                .duplicate_identifiers()
                .is_empty()
        );
    }

    #[test]
    fn identifiers_follow_the_open_identifier_grammar() {
        let catalog = ProtocolCatalog::current();
        let identifiers = catalog
            .commands
            .iter()
            .chain(&catalog.queries)
            .chain(&catalog.subscriptions)
            .chain(&catalog.events)
            .chain(&catalog.sync_messages)
            .chain(&catalog.capabilities)
            .chain(&catalog.permissions)
            .chain(&catalog.error_codes)
            .chain(&catalog.message_ids);

        for identifier in identifiers {
            assert!(crate::transport::CapabilityId::parse(identifier.clone()).is_ok());
        }
    }
}
