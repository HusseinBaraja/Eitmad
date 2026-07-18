use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    authorization::{RelationId, RelationshipId, RelationshipMutationResult, RelationshipSubject},
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
pub struct GrantScopeRelationship {
    pub expected_policy_version: u64,
    pub subject: RelationshipSubject,
    pub relation: RelationId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevokeScopeRelationship {
    pub expected_policy_version: u64,
    pub relationship_id: RelationshipId,
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
        GrantScopeRelationship(GrantScopeRelationship) => "eitmad.authorization.relationship.grant.v1",
        RevokeScopeRelationship(RevokeScopeRelationship) => "eitmad.authorization.relationship.revoke.v1",
        CancelOperation(CancelOperation) => "eitmad.operation.cancel.v1",
        ReportInstallerOutcome(ReportInstallerOutcome) => "eitmad.update.report-installer-outcome.v1"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "payload", rename_all = "camelCase")]
pub enum CommandResult {
    ConfigurationUpdated(ConfigSnapshot),
    RelationshipGranted(RelationshipMutationResult),
    RelationshipRevoked(RelationshipMutationResult),
    OperationCancelled { operation_id: OperationId },
    InstallerOutcomeRecorded(UpdateState),
}
