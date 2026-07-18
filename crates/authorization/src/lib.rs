//! Direct principal-to-scope relationship authorization with Rust-owned policy.

use std::{
    fmt::Write as _,
    time::{SystemTime, UNIX_EPOCH},
};

use eitmad_contracts::{
    authorization::{
        AuthorizationPolicyChangeNotice, RelationId, RelationshipId, RelationshipMutationResult,
        RelationshipPage, RelationshipSubject,
    },
    commands::{GrantScopeRelationship, RevokeScopeRelationship},
    events::Event,
    identity::{AuthorizationContext, PrincipalKind, ScopeRef},
    permissions::{EffectivePermission, EffectivePermissions, PermissionDecision, PermissionId},
    queries::ListScopeRelationships,
    transport::{CausationId, CorrelationId, IdempotencyKey, UnixMillis},
};
use eitmad_observability_audit::{AuditOutcome, MutationAuditRecord};
use eitmad_storage::{
    AuthorityStore, DurableIdempotency, DurablePublication, RelationshipCommitOutcome, StorageError,
};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

pub const OWNER_RELATION: &str = "eitmad.relation.organization.owner.v1";
pub const CONFIG_MANAGER_RELATION: &str = "eitmad.relation.organization.config-manager.v1";
pub const MEMBER_RELATION: &str = "eitmad.relation.organization.member.v1";

pub const CONFIG_READ_PERMISSION: &str = "eitmad.permission.config.read.v1";
pub const CONFIG_WRITE_PERMISSION: &str = "eitmad.permission.config.write.v1";
pub const CONFIG_IMPORT_PERMISSION: &str = "eitmad.permission.config.import.v1";
pub const CONFIG_EXPORT_PERMISSION: &str = "eitmad.permission.config.export.v1";
pub const AUTHORIZATION_MANAGE_PERMISSION: &str = "eitmad.permission.authorization.manage.v1";
pub const PERMISSIONS_READ_PERMISSION: &str = "eitmad.permission.permissions.read.v1";

const ORGANIZATION_SCOPE: &str = "organization";

const POLICY_PERMISSIONS: &[&str] = &[
    AUTHORIZATION_MANAGE_PERMISSION,
    CONFIG_EXPORT_PERMISSION,
    CONFIG_IMPORT_PERMISSION,
    CONFIG_READ_PERMISSION,
    CONFIG_WRITE_PERMISSION,
    PERMISSIONS_READ_PERMISSION,
];

#[derive(Clone, Debug)]
pub struct MutationContext {
    pub authorization: AuthorizationContext,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub idempotency_key: IdempotencyKey,
    pub occurred_at: UnixMillis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationError {
    Denied,
    UnsupportedScope,
    InvalidRelation,
    PolicyConflict {
        expected_version: u64,
        actual_version: u64,
    },
    LastOwner,
    RelationshipNotFound,
    BootstrapUnavailable,
    IdempotencyMismatch,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct AuthorizationService {
    store: AuthorityStore,
    development_ephemeral_owner: bool,
}

impl AuthorizationService {
    #[must_use]
    pub const fn new(store: AuthorityStore) -> Self {
        Self {
            store,
            development_ephemeral_owner: false,
        }
    }

    #[must_use]
    pub const fn with_development_ephemeral_owner(mut self, enabled: bool) -> Self {
        self.development_ephemeral_owner = enabled;
        self
    }

    /// Evaluates the Rust policy for the authenticated principal and exact scope.
    ///
    /// # Errors
    ///
    /// Returns [`AuthorizationError::Unavailable`] when relationship state cannot be read.
    pub fn effective_permissions(
        &self,
        context: &AuthorizationContext,
    ) -> Result<EffectivePermissions, AuthorizationError> {
        let effective = self.evaluate_permissions(context)?;
        let may_read = effective.permissions.iter().any(|permission| {
            permission.permission.as_str() == PERMISSIONS_READ_PERMISSION
                && permission.decision == PermissionDecision::Granted
        });
        may_read
            .then_some(effective)
            .ok_or(AuthorizationError::Denied)
    }

    fn evaluate_permissions(
        &self,
        context: &AuthorizationContext,
    ) -> Result<EffectivePermissions, AuthorizationError> {
        validate_scope(&context.scope)?;
        let subject = RelationshipSubject {
            principal_id: context.identity.principal_id,
            principal_kind: context.identity.principal_kind,
        };
        let relationships = self
            .store
            .relationships_for_subject(&context.scope, &subject)
            .map_err(|_| AuthorizationError::Unavailable)?;
        let ephemeral_owner = self.development_ephemeral_owner;
        let owner = ephemeral_owner
            || relationships
                .iter()
                .any(|relationship| relationship.relation.as_str() == OWNER_RELATION);
        let manager = owner
            || relationships
                .iter()
                .any(|relationship| relationship.relation.as_str() == CONFIG_MANAGER_RELATION);
        let member = manager
            || relationships
                .iter()
                .any(|relationship| relationship.relation.as_str() == MEMBER_RELATION);
        let permissions = POLICY_PERMISSIONS
            .iter()
            .map(|permission| EffectivePermission {
                permission: permission_id(permission),
                decision: if grants(permission, owner, manager, member) {
                    PermissionDecision::Granted
                } else {
                    PermissionDecision::Denied
                },
            })
            .collect();
        let policy_version = self
            .store
            .policy_version(&context.scope)
            .map_err(|_| AuthorizationError::Unavailable)?;
        Ok(EffectivePermissions {
            policy_version,
            permissions,
        })
    }

    /// Checks one registered permission without revealing the relationship graph.
    ///
    /// # Errors
    ///
    /// Returns `Denied` for absent permission and `Unavailable` for storage failure.
    pub fn authorize(
        &self,
        context: &AuthorizationContext,
        permission: &str,
    ) -> Result<(), AuthorizationError> {
        let effective = self.evaluate_permissions(context)?;
        let granted = effective.permissions.iter().any(|candidate| {
            candidate.permission.as_str() == permission
                && candidate.decision == PermissionDecision::Granted
        });
        granted.then_some(()).ok_or(AuthorizationError::Denied)
    }

    /// Creates the first persisted owner for an empty scope.
    ///
    /// # Errors
    ///
    /// Returns `BootstrapUnavailable` once an owner exists, or a sanitized
    /// storage failure if the transaction cannot commit.
    pub fn bootstrap_owner(
        &self,
        context: &MutationContext,
        subject: &RelationshipSubject,
    ) -> Result<RelationshipMutationResult, AuthorizationError> {
        validate_scope(&context.authorization.scope)?;
        let relation = relation_id(OWNER_RELATION);
        let audit = audit_record(
            context,
            "eitmad.authorization.bootstrap-owner.v1",
            vec![OWNER_RELATION.to_owned()],
        );
        match self.store.bootstrap_relationship(
            &context.authorization.scope,
            RelationshipId::new(Uuid::new_v4()),
            subject,
            &relation,
            &audit,
        ) {
            Ok(RelationshipCommitOutcome::Committed(result)) => Ok(result),
            Ok(RelationshipCommitOutcome::BootstrapUnavailable) => {
                Err(AuthorizationError::BootstrapUnavailable)
            }
            Ok(_) | Err(_) => Err(AuthorizationError::Unavailable),
        }
    }

    /// Grants one registered direct relationship in the authenticated scope.
    ///
    /// # Errors
    ///
    /// Returns structured authorization, validation, conflict, idempotency, or
    /// sanitized storage failures.
    pub fn grant_relationship(
        &self,
        context: &MutationContext,
        command: &GrantScopeRelationship,
    ) -> Result<RelationshipMutationResult, AuthorizationError> {
        self.authorize_mutation(context, command_kind_grant())?;
        if !registered_relation(&command.relation) {
            self.audit_failure(
                context,
                command_kind_grant(),
                AuditOutcome::Invalid,
                "eitmad.error.authorization-relation-invalid.v1",
            )?;
            return Err(AuthorizationError::InvalidRelation);
        }
        let idempotency = durable_idempotency(context, command)?;
        let relationship_id = RelationshipId::new(Uuid::new_v4());
        let publication = policy_publication(
            &context.authorization.scope,
            command
                .expected_policy_version
                .checked_add(1)
                .ok_or(AuthorizationError::Unavailable)?,
        );
        let audit = audit_record(
            context,
            command_kind_grant(),
            grant_audit_targets(relationship_id, &command.subject, &command.relation),
        );
        map_relationship_outcome(
            self.store.grant_relationship(
                &context.authorization.scope,
                command.expected_policy_version,
                relationship_id,
                &command.subject,
                &command.relation,
                command_kind_grant(),
                &idempotency,
                &audit,
                Some(&publication),
            ),
            command.expected_policy_version,
        )
    }

    /// Revokes one relationship in the authenticated scope.
    ///
    /// # Errors
    ///
    /// Returns structured authorization, conflict, last-owner, idempotency, or
    /// sanitized storage failures.
    pub fn revoke_relationship(
        &self,
        context: &MutationContext,
        command: &RevokeScopeRelationship,
    ) -> Result<RelationshipMutationResult, AuthorizationError> {
        self.authorize_mutation(context, command_kind_revoke())?;
        let idempotency = durable_idempotency(context, command)?;
        let audit = audit_record(
            context,
            command_kind_revoke(),
            vec![command.relationship_id.value().to_string()],
        );
        let publication = policy_publication(
            &context.authorization.scope,
            command
                .expected_policy_version
                .checked_add(1)
                .ok_or(AuthorizationError::Unavailable)?,
        );
        map_relationship_outcome(
            self.store.revoke_relationship(
                &context.authorization.scope,
                command.expected_policy_version,
                command.relationship_id,
                &relation_id(OWNER_RELATION),
                command_kind_revoke(),
                &idempotency,
                &audit,
                Some(&publication),
            ),
            command.expected_policy_version,
        )
    }

    /// Lists a bounded page of relationships in the authenticated owner scope.
    ///
    /// # Errors
    ///
    /// Returns `Denied` for non-owners and `Unavailable` for storage failure.
    pub fn list_relationships(
        &self,
        context: &AuthorizationContext,
        query: &ListScopeRelationships,
    ) -> Result<RelationshipPage, AuthorizationError> {
        self.authorize(context, AUTHORIZATION_MANAGE_PERMISSION)?;
        let page = self
            .store
            .list_relationships(&context.scope, query.after, query.limit())
            .map_err(|_| AuthorizationError::Unavailable)?;
        Ok(RelationshipPage {
            policy_version: page.policy_version,
            relationships: page.relationships,
            next_after: page.next_after,
        })
    }

    fn authorize_mutation(
        &self,
        context: &MutationContext,
        operation: &str,
    ) -> Result<(), AuthorizationError> {
        match self.authorize(&context.authorization, AUTHORIZATION_MANAGE_PERMISSION) {
            Ok(()) => Ok(()),
            Err(AuthorizationError::Denied) => {
                self.audit_failure(
                    context,
                    operation,
                    AuditOutcome::Denied,
                    "eitmad.error.authorization-denied.v1",
                )?;
                Err(AuthorizationError::Denied)
            }
            Err(error) => Err(error),
        }
    }

    fn audit_failure(
        &self,
        context: &MutationContext,
        operation: &str,
        outcome: AuditOutcome,
        error_code: &str,
    ) -> Result<(), AuthorizationError> {
        let record = audit_record(context, operation, Vec::new())
            .with_outcome(outcome, Some(error_code.to_owned()));
        self.store
            .append_audit(&record)
            .map_err(|_| AuthorizationError::Unavailable)
    }
}

fn grants(permission: &str, owner: bool, manager: bool, member: bool) -> bool {
    match permission {
        AUTHORIZATION_MANAGE_PERMISSION => owner,
        CONFIG_WRITE_PERMISSION | CONFIG_IMPORT_PERMISSION | CONFIG_EXPORT_PERMISSION => manager,
        CONFIG_READ_PERMISSION | PERMISSIONS_READ_PERMISSION => member,
        _ => false,
    }
}

fn registered_relation(relation: &RelationId) -> bool {
    matches!(
        relation.as_str(),
        OWNER_RELATION | CONFIG_MANAGER_RELATION | MEMBER_RELATION
    )
}

fn validate_scope(scope: &ScopeRef) -> Result<(), AuthorizationError> {
    (scope.kind.as_str() == ORGANIZATION_SCOPE)
        .then_some(())
        .ok_or(AuthorizationError::UnsupportedScope)
}

fn grant_audit_targets(
    relationship_id: RelationshipId,
    subject: &RelationshipSubject,
    relation: &RelationId,
) -> Vec<String> {
    vec![
        relationship_id.value().to_string(),
        relation.as_str().to_owned(),
        subject_audit_identifier(subject),
    ]
}

fn subject_audit_identifier(subject: &RelationshipSubject) -> String {
    let principal_kind = match subject.principal_kind {
        PrincipalKind::User => "user",
        PrincipalKind::Device => "device",
        PrincipalKind::Service => "service",
    };
    let mut hasher = Sha256::new();
    hasher.update(b"eitmad.audit.relationship-subject.v1\0");
    hasher.update(principal_kind.as_bytes());
    hasher.update(subject.principal_id.value().as_bytes());
    let mut fingerprint = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(&mut fingerprint, "{byte:02x}").expect("writing to a String cannot fail");
    }
    format!("subject:{principal_kind}:sha256:{fingerprint}")
}

fn policy_publication(scope: &ScopeRef, policy_version: u64) -> DurablePublication {
    DurablePublication {
        event: Event::AuthorizationPolicyChanged(AuthorizationPolicyChangeNotice {
            scope: scope.clone(),
            policy_version,
        }),
        policy_changed: true,
    }
}

fn map_relationship_outcome(
    outcome: Result<RelationshipCommitOutcome, StorageError>,
    expected_version: u64,
) -> Result<RelationshipMutationResult, AuthorizationError> {
    match outcome {
        Ok(
            RelationshipCommitOutcome::Committed(result)
            | RelationshipCommitOutcome::Replayed(result),
        ) => Ok(result),
        Ok(RelationshipCommitOutcome::PolicyConflict { actual_version }) => {
            Err(AuthorizationError::PolicyConflict {
                expected_version,
                actual_version,
            })
        }
        Ok(RelationshipCommitOutcome::LastProtectedRelationship) => {
            Err(AuthorizationError::LastOwner)
        }
        Ok(RelationshipCommitOutcome::RelationshipNotFound) => {
            Err(AuthorizationError::RelationshipNotFound)
        }
        Ok(RelationshipCommitOutcome::IdempotencyMismatch) => {
            Err(AuthorizationError::IdempotencyMismatch)
        }
        Ok(RelationshipCommitOutcome::BootstrapUnavailable) | Err(_) => {
            Err(AuthorizationError::Unavailable)
        }
    }
}

fn durable_idempotency(
    context: &MutationContext,
    command: &impl Serialize,
) -> Result<DurableIdempotency, AuthorizationError> {
    let encoded = serde_json::to_vec(&(&context.authorization.scope, command))
        .map_err(|_| AuthorizationError::Unavailable)?;
    let request_hash: [u8; 32] = Sha256::digest(encoded).into();
    Ok(DurableIdempotency {
        key: context.idempotency_key,
        request_hash,
        response_json: Vec::new(),
    })
}

fn audit_record(
    context: &MutationContext,
    operation: &str,
    changed_identifiers: Vec<String>,
) -> MutationAuditRecord {
    MutationAuditRecord {
        audit_id: Uuid::new_v4(),
        occurred_at: context.occurred_at,
        principal_id: context.authorization.identity.principal_id,
        principal_kind: context.authorization.identity.principal_kind,
        scope: context.authorization.scope.clone(),
        correlation_id: context.correlation_id,
        causation_id: context.causation_id,
        idempotency_key: Some(context.idempotency_key),
        operation: operation.to_owned(),
        outcome: AuditOutcome::Succeeded,
        previous_revision: None,
        resulting_revision: None,
        changed_identifiers,
        error_code: None,
    }
}

fn permission_id(value: &str) -> PermissionId {
    PermissionId::parse(value).expect("static permission ID is valid")
}

fn relation_id(value: &str) -> RelationId {
    RelationId::parse(value).expect("static relation ID is valid")
}

const fn command_kind_grant() -> &'static str {
    "eitmad.authorization.relationship.grant.v1"
}

const fn command_kind_revoke() -> &'static str {
    "eitmad.authorization.relationship.revoke.v1"
}

#[must_use]
pub fn now() -> UnixMillis {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    UnixMillis(i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::identity::{
        AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, ScopeRef, SessionId,
    };
    use tempfile::TempDir;

    use super::*;

    fn authorization(principal: u128, scope: u128) -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::new_v4()),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(principal)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            scope: ScopeRef {
                kind: ScopeKind::parse("organization").unwrap(),
                id: ScopeId::new(Uuid::from_u128(scope)),
            },
        }
    }

    fn mutation(principal: u128, scope: u128, key: u128) -> MutationContext {
        MutationContext {
            authorization: authorization(principal, scope),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(key)),
            occurred_at: UnixMillis(1),
        }
    }

    fn unsupported_scope(principal: u128, scope: u128) -> AuthorizationContext {
        let mut context = authorization(principal, scope);
        context.scope.kind = ScopeKind::parse("site").unwrap();
        context
    }

    fn subject(principal: u128) -> RelationshipSubject {
        RelationshipSubject {
            principal_id: PrincipalId::new(Uuid::from_u128(principal)),
            principal_kind: PrincipalKind::User,
        }
    }

    fn service() -> (TempDir, AuthorizationService) {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let service = AuthorizationService::new(store);
        (directory, service)
    }

    fn bootstrap(service: &AuthorizationService, principal: u128, scope: u128) {
        service
            .bootstrap_owner(&mutation(principal, scope, 100), &subject(principal))
            .unwrap();
    }

    #[test]
    fn policy_maps_member_manager_and_owner_permissions() {
        let (_directory, service) = service();
        bootstrap(&service, 1, 10);
        let manager = service
            .grant_relationship(
                &mutation(1, 10, 101),
                &GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject: subject(2),
                    relation: relation_id(CONFIG_MANAGER_RELATION),
                },
            )
            .unwrap();
        service
            .grant_relationship(
                &mutation(1, 10, 102),
                &GrantScopeRelationship {
                    expected_policy_version: manager.policy_version,
                    subject: subject(3),
                    relation: relation_id(MEMBER_RELATION),
                },
            )
            .unwrap();

        assert!(
            service
                .authorize(&authorization(1, 10), AUTHORIZATION_MANAGE_PERMISSION)
                .is_ok()
        );
        assert!(
            service
                .authorize(&authorization(2, 10), CONFIG_WRITE_PERMISSION)
                .is_ok()
        );
        assert!(
            service
                .authorize(&authorization(3, 10), CONFIG_READ_PERMISSION)
                .is_ok()
        );
        assert_eq!(
            service.authorize(&authorization(3, 10), CONFIG_WRITE_PERMISSION),
            Err(AuthorizationError::Denied)
        );
    }

    #[test]
    fn denies_cross_scope_and_non_owner_administration() {
        let (_directory, service) = service();
        bootstrap(&service, 1, 10);
        assert_eq!(
            service.authorize(&authorization(1, 11), CONFIG_READ_PERMISSION),
            Err(AuthorizationError::Denied)
        );
        assert_eq!(
            service.grant_relationship(
                &mutation(2, 10, 101),
                &GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject: subject(2),
                    relation: relation_id(MEMBER_RELATION),
                }
            ),
            Err(AuthorizationError::Denied)
        );
    }

    #[test]
    fn rejects_non_organization_permission_evaluation_and_mutation() {
        let (_directory, service) = service();
        let unsupported = unsupported_scope(1, 10);
        assert_eq!(
            service.effective_permissions(&unsupported),
            Err(AuthorizationError::UnsupportedScope)
        );

        let mut mutation = mutation(1, 10, 101);
        mutation.authorization = unsupported;
        assert_eq!(
            service.grant_relationship(
                &mutation,
                &GrantScopeRelationship {
                    expected_policy_version: 0,
                    subject: subject(2),
                    relation: relation_id(MEMBER_RELATION),
                }
            ),
            Err(AuthorizationError::UnsupportedScope)
        );
        assert_eq!(
            service.bootstrap_owner(&mutation, &subject(1)),
            Err(AuthorizationError::UnsupportedScope)
        );
    }

    #[test]
    fn grant_audit_targets_identify_relationship_without_raw_subject_id() {
        let relationship_id = RelationshipId::new(Uuid::from_u128(55));
        let subject = subject(99);
        let targets = grant_audit_targets(relationship_id, &subject, &relation_id(MEMBER_RELATION));

        assert_eq!(targets[0], relationship_id.value().to_string());
        assert_eq!(targets[1], MEMBER_RELATION);
        assert!(targets[2].starts_with("subject:user:sha256:"));
        assert!(!targets[2].contains(&subject.principal_id.value().to_string()));
    }

    #[test]
    fn optimistic_mutations_replay_and_protect_last_owner() {
        let (_directory, service) = service();
        bootstrap(&service, 1, 10);
        let context = mutation(1, 10, 101);
        let command = GrantScopeRelationship {
            expected_policy_version: 1,
            subject: subject(2),
            relation: relation_id(MEMBER_RELATION),
        };
        let first = service.grant_relationship(&context, &command).unwrap();
        let replay = service.grant_relationship(&context, &command).unwrap();
        assert!(first.changed);
        assert!(!replay.changed);
        assert_eq!(first.policy_version, replay.policy_version);
        assert_eq!(first.relationship, replay.relationship);
        assert_eq!(
            service.grant_relationship(
                &mutation(1, 10, 102),
                &GrantScopeRelationship {
                    expected_policy_version: 1,
                    subject: subject(3),
                    relation: relation_id(MEMBER_RELATION),
                }
            ),
            Err(AuthorizationError::PolicyConflict {
                expected_version: 1,
                actual_version: 2
            })
        );
        let owner = service
            .list_relationships(
                &authorization(1, 10),
                &ListScopeRelationships::new(None, 10).unwrap(),
            )
            .unwrap()
            .relationships
            .into_iter()
            .find(|relationship| relationship.relation.as_str() == OWNER_RELATION)
            .unwrap();
        assert_eq!(
            service.revoke_relationship(
                &mutation(1, 10, 103),
                &RevokeScopeRelationship {
                    expected_policy_version: 2,
                    relationship_id: owner.relationship_id,
                }
            ),
            Err(AuthorizationError::LastOwner)
        );
    }

    #[test]
    fn development_owner_is_ephemeral_and_explicit() {
        let directory = TempDir::new().unwrap();
        let service = AuthorizationService::new(AuthorityStore::open(directory.path()).unwrap())
            .with_development_ephemeral_owner(true);
        assert!(
            service
                .authorize(&authorization(99, 42), AUTHORIZATION_MANAGE_PERMISSION)
                .is_ok()
        );
        assert_eq!(
            service
                .effective_permissions(&authorization(99, 42))
                .unwrap()
                .policy_version,
            0
        );
    }
}
