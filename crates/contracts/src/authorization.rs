use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::identity::{PrincipalId, PrincipalKind, ScopeRef, TenantId, WorkspaceId};

uuid_id!(RelationshipId);
uuid_id!(ObjectId);
open_id!(RelationId, "relationship identifier");
open_id!(ActionId, "authorization action identifier");
open_id!(ObjectKind, "authorization object kind");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipSubject {
    pub principal_id: PrincipalId,
    pub principal_kind: PrincipalKind,
}

/// An authorization object whose tenant and optional workspace are inseparable
/// from its identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScopedObject {
    pub tenant_id: TenantId,
    pub workspace_id: Option<WorkspaceId>,
    pub kind: ObjectKind,
    pub id: ObjectId,
}

/// A tuple subject is either an authenticated principal or another scoped
/// object such as a role, team, or parent record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum TupleSubject {
    Principal(RelationshipSubject),
    Object(ScopedObject),
}

/// Optional request attributes used by a tuple condition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operator", rename_all = "camelCase")]
pub enum AttributeCondition {
    Equals { key: String, value: String },
    All { conditions: Vec<Self> },
    Any { conditions: Vec<Self> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipTuple {
    pub subject: TupleSubject,
    pub relation: RelationId,
    pub object: ScopedObject,
    pub condition: Option<AttributeCondition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationRequest {
    pub action: ActionId,
    pub object: ScopedObject,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRule {
    pub action: ActionId,
    pub object_kind: ObjectKind,
    pub relations: Vec<RelationId>,
    pub inherits_via: Vec<RelationId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationDecision {
    pub decision: crate::permissions::PermissionDecision,
    pub relation: Option<RelationId>,
    pub source: Option<ScopedObject>,
    pub inherited: bool,
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
