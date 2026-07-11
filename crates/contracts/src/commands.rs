use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    config::{ConfigChange, ConfigSnapshot},
    transport::{OperationId, UpdateHandoffId},
    updates::{InstallerOutcome, UpdateState},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfiguration {
    pub expected_revision: u64,
    pub changes: Vec<ConfigChange>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelOperation {
    pub operation_id: OperationId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportInstallerOutcome {
    pub handoff_id: UpdateHandoffId,
    pub outcome: InstallerOutcome,
}

tagged_contract! {
    /// Authoritative state-changing requests.
    pub enum Command {
        UpdateConfiguration(UpdateConfiguration) => "eitmad.config.update.v1",
        CancelOperation(CancelOperation) => "eitmad.operation.cancel.v1",
        ReportInstallerOutcome(ReportInstallerOutcome) => "eitmad.update.report-installer-outcome.v1"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum CommandResult {
    ConfigurationUpdated(ConfigSnapshot),
    OperationCancelled { operation_id: OperationId },
    InstallerOutcomeRecorded(UpdateState),
}
