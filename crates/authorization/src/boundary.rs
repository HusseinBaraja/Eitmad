//! One authorization and audit gate for every Rust-owned execution boundary.

use eitmad_contracts::{
    authorization::AuthorizationRequest,
    identity::AuthorizationContext,
    permissions::PermissionDecision,
    transport::{CausationId, CorrelationId, IdempotencyKey, UnixMillis},
};
use eitmad_observability_audit::{
    AuditErrorClass, AuditExtensionPoint, AuditOutcome, AuditTarget, MutationAuditRecord,
    RedactedAuditError,
};
use eitmad_storage::AuthorityStore;

use crate::RelationshipPolicy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryKind {
    Command,
    Query,
    Sync,
    ExternalAdapter,
    PluginCapability,
}

impl BoundaryKind {
    const fn audit_marker(self) -> AuditExtensionPoint {
        match self {
            Self::Command => AuditExtensionPoint::CommandBoundary,
            Self::Query => AuditExtensionPoint::QueryBoundary,
            Self::Sync => AuditExtensionPoint::SyncBoundary,
            Self::ExternalAdapter => AuditExtensionPoint::ExternalAdapterBoundary,
            Self::PluginCapability => AuditExtensionPoint::PluginCapabilityBoundary,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BoundaryAuditContext {
    pub kind: BoundaryKind,
    pub operation: String,
    pub target: AuditTarget,
    pub occurred_at: UnixMillis,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub extension_points: Vec<AuditExtensionPoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundaryError {
    Denied,
    ActionFailed(RedactedAuditError),
    AuditUnavailable,
}

/// Evaluates one policy snapshot and emits one complete result record.
///
/// State-changing callers use [`AuthorizationGate::authorize`] before including
/// the state change and successful audit in one domain transaction. Only
/// read-only actions may use [`AuthorizationGate::execute_read`].
#[derive(Clone, Debug)]
pub struct AuthorizationGate {
    policy: RelationshipPolicy,
    store: AuthorityStore,
}

impl AuthorizationGate {
    #[must_use]
    pub const fn new(policy: RelationshipPolicy, store: AuthorityStore) -> Self {
        Self { policy, store }
    }

    /// Authorizes one boundary and durably records a denial.
    ///
    /// Permitted state-changing callers must include their successful or failed
    /// audit result in the same transaction as the state change.
    ///
    /// # Errors
    ///
    /// Returns a denial or an audit availability failure.
    pub fn authorize(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
    ) -> Result<(), BoundaryError> {
        let decision = self.policy.decide(actor, request);
        if decision.decision == PermissionDecision::Denied {
            self.append(
                actor,
                audit,
                AuditOutcome::Denied,
                Some(RedactedAuditError {
                    code: "eitmad.error.authorization-denied.v1".to_owned(),
                    class: AuditErrorClass::Authorization,
                }),
            )?;
            return Err(BoundaryError::Denied);
        }
        Ok(())
    }

    /// Runs an authorized read-only boundary action and records its result.
    ///
    /// The action never runs on denial. Audit persistence failure withholds the
    /// response and fails closed.
    ///
    /// # Errors
    ///
    /// Returns a denial, a caller-supplied redacted action error, or an audit
    /// availability failure.
    pub fn execute_read<T>(
        &self,
        actor: &AuthorizationContext,
        request: &AuthorizationRequest,
        audit: &BoundaryAuditContext,
        action: impl FnOnce() -> Result<T, RedactedAuditError>,
    ) -> Result<T, BoundaryError> {
        self.authorize(actor, request, audit)?;

        match action() {
            Ok(value) => {
                self.append(actor, audit, AuditOutcome::Succeeded, None)?;
                Ok(value)
            }
            Err(error) => {
                let outcome = match error.class {
                    AuditErrorClass::Authorization => AuditOutcome::Denied,
                    AuditErrorClass::Validation => AuditOutcome::Invalid,
                    AuditErrorClass::Conflict => AuditOutcome::Conflict,
                    AuditErrorClass::Dependency | AuditErrorClass::Internal => AuditOutcome::Failed,
                };
                self.append(actor, audit, outcome, Some(error.clone()))?;
                Err(BoundaryError::ActionFailed(error))
            }
        }
    }

    fn append(
        &self,
        actor: &AuthorizationContext,
        audit: &BoundaryAuditContext,
        outcome: AuditOutcome,
        error: Option<RedactedAuditError>,
    ) -> Result<(), BoundaryError> {
        let mut record = MutationAuditRecord::from_authorization(
            actor,
            audit.occurred_at,
            audit.correlation_id,
            audit.operation.clone(),
            audit.target.clone(),
        );
        record.causation_id = audit.causation_id;
        record.idempotency_key = audit.idempotency_key;
        record.outcome = outcome;
        record.redacted_error = error;
        record.extension_points.clone_from(&audit.extension_points);
        let marker = audit.kind.audit_marker();
        if !record.extension_points.contains(&marker) {
            record.extension_points.push(marker);
        }
        self.store
            .append_audit(&record)
            .map_err(|_| BoundaryError::AuditUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        sync::atomic::{AtomicBool, Ordering},
    };

    use eitmad_contracts::{
        authorization::{
            ActionId, AuthorizationRequest, ObjectId, ObjectKind, PermissionRule, ScopedObject,
        },
        identity::{
            AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId,
            ScopeKind, ScopeRef, SessionId, TenantId, WorkspaceId,
        },
    };
    use rusqlite::Connection;
    use tempfile::TempDir;
    use uuid::Uuid;

    use super::*;

    fn context() -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(1)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(2)),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(Uuid::from_u128(3)),
            workspace_id: Some(WorkspaceId::new(Uuid::from_u128(4))),
            scope: ScopeRef {
                kind: ScopeKind::parse("workspace").unwrap(),
                id: ScopeId::new(Uuid::from_u128(4)),
            },
        }
    }

    fn request(action: &str) -> AuthorizationRequest {
        AuthorizationRequest {
            action: ActionId::parse(action).unwrap(),
            object: ScopedObject {
                tenant_id: TenantId::new(Uuid::from_u128(3)),
                workspace_id: Some(WorkspaceId::new(Uuid::from_u128(4))),
                kind: ObjectKind::parse("workspace").unwrap(),
                id: ObjectId::new(Uuid::from_u128(4)),
            },
            attributes: BTreeMap::new(),
        }
    }

    fn audit(kind: BoundaryKind, operation: &str) -> BoundaryAuditContext {
        BoundaryAuditContext {
            kind,
            operation: operation.to_owned(),
            target: AuditTarget {
                kind: "workspace".to_owned(),
                identifiers: vec!["workspace:synthetic".to_owned()],
            },
            occurred_at: UnixMillis(5),
            correlation_id: CorrelationId::new(Uuid::new_v4()),
            causation_id: None,
            idempotency_key: None,
            extension_points: Vec::new(),
        }
    }

    #[test]
    fn unauthorized_reads_writes_sync_external_and_plugins_never_execute() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let policy = RelationshipPolicy::new(
            Vec::new(),
            vec![PermissionRule {
                action: ActionId::parse("eitmad.action.test.v1").unwrap(),
                object_kind: ObjectKind::parse("workspace").unwrap(),
                relations: vec![
                    eitmad_contracts::authorization::RelationId::parse(
                        "eitmad.relation.workspace.operator.v1",
                    )
                    .unwrap(),
                ],
                inherits_via: Vec::new(),
            }],
        )
        .unwrap();
        let path = store.path().to_owned();
        let gate = AuthorizationGate::new(policy, store);

        for kind in [
            BoundaryKind::Command,
            BoundaryKind::Query,
            BoundaryKind::Sync,
            BoundaryKind::ExternalAdapter,
            BoundaryKind::PluginCapability,
        ] {
            let executed = AtomicBool::new(false);
            let result = gate.execute_read(
                &context(),
                &request("eitmad.action.test.v1"),
                &audit(kind, "eitmad.test.denied.v1"),
                || {
                    executed.store(true, Ordering::SeqCst);
                    Ok(())
                },
            );
            assert_eq!(result, Err(BoundaryError::Denied));
            assert!(!executed.load(Ordering::SeqCst));
        }

        let connection = Connection::open(path).unwrap();
        let mut statement = connection
            .prepare("SELECT extension_points FROM mutation_audit ORDER BY rowid")
            .unwrap();
        let persisted = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .map(|value| serde_json::from_str::<Vec<AuditExtensionPoint>>(&value.unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            persisted,
            vec![
                vec![AuditExtensionPoint::CommandBoundary],
                vec![AuditExtensionPoint::QueryBoundary],
                vec![AuditExtensionPoint::SyncBoundary],
                vec![AuditExtensionPoint::ExternalAdapterBoundary],
                vec![AuditExtensionPoint::PluginCapabilityBoundary],
            ]
        );
    }
}
