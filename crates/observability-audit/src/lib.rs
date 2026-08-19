//! Privacy-preserving diagnostics and mutation audit records.

mod diagnostics;

pub use diagnostics::{
    CrashReport, MAX_SENSITIVE_DEBUG_DURATION, ObservationContract, ObservationContractError,
    ObservationFieldContract, ObservationValue, RedactionContext, SENSITIVE_DEBUG_WARNING,
    SensitiveDebugController, SensitiveDebugError, SensitiveDebugEvaluation,
    SensitiveDebugStatus, StructuredError, StructuredErrorClass, StructuredLog, StructuredValue,
};

use eitmad_contracts::{
    errors::ErrorCode,
    identity::{
        AuthorizationContext, DeviceId, PrincipalId, PrincipalKind, ScopeRef, SessionId, TenantId,
        WorkspaceId,
    },
    transport::{CausationId, CorrelationId, IdempotencyKey, UnixMillis},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditOutcome {
    Succeeded,
    Denied,
    Invalid,
    Conflict,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditErrorClass {
    Authorization,
    Validation,
    Conflict,
    Dependency,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedAuditError {
    /// Stable non-secret error identifier. Raw messages and payloads are forbidden.
    pub code: String,
    pub class: AuditErrorClass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditTarget {
    pub kind: String,
    pub identifiers: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuditExtensionPoint {
    Approval,
    Ledger,
    Conflict,
    SecurityEvent,
    UndoCritical,
    CommandBoundary,
    QueryBoundary,
    SyncBoundary,
    ExternalAdapterBoundary,
    PluginCapabilityBoundary,
    SensitiveDebugMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuditCompletenessError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationAuditRecord {
    pub audit_id: Uuid,
    pub occurred_at: UnixMillis,
    pub principal_id: PrincipalId,
    pub principal_kind: PrincipalKind,
    pub session_id: SessionId,
    pub device_id: Option<DeviceId>,
    pub tenant_id: TenantId,
    pub workspace_id: Option<WorkspaceId>,
    pub scope: ScopeRef,
    pub correlation_id: CorrelationId,
    pub causation_id: Option<CausationId>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub operation: String,
    pub target: AuditTarget,
    pub outcome: AuditOutcome,
    pub previous_revision: Option<u64>,
    pub resulting_revision: Option<u64>,
    pub changed_identifiers: Vec<String>,
    pub redacted_error: Option<RedactedAuditError>,
    pub extension_points: Vec<AuditExtensionPoint>,
}

impl MutationAuditRecord {
    #[must_use]
    pub fn with_outcome(mut self, outcome: AuditOutcome, error_code: Option<String>) -> Self {
        self.outcome = outcome;
        self.redacted_error = error_code.map(|code| RedactedAuditError {
            code,
            class: match outcome {
                AuditOutcome::Denied => AuditErrorClass::Authorization,
                AuditOutcome::Invalid => AuditErrorClass::Validation,
                AuditOutcome::Conflict => AuditErrorClass::Conflict,
                AuditOutcome::Failed | AuditOutcome::Succeeded => AuditErrorClass::Internal,
            },
        });
        self
    }

    #[must_use]
    pub fn from_authorization(
        authorization: &AuthorizationContext,
        occurred_at: UnixMillis,
        correlation_id: CorrelationId,
        operation: impl Into<String>,
        target: AuditTarget,
    ) -> Self {
        Self {
            audit_id: Uuid::new_v4(),
            occurred_at,
            principal_id: authorization.identity.principal_id,
            principal_kind: authorization.identity.principal_kind,
            session_id: authorization.session_id,
            device_id: authorization.identity.device_id,
            tenant_id: authorization.tenant_id,
            workspace_id: authorization.workspace_id,
            scope: authorization.scope.clone(),
            correlation_id,
            causation_id: None,
            idempotency_key: None,
            operation: operation.into(),
            target,
            outcome: AuditOutcome::Succeeded,
            previous_revision: None,
            resulting_revision: None,
            changed_identifiers: Vec::new(),
            redacted_error: None,
            extension_points: Vec::new(),
        }
    }

    /// Checks the mandatory envelope without inspecting domain payloads.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty command/target or an unclassified failure.
    pub fn validate_complete(&self) -> Result<(), AuditCompletenessError> {
        let error_code_is_valid = self
            .redacted_error
            .as_ref()
            .is_none_or(|error| ErrorCode::parse(error.code.clone()).is_ok());
        let failure_has_error =
            self.outcome == AuditOutcome::Succeeded || self.redacted_error.is_some();
        (!self.operation.trim().is_empty()
            && !self.target.kind.trim().is_empty()
            && error_code_is_valid
            && failure_has_error)
            .then_some(())
            .ok_or(AuditCompletenessError)
    }
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::identity::{
        AuthenticatedIdentity, AuthorizationContext, PrincipalId, PrincipalKind, ScopeId,
        ScopeKind, ScopeRef, SessionId, TenantId, WorkspaceId,
    };
    use uuid::Uuid;

    use super::*;

    #[test]
    fn mandatory_audit_metadata_is_complete_and_errors_are_redacted() {
        let authorization = AuthorizationContext {
            session_id: SessionId::new(Uuid::from_u128(1)),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::from_u128(2)),
                principal_kind: PrincipalKind::User,
                device_id: Some(DeviceId::new(Uuid::from_u128(3))),
                service_id: None,
            },
            tenant_id: TenantId::new(Uuid::from_u128(4)),
            workspace_id: Some(WorkspaceId::new(Uuid::from_u128(5))),
            scope: ScopeRef {
                kind: ScopeKind::parse("workspace").unwrap(),
                id: ScopeId::new(Uuid::from_u128(5)),
            },
        };
        let record = MutationAuditRecord::from_authorization(
            &authorization,
            UnixMillis(6),
            CorrelationId::new(Uuid::from_u128(7)),
            "eitmad.order.update.v1",
            AuditTarget {
                kind: "order".to_owned(),
                identifiers: vec!["order:sha256:synthetic".to_owned()],
            },
        )
        .with_outcome(
            AuditOutcome::Denied,
            Some("eitmad.error.authorization-denied.v1".to_owned()),
        );

        assert!(record.validate_complete().is_ok());
        assert_eq!(record.tenant_id, authorization.tenant_id);
        assert_eq!(record.workspace_id, authorization.workspace_id);
        assert_eq!(record.session_id, authorization.session_id);
        assert_eq!(
            record.redacted_error,
            Some(RedactedAuditError {
                code: "eitmad.error.authorization-denied.v1".to_owned(),
                class: AuditErrorClass::Authorization,
            })
        );
        assert!(!serde_json::to_string(&record).unwrap().contains("raw"));
    }

    #[test]
    fn incomplete_failure_is_rejected() {
        let mut record = MutationAuditRecord {
            audit_id: Uuid::nil(),
            occurred_at: UnixMillis(0),
            principal_id: PrincipalId::new(Uuid::nil()),
            principal_kind: PrincipalKind::Service,
            session_id: SessionId::new(Uuid::nil()),
            device_id: None,
            tenant_id: TenantId::new(Uuid::nil()),
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("tenant").unwrap(),
                id: ScopeId::new(Uuid::nil()),
            },
            correlation_id: CorrelationId::new(Uuid::nil()),
            causation_id: None,
            idempotency_key: None,
            operation: String::new(),
            target: AuditTarget {
                kind: String::new(),
                identifiers: Vec::new(),
            },
            outcome: AuditOutcome::Failed,
            previous_revision: None,
            resulting_revision: None,
            changed_identifiers: Vec::new(),
            redacted_error: None,
            extension_points: Vec::new(),
        };
        assert_eq!(record.validate_complete(), Err(AuditCompletenessError));
        record.operation = "eitmad.test.v1".to_owned();
        record.target.kind = "test".to_owned();
        assert_eq!(record.validate_complete(), Err(AuditCompletenessError));
        record.redacted_error = Some(RedactedAuditError {
            code: "raw customer error: 123".to_owned(),
            class: AuditErrorClass::Internal,
        });
        assert_eq!(record.validate_complete(), Err(AuditCompletenessError));
        record.redacted_error = Some(RedactedAuditError {
            code: "eitmad.error.test.v1".to_owned(),
            class: AuditErrorClass::Internal,
        });
        assert!(record.validate_complete().is_ok());
    }
}
