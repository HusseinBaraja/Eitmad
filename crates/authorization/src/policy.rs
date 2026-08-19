//! Policy-v2 relationship graph evaluation for scoped product objects.

use std::collections::{BTreeSet, VecDeque};

use eitmad_contracts::{
    authorization::{
        ActionId, AttributeCondition, AuthorizationDecision, AuthorizationRequest, PermissionRule,
        RelationshipSubject, RelationshipTuple, ScopedObject, TupleSubject,
    },
    identity::AuthorizationContext,
    permissions::PermissionDecision,
};

pub const ROLE_MEMBER_RELATION: &str = "eitmad.relation.role.member.v1";
const DEFAULT_MAX_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyBuildError {
    CrossTenantTuple,
    CrossWorkspaceTuple,
    DuplicateRule,
    EmptyGrantRelations,
}

/// Immutable, deny-by-default relationship policy.
///
/// Callers replace the whole value after a validated policy update. Keeping
/// evaluation immutable makes a decision observe one coherent tuple snapshot.
#[derive(Clone, Debug)]
pub struct RelationshipPolicy {
    tuples: Vec<RelationshipTuple>,
    rules: Vec<PermissionRule>,
    max_depth: usize,
}

impl RelationshipPolicy {
    /// Builds and validates one coherent policy snapshot.
    ///
    /// # Errors
    ///
    /// Rejects cross-tenant or cross-workspace object edges, duplicate action
    /// rules, and rules that grant no relationship.
    pub fn new(
        tuples: Vec<RelationshipTuple>,
        rules: Vec<PermissionRule>,
    ) -> Result<Self, PolicyBuildError> {
        Self::with_max_depth(tuples, rules, DEFAULT_MAX_DEPTH)
    }

    fn with_max_depth(
        tuples: Vec<RelationshipTuple>,
        rules: Vec<PermissionRule>,
        max_depth: usize,
    ) -> Result<Self, PolicyBuildError> {
        for tuple in &tuples {
            if let TupleSubject::Object(subject) = &tuple.subject {
                if subject.tenant_id != tuple.object.tenant_id {
                    return Err(PolicyBuildError::CrossTenantTuple);
                }
                if subject.workspace_id != tuple.object.workspace_id {
                    return Err(PolicyBuildError::CrossWorkspaceTuple);
                }
            }
        }
        for (index, rule) in rules.iter().enumerate() {
            if rule.relations.is_empty() {
                return Err(PolicyBuildError::EmptyGrantRelations);
            }
            if rules[..index].iter().any(|previous| {
                previous.action == rule.action && previous.object_kind == rule.object_kind
            }) {
                return Err(PolicyBuildError::DuplicateRule);
            }
        }
        Ok(Self {
            tuples,
            rules,
            max_depth,
        })
    }

    /// Evaluates `can(actor, action, object)` against one immutable snapshot.
    ///
    /// Missing rules, absent relationships, graph cycles, depth exhaustion,
    /// failed conditions, and tenant/workspace mismatches all deny access.
    #[must_use]
    pub fn decide(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
    ) -> AuthorizationDecision {
        if actor.tenant_id != request.object.tenant_id
            || request.object.workspace_id.is_some()
                && actor.workspace_id != request.object.workspace_id
        {
            return denied();
        }
        let Some(rule) = self
            .rules
            .iter()
            .find(|rule| rule.action == request.action && rule.object_kind == request.object.kind)
        else {
            return denied();
        };

        let mut queue = VecDeque::from([(request.object.clone(), false, 0_usize)]);
        let mut visited = BTreeSet::new();
        while let Some((source, inherited, depth)) = queue.pop_front() {
            if depth > self.max_depth || !visited.insert(source.clone()) {
                continue;
            }
            for tuple in self.tuples.iter().filter(|tuple| tuple.object == source) {
                if rule.relations.contains(&tuple.relation)
                    && condition_matches(tuple.condition.as_ref(), &request.attributes)
                    && self.subject_contains_actor(
                        &tuple.subject,
                        actor,
                        &request.attributes,
                        depth,
                        &mut BTreeSet::new(),
                    )
                {
                    return AuthorizationDecision {
                        decision: PermissionDecision::Granted,
                        relation: Some(tuple.relation.clone()),
                        source: Some(source),
                        inherited,
                    };
                }
                if rule.inherits_via.contains(&tuple.relation)
                    && condition_matches(tuple.condition.as_ref(), &request.attributes)
                {
                    if let TupleSubject::Object(parent) = &tuple.subject {
                        queue.push_back((parent.clone(), true, depth + 1));
                    }
                }
            }
        }
        denied()
    }

    /// Evaluates the canonical `can(actor, action, object)` decision with
    /// optional request attributes for conditional tuples.
    #[must_use]
    pub fn can(
        &self,
        actor: &AuthorizationContext,
        action: &ActionId,
        object: &ScopedObject,
        attributes: &std::collections::BTreeMap<String, String>,
    ) -> AuthorizationDecision {
        self.decide(
            actor,
            &AuthorizationRequest {
                action: action.clone(),
                object: object.clone(),
                attributes: attributes.clone(),
            },
        )
    }

    fn subject_contains_actor(
        &self,
        subject: &TupleSubject,
        actor: &AuthorizationContext,
        attributes: &std::collections::BTreeMap<String, String>,
        depth: usize,
        visited_roles: &mut BTreeSet<ScopedObject>,
    ) -> bool {
        match subject {
            TupleSubject::Principal(principal) => principal_matches(principal, actor),
            TupleSubject::Object(role) => {
                if depth >= self.max_depth || !visited_roles.insert(role.clone()) {
                    return false;
                }
                self.tuples.iter().any(|membership| {
                    membership.object == *role
                        && membership.relation.as_str() == ROLE_MEMBER_RELATION
                        && condition_matches(membership.condition.as_ref(), attributes)
                        && self.subject_contains_actor(
                            &membership.subject,
                            actor,
                            attributes,
                            depth + 1,
                            visited_roles,
                        )
                })
            }
        }
    }
}

fn principal_matches(subject: &RelationshipSubject, actor: &AuthorizationContext) -> bool {
    subject.principal_id == actor.identity.principal_id
        && subject.principal_kind == actor.identity.principal_kind
}

fn condition_matches(
    condition: Option<&AttributeCondition>,
    attributes: &std::collections::BTreeMap<String, String>,
) -> bool {
    condition.is_none_or(|condition| match condition {
        AttributeCondition::Equals { key, value } => attributes.get(key) == Some(value),
        AttributeCondition::All { conditions } => conditions
            .iter()
            .all(|condition| condition_matches(Some(condition), attributes)),
        AttributeCondition::Any { conditions } => conditions
            .iter()
            .any(|condition| condition_matches(Some(condition), attributes)),
    })
}

fn denied() -> AuthorizationDecision {
    AuthorizationDecision {
        decision: PermissionDecision::Denied,
        relation: None,
        source: None,
        inherited: false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use eitmad_contracts::{
        authorization::{
            ActionId, AttributeCondition, ObjectId, ObjectKind, RelationId, RelationshipSubject,
            RelationshipTuple, ScopedObject, TupleSubject,
        },
        identity::{
            AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId,
            ScopeKind, ScopeRef, SessionId, TenantId, WorkspaceId,
        },
    };
    use uuid::Uuid;

    use super::*;

    const VIEWER: &str = "eitmad.relation.record.viewer.v1";
    const PARENT: &str = "eitmad.relation.record.parent.v1";
    const VIEW: &str = "eitmad.action.record.view.v1";

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn object(tenant: u128, workspace: u128, kind: &str, value: u128) -> ScopedObject {
        ScopedObject {
            tenant_id: TenantId::new(id(tenant)),
            workspace_id: Some(WorkspaceId::new(id(workspace))),
            kind: ObjectKind::parse(kind).unwrap(),
            id: ObjectId::new(id(value)),
        }
    }

    fn actor(tenant: u128, workspace: u128, principal: u128) -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(id(90)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(id(principal)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(id(tenant)),
            workspace_id: Some(WorkspaceId::new(id(workspace))),
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(id(workspace)),
            },
        }
    }

    fn principal(value: u128) -> TupleSubject {
        TupleSubject::Principal(RelationshipSubject {
            principal_id: PrincipalId::new(id(value)),
            principal_kind: PrincipalKind::User,
        })
    }

    fn tuple(subject: TupleSubject, relation: &str, object: ScopedObject) -> RelationshipTuple {
        RelationshipTuple {
            subject,
            relation: RelationId::parse(relation).unwrap(),
            object,
            condition: None,
        }
    }

    fn rule() -> PermissionRule {
        PermissionRule {
            action: ActionId::parse(VIEW).unwrap(),
            object_kind: ObjectKind::parse("order").unwrap(),
            relations: vec![RelationId::parse(VIEWER).unwrap()],
            inherits_via: vec![RelationId::parse(PARENT).unwrap()],
        }
    }

    fn request(object: ScopedObject) -> AuthorizationRequest {
        AuthorizationRequest {
            action: ActionId::parse(VIEW).unwrap(),
            object,
            attributes: BTreeMap::new(),
        }
    }

    #[test]
    fn direct_relationship_allows_and_absent_relationship_denies() {
        let order = object(1, 2, "order", 3);
        let policy = RelationshipPolicy::new(
            vec![tuple(principal(10), VIEWER, order.clone())],
            vec![rule()],
        )
        .unwrap();

        assert_eq!(
            policy
                .decide(&actor(1, 2, 10), &request(order.clone()))
                .decision,
            PermissionDecision::Granted
        );
        assert_eq!(
            policy
                .can(
                    &actor(1, 2, 10),
                    &ActionId::parse(VIEW).unwrap(),
                    &order,
                    &BTreeMap::new(),
                )
                .decision,
            PermissionDecision::Granted
        );
        assert_eq!(
            policy.decide(&actor(1, 2, 11), &request(order)).decision,
            PermissionDecision::Denied
        );
    }

    #[test]
    fn role_membership_and_parent_relationship_inherit_permission() {
        let parent = object(1, 2, "order", 3);
        let child = object(1, 2, "order", 4);
        let role = object(1, 2, "role", 5);
        let policy = RelationshipPolicy::new(
            vec![
                tuple(principal(10), ROLE_MEMBER_RELATION, role.clone()),
                tuple(TupleSubject::Object(role), VIEWER, parent.clone()),
                tuple(TupleSubject::Object(parent), PARENT, child.clone()),
            ],
            vec![rule()],
        )
        .unwrap();

        let decision = policy.decide(&actor(1, 2, 10), &request(child));
        assert_eq!(decision.decision, PermissionDecision::Granted);
        assert!(decision.inherited);
    }

    #[test]
    fn condition_must_match_request_attributes() {
        let order = object(1, 2, "order", 3);
        let mut conditioned = tuple(principal(10), VIEWER, order.clone());
        conditioned.condition = Some(AttributeCondition::Equals {
            key: "shift".to_owned(),
            value: "day".to_owned(),
        });
        let policy = RelationshipPolicy::new(vec![conditioned], vec![rule()]).unwrap();
        let mut allowed = request(order.clone());
        allowed
            .attributes
            .insert("shift".to_owned(), "day".to_owned());

        assert_eq!(
            policy.decide(&actor(1, 2, 10), &allowed).decision,
            PermissionDecision::Granted
        );
        assert_eq!(
            policy.decide(&actor(1, 2, 10), &request(order)).decision,
            PermissionDecision::Denied
        );
    }

    #[test]
    fn tenant_and_workspace_boundaries_fail_closed() {
        let order = object(1, 2, "order", 3);
        let policy = RelationshipPolicy::new(
            vec![tuple(principal(10), VIEWER, order.clone())],
            vec![rule()],
        )
        .unwrap();

        assert_eq!(
            policy
                .decide(&actor(9, 2, 10), &request(order.clone()))
                .decision,
            PermissionDecision::Denied
        );
        assert_eq!(
            policy.decide(&actor(1, 9, 10), &request(order)).decision,
            PermissionDecision::Denied
        );
    }

    #[test]
    fn cross_scope_object_edges_are_rejected() {
        let role = object(1, 2, "role", 5);
        let order = object(1, 9, "order", 3);
        assert_eq!(
            RelationshipPolicy::new(
                vec![tuple(TupleSubject::Object(role), VIEWER, order)],
                vec![rule()]
            )
            .unwrap_err(),
            PolicyBuildError::CrossWorkspaceTuple
        );
    }
}
