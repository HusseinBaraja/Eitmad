use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::identity::{PrincipalId, PrincipalKind, ScopeRef};

uuid_id!(RelationshipId);
open_id!(RelationId, "relationship identifier");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipSubject {
    pub principal_id: PrincipalId,
    pub principal_kind: PrincipalKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRelationship {
    pub relationship_id: RelationshipId,
    pub subject: RelationshipSubject,
    pub relation: RelationId,
    pub scope: ScopeRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipMutationResult {
    pub policy_version: u64,
    pub relationship: ScopeRelationship,
    pub changed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipPage {
    pub policy_version: u64,
    pub relationships: Vec<ScopeRelationship>,
    pub next_after: Option<RelationshipId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationPolicyChangeNotice {
    pub scope: ScopeRef,
    pub policy_version: u64,
}
